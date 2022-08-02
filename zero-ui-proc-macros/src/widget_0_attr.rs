use std::fmt;
use std::{
    collections::{HashMap, HashSet},
    mem,
};

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse2, parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, FnArg, Ident, Item, ItemFn, ItemMacro, ItemMod, ItemUse, Path, Token,
};

use crate::{
    util::{self, parse_outer_attrs, Attributes, ErrorRecoverable, Errors},
    widget_new::{PropertyValue, When, WhenExprToVar},
};

pub fn expand(mixin: bool, is_base: bool, args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // the widget mod declaration.
    let mod_ = parse_macro_input!(input as ItemMod);

    let uuid = util::uuid(&args); // full path to widget should be unique.

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

    let vis = mod_.vis;
    let ident = mod_.ident;

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

    wgt_attrs.extend(quote! {
        #[doc="**`widget`** "]
    });
    for doc in &docs {
        doc.to_tokens(&mut wgt_attrs);
    }
    wgt_attrs.extend(quote! {
        ///
        /// <iframe id="wgt-docs-iframe" src="__DOCS/index.html" width="1" height="1" style="border:none;"></iframe>
    });
    let wgt_attrs = wgt_attrs;

    // a `$crate` path to the widget module.
    let mod_path;
    let mod_path_assert;
    match syn::parse::<ArgPath>(args) {
        Ok(a) => {
            let assert_mod_path = ident!("__{ident}_assert_mod_path_{}", uuid);
            mod_path = a.path;
            mod_path_assert = quote! {
                #wgt_cfg
                #[allow(unused)]
                mod #assert_mod_path {
                    macro_rules! #assert_mod_path {
                        () => {
                            use #mod_path;
                        };
                    }
                    #assert_mod_path!{}
                }
            }
        }
        Err(e) => {
            errors.push_syn(e);
            mod_path = quote! { $crate::missing_widget_path};
            mod_path_assert = quote! {};
        }
    }

    let WidgetItems {
        uses,
        inherits,
        mut properties,
        mut new_fns,
        mut others,
    } = WidgetItems::new(items, &mut errors);

    #[allow(clippy::needless_collect)] // false positive, see https://github.com/rust-lang/rust-clippy/issues/7512
    let whens: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.whens)).collect();
    let removes: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.removes)).collect();
    let mut properties: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.properties)).collect();

    if mixin {
        for (_, fn_) in new_fns.drain(..) {
            errors.push(
                format!("widget mixins do not have a `{}` function", &fn_.sig.ident),
                fn_.sig.ident.span(),
            );
            others.push(syn::Item::Fn(fn_))
        }
    }

    if is_base {
        #[cfg(inspector)]
        for priority in FnPriority::all() {
            assert!(new_fns.iter().any(|(k, _)| k == priority));
        }
    }

    // Does some validation of new signatures.
    // Further type validation is done by `rustc` when we call the function
    // in the generated `__new_child` .. `__new` functions.
    for (priority, fn_) in &mut new_fns {
        validate_new_fn(fn_, &mut errors);
        if *priority != FnPriority::NewChild && fn_.sig.inputs.is_empty() {
            errors.push(
                format!("`{}` must take at least one input that implements `UiNode`", fn_.sig.ident),
                fn_.sig.paren_token.span,
            );
            fn_.sig.inputs.push(parse_quote! { __child: impl #crate_core::UiNode });
        }
    }

    // collects name of captured properties, spans new input types and validates inputs.
    let mut new_captures = vec![]; // [Vec<Ident>]
    let mut new_captures_cfg = vec![]; // [TokenStream]
    let mut new_arg_ty_spans = vec![]; // [child_ty:Span, cap_ty:Span..]
    let mut captured_properties = HashMap::new();

    for priority in FnPriority::all() {
        if let Some((_, fn_)) = new_fns.iter().find(|(k, _)| k == priority) {
            let mut args = fn_.sig.inputs.iter();
            let mut ty_spans = vec![];
            if *priority != FnPriority::NewChild {
                let child_ty_span = args
                    .next()
                    .map(|a| if let FnArg::Typed(pt) = a { pt.ty.span() } else { a.span() })
                    .unwrap_or_else(Span::call_site);
                ty_spans.push(child_ty_span);
            } else {
                ty_spans.push(Span::call_site());
            }
            let (caps, cfgs, cap_ty_spans) = new_fn_captures(args, &mut errors);
            ty_spans.extend(cap_ty_spans);

            for cap in &caps {
                if let Some(other_fn) = captured_properties.insert(cap.clone(), *priority) {
                    captured_properties.insert(cap.clone(), other_fn);
                    errors.push(format_args!("property `{cap}` already captured in `{other_fn}`"), cap.span());
                }
            }

            new_captures.push(caps);
            new_captures_cfg.push(cfgs);
            new_arg_ty_spans.push(ty_spans);
        } else {
            new_captures.push(vec![]);
            new_captures_cfg.push(vec![]);
            new_arg_ty_spans.push(vec![]);
        }
    }

    // generate `__new_child` .. `__new` if new functions are defined in the widget.
    //
    // we use generic types to provide an error message for compile errors like the one in
    // `widget/new_fn_mismatched_capture_type1.rs`
    let mut new_declarations = vec![]; // [FnPriority:TokenStream]

    for (i, priority) in FnPriority::all().iter().enumerate() {
        if let Some((_, fn_)) = new_fns.iter().find(|(k, _)| k == priority) {
            let caps = &new_captures[i];
            let cfgs = &new_captures_cfg[i];
            let arg_ty_spans = &new_arg_ty_spans[i];

            // property modules re-exported by widget
            let prop_idents: Vec<_> = caps.iter().map(|p| ident!("__p_{p}")).collect();

            // generic types for each captured property, contains a "type" error message.
            let generic_tys: Vec<_> = caps
                .iter()
                .enumerate()
                .map(|(i, p)| ident!("{p}_type_must_match_property_definition_{i}"))
                .collect();

            // input property idents with the span of input types.
            let spanned_inputs: Vec<_> = caps
                .iter()
                .zip(arg_ty_spans.iter().skip(1))
                .enumerate()
                .map(|(i, (id, ty_span))| ident_spanned!(*ty_span=> "__{i}_{id}"))
                .collect();

            // calls to property args unwrap with the span of input types.
            let spanned_unwrap = prop_idents
                .iter()
                .zip(spanned_inputs.iter())
                .zip(cfgs.iter())
                .map(|((p, id), cfg)| {
                    quote_spanned! {id.span()=>
                        #cfg
                        self::#p::Args::unwrap(#id)
                    }
                });

            // tokens that handle the `arg0: impl UiNode`.
            let mut child_decl = TokenStream::new();
            let mut child_pass = TokenStream::new();
            if *priority != FnPriority::NewChild {
                let span = *arg_ty_spans.get(0).unwrap_or_else(|| non_user_error!(""));
                let child_ident = ident_spanned!(span=> "__child");
                let child_ty = quote_spanned! {span=>
                    impl #crate_core::UiNode
                };
                child_decl = quote! { #child_ident: #child_ty, };
                child_pass = quote_spanned! {span=> box_fix(#child_ident), }
            }

            // output type, for `new` is a copy, for others is `impl UiNode` to validate the type.
            let output;
            let out_ident;
            if *priority == FnPriority::New {
                output = fn_.sig.output.to_token_stream();
                out_ident = ident!("out"); // not used
            } else {
                let out_span = match &fn_.sig.output {
                    syn::ReturnType::Default => fn_.block.span(),
                    syn::ReturnType::Type(_, t) => t.span(),
                };
                output = quote_spanned! {out_span=>
                    -> impl #crate_core::UiNode
                };
                out_ident = ident_spanned!(out_span=> "out");
            };

            // declare `__new_*`
            let new_id = ident!("{priority}");
            let new__ = ident!("__{priority}");

            let mut r = TokenStream::new();

            if *priority != FnPriority::New {
                r.extend(quote! {
                    #[doc(hidden)]
                    #[allow(clippy::too_many_arguments)]
                    pub fn #new__<#(#cfgs #generic_tys: #prop_idents::Args),*>(#child_decl #(#cfgs #spanned_inputs : #generic_tys),*) #output {
                        // rustc gets confused about lifetimes if we call cfg_boxed directly
                        fn box_fix(node: impl #crate_core::UiNode)#output {
                            #crate_core::UiNode::cfg_boxed(node)
                        }
                        let #out_ident = self::#new_id(#child_pass #(#spanned_unwrap),*);
                        box_fix(#out_ident)
                    }
                });
            } else {
                debug_assert_eq!(*priority, FnPriority::New);
                r.extend(quote! {
                    #[doc(hidden)]
                    #[allow(clippy::too_many_arguments)]
                    pub fn #new__<#(#cfgs #generic_tys: #prop_idents::Args),*>(#child_decl #(#cfgs #spanned_inputs : #generic_tys),*) #output {
                        #crate_core::core_cfg_dyn_widget! {
                            fn box_fix(node: impl #crate_core::UiNode) -> impl #crate_core::UiNode {
                                #crate_core::UiNode::boxed(node)
                            }
                        }
                        #crate_core::core_cfg_dyn_widget! {@NOT
                            fn box_fix(node: impl #crate_core::UiNode) -> impl #crate_core::UiNode {
                                #crate_core::UiNode::cfg_boxed(node)
                            }
                        }
                        self::#new_id(#child_pass #(#spanned_unwrap),*)
                    }
                });
            }

            // declare `__new_*_inspect`
            {
                let new_inspect__ = ident!("__{priority}_inspect");
                let names = caps.iter().map(|id| id.to_string());
                let locations = caps.iter().map(|id| {
                    quote_spanned! {id.span()=>
                        #crate_core::inspector::source_location!()
                    }
                });
                let assigned_flags: Vec<_> = caps.iter().enumerate().map(|(i, id)| ident!("__{i}_{id}_user_set")).collect();

                if *priority == FnPriority::New {
                    r.extend(quote! { #crate_core::core_cfg_inspector! {

                        #[doc(hidden)]
                        #[allow(clippy::too_many_arguments)]
                        pub fn #new_inspect__(
                            __child: impl #crate_core::UiNode,
                            #(#cfgs #caps : impl self::#prop_idents::Args,)*
                            #(#cfgs #assigned_flags: bool,)*
                            __widget_name: &'static str,
                            __whens: std::vec::Vec<#crate_core::inspector::WhenInfoV1>,
                            __decl_location: #crate_core::inspector::SourceLocation,
                            __instance_location: #crate_core::inspector::SourceLocation,
                        ) #output {
                            let __child = #crate_core::UiNode::boxed(__child);
                            #[allow(unused_mut)]
                            let mut __captures = std::vec![];
                            #(
                                #cfgs
                                __captures.push(self::#prop_idents::captured_inspect(&#caps, #names, #locations, #assigned_flags));
                            )*
                            let __child = #crate_core::inspector::WidgetNewFnInfoNode::new_v1(
                                __child,
                                #crate_core::inspector::WidgetNewFnV1::New,
                                __captures,
                            );
                            let __child = #crate_core::inspector::WidgetInstanceInfoNode::new_v1(
                                __child,
                                __widget_name,
                                __decl_location,
                                __instance_location,
                                __whens,
                            );
                            self::__new(__child, #(#cfgs #caps),*)
                        }

                    }});
                } else {
                    let priority_variant = ident!("{priority:?}");
                    r.extend(quote! { #crate_core::core_cfg_inspector! {
                        #[doc(hidden)]
                        #[allow(clippy::too_many_arguments)]
                        pub fn #new_inspect__(
                            #child_decl
                            #(#cfgs #caps : impl self::#prop_idents::Args,)*
                            #(#cfgs #assigned_flags: bool,)*
                        ) -> #crate_core::inspector::WidgetNewFnInfoNode {
                            #[allow(unused_mut)]
                            let mut __captures = std::vec![];
                            #(
                                #cfgs
                                __captures.push(#prop_idents::captured_inspect(&#caps, #names, #locations, #assigned_flags));
                            )*
                            let out = self::#new__(#child_pass #(#cfgs #caps),*);

                            fn box_fix(node: impl #crate_core::UiNode) -> #crate_core::BoxedUiNode {
                                #crate_core::UiNode::boxed(node)
                            }
                            #crate_core::inspector::WidgetNewFnInfoNode::new_v1(
                                box_fix(out),
                                #crate_core::inspector::WidgetNewFnV1::#priority_variant,
                                __captures,
                            )
                        }
                    }});
                }
            }

            new_declarations.push(r);
        } else {
            new_declarations.push(TokenStream::new());
        }
    }

    let debug_info = {
        let decl_location = quote_spanned!(ident.span()=> #crate_core::inspector::source_location!());
        let wgt_name = ident.to_string();
        quote! { #crate_core::core_cfg_inspector! {
            #[doc(hidden)]
            pub fn __widget_name() -> &'static str {
                #wgt_name
            }

            #[doc(hidden)]
            pub fn __decl_location() -> #crate_core::inspector::SourceLocation {
                #decl_location
            }
        }}
    };

    // captured property existence validation happens "widget_2_declare.rs"

    // process properties
    let mut declared_properties = HashSet::new();
    let mut built_properties = TokenStream::default();
    let mut property_defaults = TokenStream::default();
    let mut property_declarations = TokenStream::default();
    let mut property_declared_idents = TokenStream::default();
    let mut property_removes = TokenStream::default();

    // process removes
    let mut visited_removes = HashSet::new();
    for ident in &removes {
        if !visited_removes.insert(ident) {
            errors.push(format_args!("property `{ident}` already removed"), ident.span());
            continue;
        }

        ident.to_tokens(&mut property_removes);
    }
    drop(visited_removes);
    drop(removes);

    // process declarations
    for property in &mut properties {
        let mut attrs = Attributes::new(mem::take(&mut property.attrs));

        // #[allowed_in_when = <bool>]
        // applies for when a capture_only property is being declared.
        let allowed_in_when = {
            if let Some(i) = attrs
                .others
                .iter()
                .position(|a| a.path.get_ident().map(|id| id == "allowed_in_when").unwrap_or_default())
            {
                let attr = attrs.others.remove(i);
                match syn::parse2::<AllowedInWhenInput>(attr.tokens) {
                    Ok(args) => args.flag.value,
                    Err(mut e) => {
                        if util::span_is_call_site(e.span()) {
                            e = syn::Error::new(attr.path.span(), e);
                        }
                        errors.push_syn(e);

                        false
                    }
                }
            } else {
                true
            }
        };
        let required = {
            if let Some(i) = attrs
                .others
                .iter()
                .position(|a| a.path.get_ident().map(|id| id == "required").unwrap_or_default())
            {
                let attr = attrs.others.remove(i);
                if !attr.tokens.is_empty() {
                    errors.push("unexpected token", attr.tokens.span());
                }
                true
            } else {
                false
            }
        };
        for invalid_attr in attrs.others.iter().chain(attrs.inline.iter()) {
            errors.push(
                "only `allowed_in_when`, `cfg`, `doc`, `required` and lint attributes are allowed in properties",
                util::path_span(&invalid_attr.path),
            );
        }

        let p_ident = property.ident();
        let p_path_span = property.path_span();
        let p_value_span = property.value_span;

        if !declared_properties.insert(p_ident) {
            errors.push(format_args!("property `{p_ident}` is already declared"), p_ident.span());
            continue;
        }

        let mut skip = false;

        // declare new capture properties.
        if let Some((_, new_type)) = &property.type_ {
            if let Some((as_, _)) = &property.alias {
                errors.push("cannot use alias while naming a new capture-only property", as_.span());
            } else if property.path.get_ident().is_none() {
                errors.push("cannot use path while naming a new capture-only property", property.path.span());
            }

            if mixin {
                errors.push(
                    format_args!("capture-only properties cannot be declared in mix-ins"),
                    p_ident.span(),
                );
                skip = true;
            } else if !captured_properties.contains_key(p_ident) {
                // new capture properties must be captured by new *new* functions.
                errors.push(
                    format_args!("property `{p_ident}` is declared in widget, but is not captured by the widget"),
                    p_ident.span(),
                );
                skip = true;
            }

            let p_mod_ident = ident!("__p_{p_ident}");
            let inputs = new_type.fn_input_tokens(p_ident);

            let docs = &attrs.docs;

            // the panic message is to cause an UUID for the property.
            property_declarations.extend(quote! {
                #(#docs)*
                #[#crate_core::property(capture_only, allowed_in_when = #allowed_in_when)]
                pub fn #p_mod_ident(#inputs) -> ! { panic!("capture-only property for {:?}", stringify!(#mod_path)) }
            });

            // so "widget_2_declare.rs" skips reexporting this one.
            p_ident.to_tokens(&mut property_declared_idents);
        }

        let mut default = false;

        // process default value or special value.
        if let Some((_, default_value)) = &property.value {
            if let PropertyValue::Special(sp, _) = default_value {
                errors.push(format_args!("unexpected `{sp}!` as default value"), sp.span());
                continue;
            } else {
                default = true;
                let cfg = &attrs.cfg;
                let lints = attrs.lints;
                let fn_ident = ident!("__d_{p_ident}");
                let p_mod_ident = ident!("__p_{p_ident}");
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

                {
                    let loc_ident = ident!("__loc_{p_ident}");
                    property_defaults.extend(quote_spanned! {p_ident.span()=>
                        #crate_core::core_cfg_inspector! {
                            #cfg
                            #[doc(hidden)]
                            pub fn #loc_ident() -> #crate_core::inspector::SourceLocation {
                                #crate_core::inspector::source_location!()
                            }
                        }
                    });
                }
            }
        }

        if let Some(cfg) = &attrs.cfg {
            let not_cfg = util::cfg_attr_not(attrs.cfg.clone());

            let cfg_ident = ident!("__cfg_{p_ident}");

            property_declarations.extend(quote! {
                #cfg
                #[doc(hidden)]
                pub use #crate_core::core_cfg_ok as #cfg_ident;

                #not_cfg
                #[doc(hidden)]
                pub use #crate_core::core_cfg_ignore as #cfg_ident;
            });
        }

        if skip {
            continue;
        }

        let docs = attrs.docs;
        let cfg = attrs.cfg.is_some();
        let path = &property.path;

        let declared = property.type_.is_some();
        built_properties.extend(quote! {
            #p_ident {
                docs { #(#docs)* }
                cfg { #cfg }
                path { #path }
                default { #default }
                required { #required }
                declared { #declared }
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
        let ident = when.make_ident("w", i, Span::call_site());

        let dbg_ident = when.make_ident("wd", i, Span::call_site());

        let attrs = Attributes::new(when.attrs);
        for invalid_attr in attrs.others.into_iter().chain(attrs.inline) {
            errors.push(
                "only `doc`, `cfg` and lint attributes are allowed in when",
                util::path_span(&invalid_attr.path),
            );
        }
        let cfg = attrs.cfg;
        let docs = attrs.docs;
        let when_lints = attrs.lints;

        let expr_str = util::format_rust_expr(when.condition_expr.to_string());

        // when condition with `self.property(.member)?` converted to `#(__property__member)` for the `expr_var` macro.
        let condition = match syn::parse2::<WhenExprToVar>(when.condition_expr) {
            Ok(c) => c,
            Err(e) => {
                errors.push_syn(e);
                continue;
            }
        };

        let mut skip = false;
        let cond_properties: HashMap<_, _> = condition
            .properties
            .into_iter()
            .filter_map(|((p_path, member), var)| {
                if let Some(p) = p_path.get_ident() {
                    Some(((p.clone(), member), var))
                } else {
                    skip = true;
                    let suggestion = &p_path.segments.last().unwrap().ident;
                    errors.push(
                        format_args!("widget properties only have a single name, try `self.{suggestion}`"),
                        p_path.span(),
                    );
                    None
                }
            })
            .collect();

        if skip {
            continue;
        }

        #[allow(unused_mut)]
        let mut assign_names: Vec<String> = vec![];
        let mut assigns = HashSet::new();
        let mut assigns_tokens = TokenStream::default();
        for assign in when.assigns {
            // property default value validation happens "widget_2_declare.rs"

            let attrs = Attributes::new(assign.attrs);
            for invalid_attr in attrs.others.into_iter().chain(attrs.inline).chain(attrs.docs) {
                errors.push(
                    "only `cfg` and lint attributes are allowed in property assign",
                    util::path_span(&invalid_attr.path),
                );
            }

            if let Some(property) = assign.path.get_ident() {
                let mut skip = false;
                // validate property only assigned once in the when block.
                if !assigns.insert(property.clone()) {
                    errors.push(
                        format_args!("property `{property}` already set in this `when` block"),
                        property.span(),
                    );
                    skip = true;
                }
                // validate value is not one of the special commands.
                // TODO: change the error message, or revise this after `Special` becomes `Unset`.
                if let PropertyValue::Special(sp, _) = &assign.value {
                    // unknown special.
                    errors.push(format_args!("unexpected `{sp}!` in property value"), sp.span());

                    skip = true;
                }

                if skip {
                    continue;
                }

                #[cfg(inspector)]
                assign_names.push(property.to_string());

                // ident of property module in the widget.
                let prop_ident = ident!("__p_{property}");
                // ident of the property value function.
                let fn_ident = ident!("{ident}__{property}");

                let cfg = util::cfg_attr_and(attrs.cfg, cfg.clone());
                let has_cfg = cfg.is_some();

                assigns_tokens.extend(quote! {
                    #property {
                        cfg { #has_cfg }
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

                if let Some(cfg) = cfg {
                    let cfg: Attribute = parse_quote!(#cfg);
                    let not_cfg = util::cfg_attr_not(Some(cfg.clone()));

                    let cfg_ident = ident!("__cfg_{fn_ident}");

                    when_defaults.extend(quote! {
                        #cfg
                        #[doc(hidden)]
                        pub use #crate_core::core_cfg_ok as #cfg_ident;
                        #not_cfg
                        #[doc(hidden)]
                        pub use #crate_core::core_cfg_ignore as #cfg_ident;
                    })
                }
            } else {
                // assign.path.get_ident() == None
                let suggestion = &assign.path.segments.last().unwrap().ident;
                errors.push(
                    format_args!("widget properties only have a single name, try `{suggestion}`"),
                    assign.path.span(),
                );
            }
        }

        // properties used in the when condition.
        let mut visited_props = HashSet::new();
        let inputs: Vec<_> = cond_properties
            .keys()
            .map(|(p, _)| p)
            .filter(|&p| visited_props.insert(p.clone()))
            .collect();

        // name of property inputs Args reference in the condition function.
        let input_idents: Vec<_> = inputs.iter().map(|p| ident!("__{p}")).collect();
        // name of property inputs in the widget module.
        let prop_idents: Vec<_> = inputs.iter().map(|p| ident_spanned!(p.span()=> "__p_{p}")).collect();

        // name of the fields for each interpolated property.
        let field_idents = cond_properties.values();
        let input_ident_per_field = cond_properties.keys().map(|(p, _)| ident!("__{p}"));
        let members = cond_properties.keys().map(|(_, m)| m);

        let expr = condition.expr;

        let ra_lints = if util::is_rust_analyzer() {
            // rust analyzer does not this attribute in the var declaration.
            quote! {
                #[allow(non_snake_case)]
            }
        } else {
            TokenStream::new()
        };

        when_conditions.extend(quote! {
            #cfg
            #(#when_lints)*
            #ra_lints
            #[doc(hidden)]
            pub fn #ident(
                #(#input_idents : &impl self::#prop_idents::Args),*
            ) -> impl #crate_core::var::Var<bool> {
                #(
                    #[allow(non_snake_case)]
                    #[allow(clippy::needless_late_init)]
                    let #field_idents;
                    #field_idents = #crate_core::var::IntoVar::into_var(
                    std::clone::Clone::clone(#input_ident_per_field.#members()));
                )*
                #crate_core::var::expr_var! {
                    #expr
                }
            }
        });

        when_conditions.extend(quote! { #crate_core::core_cfg_inspector! {
            #cfg
            #[doc(hidden)]
            pub fn #dbg_ident(
                #(#input_idents : &(impl self::#prop_idents::Args + 'static),)*
                when_infos: &mut std::vec::Vec<#crate_core::inspector::WhenInfoV1>
            ) -> impl #crate_core::var::Var<bool> + 'static {
                let var = self::#ident(#(#input_idents),*);
                when_infos.push(#crate_core::inspector::WhenInfoV1 {
                    condition_expr: #expr_str,
                    condition_var: Some(#crate_core::var::Var::boxed(std::clone::Clone::clone(&var))),
                    properties: std::vec![
                        #(#assign_names),*
                    ],
                    decl_location: #crate_core::inspector::source_location!(),
                    user_declared: false,
                });
                var
            }
        }});

        if let Some(cfg) = &cfg {
            let not_cfg = util::cfg_attr_not(Some(cfg.clone()));
            let cfg_ident = ident!("__cfg_{ident}");

            when_conditions.extend(quote! {
                #cfg
                #[doc(hidden)]
                pub use #crate_core::core_cfg_ok as #cfg_ident;
                #not_cfg
                #[doc(hidden)]
                pub use #crate_core::core_cfg_ignore as #cfg_ident;
            })
        }

        let cfg = cfg.is_some();

        let inputs = inputs.iter();
        built_whens.extend(quote! {
            #ident {
                dbg_ident { #dbg_ident }
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
    let mut inherit_paths = inherits.iter().map(|(_, p)| p);

    if mixin || is_base {
        // mixins (and the base parent) don't inherit anything implicit so we go directly to
        // stage_2_declare or to the first inherit.
        if inherits.is_empty() {
            stage_path = quote!(#crate_core::widget_declare!);
            stage_extra = TokenStream::default();
        } else {
            let cfg = cfgs.next().unwrap();
            let not_cfg = util::cfg_attr_not(cfg.clone());
            let next_path = inherit_paths.next();
            stage_path = quote!(#next_path!);
            stage_extra = quote! {
                inherit=>
                cfg { #cfg }
                not_cfg { #not_cfg }
                inherit_use { #next_path }
                inherit {
                    #(
                        #cfgs
                        #inherit_paths
                    )*
                }
            }
        }
    } else {
        // not-mixins may inherit from the implicit base parent if they are
        // not inheriting from any other widget, so we always include the base
        // data first, the final stage decides if it is needed.
        stage_path = quote!(#crate_core::widget_base::implicit_base!);
        stage_extra = quote! {
            inherit=>
            cfg { }
            not_cfg { #[cfg(zero_ui_never_set)] }
            inherit_use { #crate_core::widget_base::implicit_base }
            inherit {
                #(
                    #cfgs
                    #inherit_paths
                )*
            }
        };
    }

    let errors_mod = ident!("__{ident}_stage0_errors_{}", uuid);

    let final_macro_ident = ident!("__{ident}_{}_final", uuid);

    // rust-analyzer does not find the macro if we don't set the call_site here.
    let mod_path = util::set_stream_span(mod_path, Span::call_site());

    let rust_analyzer_extras = if util::is_rust_analyzer() {
        let widget_macro = ident!("__widget_macro_{}", uuid);
        let new_caps = new_captures.last().into_iter().flatten();

        quote! {
            #[doc(hidden)]
            pub use #crate_core::rust_analyzer_widget_new;

            #[doc(hidden)]
            #[macro_export]
            macro_rules! #widget_macro {
                (inherit=> $($tt:tt)*) => {

                };
                ($($tt:tt)*) => {
                    #mod_path::rust_analyzer_widget_new! {
                        new {
                            #mod_path::new(child, #(#new_caps),*)
                        }
                        user {
                            $($tt)*
                        }
                    }
                };
            }
            #[doc(hidden)]
            pub use #widget_macro as __widget_macro;
        }
    } else {
        TokenStream::new()
    };

    let new_fns = new_fns.iter().map(|(_, v)| v);

    let new_idents: Vec<_> = FnPriority::all().iter().map(|p| ident!("{p}")).collect();

    let new_captures_has_cfg = new_captures_cfg.iter().map(|ts| ts.is_empty());

    let r = quote! {
        #wgt_cfg
        mod #errors_mod {
            #errors
        }

        #mod_path_assert

        #wgt_cfg
        #wgt_attrs
        #vis mod #ident {
            // custom items
            #(#others)*

            // use items (after custom items in case of custom macro_rules re-export)
            #(#uses)*

            #debug_info

            #(#new_fns)*

            #property_declarations
            #property_defaults

            #when_conditions
            #when_defaults

            #[doc(hidden)]
            pub mod __core {
                pub use #crate_core::{UiNode, BoxedUiNode, widget_inherit, widget_new, var, core_cfg_inspector, core_cfg_ok};

                #crate_core::core_cfg_inspector! {
                    #[doc(hidden)]
                    pub use #crate_core::inspector::{source_location, WhenInfoV1, WidgetNewFnV1};
                }
            }

            // inherit=> will include an `inherited { .. }` block with the widget data after the
            // `inherit { .. }` block and take the next `inherit` path turn that into an `inherit=>` call.
            // This way we "eager" expand the inherited data recursively, when there no more path to inherit
            // a call to `widget_declare!` is made.
            #stage_path {
                #stage_extra

                widget {
                    call_site
                    module { #mod_path }
                    ident { #ident }
                    mixin { #mixin }
                    is_base { #is_base }

                    properties_remove {
                        #property_removes
                    }
                    properties_declared {
                        #property_declared_idents
                    }

                    properties {
                        #built_properties
                    }
                    whens {
                        #built_whens
                    }

                    new_declarations {
                        #(
                            #new_idents { #new_declarations }
                        )*
                    }
                    new_captures {
                        #(
                            #new_idents {
                                #(#new_captures {
                                    cfg { #new_captures_has_cfg }
                                })*
                            }
                        )*
                    }
                }
            }

            #rust_analyzer_extras
        }

        #wgt_cfg
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #final_macro_ident {
            (inherit=> $($tt:tt)*) => { #mod_path::__widget_macro! { inherit=> $($tt)* } };
            ($($tt:tt)*) => { #mod_path::__widget_macro! { call_site { . } $($tt)* } };
        }

        #wgt_cfg
        #[doc(hidden)]
        pub use #final_macro_ident as #ident;
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

fn new_fn_captures<'a, 'b>(
    fn_inputs: impl Iterator<Item = &'a FnArg>,
    errors: &'b mut Errors,
) -> (Vec<Ident>, Vec<TokenStream>, Vec<Span>) {
    let mut ids = vec![];
    let mut cfgs = vec![];
    let mut spans = vec![];
    for input in fn_inputs {
        match input {
            syn::FnArg::Typed(pt) => {
                // any pat : ty
                match &*pt.pat {
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
                            ids.push(ident_pat.ident.clone());
                            spans.push(pt.ty.span());
                            cfgs.push(Attributes::new(pt.attrs.clone()).cfg.to_token_stream());
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

    debug_assert_eq!(ids.len(), cfgs.len());
    debug_assert_eq!(ids.len(), spans.len());
    (ids, cfgs, spans)
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

#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) enum FnPriority {
    NewChild,

    NewChildLayout,
    NewChildContext,

    NewFill,
    NewBorder,
    NewSize,
    NewLayout,
    NewEvent,
    NewContext,

    New,
}
impl FnPriority {
    pub fn from_ident(ident: &Ident) -> Option<Self> {
        match ident.to_string().as_str() {
            "new_child" => Some(Self::NewChild),
            "new_child_layout" => Some(Self::NewChildLayout),
            "new_child_context" => Some(Self::NewChildContext),
            "new_fill" => Some(Self::NewFill),
            "new_border" => Some(Self::NewBorder),
            "new_size" => Some(Self::NewSize),
            "new_layout" => Some(Self::NewLayout),
            "new_event" => Some(Self::NewEvent),
            "new_context" => Some(Self::NewContext),
            "new" => Some(Self::New),
            _ => None,
        }
    }

    pub fn all() -> &'static [FnPriority] {
        &[
            Self::NewChild,
            Self::NewChildLayout,
            Self::NewChildContext,
            Self::NewFill,
            Self::NewBorder,
            Self::NewSize,
            Self::NewLayout,
            Self::NewEvent,
            Self::NewContext,
            Self::New,
        ]
    }
}
impl fmt::Display for FnPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FnPriority::NewChild => write!(f, "new_child"),
            FnPriority::NewChildLayout => write!(f, "new_child_layout"),
            FnPriority::NewChildContext => write!(f, "new_child_context"),
            FnPriority::NewFill => write!(f, "new_fill"),
            FnPriority::NewBorder => write!(f, "new_border"),
            FnPriority::NewSize => write!(f, "new_size"),
            FnPriority::NewLayout => write!(f, "new_layout"),
            FnPriority::NewEvent => write!(f, "new_event"),
            FnPriority::NewContext => write!(f, "new_context"),
            FnPriority::New => write!(f, "new"),
        }
    }
}

struct WidgetItems {
    uses: Vec<ItemUse>,
    inherits: Vec<Inherit>,
    properties: Vec<Properties>,
    new_fns: Vec<(FnPriority, ItemFn)>,
    others: Vec<Item>,
}
impl WidgetItems {
    fn new(items: Vec<Item>, errors: &mut Errors) -> Self {
        let mut uses = vec![];
        let mut inherits = vec![];
        let mut properties = vec![];
        let mut new_fns = vec![];
        let mut others = vec![];

        for item in items {
            enum KnownMacro {
                Properties,
                Inherit,
            }
            let mut known_macro = None;
            let known_fn;
            match item {
                Item::Use(use_) => {
                    uses.push(use_);
                }
                // match properties! or inherit!.
                Item::Macro(ItemMacro {
                    attrs, mac, ident: None, ..
                }) if {
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
                            Ok(mut ps) => {
                                ps.attrs.extend(attrs);
                                inherits.push(ps)
                            }
                            Err(e) => errors.push_syn(e),
                        },
                        None => unreachable!(),
                    }
                }
                // match fn new(..), new_inner(..) .. fn new_child(..).
                Item::Fn(fn_)
                    if {
                        known_fn = FnPriority::from_ident(&fn_.sig.ident);
                        known_fn.is_some()
                    } =>
                {
                    let key = known_fn.unwrap();
                    if !new_fns.iter().any(|(k, _)| k == &key) {
                        new_fns.push((key, fn_));
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
            new_fns,
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
            attrs: vec![],
            path: input.parse()?,
        })
    }
}

struct Properties {
    errors: Errors,
    properties: Vec<ItemProperty>,
    removes: Vec<Ident>,
    whens: Vec<When>,
}
impl Parse for Properties {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut errors = Errors::default();
        let mut properties = vec![];
        let mut removes = vec![];
        let mut whens = vec![];

        while !input.is_empty() {
            let attrs = parse_outer_attrs(input, &mut errors);

            if input.peek(keyword::when) {
                if let Some(mut when) = When::parse(input, &mut errors) {
                    when.attrs = attrs;
                    whens.push(when);
                }
            } else if input.peek(keyword::remove) && input.peek2(syn::token::Brace) {
                let input = non_user_braced!(input, "remove");
                while !input.is_empty() {
                    if input.peek2(Token![::])
                        && (input.peek(Ident) || input.peek(Token![crate]) || input.peek(Token![super]) || input.peek(Token![self]))
                    {
                        if let Ok(p) = input.parse::<Path>() {
                            errors.push("expected inherited property ident, found path", p.span());
                            let _ = input.parse::<Token![;]>();
                        }
                    }
                    match input.parse::<Ident>() {
                        Ok(ident) => {
                            if input.is_empty() {
                                // found valid last item
                                removes.push(ident);
                                break;
                            } else {
                                match input.parse::<Token![;]>() {
                                    Ok(_) => {
                                        // found valid item
                                        removes.push(ident);
                                        continue;
                                    }
                                    Err(e) => errors.push_syn(e),
                                }
                            }
                        }
                        Err(e) => errors.push("expected inherited property ident", e.span()),
                    }

                    // seek next valid item
                    while !(input.is_empty() || input.peek(Ident) && input.peek2(Token![;])) {
                        input.parse::<TokenTree>().unwrap();
                    }
                }
            } else if input.peek(Ident) || input.peek(Token![crate]) || input.peek(Token![super]) || input.peek(Token![self]) {
                // peek ident or path (including keywords because of super:: and self::).
                match input.parse::<ItemProperty>() {
                    Ok(mut p) => {
                        p.attrs = attrs;
                        if !input.is_empty() && p.semi.is_none() {
                            errors.push("expected `;`", input.span());
                            while !(input.is_empty()
                                || input.peek(Ident)
                                || input.peek(Token![crate])
                                || input.peek(Token![super])
                                || input.peek(Token![self])
                                || input.peek(Token![#]) && input.peek(token::Bracket))
                            {
                                // skip to next value item.
                                let _ = input.parse::<TokenTree>();
                            }
                        }
                        properties.push(p);
                    }
                    Err(e) => {
                        let (recoverable, e) = e.recoverable();
                        if recoverable {
                            errors.push_syn(e);
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else {
                errors.push("expected `when`, `child`, `remove` or a property declaration", input.span());

                // suppress the "unexpected token" error from syn parse.
                let _ = input.parse::<TokenStream>();

                break;
            }
        }

        Ok(Properties {
            errors,
            properties,
            removes,
            whens,
        })
    }
}

struct ItemProperty {
    pub attrs: Vec<Attribute>,
    pub path: Path,
    pub alias: Option<(Token![as], Ident)>,
    pub type_: Option<(token::Paren, PropertyType)>,
    pub value: Option<(Token![=], PropertyValue)>,
    pub value_span: Span,
    pub semi: Option<Token![;]>,
}
impl Parse for ItemProperty {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<Path>()?;

        // as ident
        let alias = if input.peek(Token![as]) {
            let as_ = input.parse::<Token![as]>().unwrap();
            let name = input.parse::<Ident>()?;
            Some((as_, name))
        } else {
            None
        };

        // (Type)
        let mut type_error = None;
        let mut type_ = None;
        if input.peek(token::Paren) {
            let inner;
            let paren = parenthesized!(inner in input);
            match PropertyType::parse_validate(&inner, paren.span) {
                Ok(t) => {
                    type_ = Some((paren, t));
                }
                Err(mut e) => {
                    if e.to_string() == "expected one of: `for`, parentheses, `fn`, `unsafe`, `extern`, identifier, `::`, `<`, square brackets, `*`, `&`, `!`, `impl`, `_`, lifetime" {
                        e = syn::Error::new(e.span(), "expected a property type");
                    }
                    // we don't return the error right now
                    // so that we can try to parse to the end of the property
                    // to make the error recoverable.
                    type_error = Some(e);
                }
            }
        }

        // = PropertyValue
        let mut value_error = None;
        let mut value = None;
        let mut value_span = Span::call_site();

        if input.peek(Token![=]) {
            let eq = input.parse::<Token![=]>().unwrap();
            value_span = input.span();
            match input.parse::<PropertyValue>() {
                Ok(v) => value = Some((eq, v)),
                Err(e) => value_error = Some(e),
            }
        }

        // ;
        let semi = if input.peek(Token![;]) {
            Some(input.parse().unwrap())
        } else {
            None
        };

        match (type_error, value_error) {
            (Some(mut t), Some(v)) => {
                t.extend(v);
                return Err(t);
            }
            (Some(t), None) | (None, Some(t)) => return Err(t),
            _ => {}
        }

        Ok(Self {
            attrs: vec![],
            path,
            alias,
            type_,
            value,
            value_span,
            semi,
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
    Named(Punctuated<NamedField, Token![,]>),
    /// `impl IntoVar<bool>, impl IntoVar<u32>`
    Unnamed(Punctuated<syn::Type, Token![,]>),
}
impl PropertyType {
    fn fn_input_tokens(&self, property: &Ident) -> TokenStream {
        match self {
            PropertyType::Named(fields) => fields.to_token_stream(),
            PropertyType::Unnamed(unnamed) => {
                if unnamed.len() == 1 {
                    quote! { #property: #unnamed }
                } else {
                    let names = (0..unnamed.len()).map(|i| ident!("arg{i}"));
                    let unnamed = unnamed.iter();
                    quote! { #(#names: #unnamed),* }
                }
            }
        }
    }
    pub fn parse_validate(input: ParseStream, group_span: Span) -> syn::Result<Self> {
        if input.is_empty() {
            Err(util::recoverable_err(group_span, "expected property type"))
        } else {
            Self::parse(input).map_err(move |e| {
                if e.to_string() == "expected one of: `for`, parentheses, `fn`, `unsafe`, `extern`, identifier, `::`, `<`, square brackets, `*`, `&`, `!`, `impl`, `_`, lifetime" {
                    util::recoverable_err(e.span(), "expected property type")
                } else {
                    e.set_recoverable()
                }
            })
        }
    }
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) && input.peek2(Token![:]) && !input.peek3(Token![:]) {
            let t: Punctuated<NamedField, Token![,]> = Punctuated::parse_terminated(input)?;
            for t in t.iter() {
                if let syn::Type::Infer(inf) = &t.ty {
                    return Err(syn::Error::new(inf.span(), "type placeholder `_` is not allowed in property types"));
                }
            }
            Ok(PropertyType::Named(t))
        } else {
            let t = Punctuated::parse_terminated(input)?;
            for t in t.iter() {
                if let syn::Type::Infer(inf) = &t {
                    return Err(syn::Error::new(inf.span(), "type placeholder `_` is not allowed in property types"));
                }
            }
            Ok(PropertyType::Unnamed(t))
        }
    }
}
#[derive(Debug)]
enum PropertyTypeTerm {
    Eq(Token![=]),
    Semi(Token![;]),
}
impl Parse for PropertyTypeTerm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![=]) {
            Ok(PropertyTypeTerm::Eq(input.parse().unwrap()))
        } else if input.peek(Token![;]) {
            Ok(PropertyTypeTerm::Semi(input.parse().unwrap()))
        } else {
            Err(syn::Error::new(input.span(), "expected `=` or `;`"))
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
    pub use crate::widget_new::keyword::when;
    syn::custom_keyword!(remove);
}

struct AllowedInWhenInput {
    flag: syn::LitBool,
}
impl Parse for AllowedInWhenInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let eq: Token![=] = input.parse()?;
        Ok(AllowedInWhenInput {
            flag: input.parse().map_err(|e| {
                if util::span_is_call_site(e.span()) {
                    syn::Error::new(eq.span(), e)
                } else {
                    e
                }
            })?,
        })
    }
}
