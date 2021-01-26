use std::{collections::HashSet, mem};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse2, parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, FnArg, Ident, Item, ItemFn, ItemMacro, ItemMod, Path, Token,
};

use crate::{
    util::{self, parse2_punctuated, Attributes, Errors},
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
    let mod_path = match syn::parse::<ArgPath>(args) {
        Ok(a) => a.path,
        Err(e) => {
            errors.push_syn(e);
            quote! { $crate::missing_widget_path}
        }
    };

    let Attributes {
        cfg: wgt_cfg,
        docs,
        lints,
        others: mut wgt_attrs,
        ..
    } = Attributes::new(mod_.attrs);
    wgt_attrs.extend(lints);
    wgt_attrs.extend(docs);
    let wgt_attrs = wgt_attrs;

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
    let mut child_properties: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.child_properties)).collect();
    let mut properties: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.properties)).collect();

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
    let new_child_declared = new_child_fn.is_some();
    let new_child = new_child_fn
        .as_ref()
        .map(|f| new_fn_captures(f.sig.inputs.iter(), &mut errors))
        .unwrap_or_default();
    let new_declared = new_fn.is_some();
    let new = new_fn
        .as_ref()
        .map(|f| new_fn_captures(f.sig.inputs.iter().skip(1), &mut errors))
        .unwrap_or_default();
    let mut captures = HashSet::new();
    for capture in new_child.iter().chain(&new) {
        if !captures.insert(capture) {
            errors.push(format_args!("property `{}` already captured", capture), capture.span());
        }
    }
    let captures = captures;

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
    // captured property existence validation happens "widget_2_declare.rs"

    // process properties
    let mut declared_properties = HashSet::new();
    let mut built_properties_child = TokenStream::default();
    let mut built_properties = TokenStream::default();
    let mut property_defaults = TokenStream::default();
    let mut property_declarations = TokenStream::default();
    let mut property_declared_idents = TokenStream::default();
    let mut property_unsets = TokenStream::default();
    for (property, is_child_property) in child_properties
        .iter_mut()
        .map(|p| (p, true))
        .chain(properties.iter_mut().map(|p| (p, false)))
    {
        let attrs = Attributes::new(mem::take(&mut property.attrs));
        for invalid_attr in attrs.others.iter().chain(attrs.inline.iter()) {
            errors.push(
                "only `doc`, `cfg` and lint attributes are allowed in properties",
                invalid_attr.span(),
            );
        }

        let p_ident = property.ident();
        if !declared_properties.insert(p_ident) {
            errors.push(format_args!("property `{}` is already declared", p_ident), p_ident.span());
            continue;
        }

        // declare new capture properties.
        if let Some((_, new_type)) = &property.type_ {
            if !captures.contains(p_ident) {
                // new capture properties must be captured by new *new* functions.
                errors.push(
                    format_args!("property `{}` is declared in widget but is not captured by widget", p_ident),
                    p_ident.span(),
                );
                continue;
            }

            let p_mod_ident = ident!("__p_{}", p_ident);
            let inputs = new_type.fn_input_tokens(p_ident);

            property_declarations.extend(quote! {
                #[doc(hidden)]
                #[#crate_core::property(capture_only)]
                pub fn #p_mod_ident(#inputs) -> ! { }
            });

            // so "widget_2_declare.rs" skips reexporting this one.
            p_ident.to_tokens(&mut property_declared_idents);
        }

        let mut default = false;
        let mut required = false;

        // process default value or special value.
        if let Some((_, default_value)) = &property.value {
            if let PropertyValue::Special(sp, _) = default_value {
                if sp == "unset" {
                    if property.alias.is_some() || property.path.get_ident().is_some() {
                        // only single name path without aliases can be referencing an inherited property.
                        errors.push("can only unset inherited property", sp.span());
                        continue;
                    }
                    // the final inherit validation is done in "widget_2_declare.rs".
                    p_ident.to_tokens(&mut property_unsets);
                } else if sp == "required" {
                    required = true;
                } else {
                    // unknown special.
                    errors.push(format_args!("unexpected `{}!` in default value", sp), sp.span());
                    continue;
                }
            } else {
                default = true;
                let cfg = &attrs.cfg;
                let lints = attrs.lints;
                let fn_ident = ident!("__d_{}", p_ident);
                let p_mod_ident = ident!("__p_{}", p_ident);
                let expr = default_value.expr_tokens(&quote! { self::#p_mod_ident });

                property_defaults.extend(quote! {
                    #cfg
                    #(#lints)*
                    #[doc(hidden)]
                    pub fn #fn_ident() -> impl self::#p_mod_ident::Args {
                        #expr
                    }
                });
            }
        }

        // all captures are required
        //
        // note: a property can become required in "widget_2_declare.rs" if the
        // widget is inheriting *new* functions.
        required |= captures.contains(p_ident);

        let docs = attrs.docs;
        let cfg = attrs.cfg;
        let path = &property.path;

        let built_properties = if is_child_property {
            &mut built_properties_child
        } else {
            &mut built_properties
        };
        built_properties.extend(quote! {
            #p_ident {
                docs { #(#docs)* }
                cfg { #cfg }
                path { #path }
                default { #default }
                required { #required }
            }
        });
    }
    drop(declared_properties);

    // process whens
    let mut built_whens = TokenStream::default();
    let mut when_conditions = TokenStream::default();
    let mut when_defaults = TokenStream::default();
    for (i, when) in whens.into_iter().enumerate() {
        // when ident, `__w{i}_{condition_expr_to_str}`
        let ident = when.make_ident(i);

        let attrs = Attributes::new(when.attrs);
        for invalid_attr in attrs.others.into_iter().chain(attrs.inline) {
            errors.push("only `doc`, `cfg` and lint attributes are allowed in when", invalid_attr.span());
        }
        let cfg = attrs.cfg;
        let docs = attrs.docs;
        let when_lints = attrs.lints;

        // when condition with `self.property(.member)?` converted to `#(__property__member)` for the `expr_var` macro.
        let condition = match syn::parse2::<WhenExprToVar>(when.condition_expr.to_token_stream()) {
            Ok(c) => c,
            Err(e) => {
                errors.push_syn(e);
                continue;
            }
        };

        let mut assigns = HashSet::new();
        let mut assigns_tokens = TokenStream::default();
        for assign in when.assigns {
            // property default value validation happens "widget_2_declare.rs"

            if let Some(property) = assign.path.get_ident() {
                let attrs = Attributes::new(assign.attrs);
                for invalid_attr in attrs.others.into_iter().chain(attrs.inline).chain(attrs.docs) {
                    errors.push("only `cfg` and lint attributes are allowed in property assign", invalid_attr.span());
                }

                // validate property only assigned once in the when block.
                if !assigns.insert(property.clone()) {
                    errors.push(
                        format_args!("property `{}` already assigned in this `when` block", property),
                        property.span(),
                    );
                    continue;
                }
                // validate value not one of the special commands (`unset!`, `required!`).
                if let PropertyValue::Special(sp, _) = &assign.value {
                    errors.push(format_args!("`{}` not allowed in `when` block", sp), sp.span());
                    continue;
                }

                // ident of property module in the widget.
                let prop_ident = ident!("__p_{}", property);
                // ident of the property value function.
                let fn_ident = ident!("{}__{}", ident, property);

                let cfg = util::merge_cfg_attr(attrs.cfg, cfg.clone());

                assigns_tokens.extend(quote! {
                    #property {
                        cfg { #cfg }
                    }
                });

                let expr = assign.value.expr_tokens(&quote!(self::#prop_ident));
                let lints = attrs.lints;

                when_defaults.extend(quote! {
                    #cfg
                    #(#when_lints)*
                    #(#lints)*
                    #[doc(hidden)]
                    pub fn #fn_ident() -> impl self::#prop_ident::Args {
                        #expr
                    }
                });
            } else {
                let suggestion = &assign.path.segments.last().unwrap().ident;
                errors.push(
                    format_args!("widget properties only have a single name, try `{}`", suggestion),
                    assign.path.span(),
                );
            }
        }

        // properties used in the when condition.
        let inputs: HashSet<_> = condition.properties.iter().map(|(p, _)| p).collect();

        // name of property inputs Args reference in the condition function.
        let input_idents = inputs.iter().map(|p| ident!("__{}", p));
        // name of property inputs in the widget module.
        let prop_idents = inputs.iter().map(|p| ident!("__p_{}", p));

        // name of the fields for each interpolated property
        let field_idents = condition.properties.iter().map(|(p, m)| ident!("__{}{}", p, m));
        let input_ident_per_field = condition.properties.iter().map(|(p, _)| ident!("__{}", p));
        let members = condition.properties.iter().map(|(_, m)| m);

        let expr = condition.expr;

        when_conditions.extend(quote! {
            #cfg
            #(#when_lints)*
            #[doc(hidden)]
            pub fn #ident(#(#input_idents : &impl self::#prop_idents::Args),*) -> impl #crate_core::var::Var<bool> {
                #(let #field_idents = #crate_core::var::IntoVar::into_var(std::clone::Clone::clone(#input_ident_per_field.#members())))*
                #crate_core::var::expr_var! {
                    #expr
                }
            }
        });

        let inputs = inputs.iter();
        built_whens.extend(quote! {
            #ident {
                docs { #(#docs)* }
                cfg { #cfg }
                inputs {
                    #(#inputs),*
                }
                assigns {
                    #assigns_tokens
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
                module { #mod_path }
                attrs { #(#wgt_attrs)* }
                cfg { #wgt_cfg }
                vis { #vis }
                ident { #ident }
                mixin { #mixin }

                properties_unset {
                    #property_unsets
                }
                properties_declared {
                    #property_declared_idents
                }

                properties_child {
                    #built_properties_child
                }
                properties {
                    #built_properties
                }
                whens {
                    #built_whens
                }

                new_child_declared { #new_child_declared }
                new_child { #(#new_child)* }
                new_declared { #new_declared }
                new { #(#new)* }

                mod_items {
                    #(#others)*
                    #new_child_fn
                    #new_fn

                    #new_child__
                    #new__

                    #property_defaults

                    #when_conditions
                    #when_defaults

                    #[doc(hidden)]
                    pub mod __core {
                        // TODO: Update widget_new2 to widget_new when it's ready.
                        pub use #crate_core::{widget_inherit, widget_new2 as widget_new, var};
                    }
                }

            }
        }
    };

    r.into()
}

struct ArgPath {
    path: TokenStream,
}

impl Parse for ArgPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![$]) && input.peek2(Token![crate]) && input.peek3(Token![::]) {
            Ok(ArgPath {
                path: input.parse().unwrap(),
            })
        } else {
            Err(syn::Error::new(
                input.span(),
                "expected a macro_rules `$crate` path to this widget mod",
            ))
        }
    }
}

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
                let input = non_user_braced!(input, "child");
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
            value: if input.peek(Token![=]) {
                Some((input.parse()?, {
                    let mut value_stream = TokenStream::new();
                    while !input.is_empty() && !input.peek(Token![;]) {
                        let tt: TokenTree = input.parse().unwrap();
                        tt.to_tokens(&mut value_stream);
                    }
                    syn::parse2(value_stream)?
                }))
            } else {
                None
            },
            semi: if input.peek(Token![;]) { Some(input.parse()?) } else { None },
        })
    }
}
impl ItemProperty {
    /// The property ident.
    fn ident(&self) -> &Ident {
        self.alias
            .as_ref()
            .map(|(_, id)| id)
            .unwrap_or_else(|| &self.path.segments.last().unwrap().ident)
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
impl PropertyType {
    fn fn_input_tokens(&self, property: &Ident) -> TokenStream {
        match self {
            PropertyType::Named(_, fields) => fields.to_token_stream(),
            PropertyType::Unnamed(unamed) => {
                if unamed.len() == 1 {
                    quote! { #property: #unamed }
                } else {
                    let names = (0..unamed.len()).map(|i| ident!("arg{}", i));
                    quote! { #(#names: #unamed),* }
                }
            }
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
impl ToTokens for NamedField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        self.colon.to_tokens(tokens);
        self.ty.to_tokens(tokens);
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
                let inner = WhenExprToVar::parse(&non_user_braced!(input))?;
                properties.extend(inner.properties);
                let inner = inner.expr;
                expr.extend(quote! { { #inner } });
            } else if input.peek(token::Paren) {
                let inner = WhenExprToVar::parse(&non_user_parenthesized!(input))?;
                properties.extend(inner.properties);
                let inner = inner.expr;
                expr.extend(quote! { ( #inner ) });
            } else if input.peek(token::Bracket) {
                let inner = WhenExprToVar::parse(&non_user_bracketed!(input))?;
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
