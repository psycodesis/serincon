// Copyright (c) 2026 Mariusz Zacirka
// 
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{any::Any, marker::PhantomData};
use demuncher::*;
use itertools::{FoldWhile::{self, Continue, Done}, Itertools, WhileSome};
use crate::visiting::{Boxed, DynSeq, Emit, IntoSeq, Seq, SeqGat};

pub trait ServiceProvider<'c, ServiceProvided: ?Sized + 'c> {
    fn get_service(&'c self) -> Box<ServiceProvided>;
}

pub trait MutServiceProvider<'c, ServiceProvided: ?Sized + 'c> {
    fn get_mut_service(&'c mut self) -> Box<ServiceProvided>;
}

pub trait MultipleServiceProvider<'c, ServiceProvidedGat: SeqGat> {
    fn get_services(&'c self) -> impl Seq<'c, ItemGat = ServiceProvidedGat>;
    fn into_services(self) -> impl Seq<'c, ItemGat = ServiceProvidedGat>;
}

pub trait MultipleMutServiceProvider<'c, ServiceProvidedGat: SeqGat> {
    fn get_mut_services(&'c mut self) -> impl Seq<'c, ItemGat = ServiceProvidedGat>;
    fn into_mut_services(self) -> impl Seq<'c, ItemGat = ServiceProvidedGat>;
}

pub trait ServiceProvidedData<ServiceData> {
    fn data(&self) -> &ServiceData;
}

pub trait ServiceProvidedServices<TContextProvided>
where
    TContextProvided: ContextProvided
{
    fn get_service<'c, ContextServiceProvided: ?Sized + 'c>(&'c self) -> Box<ContextServiceProvided>
        where TContextProvided: ServiceProvider<'c, ContextServiceProvided>;
    fn get_services<'c, ContextServiceProvidedGat: SeqGat>(&'c self) -> impl Seq<'c, ItemGat = ContextServiceProvidedGat>
        where TContextProvided: MultipleServiceProvider<'c, ContextServiceProvidedGat>;
}

pub trait MutServiceProvidedData<ServiceProvided>: ServiceProvidedData<ServiceProvided> {
    fn mut_data(&mut self) -> &mut ServiceProvided;
}

pub trait MutServiceProvidedServices<TContextProvided>: ServiceProvidedServices<TContextProvided>
where
    TContextProvided: MutContextProvided
{
    fn get_mut_service<'c, ContextServiceProvided: ?Sized + 'c>(&'c mut self) -> Box<ContextServiceProvided>
        where TContextProvided: MutServiceProvider<'c, ContextServiceProvided>;
    fn get_mut_services<'c, ContextServiceProvidedGat: SeqGat>(&'c mut self) -> impl Seq<'c, ItemGat = ContextServiceProvidedGat>
        where TContextProvided: MultipleMutServiceProvider<'c, ContextServiceProvidedGat>;
}

pub struct ServiceProvidedWithContext<ServiceData, TContextProvided, DataGetter>
where
    TContextProvided: ContextProvided,
    DataGetter: Fn(&TContextProvided::Context) -> &ServiceData
{
    data_getter: DataGetter,
    ctx_provided: TContextProvided,
    _service_phantom: PhantomData<ServiceData>
}

impl<ServiceData, TContextProvided, DataGetter> ServiceProvidedData<ServiceData>
for ServiceProvidedWithContext<ServiceData, TContextProvided, DataGetter>
where
    TContextProvided: ContextProvided,
    DataGetter: Fn(&TContextProvided::Context) -> &ServiceData
{
    fn data(&self) -> &ServiceData {
        (self.data_getter)(self.ctx_provided.ctx())
    }
}

impl<ServiceProvided, TContextProvided, DataGetter> ServiceProvidedServices<TContextProvided>
for ServiceProvidedWithContext<ServiceProvided, TContextProvided, DataGetter>
where
    TContextProvided: ContextProvided,
    DataGetter: Fn(&TContextProvided::Context) -> &ServiceProvided
{
    fn get_service<'c, ContextServiceProvided>(&'c self) -> Box<ContextServiceProvided>
    where
        TContextProvided: ServiceProvider<'c, ContextServiceProvided>,
        ContextServiceProvided: ?Sized + 'c
    {
        self.ctx_provided.get_service()
    }

    fn get_services<'c, ContextServiceProvidedGat>(&'c self) -> impl Seq<'c, ItemGat = ContextServiceProvidedGat>
    where
        TContextProvided: MultipleServiceProvider<'c, ContextServiceProvidedGat>,
        ContextServiceProvidedGat: SeqGat
    {
        self.ctx_provided.get_services()
    }
}

pub struct MutServiceProvidedWithContext<ServiceData, TContextProvided, MutDataGetter, DataGetter>
where
    TContextProvided: MutContextProvided,
    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut ServiceData,
    DataGetter: Fn(&TContextProvided::Context) -> &ServiceData
{
    data_getter: DataGetter,
    mut_data_getter: MutDataGetter,
    ctx_provided: TContextProvided,
    _service_phantom: PhantomData<ServiceData>
}

impl<ServiceData, TContextProvided, MutDataGetter, DataGetter> ServiceProvidedData<ServiceData>
for MutServiceProvidedWithContext<ServiceData, TContextProvided, MutDataGetter, DataGetter>
where
    TContextProvided: MutContextProvided,
    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut ServiceData,
    DataGetter: Fn(&TContextProvided::Context) -> &ServiceData
{
    fn data(&self) -> &ServiceData {
        (self.data_getter)(self.ctx_provided.ctx())
    }
}

impl<'c, ServiceData, TContextProvided, MutDataGetter, DataGetter> MutServiceProvidedData<ServiceData>
for MutServiceProvidedWithContext<ServiceData, TContextProvided, MutDataGetter, DataGetter>
where
    TContextProvided: MutContextProvided + 'c,
    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut ServiceData,
    DataGetter: Fn(&TContextProvided::Context) -> &ServiceData
{
    fn mut_data(&mut self) -> &mut ServiceData {
        (self.mut_data_getter)(self.ctx_provided.mut_ctx())
    }
}

impl<ServiceProvided, TContextProvided, MutDataGetter, DataGetter> ServiceProvidedServices<TContextProvided>
for MutServiceProvidedWithContext<ServiceProvided, TContextProvided, MutDataGetter, DataGetter>
where
    TContextProvided: MutContextProvided,
    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut ServiceProvided,
    DataGetter: Fn(&TContextProvided::Context) -> &ServiceProvided
{
    fn get_service<'c, ContextServiceProvided>(&'c self) -> Box<ContextServiceProvided>
    where
        TContextProvided: ServiceProvider<'c, ContextServiceProvided>,
        ContextServiceProvided: ?Sized + 'c
    {
        self.ctx_provided.get_service()
    }

    fn get_services<'c, ContextServiceProvidedGat>(&'c self) -> impl Seq<'c, ItemGat = ContextServiceProvidedGat>
    where
        TContextProvided: MultipleServiceProvider<'c, ContextServiceProvidedGat>,
        ContextServiceProvidedGat: SeqGat
    {
        self.ctx_provided.get_services()
    }
}

impl<ServiceProvided, TContextProvided, MutDataGetter, DataGetter> MutServiceProvidedServices<TContextProvided>
for MutServiceProvidedWithContext<ServiceProvided, TContextProvided, MutDataGetter, DataGetter>
where
    TContextProvided: MutContextProvided,
    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut ServiceProvided,
    DataGetter: Fn(&TContextProvided::Context) -> &ServiceProvided
{
    fn get_mut_service<'c, ContextServiceProvided: ?Sized + 'c>(&'c mut self) -> Box<ContextServiceProvided>
    where
        TContextProvided: MutServiceProvider<'c, ContextServiceProvided>
    {
        self.ctx_provided.get_mut_service()
    }
    
    fn get_mut_services<'c, ContextServiceProvidedGat>(&'c mut self) -> impl Seq<'c, ItemGat = ContextServiceProvidedGat>
    where
        TContextProvided: MultipleMutServiceProvider<'c, ContextServiceProvidedGat>,
        ContextServiceProvidedGat: SeqGat
    {
        self.ctx_provided.get_mut_services()
    }
}

pub trait ContextProvided {
    type Context;
    fn ctx(&self) -> &Self::Context;
}

pub trait MutContextProvided: ContextProvided {
    fn mut_ctx(&mut self) -> &mut Self::Context;
}

pub trait ContextProvidedWithParent: ContextProvided {
    type ParentContextProvided: ContextProvided;
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided;
}

pub trait MutContextProvidedWithParent: MutContextProvided + ContextProvidedWithParent {
    type ParentMutContextProvided: MutContextProvided;
    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided;
}

pub struct StartContextProvided<'c, Context> {
    start_ctx: &'c Context
}

impl<'c, TContext> ContextProvided for StartContextProvided<'c, TContext>
{
    type Context = TContext;

    fn ctx(&self) -> &TContext {
        self.start_ctx
    }
}

pub struct StartMutContextProvided<'c, Context> {
    start_ctx: &'c mut Context
}

impl<'c, Context> ContextProvided for StartMutContextProvided<'c, Context> {
    type Context = Context;

    fn ctx(&self) -> &Context {
        self.start_ctx
    }
}

impl<'c, Context> MutContextProvided for StartMutContextProvided<'c, Context> {
    fn mut_ctx(&mut self) -> &mut Context {
        self.start_ctx
    }
}

pub struct ForwardingContextProvided<'c, ForwardedContextProvided>
where
    ForwardedContextProvided: ContextProvided
{
    forwarded_ctx: &'c ForwardedContextProvided
}

impl<'c, ForwardedContextProvided> ContextProvided for ForwardingContextProvided<'c, ForwardedContextProvided>
where 
    ForwardedContextProvided: ContextProvided
{
    type Context = ForwardedContextProvided::Context;

    fn ctx(&self) -> &'c Self::Context {
        self.forwarded_ctx.ctx()
    }
}

impl<'c, ForwardedContextProvided> ContextProvidedWithParent for ForwardingContextProvided<'c, ForwardedContextProvided>
where 
    ForwardedContextProvided: ContextProvidedWithParent
{
    type ParentContextProvided = ForwardedContextProvided::ParentContextProvided;
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
        self.forwarded_ctx.parent_ctx_provided()
    }
}

pub struct ForwardingMutContextProvided<'c, ForwardedContextProvided>
where
    ForwardedContextProvided: MutContextProvided
{
    forwarded_ctx: &'c mut ForwardedContextProvided
}

impl<'c, ForwardedContextProvided> ContextProvided for ForwardingMutContextProvided<'c, ForwardedContextProvided>
where 
    ForwardedContextProvided: MutContextProvided
{
    type Context = ForwardedContextProvided::Context;

    fn ctx(&self) -> &Self::Context {
        self.forwarded_ctx.ctx()
    }
}

impl<'c, ForwardedContextProvided> MutContextProvided for ForwardingMutContextProvided<'c, ForwardedContextProvided>
where 
    ForwardedContextProvided: MutContextProvided
{
    fn mut_ctx(&mut self) -> &mut Self::Context {
        self.forwarded_ctx.mut_ctx()
    }
}

impl<'c, ForwardedContextProvided> ContextProvidedWithParent for ForwardingMutContextProvided<'c, ForwardedContextProvided>
where 
    ForwardedContextProvided: MutContextProvidedWithParent
{
    type ParentContextProvided = ForwardedContextProvided::ParentContextProvided;
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
        self.forwarded_ctx.parent_ctx_provided()
    }
}

impl<'c, ForwardedContextProvided> MutContextProvidedWithParent for ForwardingMutContextProvided<'c, ForwardedContextProvided>
where 
    ForwardedContextProvided: MutContextProvidedWithParent
{
    type ParentMutContextProvided = ForwardedContextProvided::ParentMutContextProvided;

    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided {
        self.forwarded_ctx.parent_mut_ctx_provided()
    }
}

pub struct SubContextProvidedWithParent<ParentContextProvided, SubContext, ContextGetter>
where
    ParentContextProvided: ContextProvided,
    ContextGetter: Fn(&ParentContextProvided::Context) -> &SubContext
{
    ctx_getter: ContextGetter,
    parent_ctx_provided: ParentContextProvided,
    _sub_context_phantom: PhantomData<SubContext>
}

impl<SubContext, ParentContextProvided, ContextGetter> ContextProvided
for SubContextProvidedWithParent<ParentContextProvided, SubContext, ContextGetter>
where
    ParentContextProvided: ContextProvided,
    ContextGetter: Fn(&ParentContextProvided::Context) -> &SubContext
{
    type Context = SubContext;

    fn ctx(&self) -> &SubContext {
        (self.ctx_getter)(self.parent_ctx_provided.ctx())
    }
}

impl<SubContext, ParentContextProvided, ContextGetter> ContextProvidedWithParent
for SubContextProvidedWithParent<ParentContextProvided, SubContext, ContextGetter>
where
    ParentContextProvided: ContextProvided,
    ContextGetter: Fn(&ParentContextProvided::Context) -> &SubContext
{
    type ParentContextProvided = ParentContextProvided;
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
        &self.parent_ctx_provided
    }
}

pub struct MutSubContextProvidedWithParent<ParentContextProvided, SubContext, ContextGetter, MutContextGetter>
where
    ParentContextProvided: MutContextProvided,
    ContextGetter: Fn(&ParentContextProvided::Context) -> &SubContext,
    MutContextGetter: Fn(&mut ParentContextProvided::Context) -> &mut SubContext
{
    ctx_getter: ContextGetter,
    mut_ctx_getter: MutContextGetter,
    parent_ctx_provided: ParentContextProvided,
    _sub_context_phantom: PhantomData<SubContext>
}

impl<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter> MutContextProvided
for MutSubContextProvidedWithParent<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter>
where
    ParentMutContextProvided: MutContextProvided,
    ContextGetter: Fn(&ParentMutContextProvided::Context) -> &SubContext,
    MutContextGetter: Fn(&mut ParentMutContextProvided::Context) -> &mut SubContext
{
    fn mut_ctx(&mut self) -> &mut SubContext {
        (self.mut_ctx_getter)(self.parent_ctx_provided.mut_ctx())
    }
}

impl<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter> ContextProvided
for MutSubContextProvidedWithParent<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter>
where
    ParentMutContextProvided: MutContextProvided,
    ContextGetter: Fn(&ParentMutContextProvided::Context) -> &SubContext,
    MutContextGetter: Fn(&mut ParentMutContextProvided::Context) -> &mut SubContext
{
    type Context = SubContext;

    fn ctx(&self) -> &SubContext {
        (self.ctx_getter)(self.parent_ctx_provided.ctx())
    }
}

impl<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter> ContextProvidedWithParent
for MutSubContextProvidedWithParent<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter>
where
    ParentMutContextProvided: MutContextProvided,
    ContextGetter: Fn(&ParentMutContextProvided::Context) -> &SubContext,
    MutContextGetter: Fn(&mut ParentMutContextProvided::Context) -> &mut SubContext
{
    type ParentContextProvided = ParentMutContextProvided;
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
        &self.parent_ctx_provided
    }
}

impl<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter> MutContextProvidedWithParent
for MutSubContextProvidedWithParent<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter>
where
    ParentMutContextProvided: MutContextProvided,
    ContextGetter: Fn(&ParentMutContextProvided::Context) -> &SubContext,
    MutContextGetter: Fn(&mut ParentMutContextProvided::Context) -> &mut SubContext
{
    type ParentMutContextProvided = ParentMutContextProvided;

    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided {
        &mut self.parent_ctx_provided
    }
}

// pub trait ContextServiceProvider<'c, ServiceProvided: ?Sized + 'c> {
//     // fn get_service(ctx_provided: &'c TContextProvided) -> Box<ServiceProvided>;
//     fn into_service<NewContextProvided>(new_ctx_provided: NewContextProvided) -> Box<ServiceProvided>
//         where
//             NewContextProvided: ContextProvided<Context = Self> + 'c;
// }

// impl<'c, TContextProvided, ServiceProvided> ServiceProvider<'c, ServiceProvided>
// for TContextProvided
// where
//     TContextProvided: ContextProvided,
//     TContextProvided::Context: ContextServiceProvider<'c, ServiceProvided>,
//     ServiceProvided: ?Sized + 'c
// {
//     fn get_service(&'c self) -> Box<ServiceProvided> {
//         TContextProvided::Context::into_service(
//             ForwardingContextProvided {
//                 forwarded_ctx: self
//             }
//         )
//     }
// }

// pub trait ContextMutServiceProvider<'c, ServiceProvided: ?Sized + 'c> {
//     // fn get_mut_service(ctx_provided: &'c mut TContextProvided) -> Box<ServiceProvided>;
//     fn into_mut_service<NewContextProvided>(new_ctx_provided: NewContextProvided) -> Box<ServiceProvided>
//         where
//             NewContextProvided: MutContextProvided<Context = Self> + 'c;
// }

// impl<'c, TContextProvided, ServiceProvided> MutServiceProvider<'c, ServiceProvided>
// for TContextProvided
// where
//     TContextProvided: MutContextProvided,
//     TContextProvided::Context: ContextMutServiceProvider<'c, ServiceProvided>,
//     ServiceProvided: ?Sized + 'c
// {
//     fn get_mut_service(&'c mut self) -> Box<ServiceProvided> {
//         TContextProvided::Context::into_mut_service(
//             ForwardingMutContextProvided {
//                 forwarded_ctx: self
//             }
//         )
//     }
// }

// pub trait ContextMultipleServiceProvider<'p, ServiceProvidedGat: SeqGat> {
//     // fn get_services(ctx_provided: &'c TContextProvided) -> Vec<Box<ServiceProvided>>;
//     fn into_services<NewContextProvided>(new_ctx_provided: NewContextProvided)
//         -> impl Seq<'p, ItemGat = ServiceProvidedGat>
//         where NewContextProvided: ContextProvided<Context = Self> + Sized + 'p;
// }

// impl<'c, TContextProvided, ServiceProvidedGat> MultipleServiceProvider<'c, ServiceProvidedGat>
// for TContextProvided
// where
//     TContextProvided: ContextProvided,
//     TContextProvided::Context: ContextMultipleServiceProvider<'c, ServiceProvidedGat>,
//     ServiceProvidedGat: SeqGat
// {
//     fn get_services(&'c self) -> impl Seq<'c, ItemGat = ServiceProvidedGat> {
//         TContextProvided::Context::into_services(
//             ForwardingContextProvided {
//                 forwarded_ctx: self
//             }
//         )
//     }
// }

// pub trait ContextMultipleMutServiceProvider<TContextProvided: MutContextProvided, ServiceProvided: ?Sized> {
//     fn get_mut_services(ctx_provided: &mut TContextProvided) -> Vec<Box<ServiceProvided>>;
// }

// impl<'c, TContextProvided, ServiceProvided> MultipleMutServiceProvider<'c, ServiceProvided>
// for TContextProvided
// where
//     TContextProvided: MutContextProvided,
//     TContextProvided::Context: ContextMultipleMutServiceProvider<TContextProvided, ServiceProvided>,
//     ServiceProvided: ?Sized + 'c
// {
//     fn get_mut_services(&'c mut self) -> Vec<Box<ServiceProvided>> {
//         TContextProvided::Context::get_mut_services(self)
//     }
// }

#[macro_export]
macro_rules! service_context {
    (
        $(#[$ctx_meta:meta])*
        $vis:vis $ctx:ident { $($impls:tt)* }
        $(traits { $($traits:tt)* })?
    ) => {
        $(#[$ctx_meta])*
        $vis struct $ctx {
            services: ::demuncher::apply_pipe!{
                {$($impls)*}
                => $crate::__take_impls_list{
                    ::demuncher::when{ { $crate::__is_impl{} } => {} else {{}} }
                }
                => $crate::__as_hlist{}
            },
            sub_contexts: ::demuncher::apply_pipe!{
                {$($impls)*}
                => $crate::__take_impls_list{
                    ::demuncher::when{ { $crate::__is_sub{} } => { ::demuncher::tail{} } else {{}} }
                }
                => $crate::__as_hlist{}
            }
        }

        ::paste::paste!{
            $vis trait [<$ctx ඞProvidedTrait>]: ContextProvided<Context = $ctx> {
                type ContextProvided: ContextProvided<Context = $ctx>;
            }

            $vis trait [<$ctx ඞMutProvidedTrait>]: [<$ctx ඞProvidedTrait>] + MutContextProvided<Context = $ctx> {
            }

            $vis trait [<$ctx ඞIntoServiceProvider>]<'c, ServiceProvided: ?Sized + 'c>
            {
                fn into_service<NewContextProvided>(new_ctx_provided: [<$ctx ඞProvided>]<NewContextProvided>) -> Box<ServiceProvided>
                    where NewContextProvided: ContextProvided<Context = $ctx> + 'c;
            }

            $vis trait [<$ctx ඞIntoMutServiceProvider>]<'c, ServiceProvided: ?Sized + 'c>
            {
                fn into_mut_service<NewContextProvided>(new_ctx_provided: [<$ctx ඞMutProvided>]<NewContextProvided>) -> Box<ServiceProvided>
                    where NewContextProvided: MutContextProvided<Context = $ctx> + 'c;
            }

            $vis struct [<$ctx ඞProvided>] <TContextProvided> (TContextProvided)
            where
                TContextProvided: ContextProvided<Context = $ctx>;

            // impl<TContextProvided> [<$ctx ඞProvidedTrait>] for [<$ctx ඞProvided>]<TContextProvided>
            // where
            //     TContextProvided: ContextProvided<Context = $ctx>
            // {
            //     type ContextProvided = TContextProvided;
            // }

            impl<TContextProvided> ContextProvided for [<$ctx ඞProvided>] <TContextProvided>
            where
                TContextProvided: ContextProvided<Context = $ctx>
            {
                type Context = $ctx;

                fn ctx(&self) -> &Self::Context {
                    self.0.ctx()
                }
            }

            $vis struct [<$ctx ඞMutProvided>] <TContextProvided> (TContextProvided)
            where
                TContextProvided: MutContextProvided<Context = $ctx>;

            // impl<TContextProvided> [<$ctx ඞProvidedTrait>] for [<$ctx ඞMutProvided>]<TContextProvided>
            // where
            //     TContextProvided: MutContextProvided<Context = $ctx>
            // {
            //     type ContextProvided = TContextProvided;
            // }

            // impl<TContextProvided> [<$ctx ඞMutProvidedTrait>] for [<$ctx ඞMutProvided>]<TContextProvided>
            // where
            //     TContextProvided: MutContextProvided<Context = $ctx>
            // {
            // }

            impl<TContextProvided> ContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
            where
                TContextProvided: MutContextProvided<Context = $ctx>
            {
                type Context = $ctx;

                fn ctx(&self) -> &Self::Context {
                    self.0.ctx()
                }
            }

            impl<TContextProvided> MutContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
            where
                TContextProvided: MutContextProvided<Context = $ctx>
            {
                fn mut_ctx(&mut self) -> &mut Self::Context {
                    self.0.mut_ctx()
                }
            }

            impl $ctx {
                pub fn provide<'c: 'p, 'p>(&'c self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx>> {
                    [<$ctx ඞProvided>](
                        StartContextProvided {
                            start_ctx: self
                        }
                    )
                }

                pub fn provide_mut<'c: 'p, 'p>(&'c mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx>> {
                    [<$ctx ඞMutProvided>](
                        StartMutContextProvided {
                            start_ctx: self
                        }
                    )
                }

            }

            ::demuncher::apply_pipe!{
                {$($impls)*}
                => $crate::__for_each_impl_with_trait{
                    {} => {
                        $crate::__pick_trait_impl{
                            ctx: $ctx,
                            ctx_prd_ty: [<$ctx ඞProvided>],
                            ctx_mut_prd_ty: [<$ctx ඞMutProvided>],
                            ctx_prd_tr: [<$ctx ඞProvidedTrait>],
                            ctx_mut_prd_tr: [<$ctx ඞMutProvidedTrait>],
                            ctx_into_ty: [<$ctx ඞIntoServiceProvider>],
                            ctx_mut_into_ty: [<$ctx ඞIntoMutServiceProvider>]
                        }
                    }
                }
            }

            $(
                ::demuncher::apply_pipe!{
                    {$($traits)*}
                    => $crate::__for_each_trait_statement{
                        $crate::__emit_trait_with_single_or_multiple_impls{
                            ctx: $ctx,
                            ctx_prd_ty: [<$ctx ඞProvided>],
                            ctx_mut_prd_ty: [<$ctx ඞMutProvided>],
                            ctx_prd_tr: [<$ctx ඞProvidedTrait>],
                            ctx_mut_prd_tr: [<$ctx ඞMutProvidedTrait>],
                            ctx_into_ty: [<$ctx ඞIntoServiceProvider>],
                            ctx_mut_into_ty: [<$ctx ඞIntoMutServiceProvider>]
                        }
                    }
                }
            )?
        }
    };
}

#[macro_export]
macro_rules! impl_service {
    // Entry point with dependency list.
    ( $impl_ty:path : $($trait_head:ident)?$(::$trait_tail:ident)* [ $($trait_list:tt)* ] { $($def:tt)* } ) => {
        ::demuncher::apply_pipe!{
            { $($trait_list)* }
            => ::demuncher::split_by{,}
            => [
                ::demuncher::debrace{}
                => $crate::__emit_dependant_trait_constraint{}
                => ::demuncher::embrace{}
            ]
            => ::demuncher::join_with{+}
            => $crate::__emit_impl_service{ impl: $impl_ty, trait: $($trait_head)?$(::$trait_tail)*, def: { $($def)* } }
        }
    };
    // Entry point with no dependencies.
    ( $impl_ty:path : $($trait_head:ident)?$(::$trait_tail:ident)* { $($def:tt)* } ) => {
        impl<'c, TContextProvided, DataGetter> $($trait_head)?$(::$trait_tail)*
            for ServiceProvidedWithContext<$impl_ty, TContextProvided, DataGetter>
        where
            TContextProvided: ContextProvided,
            DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
        { $($def)* }
    };
}

#[macro_export]
macro_rules! impl_mut_service {
    // Entry point with dependency list.
    ( $impl_ty:path : $($trait_head:ident)?$(::$trait_tail:ident)* [ $($trait_list:tt)* ] { $($def:tt)* } ) => {
        ::demuncher::apply_pipe!{
            { $($trait_list)* }
            => ::demuncher::split_by{,}
            => [
                ::demuncher::debrace{}
                => $crate::__emit_dependant_trait_constraint{}
                => ::demuncher::embrace{}
            ]
            => ::demuncher::join_with{+}
            => $crate::__emit_impl_mut_service{ impl: $impl_ty, trait: $($trait_head)?$(::$trait_tail)*, def: { $($def)* } }
        }
    };
    // Entry point with no dependencies.
    ( $impl_ty:path : $($trait_head:ident)?$(::$trait_tail:ident)* { $($def:tt)* } ) => {
        impl<'c, Context, DataGetter> $($trait_head)?$(::$trait_tail)*
            for ServiceProvidedWithContext<'c, $impl_ty, Context, DataGetter>
        where
            DataGetter: Fn(&Context) -> &$impl_ty,
        { $($def)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __as_hlist {
    (
        { $($input:tt)* } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! { { ::frunk::HList!{ $($input)* } } => $($cont_args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __for_each_impl_statement {
    (
        { $($input:tt)* } => { /* { { {impl} {traits,..} }.. } => */ $($($pipe:tt)+)? } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { $($input)* }
            => ::demuncher::split_by{;}
            => ::demuncher::skip_all_empty{}
            => [
                ::demuncher::debrace{}
                => ::demuncher::split_by{:}
                $(
                    => $($pipe)+
                )?
            ]
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __is_sub {
    (
        { sub $impl:ty } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {true} => $($cont_args)* }
    };
    (
        { $($_ignored:tt)* } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {false} => $($cont_args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __is_impl {
    (
        { sub $impl:ty } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {false} => $($cont_args)* }
    };
    (
        { extern } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {false} => $($cont_args)* }
    };
    (
        { $impl:ty } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {true} => $($cont_args)* }
    };
    (
        { $($_ignored:tt)* } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {false} => $($cont_args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __is_extern {
    (
        { extern } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {true} => $($cont_args)* }
    };
    (
        { $($_ignored:tt)* } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {false} => $($cont_args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __take_impls_list {
    (
        { $($input:tt)* } => { /* {impl} => */ $($pipe:tt)+ } => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{
            { $($input)* }
            => $crate::__for_each_impl_statement{
                ::demuncher::fork{
                    { $($pipe)+ }
                    {}
                }
            }
            => ::demuncher::skip_all_empty{}
            => ::demuncher::join_with{,}
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __pick_trait_impl {
    (
        { {extern} {$($trait_ty:tt)+} } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { {$($trait_ty)+} } => $crate::__emit_trait_from_extern_context{
                ctx: $ctx,
                ctx_prd_ty: $ctx_prd_ty,
                ctx_mut_prd_ty: $ctx_mut_prd_ty,
                ctx_prd_tr: $ctx_prd_tr,
                ctx_mut_prd_tr: $ctx_mut_prd_tr,
                ctx_into_ty: $ctx_into_ty,
                ctx_mut_into_ty: $ctx_mut_into_ty
            } => $($cont_args)*
        }
    };
    (
        { {sub $impl_ty:ty} {$($trait_ty:tt)+} } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { {$impl_ty} {$($trait_ty)+} } => $crate::__emit_trait_from_sub_context{
                ctx: $ctx,
                ctx_prd_ty: $ctx_prd_ty,
                ctx_mut_prd_ty: $ctx_mut_prd_ty,
                ctx_prd_tr: $ctx_prd_tr,
                ctx_mut_prd_tr: $ctx_mut_prd_tr,
                ctx_into_ty: $ctx_into_ty,
                ctx_mut_into_ty: $ctx_mut_into_ty
            } => $($cont_args)*
        }
    };
    (
        { {$impl_ty:ty} {$($trait_ty:tt)+} } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { {$impl_ty} {$($trait_ty)+} } => $crate::__emit_trait_impl{
                ctx: $ctx,
                ctx_prd_ty: $ctx_prd_ty,
                ctx_mut_prd_ty: $ctx_mut_prd_ty,
                ctx_prd_tr: $ctx_prd_tr,
                ctx_mut_prd_tr: $ctx_mut_prd_tr,
                ctx_into_ty: $ctx_into_ty,
                ctx_mut_into_ty: $ctx_mut_into_ty
            } => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __for_each_impl_with_trait {
    (
        { $($input:tt)* } => { { /* {impl} => */ $($($include_pipe:tt)+)? } => { /* { {impl} {trait} } => */ $($($pipe:tt)+)? } } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { $($input)* }
            => $crate::__for_each_impl_statement{
                ::demuncher::fork{
                    { ::demuncher::embrace{} }
                    { ::demuncher::split_by{,} => ::demuncher::skip_if_empty{} }
                }
                $(=> ::demuncher::when{
                    { ::demuncher::head{} => ::demuncher::debrace{} => ::demuncher::debrace{} => $($include_pipe)+ }
                    => {}
                    else {{}}
                })?
                => ::demuncher::cross{}
                => [
                    ::demuncher::debrace{}
                    $(
                        => $($pipe)+
                    )?
                ]
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __for_each_trait_statement {
    (
        { $($input:tt)* } => { /* { { {trait} {single_impl | [multiple_impls,..]} }.. } => */ $($($pipe:tt)+)? } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { $($input)* }
            => ::demuncher::split_by{;}
            => ::demuncher::skip_all_empty{}
            => [
                ::demuncher::debrace{}
                => ::demuncher::split_by{=>}
                $(
                    => $($pipe)+
                )?
            ]
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_trait_with_single_or_multiple_impls {
    (
        { { mut $($trait_head:ident)?$(::$trait_tail:ident)* } { [$($impls:tt)+] } } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { $($impls)+ }
            => ::demuncher::split_by{,}
            => [
                ::demuncher::debrace{}
                => $crate::__emit_item_impl{
                    ctx: $ctx,
                    ctx_prd_ty: $ctx_mut_prd_ty,
                    ctx_provided: ctx_provided,
                    local_impls: local_impls,
                    other_impls: other_impls,
                    trait: $($trait_head)?$(::$trait_tail)*,
                    mut: { mut }
                }
            ]
            => $crate::__emit_trait_multiple_impls{
                ctx: $ctx,
                ctx_prd_ty: $ctx_prd_ty,
                ctx_mut_prd_ty: $ctx_mut_prd_ty,
                ctx_prd_tr: $ctx_prd_tr,
                ctx_mut_prd_tr: $ctx_mut_prd_tr,
                ctx_into_ty: $ctx_into_ty,
                ctx_mut_into_ty: $ctx_mut_into_ty,
                ctx_provided: ctx_provided,
                local_impls: local_impls,
                other_impls: other_impls,
                trait: mut $($trait_head)?$(::$trait_tail)*
            }
            => $($cont_args)*
        }
    };
    (
        { { $($trait_head:ident)?$(::$trait_tail:ident)* } { [$($impls:tt)+] } } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { $($impls)+ }
            => ::demuncher::split_by{,}
            => [
                ::demuncher::debrace{}
                => $crate::__emit_item_impl{
                    ctx: $ctx,
                    ctx_prd_ty: $ctx_prd_ty,
                    ctx_provided: self,
                    local_impls: local_impls,
                    other_impls: other_impls,
                    trait: $($trait_head)?$(::$trait_tail)*,
                    mut: {}
                }
            ]
            => $crate::__emit_trait_multiple_impls{
                ctx: $ctx,
                ctx_prd_ty: $ctx_prd_ty,
                ctx_mut_prd_ty: $ctx_mut_prd_ty,
                ctx_prd_tr: $ctx_prd_tr,
                ctx_mut_prd_tr: $ctx_mut_prd_tr,
                ctx_into_ty: $ctx_into_ty,
                ctx_mut_into_ty: $ctx_mut_into_ty,
                ctx_provided: self,
                local_impls: local_impls,
                other_impls: other_impls,
                trait: $($trait_head)?$(::$trait_tail)*
            }
            => $($cont_args)*
        }
    };
    // (
    //     { { mut $($trait:tt)+ } { sub $($impl:tt)+ } } => { ctx: $ctx:ident } => $cont:path { $($cont_args:tt)* }
    // ) => {
    //     $cont! {
    //         { {$($impl)+} {$($trait)+} }
    //         => __emit_trait_from_sub_context{ ctx: $ctx }
    //         => $($cont_args)*
    //     }
    // };
    (
        { { $($trait_head:ident)?$(::$trait_tail:ident)* } { $($impl:tt)+ } } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { {$($impl)+} {$($trait_head)?$(::$trait_tail)*} }
            => $crate::__pick_trait_impl{
                ctx: $ctx,
                ctx_prd_ty: $ctx_prd_ty,
                ctx_mut_prd_ty: $ctx_mut_prd_ty,
                ctx_prd_tr: $ctx_prd_tr,
                ctx_mut_prd_tr: $ctx_mut_prd_tr,
                ctx_into_ty: $ctx_into_ty,
                ctx_mut_into_ty: $ctx_mut_into_ty
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_trait_impl {
    (
        { {$impl_ty:ty} {mut $($trait_head:ident)?$(::$trait_tail:ident)*} } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<'c, TContextProvided> MutServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                for $ctx_mut_prd_ty<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx> + 'c,
                    Self: 'c
                {
                    fn get_mut_service(&'c mut self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c> {
                        let provided_ctx = $ctx_mut_prd_ty(
                            ForwardingMutContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $ctx::into_mut_service(provided_ctx)
                    }
                }

                impl<'c> $ctx_mut_into_ty<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                for $ctx
                {
                    fn into_mut_service<NewContextProvided>(new_ctx_provided: $ctx_mut_prd_ty<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                    where
                        NewContextProvided: MutContextProvided<Context = $ctx> + 'c,
                        $ctx_mut_prd_ty<NewContextProvided>: 'c
                    {
                        ::std::boxed::Box::new(
                            MutServiceProvidedWithContext {
                                data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                                mut_data_getter: |c: &mut $ctx| -> &mut $impl_ty { c.services.get_mut() },
                                ctx_provided: new_ctx_provided,
                                _service_phantom: PhantomData
                            }
                        )
                    }
                }
                // impl<'c> ContextMutServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                // for $ctx
                // {
                //     fn into_mut_service<NewContextProvided>(new_ctx_provided: $ctx_mut_prd_ty<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>
                // where
                //     NewContextProvided: MutContextProvided<Context = $ctx> + 'c
                //     {
                //         ::std::boxed::Box::new(MutServiceProvidedWithContext {
                //             data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                //             mut_data_getter: |c: &mut $ctx| -> &mut $impl_ty { c.services.get_mut() },
                //             ctx_provided: new_ctx_provided,
                //             _service_phantom: PhantomData
                //         })
                //     }
                // }
            }
            => $($cont_args)*
        }
    };
    (
        { {$impl_ty:ty} {$($trait_head:ident)?$(::$trait_tail:ident)*} } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<'c, TContextProvided> ServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                for $ctx_prd_ty<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx> + 'c,
                    Self: 'c
                {
                    fn get_service(&'c self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c> {
                        let provided_ctx = $ctx_prd_ty(
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $ctx::into_service(provided_ctx)
                    }
                }

                impl<'c> $ctx_into_ty<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                for $ctx
                {
                    fn into_service<NewContextProvided>(new_ctx_provided: $ctx_prd_ty<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                    where
                        NewContextProvided: ContextProvided<Context = $ctx>,
                        $ctx_prd_ty<NewContextProvided>: 'c
                    {
                        ::std::boxed::Box::new(
                            ServiceProvidedWithContext {
                                data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                                ctx_provided: new_ctx_provided,
                                _service_phantom: PhantomData
                            }
                        )
                    }
                }

                impl<'c, TContextProvided> ServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                for $ctx_mut_prd_ty<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>,
                    Self: 'c
                {
                    fn get_service(&'c self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c> {
                        let provided_ctx = $ctx_prd_ty(
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $ctx::into_service(provided_ctx)
                    }
                }
                // impl<'c> ContextServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                // for $ctx
                // {
                //     fn into_service<NewContextProvided>(new_ctx_provided: $ctx_prd_ty<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>
                //     where
                //         NewContextProvided: ContextProvided<Context = $ctx> + 'c
                //     {
                //         ::std::boxed::Box::new(ServiceProvidedWithContext {
                //             data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                //             ctx_provided: new_ctx_provided,
                //             _service_phantom: PhantomData
                //         })
                //     }
                // }
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_trait_from_sub_context {
    (
        { {$impl_ty:ty} {mut $($trait_head:ident)?$(::$trait_tail:ident)*} } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<'c, TContextProvided> MutServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                for $ctx_mut_prd_ty<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx> + 'c,
                    Self: 'c
                {
                    fn get_mut_service(&'c mut self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c> {
                        let provided_ctx = $ctx_mut_prd_ty(
                            ForwardingMutContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $ctx::into_mut_service(provided_ctx)
                    }

                    // fn into_mut_service(self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c> {
                    //     let sub_ctx_provided = ::paste::paste!{[<$impl_ty ඞMutProvided>]}(
                    //         MutSubContextProvidedWithParent {
                    //             ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                    //             mut_ctx_getter: |parent: &mut $ctx| -> &mut $impl_ty { parent.sub_contexts.get_mut() },
                    //             parent_ctx_provided: self,
                    //             _sub_context_phantom: PhantomData
                    //         }
                    //     );
                    //     sub_ctx_provided.into_mut_service()
                    // }
                }
            
                impl<'c> $ctx_mut_into_ty<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                for $ctx
                {
                    fn into_mut_service<NewContextProvided>(new_ctx_provided: $ctx_mut_prd_ty<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>
                    where
                        NewContextProvided: MutContextProvided<Context = $ctx> + 'c,
                        $ctx_mut_prd_ty<NewContextProvided>: 'c
                    {
                        let sub_ctx_provided = ::paste::paste!{[<$impl_ty ඞMutProvided>]}(
                            MutSubContextProvidedWithParent {
                                ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                                mut_ctx_getter: |parent: &mut $ctx| -> &mut $impl_ty { parent.sub_contexts.get_mut() },
                                parent_ctx_provided: self,
                                _sub_context_phantom: PhantomData
                            }
                        );
                        <$impl_ty>::into_mut_service(sub_ctx_provided)
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        { {$impl_ty:ty} {$($trait_head:ident)?$(::$trait_tail:ident)*} } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<'c, TContextProvided> ServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                for $ctx_prd_ty<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx> + 'c
                {
                    fn get_service(&'c self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c> {
                        let provided_ctx = $ctx_prd_ty(
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $ctx::into_service(provided_ctx)
                    }
                }

                impl<'c> $ctx_into_ty<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                for $ctx
                {
                    fn into_service<NewContextProvided>(new_ctx_provided: $ctx_prd_ty<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                    where
                        NewContextProvided: ContextProvided<Context = $ctx> + 'c,
                        $ctx_prd_ty<NewContextProvided>: 'c
                    {
                        let sub_ctx_provided = ::paste::paste!{[<$impl_ty ඞProvided>]}(
                            SubContextProvidedWithParent {
                                ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                                parent_ctx_provided: new_ctx_provided,
                                _sub_context_phantom: PhantomData
                            }
                        );
                        <$impl_ty>::into_service(sub_ctx_provided)
                    }
                }
                
                impl<'c, NewContextProvided> ServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                for $ctx_mut_prd_ty<NewContextProvided>
                where
                    NewContextProvided: MutContextProvided<Context = $ctx> + 'c,
                    Self: 'c
                {
                    fn get_service(&'c self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c> {
                        let provided_ctx = $ctx_prd_ty(
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $ctx::into_service(provided_ctx)
                    }
                }
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_trait_from_extern_context {
    (
        { {$($trait_head:ident)?$(::$trait_tail:ident)*} } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                // impl<'c, ParentContext, ContextGetter> ServiceProvider<'c, dyn $trait_ty + 'c>
                //     for ContextProvidedWithParent<'c, $ctx, ParentContext, ContextGetter>
                // where
                //     ContextGetter: Fn(&ParentContext) -> &$ctx + Copy
                // {
                //     fn get_service(&'c self) -> ::std::boxed::Box<dyn $trait_ty + 'c> {
                //         (self.sub_contexts.get() as &$impl_ty).get_service()
                //     }
                // }
                impl<'c, NewContextProvided, ParentContextProvided> ContextServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c, NewContextProvided>
                for $ctx
                {
                    // fn get_service(ctx_provided: &'c TContextProvided) -> ::std::boxed::Box<dyn $trait_ty + 'c> {
                    //     let sub_ctx_provided = SubContextProvidedWithParent {
                    //             ctx_getter: |parent: &TContextProvided::Context| -> &$impl_ty { parent.sub_contexts.get() },
                    //             parent_ctx_provided: ctx_provided,
                    //             _sub_context_phantom: PhantomData
                    //         };
                    //     $impl_ty::get_service(&sub_ctx_provided)
                    //     // ::std::boxed::Box::new(ServiceProvidedWithContext {
                    //     //     ctx_provided: ,
                    //     //     data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                    //     // })
                    // }

                    fn into_service<NewContextProvided>(new_ctx_provided: $ctx_prd_ty<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>
                    where
                        NewContextProvided: ContextProvided<Context = $ctx> + 'c
                    {
                        let sub_ctx_provided = SubContextProvidedWithParent {
                                ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                                parent_ctx_provided: new_ctx_provided,
                                _sub_context_phantom: PhantomData
                            };
                        //<$impl_ty as ContextServiceProvider<'c, _, dyn $($trait_head)?$(::$trait_tail)* + 'c>>::into_service(sub_ctx_provided)
                        <$impl_ty>::into_service(sub_ctx_provided)
                    }
                }
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_item_impl {
    (
        { $impl_ty:ty } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: $($trait_head:ident)?$(::$trait_tail:ident)*,
            mut: {}
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                $local_impls.push(
                    Box::new(
                        |ncp: &mut $ctx_prd_ty<NewContextProvided>| {
                            Box::new(
                                ServiceProvidedWithContext {
                                    ctx_provided: $ctx_prd_ty(
                                        ForwardingContextProvided {
                                            forwarded_ctx: ncp
                                        }
                                    ),
                                    data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                                    _service_phantom: PhantomData
                                }
                            )
                        }
                    )
                );
            }
            => $($cont_args)*
        }
    };
    (
        { $impl_ty:ty } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: $($trait_head:ident)?$(::$trait_tail:ident)*,
            mut: {mut}
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                $local_impls.push(
                    Box::new(
                        |ncp: & mut $ctx_prd_ty<NewContextProvided>| {
                            Box::new(
                                MutServiceProvidedWithContext {
                                    ctx_provided: $ctx_prd_ty(
                                        MutForwardingContextProvided {
                                            forwarded_ctx: ncp
                                        }
                                    ),
                                    data_getter: |c: &mut $ctx| -> &mut $impl_ty { c.services.get_mut() },
                                    _service_phantom: PhantomData
                                }
                            )
                        }
                    )
                );
            }
            => $($cont_args)*
        }
    };
    (
        { sub $sub_ctx_ty:ty } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: $($trait_head:ident)?$(::$trait_tail:ident)*,
            mut: {$($mut:ident)?}
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            {
                $other_impls.push(
                    Box::new(
                        move |ncp: &mut $ctx_prd_ty<NewContextProvided>| {
                            let sub_ctx_provided = ::paste::paste!{[<$sub_ctx_ty ඞProvided>]}(
                                SubContextProvidedWithParent {
                                    ctx_getter: |parent: &$ctx| -> &$sub_ctx_ty { parent.sub_contexts.get() },
                                    parent_ctx_provided: ForwardingContextProvided {
                                        forwarded_ctx: ncp
                                    },
                                    _sub_context_phantom: PhantomData
                                }
                            );
                            sub_ctx_provided.into_services().as_dyn()
                        }
                    )
                );
                // $other_impls.append(
                //     &mut ($ctx_provided.ctx().sub_contexts.get() as & $($mut)? $sub_ctx_ty).get_services()
                // );
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_trait_multiple_impls {
    (
        { $($impls:tt)* } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident,
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: mut $($trait_head:ident)?$(::$trait_tail:ident)*
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<'c, TContextProvided> ContextMultipleMutServiceProvider<'c, $ctx_prd_ty<TContextProvided>, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                    for $ctx
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                {
                    fn get_mut_services($ctx_provided: &'c mut $ctx_prd_ty<TContextProvided>) -> ::std::vec::Vec<::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>> {
                        let mut $vec: ::std::vec::Vec<::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>> = vec![];
                        $($impls)*
                        $vec
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        { $($impls:tt)* } => {
            ctx: $ctx:ident,
            ctx_prd_ty: $ctx_prd_ty:ident,
            ctx_mut_prd_ty: $ctx_mut_prd_ty:ident,
            ctx_prd_tr: $ctx_prd_tr:ident,
            ctx_mut_prd_tr: $ctx_mut_prd_tr:ident,
            ctx_into_ty: $ctx_into_ty:ident,
            ctx_mut_into_ty: $ctx_mut_into_ty:ident,
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: $($trait_head:ident)?$(::$trait_tail:ident)*
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<'c, NewContextProvided> MultipleServiceProvider<'c, $crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)>
                for $ctx_prd_ty<NewContextProvided>
                where
                    NewContextProvided: ContextProvided<Context = $ctx> + 'c
                {
                    // fn get_services($ctx_provided: &'c TContextProvided) -> ::std::vec::Vec<::std::boxed::Box<dyn $trait_ty + 'c>> {
                    //     let mut $vec: ::std::vec::Vec<::std::boxed::Box<dyn $trait_ty + 'c>> = vec![];
                    //     $($impls)*
                    //     $vec
                    // }
                    fn get_services(&'c self) -> impl Seq<'c, ItemGat = $crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)> + use<'c, NewContextProvided> {
                        let provided_ctx = $ctx_prd_ty(
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        provided_ctx.into_services()
                    }

                    fn into_services(mut $ctx_provided) -> impl Seq<'c, ItemGat = $crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)> + use<'c, NewContextProvided> {
                        use ::std::{vec::Vec, vec, boxed::Box};
                        let mut $local_impls:
                            Vec<
                                Box<
                                    dyn for<'a> Fn(&'a mut $ctx_prd_ty<NewContextProvided>) -> Box<dyn $($trait_head)?$(::$trait_tail)* + 'a>
                                >
                            >
                            = vec![];
                        let mut $other_impls:
                            Vec<
                                Box<
                                    dyn for<'a> FnOnce(&'a mut $ctx_prd_ty<NewContextProvided>) -> Box<dyn DynSeq<Gat = $crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)> + 'a>
                                >
                            >
                            = vec![];
                        $($impls)*
                        $other_impls.push(
                            Box::new(
                                move |ncp: &mut $ctx_prd_ty<NewContextProvided>| {
                                    $local_impls
                                        .into_seq()
                                        .wrap(|c, next: &mut dyn Emit<$crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)>| {
                                            let service = (c)(ncp);
                                            next.emit(service)
                                        })
                                        .as_dyn()
                                }
                            )
                        );
                        $other_impls
                            .into_seq()
                            .wrap(
                                move |c, next: &mut dyn Emit<Boxed<$crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)>>| {
                                    let sub_seq = (c)(&mut $ctx_provided);
                                    next.emit(sub_seq)
                                }
                            )
                            .flatten()
                    }
                }

                impl<'c, NewContextProvided> MultipleServiceProvider<'c, $crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)>
                for $ctx_mut_prd_ty<NewContextProvided>
                where
                    NewContextProvided: MutContextProvided<Context = $ctx> + 'c
                {
                    fn get_services(&'c self) -> impl Seq<'c, ItemGat = $crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)> + use<'c, NewContextProvided> {
                        let provided_ctx = $ctx_prd_ty(
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        provided_ctx.into_services()
                    }

                    fn into_services(self) -> impl Seq<'c, ItemGat = $crate::seq_gat_for_multi_trait!($($trait_head)?$(::$trait_tail)*)> + use<'c, NewContextProvided> {
                        let provided_ctx = $ctx_prd_ty(self);
                        provided_ctx.into_services()
                    }
                }
            }
            => $($cont_args)*
        }
    };
}

// #[macro_export]
// #[doc(hidden)]
// macro_rules! __emit_item_trait_from_sub_context {
//     (
//         { {$impl_ty:ty} {$($trait_head:ident)?$(::$trait_tail:ident)*} } => { ctx: $ctx:ident } => $cont:path { $($cont_args:tt)* }
//     ) => {
//         $cont! {
//             { 
//                 ::std::boxed::Box::new(ServiceProvidedWithContext {
//                     ctx: self,
//                     data_getter: |c: &mut $ctx| -> &mut $impl_ty { c.services.get() },
//                 }),
//                 impl<'c> ServiceProvider<'c, dyn $trait_ty + 'c> for $ctx {
//                     fn get_service(&'c self) -> ::std::boxed::Box<dyn $trait_ty + 'c> {
//                         (self.sub_contexts.get() as &$impl_ty).get_service()
//                     }
//                 }
//             }
//             => $($cont_args)*
//         }
//     };
// }

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_dependant_trait_constraint {
    (
        {$dep:path} => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! { {for<'a> ServiceProvider<'a, dyn $dep + 'a>} => $($cont_args)* }
    };
    (
        {mut $dep:path} => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! { {for<'a> MutServiceProvider<'a, dyn $dep + 'a>} => $($cont_args)* }
    };
    (
        {[$($dep_head:ident)?$(::$dep_tail:ident)*]} => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! { {for<'a> MultipleServiceProvider<'a, seq_gat_for_multi_trait!($($dep_head)?$(::$dep_tail)*)>} => $($cont_args)* }
    };
    (
        {[mut $dep:path]} => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! { {for<'a> MultipleMutServiceProvider<'a, dyn $dep + 'a>} => $($cont_args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_impl_service {
    (
        {$($dependant_trait_constraints:tt)*}
        => { impl: $impl_ty:path, trait: $($trait_head:ident)?$(::$trait_tail:ident)*, def: { $($def:tt)* } }
         => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                impl<'c, TContextProvided, DataGetter> $($trait_head)?$(::$trait_tail)*
                    for ServiceProvidedWithContext<$impl_ty, TContextProvided, DataGetter>
                where
                    TContextProvided: ContextProvided,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty,
                    TContextProvided: $($dependant_trait_constraints)*
                {
                    $($def)*
                }
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_impl_mut_service {
    (
        {$($dependant_trait_constraints:tt)*}
        => { impl: $impl_ty:path, trait: $($trait_head:ident)?$(::$trait_tail:ident)*, def: { $($def:tt)* } }
         => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                impl<'c, TContextProvided, MutDataGetter, DataGetter> $($trait_head)?$(::$trait_tail)*
                    for MutServiceProvidedWithContext<$impl_ty, TContextProvided, MutDataGetter, DataGetter>
                where
                    TContextProvided: MutContextProvided,
                    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut $impl_ty,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty,
                    TContextProvided: $($dependant_trait_constraints)*
                {
                    $($def)*
                }
            }
            => $($cont_args)*
        }
    };
}

#[cfg(test)]
mod tests {
    use std::{cell::{Ref, RefCell}};


    use crate::{def_seq_gat_for_multi_trait, seq_gat_for_multi_trait};

    use super::*;

    #[test]
    fn service_calls_another_service_with_additional_service() {
        let mut context = ServiceCallsAnotherServiceWithAdditionalServiceContext::default();
        {
            let mut mut_ctx_provided = context.provide_mut();
            (mut_ctx_provided.get_mut_service() as Box<dyn AdditionalSetterService>).set_value(44);
        }
        {
            let ctx_provided = context.provide();
            {
            let cs: Box<dyn CallingService> = ctx_provided.get_service();
            cs.call_service(2);
            }
        }
        let expected_calls = vec![
            "+additional_setter_service(44)",
            "-additional_setter_service(44)",
            "+calling_service(2)",
            "+called_service(2)",
            "+additional_service(2, 44)",
            "-additional_service(2, 44)",
            "-called_service(2)",
            "-calling_service(2)"
        ];
        assert_eq!(*(context.provide().get_service() as Box<dyn LoggedService>).get_calls(), expected_calls);
    }
    
    //trace_macros!(true);
    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsAnotherServiceWithAdditionalServiceContext {
            CalledServiceImpl1: CalledService;
            CallingServiceImpl1: CallingService;
            LoggingServiceImpl1: LoggingService, LoggedService;
            AdditionalServiceImpl1: AdditionalService, mut AdditionalSetterService;
        }
    }
    //trace_macros!(false);

    #[test]
    fn service_calls_multiple_implementations_of_one_service() {
        let mut context = ServiceCallsMultipleImplementationsOfOneServiceContext::default();
        (context.provide_mut().get_mut_service() as Box<dyn SetterMultiService>).set_value(44);
        (context.provide().get_service() as Box<dyn CallingService>).call_service(2);
        let expected_calls = vec![
            "+setter_multi_service(44)",
            "-setter_multi_service(44)",
            "+calling_service(2)",
            "+called_multi_service(0)",
            "-called_multi_service(0)",
            "+called_multi_service(1, 44)",
            "-called_multi_service(1, 44)",
            "-calling_service(2)"
        ];
        assert_eq!(*(context.provide().get_service() as Box<dyn LoggedService>).get_calls(), expected_calls);
    }

    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsMultipleImplementationsOfOneServiceContext {
            CallingMultipleServiceImpl1: CallingService;
            CalledMultiServiceImpl1;
            CalledMultiServiceImpl2: mut SetterMultiService;
            LoggingServiceImpl1: LoggingService, LoggedService
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl1, CalledMultiServiceImpl2]
        }
    }

    #[test]
    fn service_calls_implementations_from_sub_context() {
        let mut context = ServiceCallsImplementationsFromSubContextCallingContext::default();
        (context.sub_contexts.get_mut().provide_mut().get_mut_service() as Box<dyn SetterMultiService>).set_value(44);
        (context.provide().get_service() as Box<dyn CallingService>).call_service(2);
        let expected_calls = vec![
            "+setter_multi_service(44)",
            "-setter_multi_service(44)",
            "+calling_service(2)",
            "+called_multi_service(0, 44)",
            "-called_multi_service(0, 44)",
            "+called_multi_service(1)",
            "-called_multi_service(1)",
            "-calling_service(2)"
        ];
        assert_eq!(*(context.sub_contexts.get_mut().provide().get_service() as Box<dyn LoggedService>).get_calls(), expected_calls);
    }

    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsImplementationsFromSubContextCallingContext {
            CallingMultipleServiceImpl1: CallingService;
            CalledMultiServiceImpl1;
            sub ServiceCallsImplementationsFromSubContextCalledContext: LoggingService
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl1, sub ServiceCallsImplementationsFromSubContextCalledContext]
        }
    }

    //trace_macros!(true);
    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsImplementationsFromSubContextCalledContext {
            CalledMultiServiceImpl2: mut SetterMultiService;
            LoggingServiceImpl1: LoggingService, LoggedService
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl2]
        }
    }
    //trace_macros!(false);

    // #[test]
    // fn service_calls_implementations_from_parent_context() {
    //     let mut context = ServiceCallsImplementationsFromParentContextCallingContext::default();
    //     (context.provide_mut().get_mut_service() as Box<dyn SetterMultiService>).set_value(44);
    //     (context.provide().get_service() as Box<dyn CallingService>).call_service(2);
    //     let expected_calls = vec![
    //         "+setter_multi_service(44)",
    //         "-setter_multi_service(44)",
    //         "+calling_service(2)",
    //         "+called_multi_service(0)",
    //         "-called_multi_service(0)",
    //         "+called_multi_service(1, 44)",
    //         "-called_multi_service(1, 44)",
    //         "-calling_service(2)"
    //     ];
    //     assert_eq!(*(context.provide_mut().get_service() as Box<dyn LoggedService>).get_calls(), expected_calls);
    // }

    // service_context! {
    //     #[derive(Debug, Default)]
    //     ServiceCallsImplementationsFromParentContextCallingContext {
    //         CallingMultipleServiceImpl1: CallingService;
    //         CalledMultiServiceImpl1;
    //         LoggingServiceImpl1: LoggingService, LoggedService;
    //         sub ServiceCallsImplementationsFromParentContextCalledContext: mut SetterMultiService;
    //     }
    //     traits {
    //         CalledMultiService => [CalledMultiServiceImpl1, sub ServiceCallsImplementationsFromParentContextCalledContext]
    //     }
    // }

    // service_context! {
    //     #[derive(Debug, Default)]
    //     ServiceCallsImplementationsFromParentContextCalledContext {
    //         CalledMultiServiceImpl2: mut SetterMultiService;
    //         extern: LoggingService;
    //     }
    //     traits {
    //         CalledMultiService => [CalledMultiServiceImpl2];
    //     }
    // }

    #[derive(Debug, Default)]
    struct CallingMultipleServiceImpl1; 

    impl_service! {
        CallingMultipleServiceImpl1: CallingService[LoggingService, [CalledMultiService]]
        {
            fn call_service(&self, id: i32) {
                let logging: Box<dyn LoggingService> = self.get_service();
                logging.log(&format!("+calling_service({id})"));
                self.get_services().enumerate().for_each(|(idx, service)| {
                    (*(service as Box<dyn CalledMultiService>)).call_service(idx);
                });
                // for (idx, service) in self.get_services().enumerate() {
                //     (*(service as Box<dyn CalledMultiService>)).call_service(idx);
                // }
                logging.log(&format!("-calling_service({id})"));
            }
        }
    }

    trait CalledMultiService {
        fn call_service(&self, idx: usize);
    }

    def_seq_gat_for_multi_trait!(CalledMultiService);
    
    #[derive(Debug, Default)]
    struct CalledMultiServiceImpl1; 

    impl_service! {
        CalledMultiServiceImpl1: CalledMultiService[LoggingService]
        {
            fn call_service(&self, idx: usize) {
                let logging: Box<dyn LoggingService> = self.get_service();
                logging.log(&format!("+called_multi_service({idx})"));
                logging.log(&format!("-called_multi_service({idx})"));
            }
        }
    }

    trait SetterMultiService {
        fn set_value(&mut self, value: i32);
    }

    def_seq_gat_for_multi_trait!(SetterMultiService);

    #[derive(Debug, Default)]
    struct CalledMultiServiceImpl2 {
        pub value: i32
    }

    impl_service! {
        CalledMultiServiceImpl2: CalledMultiService[LoggingService]
        {
            fn call_service(&self, idx: usize) {
                let logging: Box<dyn LoggingService> = self.get_service();
                let value = self.data().value;
                logging.log(&format!("+called_multi_service({idx}, {value})"));
                logging.log(&format!("-called_multi_service({idx}, {value})"));
            }
        }
    }

    impl_mut_service! {
        CalledMultiServiceImpl2: SetterMultiService[LoggingService]
        {
            fn set_value(&mut self, value: i32) {
                (self.get_service() as Box<dyn LoggingService>).log(&format!("+setter_multi_service({value})"));
                self.mut_data().value = value;
                (self.get_service() as Box<dyn LoggingService>).log(&format!("-setter_multi_service({value})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct CallingServiceImpl1; 

    trait CallingService {
        fn call_service(&self, id: i32);
    }

    impl_service! {
        CallingServiceImpl1: CallingService[LoggingService, CalledService]
        {
            fn call_service(&self, id: i32) {
                let logging: Box<dyn LoggingService> = self.get_service();
                logging.log(&format!("+calling_service({id})"));
                (self.get_service() as Box<dyn CalledService>).call_service(id);
                logging.log(&format!("-calling_service({id})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct CalledServiceImpl1;

    trait CalledService {
        fn call_service(&self, id: i32);
    }

    impl_service! {
        CalledServiceImpl1: CalledService[LoggingService, AdditionalService]
        {
            fn call_service(&self, id: i32) {
                let logging: Box<dyn LoggingService> = self.get_service();
                logging.log(&format!("+called_service({id})"));
                (self.get_service() as Box<dyn AdditionalService>).call_service(id);
                logging.log(&format!("-called_service({id})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct AdditionalServiceImpl1 {
        value: i32
    }

    trait AdditionalService {
        fn call_service(&self, id: i32);
    }

    trait AdditionalSetterService {
        fn set_value(&mut self, value: i32);
    }

    impl_service! {
        AdditionalServiceImpl1: AdditionalService[LoggingService]
        {
            fn call_service(&self, id: i32) {
                let logging: Box<dyn LoggingService> = self.get_service();
                let value = self.data().value;
                logging.log(&format!("+additional_service({id}, {value})"));
                logging.log(&format!("-additional_service({id}, {value})"));
            }
        }
    }

    impl_mut_service! {
        AdditionalServiceImpl1: AdditionalSetterService[LoggingService]
        {
            fn set_value(&mut self, value: i32) {
                (self.get_service() as Box<dyn LoggingService>).log(&format!("+additional_setter_service({value})"));
                self.mut_data().value = value;
                (self.get_service() as Box<dyn LoggingService>).log(&format!("-additional_setter_service({value})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct LoggingServiceImpl1 {
        pub calls: RefCell<Vec<String>>
    }

    trait LoggingService {
        fn log(&self, message: &str);
    }

    impl_service! {
        LoggingServiceImpl1: LoggingService
        {
            fn log(&self, message: &str) {
                self.data().calls.borrow_mut().push(message.into());
            }
        }
    }

    trait LoggedService {
        fn get_calls(&'_ self) -> Ref<'_, Vec<String>>;
    }

    impl_service! {
        LoggingServiceImpl1: LoggedService
        {
            fn get_calls(&'_ self) -> Ref<'_, Vec<String>> {
                self.data().calls.borrow()
            }
        }
    }
}