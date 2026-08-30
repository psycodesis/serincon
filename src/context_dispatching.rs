// Copyright (c) 2026 Mariusz Zacirka
// 
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{any::Any, marker::PhantomData};
use demuncher::*;
use itertools::{FoldWhile::{self, Continue, Done}, Itertools, WhileSome};
use crate::visiting::{Boxed, DynSeq, Emit, IntoSeq, Seq, SeqGat};

pub trait ServiceProvidedData<ServiceData> {
    fn data(&self) -> &ServiceData;
}

pub trait MutServiceProvidedData<ServiceProvided>: ServiceProvidedData<ServiceProvided> {
    fn mut_data(&mut self) -> &mut ServiceProvided;
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

pub trait ContextProvided {
    type Context;
    type ParentContextProvided: ContextProvided;
    fn ctx(&self) -> &Self::Context;
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided;
}

pub trait MutContextProvided: ContextProvided {
    type ParentMutContextProvided: MutContextProvided;
    fn mut_ctx(&mut self) -> &mut Self::Context;
    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided;
}

pub struct NoContextProvided;

impl ContextProvided for NoContextProvided
{
    type Context = ();
    type ParentContextProvided = NoContextProvided;

    fn ctx(&self) -> &() { &() }
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided { &NoContextProvided }
}

pub struct NoMutContextProvided(());

impl ContextProvided for NoMutContextProvided
{
    type Context = ();
    type ParentContextProvided = NoContextProvided;

    fn ctx(&self) -> &() { &() }
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided { &NoContextProvided }
}

impl MutContextProvided for NoMutContextProvided
{
    type ParentMutContextProvided = NoMutContextProvided;

    fn mut_ctx(&mut self) -> &mut () { &mut self.0 }

    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided { self }
}


pub struct StartContextProvided<'c, Context> {
    start_ctx: &'c Context
}

impl<'c, TContext> ContextProvided for StartContextProvided<'c, TContext>
{
    type Context = TContext;
    type ParentContextProvided = NoContextProvided;

    fn ctx(&self) -> &TContext {
        self.start_ctx
    }
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
        &NoContextProvided
    }
}

pub struct StartMutContextProvided<'c, Context> {
    start_ctx: &'c mut Context
}

impl<'c, TContext> ContextProvided for StartMutContextProvided<'c, TContext> {
    type Context = TContext;
    type ParentContextProvided = NoContextProvided;

    fn ctx(&self) -> &TContext {
        self.start_ctx
    }
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
        &NoContextProvided
    }
}

impl<'c, TContext> MutContextProvided for StartMutContextProvided<'c, TContext> {
    type ParentMutContextProvided = NoMutContextProvided;
    
    fn mut_ctx(&mut self) -> &mut TContext {
        self.start_ctx
    }
    
    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided {
        todo!()
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
    type ParentContextProvided = ForwardedContextProvided::ParentContextProvided;

    fn ctx(&self) -> &'c Self::Context {
        self.forwarded_ctx.ctx()
    }
    
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
    type ParentContextProvided = ForwardedContextProvided::ParentContextProvided;

    fn ctx(&self) -> &Self::Context {
        self.forwarded_ctx.ctx()
    }
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
        self.forwarded_ctx.parent_ctx_provided()
    }
}

impl<'c, ForwardedContextProvided> MutContextProvided for ForwardingMutContextProvided<'c, ForwardedContextProvided>
where 
    ForwardedContextProvided: MutContextProvided
{
    type ParentMutContextProvided = ForwardedContextProvided::ParentMutContextProvided;
    
    fn mut_ctx(&mut self) -> &mut Self::Context {
        self.forwarded_ctx.mut_ctx()
    }
    
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
    _sub_context_phantom: PhantomData<fn(&i32) -> &SubContext>
}

impl<SubContext, ParentContextProvided, ContextGetter> ContextProvided
for SubContextProvidedWithParent<ParentContextProvided, SubContext, ContextGetter>
where
    ParentContextProvided: ContextProvided,
    ContextGetter: Fn(&ParentContextProvided::Context) -> &SubContext
{
    type Context = SubContext;
    type ParentContextProvided = ParentContextProvided;

    fn ctx(&self) -> &SubContext {
        (self.ctx_getter)(self.parent_ctx_provided.ctx())
    }
    
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

impl<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter> ContextProvided
for MutSubContextProvidedWithParent<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter>
where
    ParentMutContextProvided: MutContextProvided,
    ContextGetter: Fn(&ParentMutContextProvided::Context) -> &SubContext,
    MutContextGetter: Fn(&mut ParentMutContextProvided::Context) -> &mut SubContext
{
    type Context = SubContext;
    type ParentContextProvided = ParentMutContextProvided;

    fn ctx(&self) -> &SubContext {
        (self.ctx_getter)(self.parent_ctx_provided.ctx())
    }
    
    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
        &self.parent_ctx_provided
    }
}

impl<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter> MutContextProvided
for MutSubContextProvidedWithParent<ParentMutContextProvided, SubContext, ContextGetter, MutContextGetter>
where
    ParentMutContextProvided: MutContextProvided,
    ContextGetter: Fn(&ParentMutContextProvided::Context) -> &SubContext,
    MutContextGetter: Fn(&mut ParentMutContextProvided::Context) -> &mut SubContext
{
    type ParentMutContextProvided = ParentMutContextProvided;
    
    fn mut_ctx(&mut self) -> &mut SubContext {
        (self.mut_ctx_getter)(self.parent_ctx_provided.mut_ctx())
    }
    
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
        $vis:vis $ctx:ident $(as $ctx_label:ident)? { $($impls:tt)* }
        $(traits { $($traits:tt)* })?
    ) => {
        $(#[$ctx_meta])*
        #[allow(dead_code)]
        $vis struct $ctx {
            services: ::demuncher::apply_pipe!{
                {$($impls)*}
                => $crate::__take_impls_list{
                    ::demuncher::when{ { $crate::__is_impl{} } => { ::demuncher::embrace{} } else { ::demuncher::reset{} } }
                }
                => $crate::__as_hlist{}
            },
            sub_contexts: ::demuncher::apply_pipe!{
                {$($impls)*}
                => $crate::__take_impls_list{
                    ::demuncher::when{ { $crate::__is_sub{} } => { ::demuncher::tail{} => ::demuncher::embrace{} } else { ::demuncher::reset{} } }
                }
                => $crate::__as_hlist{}
            }
        }

        ::demuncher::apply_pipe!{
            { {$($impls)* } {$($($traits)*)?} }
            => $crate::__emit_extern_deps{}
            => $crate::__emit_ctx_provided{ vis: $vis, ctx: $ctx $(, ctx_label: $ctx_label)? }
            => $crate::__apply_paste{}
        }

        // ::paste::paste!{
            // $vis trait [<$ctx ඞIntoServiceProvider>]<'c, ServiceProvided: ?Sized + 'c>
            // {
            //     fn into_service<NewContextProvided>(new_ctx_provided: [<$ctx ඞProvided>]<NewContextProvided>) -> Box<ServiceProvided>
            //         where NewContextProvided: ContextProvided<Context = $ctx> + 'c;
            // }

            // $vis trait [<$ctx ඞIntoMutServiceProvider>]<'c, ServiceProvided: ?Sized + 'c>
            // {
            //     fn into_mut_service<NewContextProvided>(new_ctx_provided: [<$ctx ඞMutProvided>]<NewContextProvided>) -> Box<ServiceProvided>
            //         where NewContextProvided: MutContextProvided<Context = $ctx> + 'c;
            // }

            // $(
            //     $vis trait [<$ctx ඞSubContextProvider>] {
            //         fn get_sub_ctx(&self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx>>;

            //         fn [<get_ $ctx_label>](&self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx>> {
            //             self.get_sub_ctx()
            //         }
            //     }

            //     $vis trait [<$ctx ඞMutSubContextProvider>] {
            //         fn get_mut_sub_ctx(&mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx>>;

            //         fn [<get_mut_ $ctx_label>](&mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx>> {
            //             self.get_mut_sub_ctx()
            //         }
            //     }
            // )?
        // }

        ::demuncher::apply_pipe!{
            {$($impls)*}
            => $crate::__for_each_impl_with_trait{
                {} => {
                    ::demuncher::fork{
                        { ::demuncher::pass{} }
                        {
                            ::demuncher::debrace{}
                            => $crate::__split_service_kind{}
                            => $crate::__append_service_provider{}
                        }
                        {
                            ::demuncher::reset{ {$($impls)* } {$($($traits)*)?} }
                            => $crate::__emit_extern_deps{}
                            => ::demuncher::embrace{}
                        }
                    }
                    => $crate::__pick_trait_impl{
                        ctx: $ctx
                    }
                }
            }
            => $crate::__apply_paste{}
        }

        ::demuncher::apply_pipe!{
            {$($impls)*}
            => $crate::__for_each_impl_statement{
                ::demuncher::head{}
                => ::demuncher::debrace{}
                => ::demuncher::when{
                    { $crate::__is_sub{} }
                    => {
                        ::demuncher::tail{}
                        => ::demuncher::fork{
                            { ::demuncher::embrace{} }
                            {
                                ::demuncher::reset{ {$($impls)* } {$($($traits)*)?} }
                                => $crate::__emit_extern_deps{}
                                => ::demuncher::embrace{}
                            }
                            {
                                ::demuncher::reset{ {$($impls)* } {$($($traits)*)?} }
                                => $crate::__emit_deps_all_along_no_mut{}
                            }
                        }
                        => $crate::__emit_sub_ctx_provider_impl{ ctx: $ctx }
                    } else {
                        ::demuncher::reset{}
                    }
                }
            }
            => $crate::__apply_paste{}
        }

        ::demuncher::apply_pipe!{
            {$($($traits)*)?}
            => $crate::__for_each_trait_statement{
                ::demuncher::when{
                    { ::demuncher::tail{} => ::demuncher::debrace{} => $crate::__is_multiple{} } => {
                        ::demuncher::fork{
                            {
                                ::demuncher::debrace{}
                                => $crate::__split_service_kind{multiple}
                                => $crate::__append_service_provider{}
                            }
                            { ::demuncher::pass{} }
                            {
                                ::demuncher::reset{ {$($impls)* } {$($($traits)*)?} }
                                => $crate::__emit_extern_deps{}
                                => ::demuncher::embrace{}
                            }
                        }
                        => $crate::__emit_trait_with_multiple_impls{
                            ctx: $ctx
                        }
                    } else {
                        ::demuncher::swap{}
                        ::demuncher::fork{
                            { ::demuncher::pass{} }
                            {
                                ::demuncher::debrace{}
                                => $crate::__split_service_kind{}
                                => $crate::__append_service_provider{}
                            }
                            {
                                ::demuncher::reset{ {$($impls)* } {$($($traits)*)?} }
                                => $crate::__emit_extern_deps{}
                                => ::demuncher::embrace{}
                            }
                        }
                        => $crate::__pick_trait_impl{
                            ctx: $ctx
                        }
                    }
                }
            }
            => $crate::__apply_paste{}
        }
    };
}

#[macro_export]
macro_rules! impl_service {
    // Entry point with dependency list.
    ( impl $($trait_head:ident)?$(::$trait_tail:ident)* for $impl_ty:path where deps: $($rest:tt)+ ) => {
        ::demuncher::apply_pipe!{
            { $($rest)+ }
            => $crate::__split_deps_and_def{}
            => $crate::__emit_impl_service{ impl: $impl_ty, trait: $($trait_head)?$(::$trait_tail)* }
            => $crate::__apply_paste{}
        }
    };
    // Entry point with no dependencies.
    ( impl $($trait_head:ident)?$(::$trait_tail:ident)* for $impl_ty:path { $($def:tt)* } ) => { // $( where deps: $($deps:ident),+ )? { $($def:tt)* }
        impl<TContextProvided, DataGetter> $($trait_head)?$(::$trait_tail)*
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
    ( impl $($trait_head:ident)?$(::$trait_tail:ident)* for $impl_ty:path where deps: $($rest:tt)+ ) => {
        ::demuncher::apply_pipe!{
            { $($rest)+ }
            => $crate::__split_deps_and_def{}
            => $crate::__emit_impl_mut_service{ impl: $impl_ty, trait: $($trait_head)?$(::$trait_tail)* }
            => $crate::__apply_paste{}
        }
    };
    // Entry point with no dependencies.
    ( impl $($trait_head:ident)?$(::$trait_tail:ident)* for $impl_ty:path { $($def:tt)* } ) => { // $( where deps: $($deps:ident),+ )? { $($def:tt)* }
        impl<TContextProvided, MutDataGetter, DataGetter> $($trait_head)?$(::$trait_tail)*
            for MutServiceProvidedWithContext<$impl_ty, TContextProvided, MutDataGetter, DataGetter>
        where
            TContextProvided: MutContextProvided,
            MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut $impl_ty,
            DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
        { $($def)* }
    };
}

#[macro_export]
macro_rules! def_service {
    (
        $(#[$ctx_meta:meta])*
        $vis:vis trait $trait_ty:ident as $name:ident {
            $($def:tt)*
        }
    ) => {
        $(#[$ctx_meta])*
        $vis trait $trait_ty {
            $($def)*
        }

        ::paste::paste!{
            $vis trait [<$trait_ty ඞServiceProvider>] {
                fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $trait_ty + 's>;

                fn [<get_ $name>]<'s>(&'s self) -> ::std::boxed::Box<dyn $trait_ty + 's> {
                    self.get_service()
                }
            }
            
            $vis trait [<$trait_ty ඞIntoServiceProvider>]: [<$trait_ty ඞServiceProvider>] {
                fn into_service<'s>(self) -> ::std::boxed::Box<dyn $trait_ty + 's> where Self: 's;
            }

            $vis trait [<$trait_ty ඞMutServiceProvider>] {
                fn get_mut_service<'s>(&'s mut self) -> ::std::boxed::Box<dyn $trait_ty + 's>;

                fn [<get_mut_ $name>]<'s>(&'s mut self) -> ::std::boxed::Box<dyn $trait_ty + 's> {
                    self.get_mut_service()
                }
            }
            
            $vis trait [<$trait_ty ඞIntoMutServiceProvider>]: [<$trait_ty ඞMutServiceProvider>] {
                fn into_mut_service<'s>(self) -> ::std::boxed::Box<dyn $trait_ty + 's> where Self: 's;
            }

            $vis trait [<$trait_ty ඞMultipleServiceProvider>] {
                fn get_services<'s>(&'s self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($trait_ty)>;

                fn [<iter_ $name>]<'s>(&'s self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($trait_ty)> {
                    self.get_services()
                }
            }
            
            $vis trait [<$trait_ty ඞIntoMultipleServiceProvider>]: [<$trait_ty ඞMultipleServiceProvider>] {
                fn into_services<'s>(self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($trait_ty)> where Self: 's;
            }

            $vis trait [<$trait_ty ඞMultipleMutServiceProvider>] {
                fn get_mut_services<'s>(&'s mut self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($trait_ty)>;

                fn [<iter_mut_ $name>]<'s>(&'s mut self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($trait_ty)> {
                    self.get_mut_services()
                }
            }
            
            $vis trait [<$trait_ty ඞIntoMultipleMutServiceProvider>]: [<$trait_ty ඞMultipleMutServiceProvider>] {
                fn into_mut_services<'s>(self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($trait_ty)> where Self: 's;
            }

            $crate::def_seq_gat_for_multi_trait!($trait_ty);
            // $vis trait [<$trait_ty ඞIter>] {

            // }

            // $vis struct [<$trait_ty ඞImplsIter>]<'s, TContextProvided> {
            //     pub ctx_provided: [<$ctx ඞProvided>]<TContextProvided>
            //     pub impls: Vec<Box<dyn FnOnce(&'s Self) -> ::std::boxed::Box<dyn ::std::iter::Iterator<Item = ::std::boxed::Box<dyn $trait_ty + 's>> + 's> + 's>>
            // }

            // impl<'s> Iterator for [<$trait_ty ඞImplsIter>]<'s> {
            //     type Item = ::std::boxed::Box<dyn $trait_ty + 's>;

            //     fn next(&mut self) -> Option<Self::Item> {

            //     }
            // }
        }

        // ::paste::paste!{
        //     $vis struct [<$trait_ty ඞServiceGat>];

        //     impl ServiceGat for [<$trait_ty ඞServiceGat>] {
        //         type ServiceProvided<'a> = ::std::boxed::Box<dyn $trait_ty + 'a>;
        //     }
        // }
    };
}

#[macro_export]
macro_rules! service_deps {
    ( $impl_ty:ident : $($trait_list:tt)* ) => {
        ::demuncher::apply_pipe!{
            { $($trait_list)* }
            => ::demuncher::split_by{,}
            => [
                ::demuncher::debrace{}
                => $crate::__split_service_kind{}
                => $crate::__emit_dependant_trait_impl{ impl: $impl_ty }
            ]
            => $crate::__apply_paste{}
        }
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
macro_rules! __is_mut {
    (
        { [mut $($_ignored:tt)*] } => {} => $cont:path{ $($cont_args:tt)* }
    ) => {
        $cont!{ {true} => $($cont_args)* }
    };
    (
        { mut $($_ignored:tt)* } => {} => $cont:path{ $($cont_args:tt)* }
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
macro_rules! __is_where {
    (
        { where } => {} => $cont:path{ $($cont_args:tt)* }
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
macro_rules! __is_multiple {
    (
        { [ $($_ignored:tt)* ] } => {} => $cont:path{ $($cont_args:tt)* }
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
macro_rules! __is_in_braces {
    (
        { { $($_ignored:tt)* } } => {} => $cont:path{ $($cont_args:tt)* }
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
                    { ::demuncher::debrace{} => $($pipe)+ }
                    { ::demuncher::reset{} }
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
        { {extern} {$($service_kind:tt)*} {$($trait_ty:tt)+} {$($trait_provider_ty:tt)+} {$($into_trait_provider_ty:tt)+} {$($extern_deps:tt)*} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { {$($service_kind)*} {$($trait_ty)+} {$($trait_provider_ty)+} {$($into_trait_provider_ty)+} } => $crate::__emit_trait_from_extern_context{
                ctx: $ctx
            } => $($cont_args)*
        }
    };
    (
        { {sub $impl_ty:ty} {$($service_kind:tt)*} {$($trait_ty:tt)+} {$($trait_provider_ty:tt)+} {$($into_trait_provider_ty:tt)+} {$($extern_deps:tt)*} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { {$impl_ty} {$($service_kind)*} {$($trait_ty)+} {$($trait_provider_ty)+} {$($into_trait_provider_ty)+} {$($extern_deps)*} } => $crate::__emit_trait_from_sub_context{
                ctx: $ctx
            } => $($cont_args)*
        }
    };
    (
        { {$impl_ty:ty} {$($service_kind:tt)*} {$($trait_ty:tt)+} {$($trait_provider_ty:tt)+} {$($into_trait_provider_ty:tt)+} {$($extern_deps:tt)*} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { {$impl_ty} {$($service_kind)*} {$($trait_ty)+} {$($trait_provider_ty)+} {$($into_trait_provider_ty)+} {$($extern_deps)*} } => $crate::__emit_trait_impl{
                ctx: $ctx
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
                    { ::demuncher::perhaps_debrace{} => ::demuncher::split_by{,} => ::demuncher::skip_if_empty{} => ::demuncher::embrace{} }
                }
                $(=> ::demuncher::when{
                    { ::demuncher::head{} => ::demuncher::debrace{} => ::demuncher::debrace{} => $($include_pipe)+ }
                    => { ::demuncher::pass{} }
                    else { ::demuncher::reset{} }
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
macro_rules! __emit_trait_with_multiple_impls {
    (
        { {$($service_kind:tt)*} {$($trait_ty:tt)+} {$($trait_provider_ty:tt)+} {$($into_trait_provider_ty:tt)+} { [$($impls:tt)+] } {$($extern_deps:tt)*} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { $($impls)+ }
            => ::demuncher::split_by{,}
            => [
                ::demuncher::debrace{}
                => $crate::__emit_item_impl{
                    ctx: $ctx,
                    ctx_provided: self,
                    local_impls: local_impls,
                    other_impls: other_impls,
                    trait: { $($trait_ty)+ },
                    trait_provider: { $($trait_provider_ty)+ },
                    into_trait_provider: { $($into_trait_provider_ty)+ },
                    service_kind: { $($service_kind)* }
                }
            ]
            => $crate::__emit_trait_multiple_impls{
                ctx: $ctx,
                ctx_provided: self,
                local_impls: local_impls,
                other_impls: other_impls,
                trait: { $($trait_ty)+ },
                trait_provider: { $($trait_provider_ty)+ },
                into_trait_provider: { $($into_trait_provider_ty)+ },
                service_kind: { $($service_kind)* },
                extern_deps: { $($extern_deps)* }
            }
            => $($cont_args)*
        }
    };
    // (
    //     { { $($trait_head:ident)?$(::$trait_tail:ident)* } { [$($impls:tt)+] } } => {
    //         ctx: $ctx:ident
    //     } => $cont:path { $($cont_args:tt)* }
    // ) => {
    //     $cont! {
    //         { $($impls)+ }
    //         => ::demuncher::split_by{,}
    //         => [
    //             ::demuncher::debrace{}
    //             => $crate::__emit_item_impl{
    //                 ctx: $ctx,
    //                 ctx_provided: self,
    //                 local_impls: local_impls,
    //                 other_impls: other_impls,
    //                 trait: $($trait_head)?$(::$trait_tail)*,
    //                 mut: {}
    //             }
    //         ]
    //         => $crate::__emit_trait_multiple_impls{
    //             ctx: $ctx,
    //             ctx_provided: self,
    //             local_impls: local_impls,
    //             other_impls: other_impls,
    //             trait: $($trait_head)?$(::$trait_tail)*
    //         }
    //         => $($cont_args)*
    //     }
    // };
    
    // (
    //     { { mut $($trait:tt)+ } { sub $($impl:tt)+ } } => { ctx: $ctx:ident } => $cont:path { $($cont_args:tt)* }
    // ) => {
    //     $cont! {
    //         { {$($impl)+} {$($trait)+} }
    //         => __emit_trait_from_sub_context{ ctx: $ctx }
    //         => $($cont_args)*
    //     }
    // };
    // (
    //     { { $($trait_head:ident)?$(::$trait_tail:ident)* } { $($impl:tt)+ } } => {
    //         ctx: $ctx:ident
    //     } => $cont:path { $($cont_args:tt)* }
    // ) => {
    //     $cont! {
    //         { {$($impl)+} {$($trait_head)?$(::$trait_tail)*} }
    //         => $crate::__pick_trait_impl{
    //             ctx: $ctx
    //         }
    //         => $($cont_args)*
    //     }
    // };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __path_with_suffix {
    (
        {$last:ident} => {$suffix:ident} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ {[<$last $suffix>]} => $($cont_args)* }
    };
    (
        {::$last:ident} => {$suffix:ident} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ {::[<$last $suffix>]} => $($cont_args)* }
    };
    (
        {$($prefix:ident::)+$last:ident} => {$suffix:ident} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ {$($prefix::)+[<$last $suffix>]} => $($cont_args)* }
    };
    (
        {::$($prefix:ident::)+$last:ident} => {$suffix:ident} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ {::$($prefix::)+[<$last $suffix>]} => $($cont_args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __apply_paste {
    (
        { $($input:tt)* } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { ::paste::paste!{ $($input)* } }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_trait_impl {
    (
        { {$impl_ty:ty} {mut} { $($trait_head:ident)?$(::$trait_tail:ident)* } { $($trait_provider_ty:tt)+ } {$($into_trait_provider_ty:tt)+} {$($($extern_deps:tt)+)?} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_mut_service<'s>(&'s mut self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        let provided_ctx = [<$ctx ඞMutProvided>](
                            ForwardingMutContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_mut_service(provided_ctx)
                    }
                }
                
                impl<TContextProvided> $($into_trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn into_mut_service<'s>(self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's>
                    where Self: 's
                    {
                        ::std::boxed::Box::new(
                            MutServiceProvidedWithContext {
                                data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                                mut_data_getter: |c: &mut $ctx| -> &mut $impl_ty { c.services.get_mut() },
                                ctx_provided: self,
                                _service_phantom: PhantomData
                            }
                        )
                    }
                }

                // impl<'c> [<$ctx ඞIntoMutServiceProvider>]<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                // for $ctx
                // {
                //     fn into_mut_service<NewContextProvided>(new_ctx_provided: [<$ctx ඞMutProvided>]<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                //     where
                //         NewContextProvided: MutContextProvided<Context = $ctx> + 'c,
                //         [<$ctx ඞMutProvided>]<NewContextProvided>: 'c
                //     {
                //         ::std::boxed::Box::new(
                //             MutServiceProvidedWithContext {
                //                 data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                //                 mut_data_getter: |c: &mut $ctx| -> &mut $impl_ty { c.services.get_mut() },
                //                 ctx_provided: new_ctx_provided,
                //                 _service_phantom: PhantomData
                //             }
                //         )
                //     }
                //}
                // impl<'c> ContextMutServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                // for $ctx
                // {
                //     fn into_mut_service<NewContextProvided>(new_ctx_provided: [<$ctx ඞMutProvided>]<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>
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
        { {$impl_ty:ty} {} { $($trait_head:ident)?$(::$trait_tail:ident)* } { $($trait_provider_ty:tt)+ } {$($into_trait_provider_ty:tt)+} {$($($extern_deps:tt)+)?} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        let provided_ctx = [<$ctx ඞProvided>](
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_service(provided_ctx)
                    }
                }
                
                impl<TContextProvided> $($into_trait_provider_ty)+ for [<$ctx ඞProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn into_service<'s>(self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's>
                    where Self: 's
                    {
                        ::std::boxed::Box::new(
                            ServiceProvidedWithContext {
                                data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                                ctx_provided: self,
                                _service_phantom: PhantomData
                            }
                        )
                    }
                }

                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        let provided_ctx = [<$ctx ඞProvided>](
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_service(provided_ctx)
                    }
                }
                
                // impl<TContextProvided> $($into_trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                // where
                //     TContextProvided: MutContextProvided<Context = $ctx>
                // {
                //     fn into_service<'s>(self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's>
                //     where Self: 's
                //     {
                //         ::std::boxed::Box::new(
                //             ServiceProvidedWithContext {
                //                 data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                //                 ctx_provided: self,
                //                 _service_phantom: PhantomData
                //             }
                //         )
                //     }
                // }

                // impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                // where
                //     TContextProvided: MutContextProvided<Context = $ctx>
                // {
                //     fn get_mut_service<'s>(&'s mut self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                //         let provided_ctx = [<$ctx ඞMutProvided>](
                //             ForwardingMutContextProvided {
                //                 forwarded_ctx: self
                //             }
                //         );
                //         ::std::boxed::Box::new(
                //             MutServiceProvidedWithContext {
                //                 data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                //                 mut_data_getter: |c: &mut $ctx| -> &mut $impl_ty { c.services.get_mut() },
                //                 ctx_provided: provided_ctx,
                //                 _service_phantom: PhantomData
                //             }
                //         )
                //     }
                // }


                // impl<'c> [<$ctx ඞIntoServiceProvider>]<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                // for $ctx
                // {
                //     fn into_service<NewContextProvided>(new_ctx_provided: [<$ctx ඞProvided>]<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                //     where
                //         NewContextProvided: ContextProvided<Context = $ctx>,
                //         [<$ctx ඞProvided>]<NewContextProvided>: 'c
                //     {
                //         ::std::boxed::Box::new(
                //             ServiceProvidedWithContext {
                //                 data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                //                 ctx_provided: new_ctx_provided,
                //                 _service_phantom: PhantomData
                //             }
                //         )
                //     }
                // }

                // impl<'c, TContextProvided> ServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c>
                // for [<$ctx ඞMutProvided>]<TContextProvided>
                // where
                //     TContextProvided: MutContextProvided<Context = $ctx>,
                //     Self: 'c
                // {
                //     fn get_service(&'c self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)*<'c> + 'c> {
                //         let provided_ctx = [<$ctx ඞProvided>](
                //             ForwardingContextProvided {
                //                 forwarded_ctx: self
                //             }
                //         );
                //         $ctx::into_service(provided_ctx)
                //     }
                // }
                // impl<'c> ContextServiceProvider<'c, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                // for $ctx
                // {
                //     fn into_service<NewContextProvided>(new_ctx_provided: [<$ctx ඞProvided>]<NewContextProvided>) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>
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
        { {$impl_ty:ty} {mut} { $($trait_head:ident)?$(::$trait_tail:ident)* } { $($trait_provider_ty:tt)+ } {$($into_trait_provider_ty:tt)+} {$($($extern_deps:tt)+)?} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_mut_service<'s>(&'s mut self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        let provided_ctx = [<$ctx ඞMutProvided>](
                            ForwardingMutContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_mut_service(provided_ctx)
                    }
                }

                impl<TContextProvided> $($into_trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn into_mut_service<'s>(self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's>
                    where Self: 's
                    {
                        let sub_ctx = [<$impl_ty ඞMutProvided>](
                            MutSubContextProvidedWithParent {
                                ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                                mut_ctx_getter: |parent: &mut $ctx| -> &mut $impl_ty { parent.sub_contexts.get_mut() },
                                parent_ctx_provided: self,
                                _sub_context_phantom: PhantomData
                            }
                        );
                        $($into_trait_provider_ty)+::into_mut_service(sub_ctx)
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        { {$impl_ty:ty} {} { $($trait_head:ident)?$(::$trait_tail:ident)* } { $($trait_provider_ty:tt)+ } {$($into_trait_provider_ty:tt)+} {$($($extern_deps:tt)+)?} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        let provided_ctx = [<$ctx ඞProvided>](
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_service(provided_ctx)
                    }
                }
                
                impl<TContextProvided> $($into_trait_provider_ty)+ for [<$ctx ඞProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn into_service<'s>(self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's>
                    where Self: 's
                    {
                        let sub_ctx = [<$impl_ty ඞProvided>](
                            SubContextProvidedWithParent {
                                ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                                parent_ctx_provided: self,
                                _sub_context_phantom: PhantomData
                            }
                        );
                        $($into_trait_provider_ty)+::into_service(sub_ctx)
                    }
                }

                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        let provided_ctx = [<$ctx ඞProvided>](
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_service(provided_ctx)
                    }
                }
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_sub_ctx_provider_impl {
    (
        { { $impl_ty:ty } { $($($extern_deps:tt)+)? } { $($($deps:tt)+)? } { $($($no_mut_deps:tt)+)? } } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<TContextProvided> [<$impl_ty ඞSubContextProvider>] for [<$ctx ඞProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_sub_ctx(&self) -> [<$impl_ty ඞProvided>]<impl ContextProvided<Context = $impl_ty $(, ParentContextProvided: $($no_mut_deps)+ )?>> {
                        let provided_ctx = [<$ctx ඞProvided>](
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        [<$impl_ty ඞProvided>](
                            SubContextProvidedWithParent {
                                ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                                parent_ctx_provided: provided_ctx,
                                _sub_context_phantom: PhantomData
                            }
                        )
                    }
                }

                impl<TContextProvided> [<$impl_ty ඞMutSubContextProvider>] for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_mut_sub_ctx(&mut self) -> [<$impl_ty ඞMutProvided>]<impl MutContextProvided<Context = $impl_ty $(, ParentContextProvided: $($deps)+ )?>> {
                        let provided_ctx = [<$ctx ඞMutProvided>](
                            ForwardingMutContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        [<$impl_ty ඞMutProvided>](
                            MutSubContextProvidedWithParent {
                                ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                                mut_ctx_getter: |parent: &mut $ctx| -> &mut $impl_ty { parent.sub_contexts.get_mut() },
                                parent_ctx_provided: provided_ctx,
                                _sub_context_phantom: PhantomData
                            }
                        )
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
        { {mut} { $($trait_head:ident)?$(::$trait_tail:ident)* } { $($trait_provider_ty:tt)+ } {$($into_trait_provider_ty:tt)+} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvidedWithParent<Context = $ctx>
                {
                    fn get_mut_service<'s>(&'s mut self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        let provided_ctx = [<$ctx ඞMutProvided>](
                            ForwardingMutContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_mut_service(provided_ctx)
                    }
                }

                impl<TContextProvided> $($into_trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                {
                    fn into_mut_service<'s>(self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's>
                    where Self: 's
                    {
                        let sub_ctx = [<$impl_ty ඞMutProvided>](
                            MutSubContextProvidedWithParent {
                                ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                                mut_ctx_getter: |parent: &mut $ctx| -> &mut $impl_ty { parent.sub_contexts.get_mut() },
                                parent_ctx_provided: self,
                                _sub_context_phantom: PhantomData
                            }
                        );
                        $($into_trait_provider_ty)+::into_mut_service(sub_ctx)
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        { {} { $($trait_head:ident)?$(::$trait_tail:ident)* } { $($trait_provider_ty:tt)+ } {$($into_trait_provider_ty:tt)+} } => {
            ctx: $ctx:ident
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>,
                    TContextProvided::ParentContextProvided: $($trait_provider_ty)+
                {
                    fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        self.parent_ctx_provided().get_service()
                        // let provided_ctx = [<$ctx ඞProvided>](
                        //     ForwardingContextProvided {
                        //         forwarded_ctx: self
                        //     }
                        // );
                        // $($into_trait_provider_ty)+::into_service(provided_ctx)
                    }
                }

                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>,
                    TContextProvided::ParentContextProvided: $($trait_provider_ty)+
                {
                    fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                        self.parent_ctx_provided().get_service()
                        // let provided_ctx = [<$ctx ඞProvided>](
                        //     ForwardingContextProvided {
                        //         forwarded_ctx: self
                        //     }
                        // );
                        // $($into_trait_provider_ty)+::into_service(provided_ctx)
                    }
                }
                
                // impl<TContextProvided> $($into_trait_provider_ty)+ for [<$ctx ඞProvided>]<TContextProvided>
                // where
                //     TContextProvided: ContextProvided<Context = $ctx>
                // {
                //     fn into_service<'s>(self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's>
                //     where Self: 's
                //     {
                //         let sub_ctx = [<$impl_ty ඞProvided>](
                //             SubContextProvidedWithParent {
                //                 ctx_getter: |parent: &$ctx| -> &$impl_ty { parent.sub_contexts.get() },
                //                 parent_ctx_provided: self,
                //                 _sub_context_phantom: PhantomData
                //             }
                //         );
                //         $($into_trait_provider_ty)+::into_service(sub_ctx)
                //     }
                // }

                // impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                // where
                //     TContextProvided: MutContextProvided<Context = $ctx>
                // {
                //     fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 's> {
                //         let provided_ctx = [<$ctx ඞProvided>](
                //             ForwardingContextProvided {
                //                 forwarded_ctx: self
                //             }
                //         );
                //         $($into_trait_provider_ty)+::into_service(provided_ctx)
                //     }
                // }
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
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: { $($trait_ty:tt)+ },
            trait_provider: { $($trait_provider_ty:tt)+ },
            into_trait_provider: { $($into_trait_provider_ty:tt)+ },
            service_kind: {}
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                $local_impls.push(
                    Box::new(|cp| {
                        Box::new(
                            ServiceProvidedWithContext {
                                ctx_provided: [<$ctx ඞProvided>](
                                    ForwardingContextProvided {
                                        forwarded_ctx: cp
                                    }
                                ),
                                data_getter: |c: &$ctx| -> &$impl_ty { c.services.get() },
                                _service_phantom: PhantomData
                            }
                        )
                    })
                );
            }
            => $($cont_args)*
        }
    };
    (
        { $impl_ty:ty } => {
            ctx: $ctx:ident,
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: { $($trait_ty:tt)+ },
            trait_provider: { $($trait_provider_ty:tt)+ },
            into_trait_provider: { $($into_trait_provider_ty:tt)+ },
            service_kind: { mut }
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                $local_impls.push(
                    Box::new(
                        MutServiceProvidedWithContext {
                            ctx_provided: [<$ctx ඞProvided>](
                                MutForwardingContextProvided {
                                    forwarded_ctx: $ctx_provided
                                }
                            ),
                            data_getter: |c: &mut $ctx| -> &mut $impl_ty { c.services.get_mut() },
                            _service_phantom: PhantomData
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
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: { $($trait_ty:tt)+ },
            trait_provider: { $($trait_provider_ty:tt)+ },
            into_trait_provider: { $($into_trait_provider_ty:tt)+ },
            service_kind: { $($service_kind:tt)* }
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            {
                $other_impls.push(
                    Box::new(
                        move |ncp: &mut Self| {
                            let sub_ctx_provided = [<$sub_ctx_ty ඞProvided>](
                                SubContextProvidedWithParent {
                                    ctx_getter: |parent: &$ctx| -> &$sub_ctx_ty { parent.sub_contexts.get() },
                                    parent_ctx_provided: [<$ctx ඞProvided>](ForwardingContextProvided {
                                        forwarded_ctx: ncp
                                    }),
                                    _sub_context_phantom: PhantomData
                                }
                            );
                            sub_ctx_provided.into_services().as_dyn()
                        }
                    )
                );
                // {
                //     let sub_ctx_provided = [<$sub_ctx_ty ඞProvided>](
                //         SubContextProvidedWithParent {
                //             ctx_getter: |parent: &$ctx| -> &$sub_ctx_ty { parent.sub_contexts.get() },
                //             parent_ctx_provided: ForwardingContextProvided {
                //                 forwarded_ctx: $ctx_provided
                //             },
                //             _sub_context_phantom: PhantomData
                //         }
                //     );
                //     $local_impls.extend(sub_ctx_provided.get_services())
                // }
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
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: { $($trait_ty:tt)+ },
            trait_provider: { $($trait_provider_ty:tt)+ },
            into_trait_provider: { $($into_trait_provider_ty:tt)+ },
            service_kind: { mut },
            extern_deps: { $($($extern_deps:tt)+)? }
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<'c, TContextProvided> ContextMultipleMutServiceProvider<'c, [<$ctx ඞProvided>]<TContextProvided>, dyn $($trait_head)?$(::$trait_tail)* + 'c>
                    for $ctx
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_mut_services($ctx_provided: &'c mut [<$ctx ඞProvided>]<TContextProvided>) -> ::std::vec::Vec<::std::boxed::Box<dyn $($trait_head)?$(::$trait_tail)* + 'c>> {
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
            ctx_provided: $ctx_provided:ident,
            local_impls: $local_impls:ident,
            other_impls: $other_impls:ident,
            trait: { $($trait_ty:tt)+ },
            trait_provider: { $($trait_provider_ty:tt)+ },
            into_trait_provider: { $($into_trait_provider_ty:tt)+ },
            service_kind: {},
            extern_deps: { $($($extern_deps:tt)+)? }
        } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont! {
            { 
                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_services<'s>(&'s self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($($trait_ty)+)> {
                        let provided_ctx = [<$ctx ඞProvided>](
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_services(provided_ctx)
                    }
                }
                
                impl<TContextProvided> $($into_trait_provider_ty)+ for [<$ctx ඞProvided>]<TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn into_services<'s>(mut $ctx_provided) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($($trait_ty)+)>
                    where Self: 's
                    {
                        // let mut $local_impls: Vec<Box<dyn for<'a> FnOnce(&'a Self) -> ::std::boxed::Box<dyn $($trait_ty)+ + 'a>>> = vec![];
                        // //let mut $local_impls: Vec<Box<dyn $($trait_ty)+ + 's>> = vec![];
                        // let mut $other_impls: Vec<Box<dyn for<'a> FnOnce(&'a Self) -> ::std::boxed::Box<dyn ::std::iter::Iterator<Item = ::std::boxed::Box<dyn $($trait_ty)+ + 's>> + 's> + 's>> = vec![];
                        // $($impls)*
                        // $other_impls.push(::std::boxed::Box::new(move |cp| {
                        //     ::std::boxed::Box::new(
                        //         $local_impls.into_iter().map(move |f| {
                        //             f(cp)
                        //         })
                        //     )
                        // }));
                        // $other_impls.into_iter().flat_map(move |f| {
                        //     f(&$ctx_provided)
                        // })
                        use ::std::{vec::Vec, vec, boxed::Box};
                        let mut $local_impls:
                            Vec<
                                Box<
                                    dyn for<'a> Fn(&'a mut Self) -> Box<dyn $($trait_ty)+ + 'a>
                                >
                            >
                            = vec![];
                        let mut $other_impls:
                            Vec<
                                Box<
                                    dyn for<'a> FnOnce(&'a mut Self) -> Box<dyn DynSeq<Gat = $crate::seq_gat_for_multi_trait!($($trait_ty)+)> + 'a>
                                >
                            >
                            = vec![];
                        $($impls)*
                        $other_impls.push(
                            Box::new(
                                move |ncp: &mut Self| {
                                    $local_impls
                                        .into_seq()
                                        .wrap(|c, next: &mut dyn Emit<$crate::seq_gat_for_multi_trait!($($trait_ty)+)>| {
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
                                move |c, next: &mut dyn Emit<Boxed<$crate::seq_gat_for_multi_trait!($($trait_ty)+)>>| {
                                    let sub_seq = (c)(&mut $ctx_provided);
                                    next.emit(sub_seq)
                                }
                            )
                            .flatten()
                    }
                }

                impl<TContextProvided> $($trait_provider_ty)+ for [<$ctx ඞMutProvided>]<TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    $(, TContextProvided::ParentContextProvided: $($extern_deps)+)?
                {
                    fn get_services<'s>(&'s self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($($trait_ty)+)> {
                        let provided_ctx = [<$ctx ඞProvided>](
                            ForwardingContextProvided {
                                forwarded_ctx: self
                            }
                        );
                        $($into_trait_provider_ty)+::into_services(provided_ctx)
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
macro_rules! __prepend_service_kind {
    (
        { [mut $($trait_ty:tt)+] } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { { [mut] } { [mut $($trait_ty)+] } } => $($cont_args)* }
    };
    (
        { [$($trait_ty:tt)+] } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { { [] } { [$($trait_ty)+] } } => $($cont_args)* }
    };
    (
        { mut $($trait_ty:tt)+ } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { { mut } { mut $($trait_ty)+ } } => $($cont_args)* }
    };
    (
        { $($trait_ty:tt)+ } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { {} { $($trait_ty)+ } } => $($cont_args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __split_service_kind {
    (
        { [mut $($trait_ty:tt)+] } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { { [mut] } { $($trait_ty)+ } } => $($cont_args)* }
    };
    (
        { [$($trait_ty:tt)+] } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { { [] } { $($trait_ty)+ } } => $($cont_args)* }
    };
    (
        { mut $($trait_ty:tt)+ } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { { mut } { $($trait_ty)+ } } => $($cont_args)* }
    };
    (
        { $($trait_ty:tt)+ } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { {} { $($trait_ty)+ } } => $($cont_args)* }
    };
    (
        { mut $($trait_ty:tt)+ } => {multiple} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { { [mut] } { $($trait_ty)+ } } => $($cont_args)* }
    };
    (
        { $($trait_ty:tt)+ } => {multiple} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{ { { [] } { $($trait_ty)+ } } => $($cont_args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __repeat_with_suffix {
    (
        { { $($trait_ty:tt)+ } } => { $suffix:ident } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { $($trait_ty)+ }
            => ::demuncher::repeat{ {} {} }
            => ::demuncher::fork{
                { ::demuncher::pass{} }
                { ::demuncher::debrace{} => $crate::__path_with_suffix{ $suffix } => ::demuncher::embrace{} }
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __append_service_provider {
    (
        { { [mut] } { $($trait_ty:tt)+ } } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { { mut } { $($trait_ty)+ } }
            => ::demuncher::fork{
                { ::demuncher::pass{} }
                {
                    ::demuncher::repeat{ 1 2 }
                    => ::demuncher::fork {
                        { $crate::__repeat_with_suffix{ ඞMultipleMutServiceProvider } }
                        { ::demuncher::debrace{} => $crate::__path_with_suffix{ ඞIntoMultipleMutServiceProvider } => ::demuncher::embrace{} }
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        { { [] } { $($trait_ty:tt)+ } } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { {} { $($trait_ty)+ } }
            => ::demuncher::fork{
                { ::demuncher::pass{} }
                {
                    ::demuncher::repeat{ 1 2 }
                    => ::demuncher::fork {
                        { $crate::__repeat_with_suffix{ ඞMultipleServiceProvider } }
                        { ::demuncher::debrace{} => $crate::__path_with_suffix{ ඞIntoMultipleServiceProvider } => ::demuncher::embrace{} }
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        { { mut } { $($trait_ty:tt)+ } } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { { mut } { $($trait_ty)+ } }
            => ::demuncher::fork{
                { ::demuncher::pass{} }
                {
                    ::demuncher::repeat{ 1 2 }
                    => ::demuncher::fork {
                        { $crate::__repeat_with_suffix{ ඞMutServiceProvider } }
                        { ::demuncher::debrace{} => $crate::__path_with_suffix{ ඞIntoMutServiceProvider } => ::demuncher::embrace{} }
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        { {} { $($trait_ty:tt)+ } } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { {} { $($trait_ty)+ } }
            => ::demuncher::fork{
                { ::demuncher::pass{} }
                {
                    ::demuncher::repeat{ 1 2 }
                    => ::demuncher::fork {
                        { $crate::__repeat_with_suffix{ ඞServiceProvider } }
                        { ::demuncher::debrace{} => $crate::__path_with_suffix{ ඞIntoServiceProvider } => ::demuncher::embrace{} }
                    }
                }
            }
            => $($cont_args)*
        }
    };
}
#[macro_export]
#[doc(hidden)]
macro_rules! __emit_dependant_trait_constraint {
    (
        { [mut $($dep_trait:tt)+] } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { $($dep_trait)+ }
            => $crate::__path_with_suffix{ ඞMultipleMutServiceProvider }
            => $($cont_args)*
        }
    };
    (
        { [$($dep_trait:tt)+] } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { $($dep_trait)+ }
            => $crate::__path_with_suffix{ ඞMultipleServiceProvider }
            => $($cont_args)*
        }
    };
    (
        { mut $($dep_trait:tt)+ } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { $($dep_trait)+ }
            => $crate::__path_with_suffix{ ඞMutServiceProvider }
            => $($cont_args)*
        }
    };
    (
        { $($dep_trait:tt)+ } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { $($dep_trait)+ }
            => $crate::__path_with_suffix{ ඞServiceProvider }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_dependant_trait_impl {
    (
        { { [mut] } { $($dep_trait:tt)+ } } => { impl: $impl_ty:path } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { { $($dep_trait)+ } }
            => $crate::__repeat_with_suffix{ ඞMultipleMutServiceProvider }
            => __emit_dependant_multiple_mut_service_trait_impl{ impl: $impl_ty }
            => $($cont_args)*
        }
    };
    (
        { { [] } { $($dep_trait:tt)+ } } => { impl: $impl_ty:path } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { { $($dep_trait)+ } }
            => $crate::__repeat_with_suffix{ ඞMultipleServiceProvider }
            => __emit_dependant_multiple_service_trait_impl{ impl: $impl_ty }
            => $($cont_args)*
        }
    };
    (
        { { mut } { $($dep_trait:tt)+ } } => { impl: $impl_ty:path } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { { $($dep_trait)+ } }
            => $crate::__repeat_with_suffix{ ඞMutServiceProvider }
            => __emit_dependant_mut_service_trait_impl{ impl: $impl_ty }
            => $($cont_args)*
        }
    };
    (
        { {} { $($dep_trait:tt)+ } } => { impl: $impl_ty:path } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { { $($dep_trait)+ } }
            => $crate::__repeat_with_suffix{ ඞServiceProvider }
            => __emit_dependant_service_trait_impl{ impl: $impl_ty }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_dependant_service_trait_impl {
    (
        { { $($dep_trait:tt)+ } { $($dep_trait_provider:tt)+ } } => { impl: $impl_ty:path } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                impl<TContextProvided, DataGetter> $($dep_trait_provider)+
                for ServiceProvidedWithContext<$impl_ty, TContextProvided, DataGetter>
                where
                    TContextProvided: ContextProvided + $($dep_trait_provider)+,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
                {
                    fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($dep_trait)+ + 's> {
                        self.ctx_provided.get_service()
                    }
                }
                
                impl<TContextProvided, MutDataGetter, DataGetter> $($dep_trait_provider)+
                for MutServiceProvidedWithContext<$impl_ty, TContextProvided, MutDataGetter, DataGetter>
                where
                    TContextProvided: MutContextProvided + $($dep_trait_provider)+,
                    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut $impl_ty,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
                {
                    fn get_service<'s>(&'s self) -> ::std::boxed::Box<dyn $($dep_trait)+ + 's> {
                        self.ctx_provided.get_service()
                    }
                }
            } => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_dependant_mut_service_trait_impl {
    (
        { { $($dep_trait:tt)+ } { $($dep_trait_provider:tt)+ } } => { impl: $impl_ty:path } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                impl<TContextProvided, MutDataGetter, DataGetter> $($dep_trait_provider)+
                for MutServiceProvidedWithContext<$impl_ty, TContextProvided, MutDataGetter, DataGetter>
                where
                    TContextProvided: MutContextProvided,
                    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut $impl_ty,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
                {
                    fn get_mut_service<'s>(&'s mut self) -> ::std::boxed::Box<dyn $($dep_trait)+ + 's> {
                        self.ctx_provided.get_mut_service()
                    }
                }
            } => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_dependant_multiple_service_trait_impl {
    (
        { { $($dep_trait:tt)+ } { $($dep_trait_provider:tt)+ } } => { impl: $impl_ty:path } => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                impl<TContextProvided, DataGetter> $($dep_trait_provider)+
                for ServiceProvidedWithContext<$impl_ty, TContextProvided, DataGetter>
                where
                    TContextProvided: ContextProvided + $($dep_trait_provider)+,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
                {
                    fn get_services<'s>(&'s self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($($dep_trait)+)> {
                        self.ctx_provided.get_services()
                    }
                }
                
                impl<TContextProvided, MutDataGetter, DataGetter> $($dep_trait_provider)+
                for MutServiceProvidedWithContext<$impl_ty, TContextProvided, MutDataGetter, DataGetter>
                where
                    TContextProvided: MutContextProvided + $($dep_trait_provider)+,
                    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut $impl_ty,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
                {
                    fn get_services<'s>(&'s self) -> impl Seq<'s, ItemGat = $crate::seq_gat_for_multi_trait!($($dep_trait)+)> {
                        self.ctx_provided.get_services()
                    }
                }
            } => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __split_deps_and_def {
    (
        { $($input:tt)+ } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { $($input)+ }
            => ::demuncher::split_prefix_until{ $crate::__is_in_braces{} }
            => ::demuncher::fork{
                {
                    ::demuncher::debrace{}
                    => ::demuncher::split_by{+}
                    => [
                        ::demuncher::debrace{}
                        => $crate::__emit_dependant_trait_constraint{}
                        => ::demuncher::embrace{}
                    ]
                    => ::demuncher::join_with{+}
                    => ::demuncher::embrace{}
                }
                { ::demuncher::debrace{} }
            }
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_impl_service {
    (
        { {$($dependant_trait_constraints:tt)+} { $($def:tt)* } }
        => { impl: $impl_ty:path, trait: $($trait_head:ident)?$(::$trait_tail:ident)* }
         => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                impl<'c, TContextProvided, DataGetter> $($trait_head)?$(::$trait_tail)*
                    for ServiceProvidedWithContext<$impl_ty, TContextProvided, DataGetter>
                where
                    TContextProvided: ContextProvided + $($dependant_trait_constraints)+,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
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
        { {$($dependant_trait_constraints:tt)+} { $($def:tt)* } }
        => { impl: $impl_ty:path, trait: $($trait_head:ident)?$(::$trait_tail:ident)* }
         => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                impl<'c, TContextProvided, MutDataGetter, DataGetter> $($trait_head)?$(::$trait_tail)*
                    for MutServiceProvidedWithContext<$impl_ty, TContextProvided, MutDataGetter, DataGetter>
                where
                    TContextProvided: MutContextProvided + $($dependant_trait_constraints)+,
                    MutDataGetter: Fn(&mut TContextProvided::Context) -> &mut $impl_ty,
                    DataGetter: Fn(&TContextProvided::Context) -> &$impl_ty
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
macro_rules! __emit_extern_deps {
    (
        { { $($impls:tt)* } { $($traits:tt)* } } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { { $($impls)* } { $($traits)* } }
            => ::demuncher::fork{
                {
                    ::demuncher::debrace{}
                    => $crate::__for_each_impl_with_trait{
                        { __is_extern{} } => {
                            ::demuncher::tail{}
                            => ::demuncher::debrace{}
                            => ::demuncher::split_by{,}
                            => [
                                ::demuncher::debrace{}
                                => $crate::__emit_dependant_trait_constraint{}
                                => ::demuncher::embrace{}
                            ]
                        }
                    }
                }
                {
                    ::demuncher::debrace{}
                    => $crate::__for_each_trait_statement{
                        ::demuncher::when{
                            { ::demuncher::tail{} => ::demuncher::debrace{} => $crate::__is_multiple{} }=> {
                                ::demuncher::when{
                                    {
                                        ::demuncher::tail{}
                                        => ::demuncher::debrace{}
                                        => ::demuncher::strip_brackets{}
                                        => ::demuncher::split_by{,}
                                        => ::demuncher::any{ ::demuncher::debrace{} => $crate::__is_extern{} }
                                    } => {
                                        ::demuncher::head{}
                                        => ::demuncher::debrace{}
                                        => ::demuncher::in_brackets{}
                                        => $crate::__emit_dependant_trait_constraint{}
                                        => ::demuncher::embrace{}
                                    } else {
                                        ::demuncher::reset{}
                                    }
                                }
                            } else {
                                ::demuncher::when{
                                    { ::demuncher::tail{} => ::demuncher::debrace{} => $crate::__is_extern{} } => {
                                        ::demuncher::head{}
                                        => ::demuncher::debrace{}
                                        => $crate::__emit_dependant_trait_constraint{}
                                        => ::demuncher::embrace{}
                                    } else {
                                        ::demuncher::reset{}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            => ::demuncher::skip_all_empty{}
            => ::demuncher::join_with{+}
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_deps_all_along_no_mut {
    (
        { { $($impls:tt)* } { $($traits:tt)* } } => {} => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            { { $($impls)* } { $($traits)* } }
            => ::demuncher::fork{
                {
                    ::demuncher::debrace{}
                    => $crate::__for_each_impl_with_trait{
                        {} => {
                            ::demuncher::tail{}
                            => ::demuncher::debrace{}
                            => ::demuncher::split_by{,}
                            => [
                                ::demuncher::debrace{}
                                => ::demuncher::when{
                                    { $crate::__is_mut{} } => {
                                        $crate::__emit_dependant_trait_constraint{}
                                        => ::demuncher::embrace{}
                                    } else {
                                        $crate::__emit_dependant_trait_constraint{}
                                        => ::demuncher::repeat{ {} {} }
                                    }
                                }
                                => ::demuncher::embrace{}
                            ]
                        }
                    }
                }
                {
                    ::demuncher::debrace{}
                    => $crate::__for_each_trait_statement{
                        ::demuncher::when{
                            { ::demuncher::tail{} => ::demuncher::debrace{} => $crate::__is_multiple{} }=> {
                                ::demuncher::head{} => ::demuncher::debrace{} => ::demuncher::in_brackets{}
                            } else {
                                ::demuncher::head{} => ::demuncher::debrace{}
                            }
                        }
                        => ::demuncher::when{
                            { $crate::__is_mut{} } => {
                                $crate::__emit_dependant_trait_constraint{}
                                => ::demuncher::embrace{}
                            } else {
                                $crate::__emit_dependant_trait_constraint{}
                                => ::demuncher::repeat{ {} {} }
                            }
                        }
                        => ::demuncher::embrace{}
                    }
                }
            }
            => ::demuncher::transpose{}
            => [
                ::demuncher::debrace{}
                => ::demuncher::skip_all_empty{}
                => ::demuncher::join_with{+}
                => ::demuncher::embrace{}
            ]
            => $($cont_args)*
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_ctx_provided {
    (
        { $($dependant_trait_constraints:tt)+ }
        => { vis: $vis:vis, ctx: $ctx:ident }
        => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                $vis struct [<$ctx ඞProvided>] <TContextProvided> (TContextProvided)
                where
                    TContextProvided: ContextProvided<Context = $ctx>,
                    TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+;

                impl<TContextProvided> ContextProvided for [<$ctx ඞProvided>] <TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    , TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+
                {
                    type Context = $ctx;
                    type ParentContextProvided = TContextProvided::ParentContextProvided;

                    fn ctx(&self) -> &Self::Context {
                        self.0.ctx()
                    }
                    
                    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
                        self.0.parent_ctx_provided()
                    }
                }

                $vis struct [<$ctx ඞMutProvided>] <TContextProvided> (TContextProvided)
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    , TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+;

                impl<TContextProvided> ContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    , TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+
                {
                    type Context = $ctx;
                    type ParentContextProvided = TContextProvided::ParentContextProvided;

                    fn ctx(&self) -> &Self::Context {
                        self.0.ctx()
                    }
                    
                    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
                        self.0.parent_ctx_provided()
                    }
                }

                impl<TContextProvided> MutContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    , TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+
                {
                    type ParentMutContextProvided = TContextProvided::ParentMutContextProvided;

                    fn mut_ctx(&mut self) -> &mut Self::Context {
                        self.0.mut_ctx()
                    }
        
                    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided {
                        self.0.parent_mut_ctx_provided()
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        {}
        => { vis: $vis:vis, ctx: $ctx:ident }
        => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                $vis struct [<$ctx ඞProvided>] <TContextProvided> (TContextProvided)
                where
                    TContextProvided: ContextProvided<Context = $ctx>;

                impl<TContextProvided> ContextProvided for [<$ctx ඞProvided>] <TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                {
                    type Context = $ctx;
                    type ParentContextProvided = TContextProvided::ParentContextProvided;

                    fn ctx(&self) -> &Self::Context {
                        self.0.ctx()
                    }
                    
                    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
                        self.0.parent_ctx_provided()
                    }
                }

                $vis struct [<$ctx ඞMutProvided>] <TContextProvided> (TContextProvided)
                where
                    TContextProvided: MutContextProvided<Context = $ctx>;

                impl<TContextProvided> ContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                {
                    type Context = $ctx;
                    type ParentContextProvided = TContextProvided::ParentContextProvided;

                    fn ctx(&self) -> &Self::Context {
                        self.0.ctx()
                    }
                    
                    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
                        self.0.parent_ctx_provided()
                    }
                }

                impl<TContextProvided> MutContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                {
                    type ParentMutContextProvided = TContextProvided::ParentMutContextProvided;

                    fn mut_ctx(&mut self) -> &mut Self::Context {
                        self.0.mut_ctx()
                    }
        
                    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided {
                        self.0.parent_mut_ctx_provided()
                    }
                }

                impl $ctx {
                    pub fn provide(&self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx>> {
                        [<$ctx ඞProvided>](
                            StartContextProvided {
                                start_ctx: self
                            }
                        )
                    }

                    pub fn provide_mut(&mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx>> {
                        [<$ctx ඞMutProvided>](
                            StartMutContextProvided {
                                start_ctx: self
                            }
                        )
                    }

                }
            }
            => $($cont_args)*
        }
    };
    (
        { $($dependant_trait_constraints:tt)+ }
        => { vis: $vis:vis, ctx: $ctx:ident, ctx_label: $ctx_label:ident }
        => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                $vis struct [<$ctx ඞProvided>] <TContextProvided> (TContextProvided)
                where
                    TContextProvided: ContextProvided<Context = $ctx>,
                    TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+;

                impl<TContextProvided> ContextProvided for [<$ctx ඞProvided>] <TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                    , TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+
                {
                    type Context = $ctx;
                    type ParentContextProvided = TContextProvided::ParentContextProvided;

                    fn ctx(&self) -> &Self::Context {
                        self.0.ctx()
                    }
                    
                    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
                        self.0.parent_ctx_provided()
                    }
                }

                $vis struct [<$ctx ඞMutProvided>] <TContextProvided> (TContextProvided)
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    , TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+;

                impl<TContextProvided> ContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    , TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+
                {
                    type Context = $ctx;
                    type ParentContextProvided = TContextProvided::ParentContextProvided;

                    fn ctx(&self) -> &Self::Context {
                        self.0.ctx()
                    }
                    
                    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
                        self.0.parent_ctx_provided()
                    }
                }

                impl<TContextProvided> MutContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                    , TContextProvided::ParentContextProvided: $($dependant_trait_constraints)+
                {
                    type ParentMutContextProvided = TContextProvided::ParentMutContextProvided;

                    fn mut_ctx(&mut self) -> &mut Self::Context {
                        self.0.mut_ctx()
                    }
        
                    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided {
                        self.0.parent_mut_ctx_provided()
                    }
                }

                $vis trait [<$ctx ඞSubContextProvider>] {
                    fn get_sub_ctx(&self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx, ParentContextProvided: $($dependant_trait_constraints)+>>;

                    fn [<get_ $ctx_label>](&self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx, ParentContextProvided: $($dependant_trait_constraints)+>> {
                        self.get_sub_ctx()
                    }
                }

                $vis trait [<$ctx ඞMutSubContextProvider>] {
                    fn get_mut_sub_ctx(&mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx, ParentContextProvided: $($dependant_trait_constraints)+>>;

                    fn [<get_mut_ $ctx_label>](&mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx, ParentContextProvided: $($dependant_trait_constraints)+>> {
                        self.get_mut_sub_ctx()
                    }
                }
            }
            => $($cont_args)*
        }
    };
    (
        {}
        => { vis: $vis:vis, ctx: $ctx:ident, ctx_label: $ctx_label:ident }
        => $cont:path { $($cont_args:tt)* }
    ) => {
        $cont!{
            {
                $vis struct [<$ctx ඞProvided>] <TContextProvided> (TContextProvided)
                where
                    TContextProvided: ContextProvided<Context = $ctx>;

                impl<TContextProvided> ContextProvided for [<$ctx ඞProvided>] <TContextProvided>
                where
                    TContextProvided: ContextProvided<Context = $ctx>
                {
                    type Context = $ctx;
                    type ParentContextProvided = TContextProvided::ParentContextProvided;

                    fn ctx(&self) -> &Self::Context {
                        self.0.ctx()
                    }
                    
                    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
                        self.0.parent_ctx_provided()
                    }
                }

                $vis struct [<$ctx ඞMutProvided>] <TContextProvided> (TContextProvided)
                where
                    TContextProvided: MutContextProvided<Context = $ctx>;

                impl<TContextProvided> ContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                {
                    type Context = $ctx;
                    type ParentContextProvided = TContextProvided::ParentContextProvided;

                    fn ctx(&self) -> &Self::Context {
                        self.0.ctx()
                    }
                    
                    fn parent_ctx_provided(&self) -> &Self::ParentContextProvided {
                        self.0.parent_ctx_provided()
                    }
                }

                impl<TContextProvided> MutContextProvided for [<$ctx ඞMutProvided>] <TContextProvided>
                where
                    TContextProvided: MutContextProvided<Context = $ctx>
                {
                    type ParentMutContextProvided = TContextProvided::ParentMutContextProvided;

                    fn mut_ctx(&mut self) -> &mut Self::Context {
                        self.0.mut_ctx()
                    }
        
                    fn parent_mut_ctx_provided(&mut self) -> &mut Self::ParentMutContextProvided {
                        self.0.parent_mut_ctx_provided()
                    }
                }

                impl $ctx {
                    pub fn provide(&self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx>> {
                        [<$ctx ඞProvided>](
                            StartContextProvided {
                                start_ctx: self
                            }
                        )
                    }

                    pub fn provide_mut(&mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx>> {
                        [<$ctx ඞMutProvided>](
                            StartMutContextProvided {
                                start_ctx: self
                            }
                        )
                    }

                }

                $vis trait [<$ctx ඞSubContextProvider>] {
                    fn get_sub_ctx(&self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx>>;

                    fn [<get_ $ctx_label>](&self) -> [<$ctx ඞProvided>]<impl ContextProvided<Context = $ctx>> {
                        self.get_sub_ctx()
                    }
                }

                $vis trait [<$ctx ඞMutSubContextProvider>] {
                    fn get_mut_sub_ctx(&mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx>>;

                    fn [<get_mut_ $ctx_label>](&mut self) -> [<$ctx ඞMutProvided>]<impl MutContextProvided<Context = $ctx>> {
                        self.get_mut_sub_ctx()
                    }
                }
            }
            => $($cont_args)*
        }
    };
}

#[cfg(test)]
mod tests {
    use std::{cell::{Ref, RefCell}};

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
            let cs = ctx_provided.get_calling_service();
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
        assert_eq!(*context.provide().get_logged_service().get_calls(), expected_calls);
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
        (context.provide_mut().get_mut_setter_multi_service() as Box<dyn SetterMultiService>).set_value(44);
        (context.provide().get_calling_service() as Box<dyn CallingService>).call_service(2);
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
        assert_eq!(*(context.provide().get_logged_service() as Box<dyn LoggedService>).get_calls(), expected_calls);
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
        context.provide_mut().get_mut_called_sub_ctx().get_mut_setter_multi_service().set_value(44);
        context.provide().get_calling_service().call_service(2);
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
        assert_eq!(*context.provide().get_called_sub_ctx().get_logged_service().get_calls(), expected_calls);
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
        ServiceCallsImplementationsFromSubContextCalledContext as called_sub_ctx {
            CalledMultiServiceImpl2: mut SetterMultiService;
            LoggingServiceImpl1: LoggingService, LoggedService
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl2]
        }
    }
    //trace_macros!(false);

    #[test]
    fn service_calls_implementations_from_parent_context() {
        let mut context = ServiceCallsImplementationsFromParentContextCallingContext::default();
        context.provide_mut().get_mut_setter_multi_service().set_value(44);
        context.provide().get_calling_service().call_service(2);
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
        assert_eq!(*context.provide_mut().get_logged_service().get_calls(), expected_calls);
    }

    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsImplementationsFromParentContextCallingContext {
            CallingMultipleServiceImpl1: CallingService;
            CalledMultiServiceImpl1;
            LoggingServiceImpl1: LoggingService, LoggedService;
            sub ServiceCallsImplementationsFromParentContextCalledContext: mut SetterMultiService;
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl1, sub ServiceCallsImplementationsFromParentContextCalledContext]
        }
    }

    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsImplementationsFromParentContextCalledContext as called_sub_ctx  {
            CalledMultiServiceImpl2: mut SetterMultiService;
            extern: LoggingService;
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl2];
        }
    }

    #[test]
    fn service_calls_implementations_between_3_levels_of_contexts() {
        let mut context = ServiceCallsImplementationsBetween3LevelsOfContextsTop::default();
        context.provide_mut().get_mut_setter_multi_service().set_value(44);
        context.provide_mut().get_mut_called_middle().get_mut_called_bottom().get_mut_additional_setter_service().set_value(3);
        context.provide_mut().get_mut_called_id_setter_service().set_id(5);
        context.provide().get_calling_service().call_service(2);
        let expected_calls = vec![
            "+setter_multi_service(44)",
            "-setter_multi_service(44)",
            "+additional_setter_service(3)",
            "-additional_setter_service(3)",
            "+called_id_setter_service(5)",
            "-called_id_setter_service(5)",
            "+calling_service(2)",
            "+called_multi_service3(0, 5)",
            "+additional_service(5, 3)",
            "-additional_service(5, 3)",
            "-called_multi_service3(0, 5)",
            "+called_multi_service(1, 44)",
            "-called_multi_service(1, 44)",
            "+called_multi_service(2)",
            "-called_multi_service(2)",
            "-calling_service(2)"
        ];
        assert_eq!(*context.provide_mut().get_logged_service().get_calls(), expected_calls);
    }

    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsImplementationsBetween3LevelsOfContextsTop {
            CallingMultipleServiceImpl1: CallingService;
            CalledMultiServiceImpl1;
            LoggingServiceImpl1: LoggingService, LoggedService;
            sub ServiceCallsImplementationsBetween3LevelsOfContextsMiddle: mut SetterMultiService, mut CalledIdSetterService;
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl1, sub ServiceCallsImplementationsBetween3LevelsOfContextsMiddle]
        }
    }

    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsImplementationsBetween3LevelsOfContextsMiddle as called_middle  {
            CalledMultiServiceImpl2: mut SetterMultiService;
            extern: LoggingService;
            sub ServiceCallsImplementationsBetween3LevelsOfContextsBottom: mut CalledIdSetterService;
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl2, sub ServiceCallsImplementationsBetween3LevelsOfContextsBottom];
        }
    }

    service_context! {
        #[derive(Debug, Default)]
        ServiceCallsImplementationsBetween3LevelsOfContextsBottom as called_bottom  {
            CalledMultiServiceImpl3: mut CalledIdSetterService;
            extern: LoggingService;
            AdditionalServiceImpl1: AdditionalService, mut AdditionalSetterService;
        }
        traits {
            CalledMultiService => [CalledMultiServiceImpl3];
        }
    }

    #[derive(Debug, Default)]
    struct CallingMultipleServiceImpl1;

    service_deps!(CallingMultipleServiceImpl1: LoggingService, [CalledMultiService]);

    impl_service! {
        impl CallingService for CallingMultipleServiceImpl1 where deps: LoggingService + [CalledMultiService]
        {
            fn call_service(&self, id: i32) {
                let logging: Box<dyn LoggingService> = self.get_logging_service();
                logging.log(&format!("+calling_service({id})"));
                self.iter_called_multi_service().enumerate().for_each(|(idx, service)| {
                    (*(service as Box<dyn CalledMultiService>)).call_service(idx);
                });
                // for (idx, service) in self.get_services().enumerate() {
                //     (*(service as Box<dyn CalledMultiService>)).call_service(idx);
                // }
                logging.log(&format!("-calling_service({id})"));
            }
        }
    }

    def_service!{
        trait CalledMultiService as called_multi_service {
            fn call_service(&self, idx: usize);
        }
    }

    #[derive(Debug, Default)]
    struct CalledMultiServiceImpl1;

    service_deps!(CalledMultiServiceImpl1: LoggingService);

    impl_service! {
        impl CalledMultiService for CalledMultiServiceImpl1 where deps: LoggingService
        {
            fn call_service(&self, idx: usize) {
                let logging: Box<dyn LoggingService> = self.get_logging_service();
                logging.log(&format!("+called_multi_service({idx})"));
                logging.log(&format!("-called_multi_service({idx})"));
            }
        }
    }

    def_service!{
        trait SetterMultiService as setter_multi_service {
            fn set_value(&mut self, value: i32);
        }
    }

    #[derive(Debug, Default)]
    struct CalledMultiServiceImpl2 {
        pub value: i32
    }

    service_deps!(CalledMultiServiceImpl2: LoggingService);

    impl_service! {
        impl CalledMultiService for CalledMultiServiceImpl2 where deps: LoggingService
        {
            fn call_service(&self, idx: usize) {
                let logging: Box<dyn LoggingService> = self.get_logging_service();
                let value = self.data().value;
                logging.log(&format!("+called_multi_service({idx}, {value})"));
                logging.log(&format!("-called_multi_service({idx}, {value})"));
            }
        }
    }

    impl_mut_service! {
        impl SetterMultiService for CalledMultiServiceImpl2 where deps: LoggingService
        {
            fn set_value(&mut self, value: i32) {
                (self.get_logging_service() as Box<dyn LoggingService>).log(&format!("+setter_multi_service({value})"));
                self.mut_data().value = value;
                (self.get_logging_service() as Box<dyn LoggingService>).log(&format!("-setter_multi_service({value})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct CalledMultiServiceImpl3 {
        id: i32
    }

    service_deps!(CalledMultiServiceImpl3: LoggingService, AdditionalService);

    def_service!{
        trait CalledIdSetterService as called_id_setter_service {
            fn set_id(&mut self, id: i32);
        }
    }

    impl_service! {
        impl CalledMultiService for CalledMultiServiceImpl3 where deps: LoggingService + AdditionalService
        {
            fn call_service(&self, idx: usize) {
                let logging = self.get_logging_service();
                let id = self.data().id;
                logging.log(&format!("+called_multi_service3({idx}, {id})"));
                self.get_additional_service().call_service(id);
                logging.log(&format!("-called_multi_service3({idx}, {id})"));
            }
        }
    }

    impl_mut_service!{
        impl CalledIdSetterService for CalledMultiServiceImpl3 where deps: LoggingService {
            fn set_id(&mut self, id: i32) {
                self.get_logging_service().log(&format!("+called_id_setter_service({id})"));
                self.mut_data().id = id;
                self.get_logging_service().log(&format!("-called_id_setter_service({id})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct CallingServiceImpl1;

    service_deps!(CallingServiceImpl1: LoggingService, CalledService);

    def_service!{
        trait CallingService as calling_service {
            fn call_service(&self, id: i32);
        }
    }

    impl_service! {
        impl CallingService for CallingServiceImpl1 where deps: LoggingService + CalledService
        {
            fn call_service(&self, id: i32) {
                let logging = self.get_logging_service();
                logging.log(&format!("+calling_service({id})"));
                self.get_called_service().call_service(id);
                logging.log(&format!("-calling_service({id})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct CalledServiceImpl1;

    service_deps!(CalledServiceImpl1: LoggingService, AdditionalService);

    def_service!{
        trait CalledService as called_service {
            fn call_service(&self, id: i32);
        }
    }

    impl_service! {
        impl CalledService for CalledServiceImpl1 where deps: LoggingService + AdditionalService
        {
            fn call_service(&self, id: i32) {
                let logging = self.get_logging_service();
                logging.log(&format!("+called_service({id})"));
                self.get_additional_service().call_service(id);
                logging.log(&format!("-called_service({id})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct AdditionalServiceImpl1 {
        value: i32
    }

    service_deps!(AdditionalServiceImpl1: LoggingService);

    def_service!{
        trait AdditionalService as additional_service {
            fn call_service(&self, id: i32);
        }
    }

    def_service!{
        trait AdditionalSetterService as additional_setter_service {
            fn set_value(&mut self, value: i32);
        }
    }

    impl_service! {
        impl AdditionalService for AdditionalServiceImpl1 where deps: LoggingService
        {
            fn call_service(&self, id: i32) {
                let logging: Box<dyn LoggingService> = self.get_logging_service();
                let value = self.data().value;
                logging.log(&format!("+additional_service({id}, {value})"));
                logging.log(&format!("-additional_service({id}, {value})"));
            }
        }
    }

    impl_mut_service! {
        impl AdditionalSetterService for AdditionalServiceImpl1 where deps: LoggingService
        {
            fn set_value(&mut self, value: i32) {
                (self.get_logging_service() as Box<dyn LoggingService>).log(&format!("+additional_setter_service({value})"));
                self.mut_data().value = value;
                (self.get_logging_service() as Box<dyn LoggingService>).log(&format!("-additional_setter_service({value})"));
            }
        }
    }

    #[derive(Debug, Default)]
    struct LoggingServiceImpl1 {
        pub calls: RefCell<Vec<String>>
    }

    def_service!{
        trait LoggingService as logging_service {
            fn log(&self, message: &str);
        }
    }

    impl_service! {
        impl LoggingService for LoggingServiceImpl1
        {
            fn log(&self, message: &str) {
                self.data().calls.borrow_mut().push(message.into());
            }
        }
    }

    def_service!{
        trait LoggedService as logged_service {
            fn get_calls(&'_ self) -> Ref<'_, Vec<String>>;
        }
    }

    impl_service! {
        impl LoggedService for LoggingServiceImpl1
        {
            fn get_calls(&'_ self) -> Ref<'_, Vec<String>> {
                self.data().calls.borrow()
            }
        }
    }
}