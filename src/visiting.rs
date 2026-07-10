// Copyright (c) 2026 Mariusz Zacirka
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::marker::PhantomData;
use itertools::{FoldWhile::{self, Continue, Done}, Itertools};
use paste::paste;

/// A type-level function `'j -> Item<'j>`: the family of item types a `Seq`
/// can yield, one per (per-call) lifetime `'j`. The GAT lives HERE rather than
/// on `Seq`/`DynSeq`/`Emit`, which keeps those traits object-safe while still
/// letting an item borrow from per-iteration context for exactly `'j`.
pub trait SeqGat {
    type Item<'j>;
}

/// Family of owned items: the lifetime is unused, so `Item<'j> = T`.
pub struct Owned<T>(PhantomData<fn() -> T>);
impl<T> SeqGat for Owned<T> {
    type Item<'j> = T;
}

/// Family of boxed sub-sequences, parameterized by the per-call lifetime `'j`.
/// This is the family `flatten` consumes — each item is itself a sequence that
/// may borrow context for `'j`.
pub struct Boxed<Inner>(PhantomData<fn() -> Inner>);
impl<Inner: SeqGat> SeqGat for Boxed<Inner> {
    type Item<'j> = Box<dyn DynSeq<Gat = Inner> + 'j>;
}

/// Family of `(index, item)` pairs that preserves the inner item's lifetime.
pub struct Indexed<F>(PhantomData<fn() -> F>);
impl<F: SeqGat> SeqGat for Indexed<F> {
    type Item<'j> = (usize, F::Item<'j>);
}

/// Downstream consumer of a `Seq`. `emit` is generic over the *item* lifetime
/// `'j`, so each call chooses its own `'j`. Because `'j` lives on the method
/// (not in the trait-object type), an `&mut dyn Emit<_>` does not pin `'j`,
/// which is what lets items borrowing per-call context flow through
/// `wrap`/`flatten`. A blanket impl makes every higher-ranked `FnMut` an
/// `Emit`, so callers keep passing closures.
pub trait Emit<F: SeqGat + ?Sized> {
    fn emit<'j>(&mut self, item: F::Item<'j>) -> bool;
}

impl<F: SeqGat, G> Emit<F> for G
where
    G: for<'j> FnMut(F::Item<'j>) -> bool,
{
    fn emit<'j>(&mut self, item: F::Item<'j>) -> bool {
        self(item)
    }
}

pub trait Seq<'i>: Sized {
    type ItemGat: SeqGat;

    fn for_each_while(self, op: impl for<'j> FnMut(<Self::ItemGat as SeqGat>::Item<'j>) -> bool) -> bool;

    fn for_each(self, mut op: impl for<'j> FnMut(<Self::ItemGat as SeqGat>::Item<'j>)) {
        self.for_each_while(move |item| { op(item); false });
    }

    fn fold_while<Acc>(self, acc: Acc, mut op: impl for<'j> FnMut(Acc, <Self::ItemGat as SeqGat>::Item<'j>) -> FoldWhile<Acc>) -> FoldWhile<Acc> {
        let mut opt_acc = Some(acc);
        let result = self.for_each_while(|item| {
            let r_while = op(opt_acc.take().unwrap(), item);
            let done = r_while.is_done();
            opt_acc = Some(r_while.into_inner());
            done
        });
        if result { Done(opt_acc.unwrap()) } else { Continue(opt_acc.unwrap()) }
    }

    fn fold<Acc>(self, acc: Acc, mut op: impl for<'j> FnMut(Acc, <Self::ItemGat as SeqGat>::Item<'j>) -> Acc) {
        self.fold_while(acc, move |a, item| { Continue(op(a, item)) });
    }

    #[allow(clippy::wrong_self_convention)] // consumes the sequence to box it
    fn as_dyn(self) -> Box<dyn DynSeq<Gat = Self::ItemGat> + 'i> where Self: 'i {
        Box::new(WrappingDynSeq { wrapped: self, _life_phantom: PhantomData })
    }

    fn flatten<InnerGat>(self) -> impl Seq<'i, ItemGat = InnerGat>
    where
        InnerGat: SeqGat + 'i,
        Self: Seq<'i, ItemGat = Boxed<InnerGat>> + 'i,
    {
        FlattenSeq { source: self, _out_fam_phantom: PhantomData }
    }

    fn map<OutItem, Mapper>(self, mapper: Mapper) -> impl Seq<'i, ItemGat = Owned<OutItem>>
    where
        Self: 'i,
        OutItem: 'i,
        Mapper: 'i + for<'j> FnMut(<Self::ItemGat as SeqGat>::Item<'j>) -> OutItem,
    {
        MapSeq { source: self, mapper, _out_item_phantom: PhantomData }
    }

    fn scan<State, OutItem, Mapper>(self, state: State, mapper: Mapper) -> impl Seq<'i, ItemGat = Owned<OutItem>>
    where
        Self: 'i,
        State: 'i,
        OutItem: 'i,
        Mapper: 'i + for<'j> FnMut(&mut State, <Self::ItemGat as SeqGat>::Item<'j>) -> Option<OutItem>,
    {
        ScanSeq { source: self, state, mapper, _out_item_phantom: PhantomData }
    }

    fn wrap<OutFam, Op>(self, op: Op) -> impl Seq<'i, ItemGat = OutFam>
    where
        Self: 'i,
        OutFam: SeqGat + 'i,
        Op: 'i + for<'j> FnMut(<Self::ItemGat as SeqGat>::Item<'j>, &mut dyn Emit<OutFam>) -> bool,
    {
        WrapSeq { source: self, op, _out_fam_phantom: PhantomData }
    }

    fn enumerate(self) -> impl Seq<'i, ItemGat = Indexed<Self::ItemGat>>
    where Self: 'i
    {
        let mut idx = 0;
        self.wrap(move |item, next| {
            let cur_idx = idx;
            idx += 1;
            next.emit((cur_idx, item))
        })
    }
}

pub trait DynSeq {
    type Gat: SeqGat;

    fn dyn_for_each_while(self: Box<Self>, op: &mut dyn Emit<Self::Gat>) -> bool;
}

struct WrappingDynSeq<'i, TSeq> {
    wrapped: TSeq,
    _life_phantom: PhantomData<&'i ()>,
}

impl<'i, TSeq> DynSeq for WrappingDynSeq<'i, TSeq>
where
    TSeq: Seq<'i>,
{
    type Gat = TSeq::ItemGat;

    fn dyn_for_each_while(self: Box<Self>, op: &mut dyn Emit<Self::Gat>) -> bool {
        self.wrapped.for_each_while(|item| op.emit(item))
    }
}

impl<'i, F: SeqGat> Seq<'i> for Box<dyn DynSeq<Gat = F> + 'i> {
    type ItemGat = F;

    fn for_each_while(self, mut op: impl for<'j> FnMut(F::Item<'j>) -> bool) -> bool {
        self.dyn_for_each_while(&mut op)
    }
}

struct FlattenSeq<SourceSeq, Inner> {
    source: SourceSeq,
    _out_fam_phantom: PhantomData<fn() -> Inner>,
}

impl<'i, SourceSeq, InnerGat> Seq<'i> for FlattenSeq<SourceSeq, InnerGat>
where
    SourceSeq: Seq<'i, ItemGat = Boxed<InnerGat>> + 'i,
    InnerGat: SeqGat,
{
    type ItemGat = InnerGat;

    fn for_each_while(self, mut op: impl for<'j> FnMut(InnerGat::Item<'j>) -> bool) -> bool {
        self.source.for_each_while(|boxed| boxed.for_each_while(&mut op))
    }
}

struct MapSeq<SourceSeq, Mapper, OutItem> {
    source: SourceSeq,
    mapper: Mapper,
    _out_item_phantom: PhantomData<fn() -> OutItem>,
}

impl<'i, SourceSeq, Mapper, OutItem> Seq<'i> for MapSeq<SourceSeq, Mapper, OutItem>
where
    SourceSeq: Seq<'i>,
    Mapper: for<'j> FnMut(<SourceSeq::ItemGat as SeqGat>::Item<'j>) -> OutItem,
{
    type ItemGat = Owned<OutItem>;

    fn for_each_while(mut self, mut op: impl for<'j> FnMut(OutItem) -> bool) -> bool {
        self.source.for_each_while(|in_item| op((self.mapper)(in_item)))
    }
}

struct ScanSeq<SourceSeq, State, Mapper, OutItem> {
    source: SourceSeq,
    state: State,
    mapper: Mapper,
    _out_item_phantom: PhantomData<fn() -> OutItem>,
}

impl<'i, SourceSeq, State, Mapper, OutItem> Seq<'i> for ScanSeq<SourceSeq, State, Mapper, OutItem>
where
    SourceSeq: Seq<'i>,
    Mapper: for<'j> FnMut(&mut State, <SourceSeq::ItemGat as SeqGat>::Item<'j>) -> Option<OutItem>,
{
    type ItemGat = Owned<OutItem>;

    fn for_each_while(mut self, mut op: impl for<'j> FnMut(OutItem) -> bool) -> bool {
        self.source.for_each_while(|in_item| {
            match (self.mapper)(&mut self.state, in_item) {
                Some(mapped) => op(mapped),
                None => false,
            }
        })
    }
}

struct WrapSeq<SourceSeq, Op, OutFam> {
    source: SourceSeq,
    op: Op,
    _out_fam_phantom: PhantomData<fn() -> OutFam>,
}

impl<'i, SourceSeq, Op, OutFam> Seq<'i> for WrapSeq<SourceSeq, Op, OutFam>
where
    SourceSeq: Seq<'i>,
    OutFam: SeqGat,
    Op: for<'j> FnMut(<SourceSeq::ItemGat as SeqGat>::Item<'j>, &mut dyn Emit<OutFam>) -> bool,
{
    type ItemGat = OutFam;

    fn for_each_while(mut self, mut next_op: impl for<'j> FnMut(OutFam::Item<'j>) -> bool) -> bool {
        self.source.for_each_while(|item| (self.op)(item, &mut next_op))
    }
}

struct EnumerateSeq<SourceSeq> {
    source: SourceSeq,
}

impl<'i, SourceSeq> Seq<'i> for EnumerateSeq<SourceSeq>
where
    SourceSeq: Seq<'i>,
{
    type ItemGat = Indexed<SourceSeq::ItemGat>;

    fn for_each_while(self, mut op: impl for<'j> FnMut((usize, <SourceSeq::ItemGat as SeqGat>::Item<'j>)) -> bool) -> bool {
        let mut idx = 0;
        self.source.for_each_while(|item| {
            let i = idx;
            idx += 1;
            op((i, item))
        })
    }
}

pub trait IntoSeq<TItem> {
    fn into_seq<'i>(self) -> impl Seq<'i, ItemGat = Owned<TItem>>;
}

impl<TItem> IntoSeq<TItem> for Vec<TItem> {
    fn into_seq<'i>(self) -> impl Seq<'i, ItemGat = Owned<TItem>> {
        VecSeq { source: self }
    }
}

struct VecSeq<TItem> {
    source: Vec<TItem>,
}

impl<'i, TItem> Seq<'i> for VecSeq<TItem> {
    type ItemGat = Owned<TItem>;

    fn for_each_while(self, mut op: impl for<'j> FnMut(TItem) -> bool) -> bool {
        self.source
            .into_iter()
            .fold_while((), |acc, item| {
                if op(item) { Done(acc) } else { Continue(acc) }
            })
            .is_done()
    }
}

#[macro_export]
macro_rules! def_seq_gat_for_multi_trait {
    ($trait_ty:ident) => {
        ::paste::paste! {
            struct [<$trait_ty ඞGat>];
            impl super::SeqGat for [<$trait_ty ඞGat>] {
                type Item<'j> = Box<dyn $trait_ty + 'j>;
            }
        }
    };
}

#[macro_export]
macro_rules! seq_gat_for_multi_trait {
    ($head:ident $(::$tail:ident)+) => { $head :: $crate::seq_gat_for_multi_trait!($tail) };
    ($trait_ty:ident) => { ::paste::paste!{[<$trait_ty ඞGat>]} };
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use super::{Boxed, DynSeq, Emit, IntoSeq, Owned, Seq, SeqGat};

    #[test]
    fn map_vec_with_external_closure() {
        let d = 3;
        let v = vec![1, 2, 3, 4];
        let c = |a: i32| { a + d };
        let mut out = vec![];
        v.into_seq()
            .map(c)
            .for_each(|item| { out.push(item); });
        assert_eq!(out, [4, 5, 6, 7]);
    }

    #[test]
    fn map_vec_of_closures_with_dyn_mapped_closures() {
        {
            struct St {
                a: i32
            }

            impl St {
                fn test(&self) -> i32 {
                    self.a
                }
            }

            let s = St { a: 3 };
        }
        let ctx = (1, "a");
        let v: Vec<Box<dyn FnMut((i32, &str)) -> Box<dyn DynSeq<Gat = Owned<(i32, String)>> + '_>>> = vec![
            Box::new(|(i, s)| {
                let v = vec![10, 20, 30];
                v.into_seq()
                    .map(move |item| (item + i, s.to_owned() + "1"))
                    .as_dyn()
            }),
            Box::new(|(i, s)| {
                let v = vec!["a", "b", "c"];
                v.into_seq()
                    .map(move |item| (i, s.to_owned() + "_" + item))
                    .as_dyn()
            }),
        ];
        let mut out = vec![];
        v.into_seq()
            // each closure produces a boxed sub-sequence borrowing the call;
            // `wrap` emits it as a `Boxed<_>` item, then `flatten` concatenates.
            .wrap(|mut f, next: &mut dyn Emit<Boxed<Owned<(i32, String)>>>| {
                next.emit(f(ctx))
            })
            .flatten()
            .enumerate()
            .for_each(|(idx, (i, s))| {
                out.push((idx, i, s));
            });
        assert_eq!(out, [
            (0, 11, "a1".to_owned()),
            (1, 21, "a1".to_owned()),
            (2, 31, "a1".to_owned()),
            (3, 1, "a_a".to_owned()),
            (4, 1, "a_b".to_owned()),
            (5, 1, "a_c".to_owned()),
        ]);
    }

    struct Ctx {}

    trait ContextProvided {
        type Context;
    }

    pub trait Service {
        fn id(&self) -> i32;
    }

    def_seq_gat_for_multi_trait!(Service);
    

    // mod gat {
    //     pub struct Service;
    //     impl super::SeqGat for Service {
    //         type Item<'j> = Box<dyn super::Service + 'j>;
    //     }
    // }
    
    pub trait Service2 {
        fn id2(&self) -> i32;
    }
    //trace_macros!(true);
    def_seq_gat_for_multi_trait!(Service2);

    fn into_services<'p, NewContextProvided>(mut ctx_provided: NewContextProvided)
        -> impl Seq<'p, ItemGat = seq_gat_for_multi_trait!(Service)> + use<'p, NewContextProvided>
        where
            NewContextProvided: ContextProvided<Context = Ctx> + 'p
    {
        use ::std::{vec::Vec, vec, boxed::Box};

        // trait GetLocalImpl {
        //     fn call<'l>(&mut self, item: )
        // }

        let local_impls:
            Vec<
                Box<
                    dyn for<'a> Fn(&'a mut NewContextProvided) -> Box<dyn Service + 'a>
                >
            >
            = vec![];
        let mut other_impls:
            Vec<
                Box<
                    dyn for<'a> FnOnce(&'a mut NewContextProvided) -> Box<dyn DynSeq<Gat = seq_gat_for_multi_trait!(Service)> + 'a>
                >
            >
            = vec![];
        //$($impls)*
        other_impls.push(
            Box::new(
                move |ncp: &mut NewContextProvided| {
                    local_impls
                        .into_seq()
                        // each local impl borrows `ncp` for exactly this emit;
                        // `wrap` keeps the borrow scoped while pushing downstream.
                        .wrap(|c, next: &mut dyn Emit<seq_gat_for_multi_trait!(Service)>| {
                            let service = (c)(ncp);
                            next.emit(service)
                        })
                        .as_dyn()
                }
            )
        );
        other_impls
            .into_seq()
            .wrap(
                move |c, next: &mut dyn Emit<Boxed<seq_gat_for_multi_trait!(Service)>>| {
                    let subseq = (c)(&mut ctx_provided);
                    next.emit(subseq)
                }
            )
            .flatten()
    }
    //trace_macros!(false);

    #[test]
    fn into_services_yields_borrowing_services() {
        struct Provider { calls: i32 }
        impl ContextProvided for Provider { type Context = Ctx; }

        struct Counter<'a> { provider: &'a mut Provider }
        impl<'a> Service for Counter<'a> {
            fn id(&self) -> i32 { self.provider.calls }
        }

        // build a provider-backed group of two services that each borrow it
        let provider = Provider { calls: 7 };
        let services = {
            // reuse `into_services`' machinery directly to prove the borrow flows
            use super::{IntoSeq, Seq, Emit, Boxed};
            let local: Vec<Box<dyn for<'a> Fn(&'a mut Provider) -> Box<dyn Service + 'a>>> = vec![
                Box::new(|p| Box::new(Counter { provider: p })),
                Box::new(|p| Box::new(Counter { provider: p })),
            ];
            let mut groups: Vec<Box<dyn for<'a> FnOnce(&'a mut Provider) -> Box<dyn DynSeq<Gat = seq_gat_for_multi_trait!(Service)> + 'a>>> = vec![];
            groups.push(Box::new(move |ncp: &mut Provider| {
                local.into_seq()
                    .wrap(|c, next: &mut dyn Emit<seq_gat_for_multi_trait!(Service)>| next.emit((c)(ncp)))
                    .as_dyn()
            }));
            let mut provider = provider;
            let mut ids = vec![];
            groups.into_seq()
                .wrap(move |c, next: &mut dyn Emit<Boxed<seq_gat_for_multi_trait!(Service)>>| next.emit((c)(&mut provider)))
                .flatten()
                .for_each(|svc| ids.push(svc.id()));
            ids
        };
        assert_eq!(services, [7, 7]);

        // `into_services` itself must compile and produce an (empty) sequence
        let mut count = 0;
        into_services(Provider { calls: 0 }).for_each(|_svc| count += 1);
        assert_eq!(count, 0);
    }
}
