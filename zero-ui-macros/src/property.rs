use crate::util;
use proc_macro2::Span;
use std::mem;
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{parse::*, *};

pub mod keyword {
    syn::custom_keyword!(context);
    syn::custom_keyword!(event);
    syn::custom_keyword!(outer);
    syn::custom_keyword!(inner);
}

#[allow(clippy::cognitive_complexity)]
pub fn expand_property(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let priority = parse_macro_input!(args as Priority);
    let mut fn_ = parse_macro_input!(input as ItemFn);

    if fn_.sig.inputs.len() < 2 {
        abort!(fn_.sig.inputs.span(), "cannot be property, expected at least two arguments")
    }
    if fn_.sig.generics.lifetimes().next().is_some() {
        abort!(fn_.sig.generics.span(), "lifetimes are not supported in property functions");
    }
    if fn_.sig.generics.const_params().next().is_some() {
        abort!(fn_.sig.generics.span(), "const generics are not supported in property functions");
    }

    // extract stuff for new mod and convert the input fn into the set fn.
    let ident = mem::replace(&mut fn_.sig.ident, ident!("set"));
    let vis = mem::replace(&mut fn_.vis, util::pub_vis());
    let (docs_attrs, other_attrs) = util::split_doc_other(&mut fn_.attrs);

    let mut arg_idents = vec![];
    let mut arg_tys = vec![];
    let mut gen_idents = vec![];
    let mut gen_bounds = vec![];

    // collect basic generics (fn property<T: Bounds>)
    for g in fn_.sig.generics.type_params() {
        gen_idents.push(g.ident.clone());
        gen_bounds.push(g.bounds.clone());
    }

    // merge where bounds into basic generics.
    if let Some(where_) = &fn_.sig.generics.where_clause {
        for p in &where_.predicates {
            if let WherePredicate::Type(p) = p {
                if p.lifetimes.is_some() {
                    abort!(p.span(), "lifetime bounds are not supported in property functions")
                }

                if let Type::Path(bt) = &p.bounded_ty {
                    if bt.qself.is_none() {
                        if let Some(ident) = bt.path.get_ident() {
                            if let Some(i) = gen_idents.iter().position(|gi| gi == ident) {
                                // where T: Bounds and T is a known generic.
                                for b in &p.bounds {
                                    gen_bounds[i].push(b.clone());
                                }
                                continue;
                            }
                        }
                    }
                }

                abort!(p.span(), "only bounds to local generic types are supported in property functions")
            } else {
                abort!(p.span(), "only type where predicates are supported in property functions")
            }
        }
    }

    // collect arg types and normalize impl Trait generics.
    let mut t_impl_n = 0;
    for a in fn_.sig.inputs.iter().skip(1) {
        if let FnArg::Typed(a) = a {
            if let Pat::Ident(id) = &*a.pat {
                if id.subpat.is_none() {
                    arg_idents.push(id.ident.clone());

                    let ty = &*a.ty;
                    if let Type::ImplTrait(it) = ty {
                        let ident = ident!("TImpl{}", t_impl_n);
                        t_impl_n += 1;

                        arg_tys.push(parse_quote!(#ident));

                        gen_idents.push(ident.clone());
                        gen_bounds.push(it.bounds.clone());
                    } else {
                        arg_tys.push(ty.clone());
                    }
                    continue;
                }
            }
        }

        abort!(
            a.span(),
            "only single type ascription (name: T) arguments are supported in property functions"
        )
    }

    // remove generics that are not used in arguments.
    let mut unused_generics = gen_idents.clone();
    let mut visitor = RemoveVisitedIdents(&mut unused_generics);
    for ty in &mut arg_tys {
        visitor.visit_type_mut(ty);
    }
    for bos in &mut gen_bounds {
        for bo in bos.iter_mut() {
            visitor.visit_type_param_bound_mut(bo);
        }
    }
    let unused_gen_idx: Vec<_> = gen_idents
        .iter()
        .enumerate()
        .filter(|(_, id)| unused_generics.contains(id))
        .map(|(i, _)| i)
        .collect();
    for i in unused_gen_idx.into_iter().rev() {
        gen_idents.remove(i);
        gen_bounds.remove(i);
    }

    // generic types that are not used in struct concrete types need to
    // be placed in a PhantomData member.
    let mut phantom_idents = gen_idents.clone();
    let mut visitor = RemoveVisitedIdents(&mut phantom_idents);
    for ty in &mut arg_tys {
        visitor.visit_type_mut(ty);
    }

    let mut arg_return_tys = arg_tys.clone();
    let mut visitor = PrependSelfIfPathIdent(&gen_idents);
    for ty in &mut arg_return_tys {
        visitor.visit_type_mut(ty);
    }
    let mut gen_bounds_ty = gen_bounds.clone();
    for bos in &mut gen_bounds_ty {
        for bo in bos.iter_mut() {
            visitor.visit_type_param_bound_mut(bo);
        }
    }

    let args_gen_decl = if gen_idents.is_empty() {
        quote!()
    } else {
        quote!(<#(#gen_idents: #gen_bounds),*>)
    };
    let args_gen_use = if gen_idents.is_empty() {
        quote!()
    } else {
        quote!(<#(#gen_idents),*>)
    };

    // templates for compile-time sorting functions:
    // widget_new! will generate a call to all widget properties set_context,
    // then set_event for all, etc., the returns args of set_context are fed into
    // set_event end so on, so we need to generate dummy functions for before and after
    // or actual set:
    //
    // 1 - for before we take the set(args) and returns then.
    let set_not_yet = |fn_: &str| {
        let fn_ = ident!(fn_);
        quote! {
            #[doc(hidden)]
            #[inline]
            pub fn #fn_<C: zero_ui::core::UiNode, A: Args>(child: C, args: A) -> (C, A) {
                (child, args)
            }
        }
    };

    // 2 - for our actual set we call the property::set function to make or new child
    // and then return the new child with place-holder nil ()
    let set_now = |fn_: &str| {
        let fn_ = ident!(fn_);
        quote! {
            #[doc(hidden)]
            #[inline]
            pub fn #fn_(child: impl zero_ui::core::UiNode, args: impl Args) -> (impl zero_ui::core::UiNode, ()) {
                let (#(#arg_idents,)*) = args.pop();
                (set(child, #(#arg_idents),*), ())
            }
        }
    };

    // 3 - for after we set we just pass along the nil
    let set_already_done = |fn_: &str| {
        let fn_ = ident!(fn_);
        quote! {
            #[doc(hidden)]
            #[inline]
            pub fn #fn_<C: zero_ui::core::UiNode>(child: C, args: ()) -> (C, ()) {
                (child, ())
            }
        }
    };
    let mut sets = vec![];
    match priority {
        Priority::Inner => {
            sets.push(set_now("set_inner"));
            sets.push(set_already_done("set_outer"));
            sets.push(set_already_done("set_event"));
            sets.push(set_already_done("set_context"));
        }
        Priority::Outer => {
            sets.push(set_not_yet("set_inner"));
            sets.push(set_now("set_outer"));
            sets.push(set_already_done("set_event"));
            sets.push(set_already_done("set_context"));
        }
        Priority::Event => {
            sets.push(set_not_yet("set_inner"));
            sets.push(set_not_yet("set_outer"));
            sets.push(set_now("set_event"));
            sets.push(set_already_done("set_context"));
        }
        Priority::Context => {            
            sets.push(set_not_yet("set_inner"));
            sets.push(set_not_yet("set_outer"));
            sets.push(set_not_yet("set_event"));
            sets.push(set_now("set_context"));
        }
    }

    // generate documentation that must be formatted.
    let mod_property_doc = doc!("This module is a widget `{}` property.", priority);
    let fn_set_doc = doc!(
        "Manually sets the [`{0}`]({0}) property.\n\nThis property must be set with `{1}` priority to work properly.",
        ident,
        priority
    );
    let fn_args_doc = doc!("Collects [`set`](set) arguments into a [named args](Args) view.");
    let args_doc = doc!("Named arguments of the [`{0}`]({0}) property.", ident);
    let mtd_pop_doc = doc!("Moved the args to a tuple sorted by their position of [`args`](args) and [`set`](set).");

    let r = quote! {

        #(#docs_attrs)*
        ///
        /// # Property
        #mod_property_doc
        #vis mod #ident {
            use super::*;

            #fn_set_doc
            #(#other_attrs)*
            #fn_

            #(#sets)*

            #fn_args_doc
            #[inline]
            pub fn args#args_gen_decl(#(#arg_idents: #arg_tys),*) -> impl Args {
                NamedArgs {
                    _phantom: std::marker::PhantomData,
                    #(#arg_idents,)*
                }
            }

            #args_doc
            pub trait Args {
                #(type #gen_idents: #gen_bounds_ty;)*

                #(fn #arg_idents(&self) -> &#arg_return_tys;)*

                #mtd_pop_doc
                fn pop(self) -> (#(#arg_return_tys,)*);
            }

            #[doc(hidden)]
            pub struct NamedArgs#args_gen_decl {
                pub _phantom: std::marker::PhantomData<(#(#phantom_idents),*)>,
                #(pub #arg_idents: #arg_tys,)*
            }

            impl#args_gen_decl Args for NamedArgs#args_gen_use {
                #(type #gen_idents = #gen_idents;)*

                #(

                #[inline]
                fn #arg_idents(&self) -> &#arg_return_tys {
                    &self.#arg_idents
                }

                )*

                #[inline]
                fn pop(self) -> (#(#arg_return_tys,)*) {
                    (#(self.#arg_idents,)*)
                }
            }
        }
    };

    //panic!("{}", r);

    r.into()
}

#[derive(Clone, Copy)]
enum Priority {
    Context,
    Event,
    Outer,
    Inner,
}
impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Priority::Context => write!(f, "context"),
            Priority::Event => write!(f, "event"),
            Priority::Outer => write!(f, "outer"),
            Priority::Inner => write!(f, "inner"),
        }
    }
}
impl Parse for Priority {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(keyword::context) {
            input.parse::<keyword::context>()?;
            Ok(Priority::Context)
        } else if lookahead.peek(keyword::event) {
            input.parse::<keyword::event>()?;
            Ok(Priority::Event)
        } else if lookahead.peek(keyword::outer) {
            input.parse::<keyword::outer>()?;
            Ok(Priority::Outer)
        } else if lookahead.peek(keyword::inner) {
            input.parse::<keyword::inner>()?;
            Ok(Priority::Inner)
        } else {
            Err(lookahead.error())
        }
    }
}

struct RemoveVisitedIdents<'a>(&'a mut Vec<Ident>);

impl<'a> VisitMut for RemoveVisitedIdents<'a> {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        if let Some(idx) = self.0.iter().position(|id| id == i) {
            self.0.swap_remove(idx);
        }
        visit_mut::visit_ident_mut(self, i);
    }
}

struct PrependSelfIfPathIdent<'a>(&'a [Ident]);

impl<'a> VisitMut for PrependSelfIfPathIdent<'a> {
    fn visit_path_mut(&mut self, i: &mut Path) {
        visit_mut::visit_path_mut(self, i);

        if let Some(s) = i.segments.first() {
            if self.0.contains(&s.ident) {
                i.segments.insert(0, parse_quote!(Self));
            }
        }
    }
}
