use crate::util;
use proc_macro2::{Span, TokenStream};
use std::mem;
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{parse::*, *};

pub mod keyword {
    syn::custom_keyword!(context);
    syn::custom_keyword!(event);
    syn::custom_keyword!(outer);
    syn::custom_keyword!(size);
    syn::custom_keyword!(inner);
    syn::custom_keyword!(not_when);
}

#[allow(clippy::cognitive_complexity)]
pub fn expand_property(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let PropertyArgs { priority, not_when } = parse_macro_input!(args as PropertyArgs);
    let mut fn_ = parse_macro_input!(input as ItemFn);

    let crate_ = util::zero_ui_crate_ident();

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
            pub fn #fn_<C: #crate_::core::UiNode, A: Args>(child: C, args: A) -> (C, A) {
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
            pub fn #fn_(child: impl #crate_::core::UiNode, args: impl Args) -> (impl #crate_::core::UiNode, ()) {
                let (#(#arg_idents,)*) = ArgsUnwrap::unwrap(args);
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
            pub fn #fn_<C: #crate_::core::UiNode>(child: C, args: ()) -> (C, ()) {
                (child, ())
            }
        }
    };
    let mut sets = vec![];
    match priority {
        Priority::Inner => {
            sets.push(set_now("set_inner"));
            sets.push(set_already_done("set_size"));
            sets.push(set_already_done("set_outer"));
            sets.push(set_already_done("set_event"));
            sets.push(set_already_done("set_context"));
        }
        Priority::Size => {
            sets.push(set_not_yet("set_inner"));
            sets.push(set_now("set_size"));
            sets.push(set_already_done("set_outer"));
            sets.push(set_already_done("set_event"));
            sets.push(set_already_done("set_context"));
        }
        Priority::Outer => {
            sets.push(set_not_yet("set_inner"));
            sets.push(set_not_yet("set_size"));
            sets.push(set_now("set_outer"));
            sets.push(set_already_done("set_event"));
            sets.push(set_already_done("set_context"));
        }
        Priority::Event => {
            sets.push(set_not_yet("set_inner"));
            sets.push(set_not_yet("set_size"));
            sets.push(set_not_yet("set_outer"));
            sets.push(set_now("set_event"));
            sets.push(set_already_done("set_context"));
        }
        Priority::Context => {
            sets.push(set_not_yet("set_inner"));
            sets.push(set_not_yet("set_size"));
            sets.push(set_not_yet("set_outer"));
            sets.push(set_not_yet("set_event"));
            sets.push(set_now("set_context"));
        }
    }

    let argi: Vec<_> = (0..arg_idents.len()).map(|i| ident!("arg{}", i)).collect();
    let args: Vec<_> = fn_.sig.inputs.iter().skip(1).collect();

    let when_assert = if not_when {
        quote! {}
    } else {
        let gen_params = fn_.sig.generics.params.clone();
        let gen_params = if gen_params.is_empty() {
            quote! {}
        } else {
            quote! {<#gen_params>}
        };
        let gen_where = fn_.sig.generics.where_clause.clone();
        let arg_clone: Vec<_> = args.iter().map(|a| impl_clone(a.span(), &crate_)).collect();
        quote! {
            // this mod is used in widgets to assert the property is permitted in when conditions.
            #[doc(hidden)]
            pub mod is_allowed_in_when {
                use super::*;

                // this function is used to assert the property arguments can be used in when conditions
                // at compile time.
                #[doc(hidden)]
                #[allow(unused)]
                fn assert#gen_params(#(#args),*) -> (#(#arg_clone,)*) #gen_where {
                    (#(#arg_idents,)*)
                }
            }

        }
    };

    // generate documentation that must be formatted.
    let mod_property_doc = doc!(
        "This module is a widget `{}` property. It {} be used in widget `when` condition expressions.",
        priority,
        if not_when { "cannot" } else { "can also" }
    );

    let mut mod_property_args_doc = String::new();
    let mut z_args = Vec::with_capacity(1);
    {
        use std::fmt::Write;
        let b = &mut mod_property_args_doc;
        macro_rules! wln { ($($tt:tt)*) => { let _ = writeln!(b, $($tt)*); } }

        wln!("<div id='args_example'>\n");
        wln!("```");
        wln!("# fn args(");
        for (i, (a, t)) in arg_idents.iter().zip(arg_tys.iter()).enumerate() {
            let t = if let Some(ti) = get_ty_ident(t).and_then(|t| gen_idents.iter().position(|gt| gt == t)) {
                let bounds = &gen_bounds[ti];
                if bounds.is_empty() {
                    z_args.push(quote!(#t));
                    cleanup_arg_ty(quote!(#t).to_string())
                } else {
                    z_args.push(quote!(impl #bounds));
                    {}
                    let bounds = cleanup_arg_ty(quote!(#bounds).to_string());
                    format!("impl {}", bounds)
                }
            } else {
                z_args.push(quote!(#t));
                cleanup_arg_ty(quote!(#t).to_string())
            };

            wln!("{}: {}, // .{}", a, t, i);
        }
        wln!("# ) {{}}");
        wln!("```\n");
        wln!("</div>");
        wln!("<script>{}</script>", include_str!("property_args_ext.js"));
        wln!("<style>a[href='fn.z.html']{{ display: none; }}</style>");
        wln!("<iframe id='args_example_load' style='display:none;' src='fn.z.html'></iframe>");
    }
    let mod_property_args_doc = doc!("{}", mod_property_args_doc);
    let z_js = doc!("<span></span>\n\n<script>{}</script>", include_str!("property_z_ext.js"));

    let fn_set_doc = doc!(
        "Manually sets the [`{0}`]({0}) property.\n\nThis property must be set with `{1}` priority to work properly.",
        ident,
        priority
    );
    let hide_z = doc!("<style>a[href='fn.z.html']{{ display: none; }}</style>");
    let fn_args_doc = doc!("Collects [`set`](set) arguments into a [named args](Args) view.");
    let args_doc = doc!("Packed arguments of the [`{0}`]({0}) property.", ident);
    let args_named_doc = doc!("View of the [`{0}`]({0}) property arguments by name.", ident);
    let args_numbered_doc = doc!("View of the [`{0}`]({0}) property arguments by position.", ident);
    let args_unwrap_doc = doc!("Unpacks the arguments of the [`{0}`]({0}) property.", ident);

    let r = quote! {

        #(#docs_attrs)*
        ///
        /// # Property
        #mod_property_doc
        /// ## Arguments
        #mod_property_args_doc
        #vis mod #ident {
            use super::*;

            #fn_set_doc
            #hide_z
            #(#other_attrs)*
            #fn_

            #(#sets)*

            #fn_args_doc
            #hide_z
            #[inline]
            pub fn args#args_gen_decl(#(#arg_idents: #arg_tys),*) -> impl Args {
                NamedArgs {
                    _phantom: std::marker::PhantomData,
                    #(#arg_idents,)*
                }
            }

            #when_assert

            #args_named_doc
            pub trait ArgsNamed {
                #(type #gen_idents: #gen_bounds_ty;)*

                #(fn #arg_idents(&self) -> &#arg_return_tys;)*
            }

            #args_numbered_doc
            pub trait ArgsNumbered {
                #(type #gen_idents: #gen_bounds_ty;)*

                #(fn #argi(&self) -> &#arg_return_tys;)*
            }

            #args_unwrap_doc
            pub trait ArgsUnwrap {
                #(type #gen_idents: #gen_bounds_ty;)*

                fn unwrap(self) -> (#(#arg_return_tys,)*);
            }

            #args_doc
            pub trait Args: ArgsNamed + ArgsNumbered + ArgsUnwrap { }

            #[doc(hidden)]
            pub struct NamedArgs#args_gen_decl {
                pub _phantom: std::marker::PhantomData<(#(#phantom_idents),*)>,
                #(pub #arg_idents: #arg_tys,)*
            }

            impl#args_gen_decl ArgsNamed for NamedArgs#args_gen_use {
                #(type #gen_idents = #gen_idents;)*

                #(

                #[inline]
                fn #arg_idents(&self) -> &#arg_return_tys {
                    &self.#arg_idents
                }

                )*
            }

            impl#args_gen_decl ArgsNumbered for NamedArgs#args_gen_use {
                #(type #gen_idents = #gen_idents;)*

                #(

                #[inline]
                fn #argi(&self) -> &#arg_return_tys {
                    &self.#arg_idents
                }

                )*
            }

            impl#args_gen_decl ArgsUnwrap for NamedArgs#args_gen_use {
                #(type #gen_idents = #gen_idents;)*

                #[inline]
                fn unwrap(self) -> (#(#arg_return_tys,)*) {
                    (#(self.#arg_idents,)*)
                }
            }

            impl#args_gen_decl Args for NamedArgs#args_gen_use { }

            #z_js
            #[allow(unused)]
            pub fn z#args_gen_decl(#(#arg_idents: #z_args),*) { }
        }
    };

    //panic!("{}", r);

    r.into()
}

struct PropertyArgs {
    priority: Priority,
    not_when: bool,
}
impl Parse for PropertyArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let priority = input.parse()?;
        let not_when = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            input.parse::<keyword::not_when>()?;
            true
        } else {
            false
        };

        Ok(PropertyArgs { priority, not_when })
    }
}

#[derive(Clone, Copy)]
enum Priority {
    Context,
    Event,
    Outer,
    Size,
    Inner,
}
impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Priority::Context => write!(f, "context"),
            Priority::Event => write!(f, "event"),
            Priority::Outer => write!(f, "outer"),
            Priority::Size => write!(f, "size"),
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
        } else if lookahead.peek(keyword::size) {
            input.parse::<keyword::size>()?;
            Ok(Priority::Size)
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

fn impl_clone(span: Span, crate_: &Ident) -> TokenStream {
    quote_spanned! {span=> impl #crate_::core::types::ArgWhenCompatible}
}

fn get_ty_ident(ty: &Type) -> Option<&Ident> {
    if let Type::Path(p) = ty {
        p.path.get_ident()
    } else {
        None
    }
}

fn cleanup_arg_ty(ty: String) -> String {
    ty.replace(" < ", "<")
        .replace(" > ", ">")
        .replace(" >", ">")
        .replace(" :: ", "::")
        .replace(" = ", "=")
}
