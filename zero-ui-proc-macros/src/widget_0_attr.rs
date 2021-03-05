use std::{collections::HashSet, mem};

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse2, parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, FnArg, Ident, Item, ItemFn, ItemMacro, ItemMod, ItemUse, Path, Token,
};

use crate::{
    util::{self, parse2_punctuated, Attributes, Errors},
    widget_new2::{PropertyValue, When},
};

pub fn expand(mixin: bool, args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // the widget mod declaration.
    let mod_ = parse_macro_input!(input as ItemMod);

    if mod_.content.is_none() {
        let mut r = syn::Error::new(mod_.semi.span(), "only modules with inline content are supported")
            .to_compile_error()
            .to_token_stream();

        mod_.to_tokens(&mut r);

        return r.into();
    }
    let (_, items) = mod_.content.unwrap();

    // accumulate the most errors as possible before returning.
    let mut errors = Errors::default();

    let crate_core = util::crate_core();

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
        others,
        ..
    } = Attributes::new(mod_.attrs);
    let mut wgt_attrs = TokenStream::default();
    wgt_attrs.extend(quote! { #(#others)* });
    wgt_attrs.extend(quote! { #(#lints)* });
    util::docs_with_first_line_js(&mut wgt_attrs, &docs, js_tag!("widget_header.js"));

    let wgt_attrs = wgt_attrs;

    let vis = mod_.vis;
    let ident = mod_.ident;

    let WidgetItems {
        uses,
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
        let p_new_child: Vec<_> = new_child.iter().map(|id| ident!("__p_{}", id)).collect();
        quote! {
            #[doc(hidden)]
            pub fn __new_child(#(#new_child : impl self::#p_new_child::Args),*) -> impl #crate_core::UiNode {
                self::new_child(#(self::#p_new_child::Args::unwrap(#new_child)),*)
            }
        }
    });
    let new__ = new_fn.as_ref().map(|f| {
        let p_new: Vec<_> = new.iter().map(|id| ident!("__p_{}", id)).collect();
        let output = &f.sig.output;
        quote! {
            #[doc(hidden)]
            pub fn __new(__child: impl #crate_core::UiNode, #(#new: impl self::#p_new::Args),*) #output {
                self::new(__child, #(self::#p_new::Args::unwrap(#new)),*)
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
        let p_path_span = property.path_span();
        let p_value_span = property.value_span;

        if !declared_properties.insert(p_ident) {
            errors.push(format_args!("property `{}` is already declared", p_ident), p_ident.span());
            continue;
        }

        // declare new capture properties.
        if let Some((_, new_type)) = &property.type_ {
            if !captures.contains(p_ident) {
                // new capture properties must be captured by new *new* functions.
                errors.push(
                    format_args!("property `{}` is declared in widget, but is not captured by the widget", p_ident),
                    p_ident.span(),
                );
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
                    if property.alias.is_some() || property.path.get_ident().is_none() {
                        // only single name path without aliases can be referencing an inherited property.
                        errors.push("can only unset inherited property", sp.span());
                        continue;
                    }
                    // the final inherit validation is done in "widget_2_declare.rs".
                    property_unsets.extend(quote! {
                        #p_ident {
                            #sp // span sample
                        }
                    });
                    continue;
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
                let expr = default_value
                    .expr_tokens(&quote_spanned! {p_path_span=> self::#p_mod_ident }, p_path_span, p_value_span)
                    .unwrap_or_else(|e| non_user_error!(e));

                property_defaults.extend(quote! {
                    #cfg
                    #(#lints)*
                    #[doc(hidden)]
                    pub fn #fn_ident() -> impl self::#p_mod_ident::Args {
                        #expr
                    }
                });

                #[cfg(debug_assertions)]
                {
                    let loc_ident = ident!("__loc_{}", p_ident);
                    property_defaults.extend(quote_spanned! {p_ident.span()=>
                        #[doc(hidden)]
                        pub fn #loc_ident() -> #crate_core::debug::SourceLocation {
                            #crate_core::debug::source_location!()
                        }
                    });
                }
            }
        }

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
        let ident = when.make_ident("w", i);

        let attrs = Attributes::new(when.attrs);
        for invalid_attr in attrs.others.into_iter().chain(attrs.inline) {
            errors.push("only `doc`, `cfg` and lint attributes are allowed in when", invalid_attr.span());
        }
        let cfg = attrs.cfg;
        let docs = attrs.docs;
        let when_lints = attrs.lints;

        let expr_str = when.condition_expr.to_string();

        // when condition with `self.property(.member)?` converted to `#(__property__member)` for the `expr_var` macro.
        let condition = match syn::parse2::<WhenExprToVar>(when.condition_expr) {
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

            let attrs = Attributes::new(assign.attrs);
            for invalid_attr in attrs.others.into_iter().chain(attrs.inline).chain(attrs.docs) {
                errors.push("only `cfg` and lint attributes are allowed in property assign", invalid_attr.span());
            }

            if let Some(property) = assign.path.get_ident() {
                let mut skip = false;
                // validate property only assigned once in the when block.
                if !assigns.insert(property.clone()) {
                    errors.push(
                        format_args!("property `{}` already assigned in this `when` block", property),
                        property.span(),
                    );
                    skip = true;
                }
                // validate value not one of the special commands (`unset!`, `required!`).
                if let PropertyValue::Special(sp, _) = &assign.value {
                    errors.push(format_args!("`{}` not allowed in `when` block", sp), sp.span());
                    skip = true;
                }

                if skip {
                    continue;
                }

                // ident of property module in the widget.
                let prop_ident = ident!("__p_{}", property);
                // ident of the property value function.
                let fn_ident = ident!("{}__{}", ident, property);

                let cfg = util::cfg_attr_and(attrs.cfg, cfg.clone());

                assigns_tokens.extend(quote! {
                    #property {
                        cfg { #cfg }
                        value_fn { #fn_ident }
                    }
                });

                let prop_span = property.span();

                let expr = assign
                    .value
                    .expr_tokens(&quote_spanned!(prop_span=> self::#prop_ident), prop_span, assign.value_span)
                    .unwrap_or_else(|e| non_user_error!(e));
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
                #(let #field_idents = #crate_core::var::IntoVar::into_var(std::clone::Clone::clone(#input_ident_per_field.#members()));)*
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
                expr_str { #expr_str }
            }
        });
    }

    // prepare stage call
    let stage_path;
    let stage_extra;

    // [(cfg, path)]
    let inherits: Vec<_> = inherits
        .into_iter()
        .map(|inh| {
            let attrs = Attributes::new(inh.attrs);
            (attrs.cfg, inh.path)
        })
        .collect();
    let mut cfgs = inherits.iter().map(|(c, _)| c);
    let paths = inherits.iter().map(|(_, p)| p);
    let mut inherit_names = paths.clone().map(|p| ident!("__{}", util::display_path(p).replace("::", "_")));

    // module that exports the inherited items
    let inherits_mod_ident = ident!("__{}_inherit", ident);
    let inherit_reexports = cfgs
        .clone()
        .zip(paths.clone())
        .zip(inherit_names.clone())
        .map(|((cfg, path), name)| {
            quote! {
                #path! {
                    reexport=> #name #cfg
                }
            }
        });

    // inherited mods are only used in the inherit mod, this can cause
    // lints for unused imports in the widget module.
    let disable_unused_warnings_for_inherits = quote! {
        #[allow(unused)]
        fn __use_inherits() {
            #(use #paths;)*
        }
    };

    if mixin {
        // mixins don't inherit the implicit_mixin so we go directly to stage_2_declare or to the first inherit.
        if inherits.is_empty() {
            stage_path = quote!(#crate_core::widget_declare!);
            stage_extra = TokenStream::default();
        } else {
            let cfg = cfgs.next().unwrap();
            let not_cfg = util::cfg_attr_not(cfg.clone());
            let next_path = inherit_names.next().unwrap();
            stage_path = quote!(#inherits_mod_ident::#next_path!);
            stage_extra = quote! {
                inherit=>
                cfg { #cfg }
                not_cfg { #not_cfg }
                inherit {
                    #(
                        #cfgs
                        #inherits_mod_ident::#inherit_names
                    )*
                }
            }
        }
    } else {
        // not-mixins inherit from the implicit_mixin first so we call inherit=> for that:
        // TODO rename implicit_mixin2 to implicit_mixin
        stage_path = quote!(#crate_core::widget_base::implicit_mixin2!);
        stage_extra = quote! {
            inherit=>
            cfg { }
            not_cfg { #[cfg(zero_ui_never_set)] }
            inherit {
                #(
                    #cfgs
                    #inherits_mod_ident::#inherit_names
                )*
            }
        };
    }

    #[cfg(debug_assertions)]
    let debug_reexport = quote! {debug::source_location};
    #[cfg(not(debug_assertions))]
    let debug_reexport = TokenStream::default();

    let r = quote! {
        #errors

        #[allow(unused)]
        mod #inherits_mod_ident {
            #(#uses)*
            #(#inherit_reexports)*
        }

        // inherit=> will include an `inherited { .. }` block with the widget data after the
        // `inherit { .. }` block and take the next `inherit` path turn that into an `inherit=>` call.
        // This way we "eager" expand the inherited data recursively, when there no more path to inherit
        // a call to `widget_declare!` is made.
        #stage_path {
            #stage_extra

            widget {
                module { #mod_path }
                attrs { #wgt_attrs }
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
                    #(#uses)*
                    #(#others)*
                    #new_child_fn
                    #new_fn

                    #new_child__
                    #new__

                    #property_declarations

                    #property_defaults

                    #when_conditions
                    #when_defaults

                    #[doc(hidden)]
                    pub mod __core {
                        // TODO: Update widget_new2 to widget_new when it's ready.
                        pub use #crate_core::{widget_inherit, widget_new2 as widget_new, var, #debug_reexport};
                    }

                    #disable_unused_warnings_for_inherits
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
        let fork = input.fork();
        match (fork.parse::<Token![$]>(), fork.parse::<syn::Path>()) {
            (Ok(_), Ok(p)) => {
                if fork.is_empty() {
                    if p.segments[0].ident == "crate" {
                        Ok(ArgPath {
                            path: input.parse().unwrap(),
                        })
                    } else {
                        Err(syn::Error::new(p.segments[0].ident.span(), "expected `crate`"))
                    }
                } else {
                    Err(syn::Error::new(fork.span(), "unexpected token"))
                }
            }
            (Ok(_), Err(e)) => {
                if !util::span_is_call_site(e.span()) {
                    Err(e)
                } else {
                    Err(syn::Error::new(util::last_span(input.parse().unwrap()), e.to_string()))
                }
            }
            _ => Err(syn::Error::new(
                input.span(),
                "expected a macro_rules `$crate` path to this widget mod",
            )),
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
    uses: Vec<ItemUse>,
    inherits: Vec<Inherit>,
    properties: Vec<Properties>,
    new_child_fn: Option<ItemFn>,
    new_fn: Option<ItemFn>,
    others: Vec<Item>,
}
impl WidgetItems {
    fn new(items: Vec<Item>, errors: &mut Errors) -> Self {
        let mut uses = vec![];
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
                Item::Use(use_) => {
                    uses.push(use_);
                }
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
            uses,
            inherits,
            properties,
            new_child_fn,
            new_fn,
            others,
        }
    }
}

struct Inherit {
    attrs: Vec<Attribute>,
    path: Path,
}
impl Parse for Inherit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Inherit {
            attrs: Attribute::parse_outer(input)?,
            path: input.parse()?,
        })
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
            } else if input.peek(Ident::peek_any) {
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
    pub value_span: Span,
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
        let path = input.parse()?;
        let alias = peek_parse![as];
        let type_ = peek_parse![:];

        let mut value_span = Span::call_site();
        let value = if input.peek(Token![=]) {
            let eq = input.parse()?;

            let mut value_stream = TokenStream::new();
            while !input.is_empty() && !input.peek(Token![;]) {
                let tt: TokenTree = input.parse().unwrap();
                tt.to_tokens(&mut value_stream);
            }
            value_span = value_stream.span();

            Some((eq, syn::parse2(value_stream)?))
        } else {
            None
        };

        Ok(ItemProperty {
            attrs: vec![],
            path,
            alias,
            type_,
            value,
            value_span,
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

    fn path_span(&self) -> Span {
        self.alias.as_ref().map(|(_, id)| id.span()).unwrap_or_else(|| self.path.span())
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
