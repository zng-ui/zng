use crate::util;
use proc_macro2::{Span, TokenStream};
use std::mem;
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{parse::*, *};
use quote::ToTokens;
use uuid::Uuid;

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

    
    let set_args_macro_name = ident!("__set_args_{}", Uuid::new_v4().to_simple());
    
    let set_args = quote! {
        #[doc(hidden)]
        #[inline]
        pub fn set_args(child: impl #crate_::core::UiNode, args: impl Args) -> impl #crate_::core::UiNode {
            let (#(#arg_idents,)*) = ArgsUnwrap::unwrap(args);
            set(child, #(#arg_idents),*)
        }

        #[doc(hidden)]
        #[macro_export]
        macro_rules! #set_args_macro_name {
            (#priority, $me:path, $child:ident, $args:ident) => {
                let $child = $me($child, $args);
            };
            ($($ignore:tt)*) => {}
        }

        #[doc(hidden)]
        pub use #set_args_macro_name as set_args;
    };
   

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
        wln!("```text");
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
                    format!("<span class='kw'>impl</span> {}", bounds)
                }
            } else {
                z_args.push(quote!(#t));
                cleanup_arg_ty(quote!(#t).to_string())
            };

            wln!("<span class='ident'>{}</span>: {}, <span class='comment'>// .{}</span>", a, t, i);
        }
        wln!("```\n");
        wln!("</div>");
        wln!("<script>{}</script>", include_str!("property_args_ext.js"));
        wln!("<style>a[href='fn.__.html']{{ display: none; }}</style>");
        wln!("<iframe id='args_example_load' style='display:none;' src='fn.__.html'></iframe>");
    }
    let mod_property_args_doc = doc!("{}", mod_property_args_doc);
    let z_js = doc!("<span></span>\n\n<script>{}</script>", include_str!("property_z_ext.js"));

    let fn_set_doc = doc!(
        "Manually sets the [`{0}`]({0}) property.\n\nThis property must be set with `{1}` priority to work properly.",
        ident,
        priority
    );
    let hide_z = doc!("<style>a[href='fn.__.html']{{ display: none; }}</style>");
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

            #set_args

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
            pub fn __#args_gen_decl(#(#arg_idents: #z_args),*) { }
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

impl ToTokens for Priority {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            Priority::Context => tokens.extend(quote!{context}),
            Priority::Event => tokens.extend(quote!{event}),
            Priority::Outer => tokens.extend(quote!{outer}),
            Priority::Size => tokens.extend(quote!{size}),
            Priority::Inner => tokens.extend(quote!{inner}),
        }
    }
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
    let mut r = String::with_capacity(ty.len());
    let mut lifetime = false;
    let mut word = String::with_capacity(3);
    for c in ty.chars() {
        if word.is_empty() {
            if c.is_alphabetic() || c == '_' {
                word.push(c);
            } else {
                push_html_scape(&mut r, c);
                lifetime |= c == '\'';
            }
        } else if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else {
            push_word(&mut r, &word, lifetime);
            push_html_scape(&mut r, c);
            word.clear();
            lifetime = false;
        }
    }
    if !word.is_empty() {
        push_word(&mut r, &word, lifetime);
    }
    if r.ends_with(' ') {
        r.truncate(r.len() - 1);
    }
    r
}

fn push_word(r: &mut String, word: &str, lifetime: bool) {
    if lifetime {
        r.push_str(word);
        r.push(' ');
    } else {
        match syn::parse_str::<syn::Ident>(word) {
            Ok(_) => {
                r.push_str("<span class='ident'>");
                r.push_str(word);
                r.push_str("</span>")
            }
            Err(_) => {
                // Ident parse does not allow keywords.
                r.push_str("<span class='kw'>");
                r.push_str(word);
                r.push_str("</span> ")
            }
        }
    }
}

fn push_html_scape(r: &mut String, c: char) {
    match c {
        ' ' => {}
        '<' => r.push_str("&lt;"),
        '>' => r.push_str("&gt;"),
        '"' => r.push_str("&quot;"),
        '&' => r.push_str("&amp;"),
        '\'' => r.push_str("&#x27;"),
        ',' => r.push_str(", "),
        '+' => r.push_str(" + "),
        c => r.push(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_cleanup_arg_ty(input: &str, expected: &str) {
        let r = cleanup_arg_ty(input.to_owned());
        assert_eq!(r.as_str(), expected);
    }

    #[test]
    fn cleanup_arg_ty_tests() {
        assert_cleanup_arg_ty("& ' static str", "&amp;&#x27;static <span class='ident'>str</span>");
        assert_cleanup_arg_ty(
            "crate_name :: ns :: IntoVar < impl Trait >",
            "<span class='ident'>crate_name</span>::<span class='ident'>ns</span>::<span class='ident'>IntoVar</span>&lt;<span class='kw'>impl</span> <span class='ident'>Trait</span>&gt;",
        );
    }
}
