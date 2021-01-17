use std::{collections::HashSet, mem};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    parse::{Parse, ParseBuffer, ParseStream},
    parse2, parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token, visit_mut, Attribute, FnArg, Ident, Item, ItemFn, ItemMacro, ItemMod, Path, Token, TypeTuple,
};
use util::{non_user_braced_id, parse2_punctuated};

use crate::{
    util::{self, crate_core, non_user_braced, non_user_bracketed, non_user_parenthesized, Attributes, Errors},
    widget_new2::{PropertyValue, When},
};

pub fn expand(mixin: bool, args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // the widget mod declaration.
    let mod_ = parse_macro_input!(input as ItemMod);
    if mod_.content.is_none() {
        return syn::Error::new(mod_.semi.span(), "only modules with inline content are supported")
            .to_compile_error()
            .to_token_stream()
            .into();
    }
    let (_, items) = mod_.content.unwrap();

    // accumulate the most errors as possible before returning.
    let mut errors = Errors::default();

    // a `$crate` path to the widget module.
    let mod_path = if mixin {
        TokenStream::new()
    } else {
        parse_mod_path(args.into(), &mut errors)
    };

    let Attributes {
        docs, cfg, others: attrs, ..
    } = Attributes::new(mod_.attrs);
    let vis = mod_.vis;
    let ident = mod_.ident;

    let WidgetItems {
        inherits,
        mut properties,
        mut new_child_fn,
        mut new_fn,
        others,
    } = WidgetItems::new(items, &mut errors);

    let whens: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.whens)).collect();
    let child_properties: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.child_properties)).collect();
    let properties: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.properties)).collect();

    if mixin {
        if let Some(child_fn_) = new_child_fn.take() {
            errors.push("widget mixins do not have a `new_child` function", child_fn_.span())
        }

        if let Some(fn_) = new_fn.take() {
            errors.push("widget mixins do not have a `new` function", fn_.span())
        }
    }

    let mut inherits = inherits.into_iter().map(|i| i.path);
    let crate_core = util::crate_core();

    let stage_path = if mixin {
        if let Some(first) = inherits.next() {
            quote!(#first::__inherit!)
        } else {
            quote!(#crate_core::widget_declare!)
        }
    } else {
        // TODO change this back to implicit_mixin after testing
        quote!(#crate_core::widget_base::implicit_mixin2::__inherit!)
    };

    // Does some validation of `new_child` and `new` signatures.
    // Further type validation is done by `rustc` when we call the function
    // in the generated `__new_child` and `__new` functions.
    if let Some(fn_) = &new_child_fn {
        validate_new_fn(fn_, &mut errors);
        if let syn::ReturnType::Default = &fn_.sig.output {
            errors.push("`new_child` must return a type that implements `UiNode`", fn_.sig.output.span())
        }
    }
    if let Some(fn_) = &new_fn {
        validate_new_fn(fn_, &mut errors);
        if fn_.sig.inputs.is_empty() {
            errors.push("`new` must take at least one input that implements `UiNode`", fn_.sig.inputs.span())
        }
    }

    // collects name of captured properties and validates inputs.
    let new_child = new_child_fn
        .as_ref()
        .map(|f| new_fn_captures(f.sig.inputs.iter(), &mut errors))
        .unwrap_or_default();
    let new = new_fn
        .as_ref()
        .map(|f| new_fn_captures(f.sig.inputs.iter().skip(1), &mut errors))
        .unwrap_or_default();
    let mut captures = HashSet::new();
    for capture in new_child.iter().chain(&new) {
        if !captures.insert(capture) {
            errors.push(format!("property `{}` already captured", capture), capture.span());
        }
    }

    // generate `__new_child` and `__new` if new functions are defined in the widget.
    let new_child__ = new_child_fn.as_ref().map(|_| {
        let p_new_child = new_child.iter().map(|id| ident!("__p_{}", id));
        quote! {
            #[doc(hidden)]
            pub fn __new_child(#(#new_child : impl self::#p_new_child::Args),*) -> impl #crate_core::UiNode {
                self::new_child(#(#new_child),*)
            }
        }
    });
    let new__ = new_fn.as_ref().map(|f| {
        let p_new = new.iter().map(|id| ident!("__p_{}", id));
        let output = &f.sig.output;
        quote! {
            #[doc(hidden)]
            pub fn __new(__child: impl #crate_core::UiNode, #(#new: impl self::#p_new::Args),*) #output {
                self::new(__child, #(#new_child),*)
            }
        }
    });

    let mut when_conditions = TokenStream::default();
    for (i, when) in whens.into_iter().enumerate() {
        let condition = match syn::parse2::<WhenExprToVar>(when.condition_expr.to_token_stream()) {
            Ok(c) => c,
            Err(e) => {
                errors.push_syn(e);
                continue;
            }
        };
        let unique_props: HashSet<_> = condition.properties.iter().map(|(p, _)| p).collect();

        let ident = when.make_ident(i);

        let input_idents = unique_props.iter().map(|p| ident!("__{}", p));
        let prop_idents = unique_props.iter().map(|p| ident!("__p_{}", p));

        let fields = condition.properties.iter().map(|(p, m)| ident!("__{}{}", p, m));
        let input_ident_per_field = condition.properties.iter().map(|(p, _)| ident!("__{}", p));
        let members = condition.properties.iter().map(|(_, m)| m);

        let expr = condition.expr;

        when_conditions.extend(quote! {
            #[doc(hidden)]
            pub fn #ident(#(#input_idents : &impl self::#prop_idents::Args),*) -> impl #crate_core::var::Var<bool> {
                #(let #fields = #crate_core::var::IntoVar::into_var(std::clone::Clone::clone(#input_ident_per_field.#members())))*
                #crate_core::var::expr_var! {
                    #expr
                }
            }
        });
    }

    let r = quote! {
        #errors

        // __inherit! will include an `inherited { .. }` block with the widget data after the
        // `inherit { .. }` block and take the next `inherit` path turn that into an `__inherit!` call.
        // This way we "eager" expand the inherited data recursively, when there no more path to inherit
        // a call to `widget_declare!` is made.
        #stage_path {

            inherit { #(#inherits;)* }

            widget {
                docs { #(#docs)* }
                ident { #ident }
                mixin { #mixin }

                properties {

                }
                whens {

                }

                new_child { #(#new_child)* }
                new { #(#new)* }

                mod {
                    #(#attrs)*
                    #cfg
                    #vis mod #ident {
                        #(#others)*
                        #new_child_fn
                        #new_fn

                        #new_child__
                        #new__

                        #when_conditions
                    }
                }
            }
        }
    };

    r.into()
}

fn parse_mod_path(args: TokenStream, errors: &mut Errors) -> TokenStream {
    let args_span = args.span();
    match syn::parse2::<Path>(args) {
        Ok(path) if path.segments.len() > 1 && path.segments[0].ident == "$crate" => path.to_token_stream(),
        _ => {
            errors.push("expected a macro_rules `$crate` path to this widget mod", args_span);
            quote! { $crate::missing_widget_mod_path }
        }
    }
}

// TODO notes:
//
// - Implement the new/new_child function validations found in
//   property.rs:283 and property.rs:408 , property.rs:223 might
//   be relevant as well.
//
// - Validate it in a separate function that handles errors and
//   returns the new and new_child information we want?
fn new_fn_captures<'a, 'b>(fn_inputs: impl Iterator<Item = &'a FnArg>, errors: &'b mut Errors) -> Vec<Ident> {
    let mut r = vec![];
    for input in fn_inputs {
        match input {
            syn::FnArg::Typed(t) => {
                // any pat : ty
                match &*t.pat {
                    syn::Pat::Ident(ident_pat) => {
                        if let Some(subpat) = &ident_pat.subpat {
                            // ident @ sub_pat : type
                            errors.push(
                                "only `field: T` pattern can be property captures, found sub-pattern",
                                subpat.0.span(),
                            );
                        } else if ident_pat.ident == "self" {
                            // self : type
                            errors.push(
                                "only `field: T` pattern can be property captures, found `self`",
                                ident_pat.ident.span(),
                            );
                        } else {
                            // VALID
                            // ident: type
                            r.push(ident_pat.ident.clone());
                        }
                    }
                    invalid => {
                        errors.push("only `field: T` pattern can be property captures", invalid.span());
                    }
                }
            }

            syn::FnArg::Receiver(invalid) => {
                // `self`
                errors.push("only `field: T` pattern can be property captures, found `self`", invalid.span())
            }
        }
    }
    r
}

fn validate_new_fn(fn_: &ItemFn, errors: &mut Errors) {
    if let Some(async_) = &fn_.sig.asyncness {
        errors.push(format!("`{}` cannot be `async`", fn_.sig.ident), async_.span());
    }
    if let Some(unsafe_) = &fn_.sig.unsafety {
        errors.push(format!("`{}` cannot be `unsafe`", fn_.sig.ident), unsafe_.span());
    }
    if let Some(abi) = &fn_.sig.abi {
        errors.push(format!("`{}` cannot be `extern`", fn_.sig.ident), abi.span());
    }
    if let Some(lifetime) = fn_.sig.generics.lifetimes().next() {
        errors.push(format!("`{}` cannot declare lifetimes", fn_.sig.ident), lifetime.span());
    }
    if let Some(const_) = fn_.sig.generics.const_params().next() {
        errors.push(format!("`{}` does not support `const` generics", fn_.sig.ident), const_.span());
    }
}

struct WidgetItems {
    inherits: Vec<Inherit>,
    properties: Vec<Properties>,
    new_child_fn: Option<ItemFn>,
    new_fn: Option<ItemFn>,
    others: Vec<Item>,
}
impl WidgetItems {
    fn new(items: Vec<Item>, errors: &mut Errors) -> Self {
        let mut inherits = vec![];
        let mut properties = vec![];
        let mut new_child_fn = None;
        let mut new_fn = None;
        let mut others = vec![];

        for item in items {
            enum KnownMacro {
                Properties,
                Inherit,
            }
            let mut known_macro = None;
            enum KnownFn {
                New,
                NewChild,
            }
            let mut known_fn = None;
            match item {
                // match properties! or inherit!.
                Item::Macro(ItemMacro { mac, ident: None, .. })
                    if {
                        if let Some(ident) = mac.path.get_ident() {
                            if ident == "properties" {
                                known_macro = Some(KnownMacro::Properties);
                            } else if ident == "inherit" {
                                known_macro = Some(KnownMacro::Inherit);
                            }
                        }
                        known_macro.is_some()
                    } =>
                {
                    match known_macro {
                        Some(KnownMacro::Properties) => match syn::parse2::<Properties>(mac.tokens) {
                            Ok(mut p) => {
                                errors.extend(mem::take(&mut p.errors));
                                properties.push(p)
                            }
                            Err(e) => errors.push_syn(e),
                        },
                        Some(KnownMacro::Inherit) => match parse2::<Inherit>(mac.tokens) {
                            Ok(ps) => inherits.push(ps),
                            Err(e) => errors.push_syn(e),
                        },
                        None => unreachable!(),
                    }
                }
                // match fn new(..) or fn new_child(..).
                Item::Fn(fn_)
                    if {
                        if fn_.sig.ident == "new" {
                            known_fn = Some(KnownFn::New);
                        } else if fn_.sig.ident == "new_child" {
                            known_fn = Some(KnownFn::NewChild);
                        }
                        known_fn.is_some()
                    } =>
                {
                    match known_fn {
                        Some(KnownFn::New) => {
                            new_fn = Some(fn_);
                        }
                        Some(KnownFn::NewChild) => {
                            new_child_fn = Some(fn_);
                        }
                        None => unreachable!(),
                    }
                }
                // other user items.
                item => others.push(item),
            }
        }

        WidgetItems {
            inherits,
            properties,
            new_child_fn,
            new_fn,
            others,
        }
    }
}

struct Inherit {
    path: Path,
}
impl Parse for Inherit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Inherit { path: input.parse()? })
    }
}

struct Properties {
    errors: Errors,
    child_properties: Vec<ItemProperty>,
    properties: Vec<ItemProperty>,
    whens: Vec<When>,
}
impl Parse for Properties {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut errors = Errors::default();
        let mut child_properties = vec![];
        let mut properties = vec![];
        let mut whens = vec![];

        while !input.is_empty() {
            let attrs = Attribute::parse_outer(input).unwrap_or_else(|e| {
                errors.push_syn(e);
                vec![]
            });
            if input.peek(keyword::when) {
                if let Some(mut when) = When::parse(input, &mut errors) {
                    when.attrs = attrs;
                    whens.push(when);
                }
            } else if input.peek(keyword::child) && input.peek2(syn::token::Brace) {
                let input = non_user_braced_id(input, "child");
                while !input.is_empty() {
                    let attrs = Attribute::parse_outer(&input).unwrap_or_else(|e| {
                        errors.push_syn(e);
                        vec![]
                    });
                    match input.parse::<ItemProperty>() {
                        Ok(mut p) => {
                            p.attrs = attrs;
                            child_properties.push(p);
                        }
                        Err(e) => errors.push_syn(e),
                    }
                }
            } else if input.peek(Ident) {
                // peek ident or path.
                match input.parse::<ItemProperty>() {
                    Ok(mut p) => {
                        p.attrs = attrs;
                        properties.push(p);
                    }
                    Err(e) => errors.push_syn(e),
                }
            } else {
                errors.push("expected `when`, `child` or a property declaration", input.span());
                break;
            }
        }

        Ok(Properties {
            errors,
            child_properties,
            properties,
            whens,
        })
    }
}

struct ItemProperty {
    pub attrs: Vec<Attribute>,
    pub path: Path,
    pub alias: Option<(Token![as], Ident)>,
    pub type_: Option<(Token![:], PropertyType)>,
    pub value: Option<(Token![=], PropertyValue)>,
    pub semi: Option<Token![;]>,
}
impl Parse for ItemProperty {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        macro_rules! peek_parse {
            ($token:tt) => {
                if input.peek(Token![$token]) {
                    Some((input.parse()?, input.parse()?))
                } else {
                    None
                }
            };
        }

        Ok(ItemProperty {
            attrs: vec![],
            path: input.parse()?,
            alias: peek_parse![as],
            type_: peek_parse![:],
            value: peek_parse![=],
            semi: if input.peek(Token![;]) { Some(input.parse()?) } else { None },
        })
    }
}

enum PropertyType {
    /// `{ name: u32 }` OR `{ name: impl IntoVar<u32> }` OR `{ name0: .., name1: .. }`
    Named(token::Brace, Punctuated<NamedField, Token![,]>),
    /// `impl IntoVar<bool>, impl IntoVar<u32>`
    Unnamed(Punctuated<syn::Type, Token![,]>),
}
impl Parse for PropertyType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Brace) {
            let named;
            let brace = braced!(named in input);
            Ok(PropertyType::Named(brace, Punctuated::parse_terminated(&named)?))
        } else {
            let mut unnamed = TokenStream::default();
            while !input.is_empty() {
                if input.peek(Token![=]) || input.peek(Token![;]) {
                    break;
                }
                input.parse::<TokenTree>().unwrap().to_tokens(&mut unnamed);
            }
            Ok(PropertyType::Unnamed(parse2_punctuated(unnamed)?))
        }
    }
}

struct NamedField {
    ident: Ident,
    colon: Token![:],
    ty: syn::Type,
}
impl Parse for NamedField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(NamedField {
            ident: input.parse()?,
            colon: input.parse()?,
            ty: input.parse()?,
        })
    }
}

mod keyword {
    pub use crate::widget_new2::keyword::when;
    syn::custom_keyword!(child);
}

struct WhenExprToVar {
    properties: HashSet<(Ident, Ident)>,
    expr: TokenStream,
}
impl Parse for WhenExprToVar {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut properties = HashSet::new();
        let mut expr = TokenStream::default();

        while !input.is_empty() {
            // look for `self.property(.member)?` and replace with `#{__property__member}`
            if input.peek(Token![self]) && input.peek2(Token![.]) {
                input.parse::<Token![self]>().unwrap();
                input.parse::<Token![.]>().unwrap();

                let property = input.parse::<Ident>()?; // parse::<Path> in widget_new.
                let member_ident = if input.peek(Token![.]) {
                    input.parse::<Token![.]>().unwrap();
                    if input.peek(Ident) {
                        let member = input.parse::<Ident>().unwrap();
                        ident!("__{}", member)
                    } else {
                        let index = input.parse::<syn::Index>().unwrap();
                        ident!("__{}", index.index)
                    }
                } else {
                    ident!("__0")
                };

                let var_ident = ident!("__{}{}", property, member_ident);

                expr.extend(quote! {
                    #{#var_ident}
                });

                properties.insert((property, member_ident));
            }
            // recursive parse groups:
            else if input.peek(token::Brace) {
                let inner = WhenExprToVar::parse(&non_user_braced(input))?;
                properties.extend(inner.properties);
                let inner = inner.expr;
                expr.extend(quote! { { #inner } });
            } else if input.peek(token::Paren) {
                let inner = WhenExprToVar::parse(&non_user_parenthesized(input))?;
                properties.extend(inner.properties);
                let inner = inner.expr;
                expr.extend(quote! { ( #inner ) });
            } else if input.peek(token::Bracket) {
                let inner = WhenExprToVar::parse(&non_user_bracketed(input))?;
                properties.extend(inner.properties);
                let inner = inner.expr;
                expr.extend(quote! { [ #inner ] });
            }
            // keep other tokens the same:
            else {
                let tt = input.parse::<TokenTree>().unwrap();
                tt.to_tokens(&mut expr)
            }
        }

        Ok(WhenExprToVar { properties, expr })
    }
}
