use proc_macro2::Span;
use std::mem;
use syn::spanned::Spanned;
use syn::{parse::*, *};
include!("util.rs");

pub mod keyword {
    syn::custom_keyword!(context);
    syn::custom_keyword!(event);
    syn::custom_keyword!(outer);
    syn::custom_keyword!(inner);
}

#[allow(clippy::cognitive_complexity)]
pub fn expand_property(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as Args);
    let mut fn_ = parse_macro_input!(input as ItemFn);

    if fn_.sig.inputs.len() < 2 {
        abort!(fn_.sig.inputs.span(), "cannot be property, expected at least two arguments")
    }

    // extract stuff for new mod and convert the input fn into the set fn.
    let ident = mem::replace(&mut fn_.sig.ident, ident!("set"));
    let vis = mem::replace(&mut fn_.vis, pub_vis());
    let (docs_attrs, other_attrs) = split_doc_other(&mut fn_.attrs);
    let fn_doc = doc!(
        "Manually sets the `{0}` property.\n\nSee [the module level documentation]({0}) for more.",
        ident
    );

    // parse arguments, convert `_: impl T` to `<TImpl0: T>`.
    // this is needed to make the struct Args bounds, which are needed
    // because type inference gets confused for closures if the bounds
    // are not immediately apparent.
    let mut arg_names = vec![];
    let mut arg_tys = vec![];
    let mut arg_decl = vec![];
    let mut arg_wheres = vec![];
    let mut arg_gen_tys = vec![];
    let mut impl_tys_count = 0;
    let mut next_impl_ty = move || {
        let n = ident!("TImpl{}", impl_tys_count);
        impl_tys_count += 1;
        n
    };
    for input in fn_.sig.inputs.iter().skip(1) {
        match input {
            FnArg::Typed(input) => {
                if let Pat::Ident(pat) = &*input.pat {
                    arg_names.push(pat.ident.clone());
                } else {
                    abort!(input.pat.span(), "cannot be property, must only use simple argument names")
                }

                match &*input.ty {
                    Type::ImplTrait(impl_) => {
                        let ty = next_impl_ty();
                        arg_tys.push(parse_quote!(#ty));
                        let bounds = &impl_.bounds;
                        arg_decl.push(parse_quote!(#ty:#bounds));
                        arg_gen_tys.push(ty);
                    }
                    Type::Path(t) => {
                        if let Some(t) = t.path.get_ident() {
                            if let Some(gen) = fn_.sig.generics.type_params().find(|p| &p.ident == t) {
                                if !arg_gen_tys.contains(t) {
                                    arg_gen_tys.push(t.clone());

                                    arg_decl.push(gen.clone());
                                    if let Some(where_) = find_where_predicate(&fn_, t) {
                                        arg_wheres.push(where_.clone());
                                    }
                                }
                            }
                        }
                        arg_tys.push(input.ty.clone())
                    }
                    _ => arg_tys.push(input.ty.clone()),
                }
            }
            // can this even happen? we parsed as ItemFn
            FnArg::Receiver(self_) => abort!(self_.span(), "cannot be property, must be stand-alone fn"),
        }
    }
    // we need to make a PhantomData member for all other generic types
    // because they may be used in parts of the generics we now are used.
    let mut arg_phantom_decl = vec![];
    let mut arg_phantom_tys = vec![];
    let mut arg_phantom_wheres = vec![];

    if !arg_gen_tys.is_empty() {
        for p in fn_.sig.generics.type_params() {
            if !arg_gen_tys.contains(&p.ident) {
                arg_phantom_tys.push(p.ident.clone());
                arg_phantom_decl.push(p.clone());

                if let Some(where_) = find_where_predicate(&fn_, &p.ident) {
                    arg_phantom_wheres.push(where_.clone());
                }
            }
        }
    }

    let arg_decl = if arg_decl.is_empty() {
        quote!()
    } else {
        quote! (<#(#arg_phantom_decl,)* #(#arg_decl),*>)
    };
    let arg_wheres = if arg_wheres.is_empty() {
        quote!()
    } else {
        quote!(where #(#arg_phantom_wheres,)* #(#arg_wheres),*)
    };
    let arg_gen_tys = if arg_gen_tys.is_empty() {
        quote!()
    } else {
        quote!(<#(#arg_phantom_tys,)* #(#arg_gen_tys),*>)
    };
    let arg_phantom_tys = if arg_phantom_tys.is_empty() {
        quote!(<()>)
    } else {
        quote! (<#(#arg_phantom_tys),*>)
    };

    // struct Args
    let struct_args = quote! {
        /// Named arguments of this property.
        #[allow(unused)]
        pub struct Args#arg_decl #arg_wheres {
            #[doc(hidden)]
            pub __phantom: std::marker::PhantomData#arg_phantom_tys,
            #(pub #arg_names: #arg_tys),*
        }
        impl#arg_decl Args#arg_gen_tys #arg_wheres {
            pub fn pop(self) -> (#(#arg_tys,)*) {
                (#(self.#arg_names,)*)
            }
        }
    };

    let child_ty = quote!(impl zero_ui::core::UiNode);

    // templates for compile-time sorting functions:
    // widget_new! will generate a call to all widget properties set_context,
    // then set_event for all, etc, the returns args of set_context are fed into
    // set_event end so on, so we need to generate dummy functions for before and after
    // or actual set:
    //
    // 1 - for before we take the set(args) and returns then.
    let set_not_yet = |fn_: &str| {
        let fn_ = ident!(fn_);
        quote! {
            #[doc(hidden)]
            #[inline]
            pub fn #fn_ #arg_decl(child: #child_ty, #(#arg_names: #arg_tys),*) -> (#child_ty, #(#arg_tys),*) #arg_wheres {
                (child, #(#arg_names),*)
            }
        }
    };

    // 2 - for our actual set we call the property::set function to make or new child
    // and then return the new child with place-holder nils ()
    let arg_nils = vec![quote![()]; arg_names.len()];
    let set_now = |fn_: &str| {
        let fn_ = ident!(fn_);
        quote! {
            #[doc(hidden)]
            #[inline]
            pub fn #fn_ #arg_decl(child: #child_ty, #(#arg_names: #arg_tys),*) -> (#child_ty, #(#arg_nils),*) #arg_wheres {
                (set(child, #(#arg_names),*), #(#arg_nils),*)
            }
        }
    };

    // 3 - for after we set we just pass along the nils
    let set_already_done = |fn_: &str| {
        let fn_ = ident!(fn_);
        quote! {
            #[doc(hidden)]
            #[inline]
            pub fn #fn_(child: #child_ty, #(_: #arg_nils),*) -> (#child_ty, #(#arg_nils),*) {
                (child, #(#arg_nils),*)
            }
        }
    };
    let mut sets = vec![];
    match args {
        Args::Outer => {
            sets.push(set_not_yet("set_context"));
            sets.push(set_not_yet("set_event"));
            sets.push(set_now("set_outer"));
            sets.push(set_already_done("set_inner"));
        }
        Args::Inner => {
            sets.push(set_not_yet("set_context"));
            sets.push(set_not_yet("set_event"));
            sets.push(set_not_yet("set_outer"));
            sets.push(set_now("set_inner"));
        }
        Args::Event => {
            sets.push(set_not_yet("set_context"));
            sets.push(set_now("set_event"));
            sets.push(set_already_done("set_outer"));
            sets.push(set_already_done("set_inner"));
        }
        Args::Context => {
            sets.push(set_now("set_context"));
            sets.push(set_already_done("set_event"));
            sets.push(set_already_done("set_outer"));
            sets.push(set_already_done("set_inner"));
        }
    }

    let r = quote! {
        #(#docs_attrs)*
        #vis mod #ident {
            use super::*;

            #struct_args

            #fn_doc
            #(#other_attrs)*
            #fn_

            #(#sets)*
        }
    };

    r.into()
}

fn find_where_predicate<'a, 'b>(fn_: &'a ItemFn, ident: &'b Ident) -> Option<&'a WherePredicate> {
    fn_.sig.generics.where_clause.as_ref().and_then(|w| {
        w.predicates.iter().find(|p| {
            if let WherePredicate::Type(p) = p {
                if let Type::Path(p) = &p.bounded_ty {
                    if let Some(id) = p.path.get_ident() {
                        return id == ident;
                    }
                }
            }
            false
        })
    })
}

#[derive(Clone, Copy)]
enum Args {
    Context,
    Event,
    Outer,
    Inner,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(keyword::context) {
            input.parse::<keyword::context>()?;
            Ok(Args::Context)
        } else if lookahead.peek(keyword::event) {
            input.parse::<keyword::event>()?;
            Ok(Args::Event)
        } else if lookahead.peek(keyword::outer) {
            input.parse::<keyword::outer>()?;
            Ok(Args::Outer)
        } else if lookahead.peek(keyword::inner) {
            input.parse::<keyword::inner>()?;
            Ok(Args::Inner)
        } else {
            Err(lookahead.error())
        }
    }
}
