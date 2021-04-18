use std::{
    collections::{HashMap, HashSet},
    mem,
};

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    ext::IdentExt,
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

    util::docs_with_first_line_js(&mut wgt_attrs, &docs, js_tag!("widget_header.js"));
    wgt_attrs.extend(quote! {
        ///
        ///
    });
    doc_extend!(wgt_attrs, js_tag!("widget_full.js"));
    wgt_attrs.extend(quote! {
        /// <iframe id='inner-docs-frame' src='__inner_docs/index.html' style='position: absolute;width:0;height:0;border:0;'></iframe>
    });
    let wgt_attrs = wgt_attrs;

    // a `$crate` path to the widget module.
    let mod_path;
    let mod_path_assert;
    match syn::parse::<ArgPath>(args) {
        Ok(a) => {
            let assert_mod_path = ident!("__{}_assert_mod_path_{}", ident, util::uuid());
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
        mut new_child_fn,
        mut new_fn,
        mut others,
    } = WidgetItems::new(items, &mut errors);

    let whens: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.whens)).collect();
    let removes: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.removes)).collect();
    let mut child_properties: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.child_properties)).collect();
    let mut properties: Vec<_> = properties.iter_mut().flat_map(|p| mem::take(&mut p.properties)).collect();

    if mixin {
        if let Some(child_fn_) = new_child_fn.take() {
            errors.push("widget mixins do not have a `new_child` function", child_fn_.sig.ident.span());
            others.push(syn::Item::Fn(child_fn_))
        }

        if let Some(fn_) = new_fn.take() {
            errors.push("widget mixins do not have a `new` function", fn_.sig.ident.span());
            others.push(syn::Item::Fn(fn_))
        }
    }

    // Does some validation of `new_child` and `new` signatures.
    // Further type validation is done by `rustc` when we call the function
    // in the generated `__new_child` and `__new` functions.
    if let Some(fn_) = &new_child_fn {
        validate_new_fn(fn_, &mut errors);
    }
    if let Some(fn_) = &mut new_fn {
        validate_new_fn(fn_, &mut errors);
        if fn_.sig.inputs.is_empty() {
            errors.push(
                "`new` must take at least one input that implements `UiNode`",
                fn_.sig.paren_token.span,
            );
            fn_.sig.inputs.push(parse_quote! { __child: impl #crate_core::UiNode });
        }
    }

    // collects name of captured properties and validates inputs.
    let (new_child, new_child_ty_sp) = new_child_fn
        .as_ref()
        .map(|f| new_fn_captures(f.sig.inputs.iter(), &mut errors))
        .unwrap_or_default();

    let ((new, new_ty_sp), new_arg0_ty_span) = new_fn
        .as_ref()
        .map(|f| {
            // skip the first arg because we expect the arg0 to be `impl UiNode`
            // but we still need the span of the arg0 type for error messages.
            let mut args = f.sig.inputs.iter();
            let new_arg0_ty_span = args.next().map(|a| if let FnArg::Typed(pt) = a { pt.ty.span() } else { a.span() });
            (new_fn_captures(args, &mut errors), new_arg0_ty_span)
        })
        .unwrap_or_default();
    let mut captures = HashSet::new();
    for capture in new_child.iter().chain(&new) {
        if !captures.insert(capture) {
            errors.push(format_args!("property `{}` already captured", capture), capture.span());
        }
    }
    let captures = captures;

    // generate `__new_child` and `__new` if new functions are defined in the widget.
    //
    // we use generic types to provide an error message for compile errors like the one in
    // `widget/new_fn_mismatched_capture_type1.rs`
    let new_child__ = new_child_fn.as_ref().map(|f| {
        let (p_new_child, generic_tys): (Vec<_>, Vec<_>) = new_child
            .iter()
            .enumerate()
            .map(|(i, id)| (ident!("__p_{}", id), ident!("{}_type_must_match_property_definition_{}", id, i)))
            .unzip();
        let span = match &f.sig.output {
            syn::ReturnType::Default => f.block.span(),
            syn::ReturnType::Type(_, t) => t.span(),
        };
        let spanned_new_child_ty: Vec<_> = new_child
            .iter()
            .zip(new_child_ty_sp)
            .enumerate()
            .map(|(i, (id, ty_span))| ident_spanned!(ty_span=> "__{}_{}", i, id))
            .collect();
        let spanned_unwrap = p_new_child.iter().zip(spanned_new_child_ty.iter()).map(|(p, id)| {
            quote_spanned! {id.span()=>
                self::#p::Args::unwrap(#id)
            }
        });

        let output = quote_spanned! {span=>
            impl #crate_core::UiNode
        };
        #[allow(unused_mut)]
        let mut r = quote! {
            #[doc(hidden)]
            #[allow(clippy::too_many_arguments)]
            pub fn __new_child<#(#generic_tys: #p_new_child::Args),*>(#(#spanned_new_child_ty : #generic_tys),*) -> #output {
                self::new_child(#(#spanned_unwrap),*)
            }
        };

        #[cfg(debug_assertions)]
        {
            let names = new_child.iter().map(|id| id.to_string());
            let locations = new_child.iter().map(|id| {
                quote_spanned! {id.span()=>
                    #crate_core::debug::source_location!()
                }
            });
            let assigned_flags: Vec<_> = new_child
                .iter()
                .enumerate()
                .map(|(i, id)| ident!("__{}_{}_user_set", i, id))
                .collect();
            r.extend(quote! {
                #[doc(hidden)]
                #[allow(clippy::too_many_arguments)]
                pub fn __new_child_debug(
                    #(#new_child : impl self::#p_new_child::Args,)*
                    #(#assigned_flags: bool,)*
                     __captures_info: &mut std::vec::Vec<#crate_core::debug::CapturedPropertyV1>
                ) -> impl #crate_core::UiNode {
                    #(__captures_info.push(#p_new_child::captured_debug(&#new_child, #names, #locations, #assigned_flags));)*
                    self::__new_child(#(#new_child),*)
                }
            });
        }

        r
    });
    let new__ = new_fn.as_ref().map(|f| {
        let (p_new, generic_tys): (Vec<_>, Vec<_>) = new
            .iter()
            .enumerate()
            .map(|(i, id)| (ident!("__p_{}", id), ident!("{}_type_must_match_property_definition_{}", id, i)))
            .unzip();
        let new_arg0_ty_span = new_arg0_ty_span.unwrap();
        let child_ident = ident_spanned!(new_arg0_ty_span=> "__child");
        let child_ty = quote_spanned! {new_arg0_ty_span=>
            impl #crate_core::UiNode
        };
        let spanned_new_ty: Vec<_> = new
            .iter()
            .zip(new_ty_sp)
            .enumerate()
            .map(|(i, (id, ty_span))| ident_spanned!(ty_span=> "__{}_{}", i, id))
            .collect();
        let spanned_unwrap = p_new.iter().zip(spanned_new_ty.iter()).map(|(p, id)| {
            quote_spanned! {id.span()=>
                self::#p::Args::unwrap(#id)
            }
        });
        let output = &f.sig.output;
        #[allow(unused_mut)]
        let mut r = quote! {
            #[doc(hidden)]
            #[allow(clippy::too_many_arguments)]
            pub fn __new<#(#generic_tys: self::#p_new::Args),*>(#child_ident: #child_ty, #(#spanned_new_ty: #generic_tys),*) #output {
                self::new(#child_ident, #(#spanned_unwrap),*)
            }
        };

        #[cfg(debug_assertions)]
        {
            let names = new.iter().map(|id| id.to_string());
            let locations = new.iter().map(|id| {
                quote_spanned! {id.span()=>
                    #crate_core::debug::source_location!()
                }
            });
            let assigned_flags: Vec<_> = new.iter().enumerate().map(|(i, id)| ident!("__{}_{}_user_set", i, id)).collect();
            let decl_location = quote_spanned!(ident.span()=> #crate_core::debug::source_location!());
            let wgt_name = ident.to_string();
            r.extend(quote! {
                #[doc(hidden)]
                #[allow(clippy::too_many_arguments)]
                pub fn __new_debug(
                    __child: impl #crate_core::UiNode,
                    #(#new : impl self::#p_new::Args,)*
                    #(#assigned_flags: bool,)*
                     __new_child_captures: std::vec::Vec<#crate_core::debug::CapturedPropertyV1>,
                     __whens: std::vec::Vec<#crate_core::debug::WhenInfoV1>,
                     __instance_location: #crate_core::debug::SourceLocation,
                ) #output {
                    let __child = #crate_core::UiNode::boxed(__child);
                    let __new_captures = std::vec![
                        #(self::#p_new::captured_debug(&#new, #names, #locations, #assigned_flags),)*
                    ];
                    let __child = #crate_core::debug::WidgetInstanceInfoNode::new_v1(
                        __child,
                        #wgt_name,
                        #decl_location,
                        __instance_location,
                        __new_child_captures,
                        __new_captures,
                        __whens,
                    );
                    self::__new(__child, #(#new),*)
                }
            });
        }

        r
    });
    // captured property existence validation happens "widget_2_declare.rs"

    // process properties
    let mut declared_properties = HashSet::new();
    let mut built_properties_child = TokenStream::default();
    let mut built_properties = TokenStream::default();
    let mut property_defaults = TokenStream::default();
    let mut property_declarations = TokenStream::default();
    let mut property_declared_idents = TokenStream::default();
    let mut property_removes = TokenStream::default();

    // process removes
    let mut visited_removes = HashSet::new();
    for ident in &removes {
        if !visited_removes.insert(ident) {
            errors.push(format_args!("property `{}` already removed", ident), ident.span());
            continue;
        }

        ident.to_tokens(&mut property_removes);
    }
    drop(visited_removes);
    drop(removes);

    // process declarations
    for (property, is_child_property) in child_properties
        .iter_mut()
        .map(|p| (p, true))
        .chain(properties.iter_mut().map(|p| (p, false)))
    {
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
            errors.push(format_args!("property `{}` is already declared", p_ident), p_ident.span());
            continue;
        }

        // declare new capture properties.
        if let Some((_, new_type)) = &property.type_ {
            if !mixin && !captures.contains(p_ident) {
                // new capture properties must be captured by new *new* functions.
                errors.push(
                    format_args!("property `{}` is declared in widget, but is not captured by the widget", p_ident),
                    p_ident.span(),
                );
            }

            let p_mod_ident = ident!("__p_{}", p_ident);
            let inputs = new_type.fn_input_tokens(p_ident);

            let docs = &attrs.docs;

            property_declarations.extend(quote! {
                #(#docs)*
                #[#crate_core::property(capture_only, allowed_in_when = #allowed_in_when)]
                pub fn #p_mod_ident(#inputs) -> ! { }
            });

            // so "widget_2_declare.rs" skips reexporting this one.
            p_ident.to_tokens(&mut property_declared_idents);
        }

        let mut default = false;

        // process default value or special value.
        if let Some((_, default_value)) = &property.value {
            if let PropertyValue::Special(sp, _) = default_value {
                errors.push(format_args!("unexpected `{}!` as default value", sp), sp.span());
                continue;
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
        let ident = when.make_ident("w", i, Span::call_site());

        #[cfg(debug_assertions)]
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
                        format_args!("widget properties only have a single name, try `self.{}`", suggestion),
                        p_path.span(),
                    );
                    None
                }
            })
            .collect();

        if skip {
            continue;
        }

        #[cfg(debug_assertions)]
        let mut assign_names = vec![];
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
                        format_args!("property `{}` already set in this `when` block", property),
                        property.span(),
                    );
                    skip = true;
                }
                // validate value is not one of the special commands.
                // TODO: change the error message, or revise this after `Special` becomes `Unset`.
                if let PropertyValue::Special(sp, _) = &assign.value {
                    // unknown special.
                    errors.push(format_args!("unexpected `{}!` in property value", sp), sp.span());

                    skip = true;
                }

                if skip {
                    continue;
                }

                #[cfg(debug_assertions)]
                assign_names.push(property.to_string());

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
                // assign.path.get_ident() == None
                let suggestion = &assign.path.segments.last().unwrap().ident;
                errors.push(
                    format_args!("widget properties only have a single name, try `{}`", suggestion),
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
        let input_idents: Vec<_> = inputs.iter().map(|p| ident!("__{}", p)).collect();
        // name of property inputs in the widget module.
        let prop_idents: Vec<_> = inputs.iter().map(|p| ident_spanned!(p.span()=> "__p_{}", p)).collect();

        // name of the fields for each interpolated property.
        let field_idents = cond_properties.values();
        let input_ident_per_field = cond_properties.keys().map(|(p, _)| ident!("__{}", p));
        let members = cond_properties.keys().map(|(_, m)| m);

        let expr = condition.expr;

        when_conditions.extend(quote! {
            #cfg
            #(#when_lints)*
            #[doc(hidden)]
            pub fn #ident(
                #(#input_idents : &impl self::#prop_idents::Args),*
            ) -> impl #crate_core::var::Var<bool> {
                #(
                    #[allow(non_snake_case)]
                    let #field_idents;
                    #field_idents = #crate_core::var::IntoVar::into_var(
                    std::clone::Clone::clone(#input_ident_per_field.#members()));
                )*
                #crate_core::var::expr_var! {
                    #expr
                }
            }
        });

        #[cfg(debug_assertions)]
        when_conditions.extend(quote! {
            #cfg
            #[doc(hidden)]
            pub fn #dbg_ident(
                #(#input_idents : &(impl self::#prop_idents::Args + 'static),)*
                when_infos: &mut std::vec::Vec<#crate_core::debug::WhenInfoV1>
            ) -> impl #crate_core::var::Var<bool> + 'static {
                let var = self::#ident(#(#input_idents),*);
                when_infos.push(#crate_core::debug::WhenInfoV1 {
                    condition_expr: #expr_str,
                    condition_var: Some(#crate_core::var::VarObj::boxed(std::clone::Clone::clone(&var))),
                    properties: std::vec![
                        #(#assign_names),*
                    ],
                    decl_location: #crate_core::debug::source_location!(),
                    user_declared: false,
                });
                var
            }
        });

        #[cfg(debug_assertions)]
        let dbg_ident = quote! {
            dbg_ident { #dbg_ident }
        };
        #[cfg(not(debug_assertions))]
        let dbg_ident = TokenStream::default();

        let inputs = inputs.iter();
        built_whens.extend(quote! {
            #ident {
                #dbg_ident
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

    if mixin {
        // mixins don't inherit the implicit_mixin so we go directly to stage_2_declare or to the first inherit.
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
        // not-mixins inherit from the implicit_mixin first so we call inherit=> for that:
        stage_path = quote!(#crate_core::widget_base::implicit_mixin!);
        stage_extra = quote! {
            inherit=>
            cfg { }
            not_cfg { #[cfg(zero_ui_never_set)] }
            inherit_use { #crate_core::widget_base::implicit_mixin }
            inherit {
                #(
                    #cfgs
                    #inherit_paths
                )*
            }
        };
    }

    #[cfg(debug_assertions)]
    let debug_reexport = quote! {debug::{source_location, WhenInfoV1}};
    #[cfg(not(debug_assertions))]
    let debug_reexport = TokenStream::default();

    let errors_mod = ident!("__{}_stage0_errors_{}", ident, util::uuid());

    let final_macro_ident = ident!("__{}_{}_final", ident, util::uuid());

    let r = quote! {
        #wgt_cfg
        mod #errors_mod {
            #errors
        }

        #mod_path_assert

        #wgt_cfg
        #wgt_attrs
        #vis mod #ident {
            // inherit=> will include an `inherited { .. }` block with the widget data after the
            // `inherit { .. }` block and take the next `inherit` path turn that into an `inherit=>` call.
            // This way we "eager" expand the inherited data recursively, when there no more path to inherit
            // a call to `widget_declare!` is made.
            #stage_path {
                #stage_extra

                widget {
                    module { #mod_path }
                    ident { #ident }
                    mixin { #mixin }

                    properties_remove {
                        #property_removes
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

                    new_child_declared { #new_child__ }
                    new_child { #(#new_child)* }
                    new_declared { #new__ }
                    new { #(#new)* }
                }
            }

            #(#uses)*

            // custom items
            #(#others)*

            #new_child_fn
            #new_fn

            #property_declarations

            #property_defaults

            #when_conditions
            #when_defaults

            #[doc(hidden)]
            pub mod __core {
                pub use #crate_core::{widget_inherit, widget_new, var, #debug_reexport};
            }
        }

        #wgt_cfg
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #final_macro_ident {
            ($($tt:tt)*) => {
                #mod_path::__widget_macro! { $($tt)* }
            };
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

fn new_fn_captures<'a, 'b>(fn_inputs: impl Iterator<Item = &'a FnArg>, errors: &'b mut Errors) -> (Vec<Ident>, Vec<Span>) {
    let mut ids = vec![];
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

    debug_assert_eq!(ids.len(), spans.len());
    (ids, spans)
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
    removes: Vec<Ident>,
    whens: Vec<When>,
}
impl Parse for Properties {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut errors = Errors::default();
        let mut child_properties = vec![];
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
            } else if input.peek(keyword::child) && input.peek2(syn::token::Brace) {
                let input = non_user_braced!(input, "child");
                while !input.is_empty() {
                    let attrs = parse_outer_attrs(&input, &mut errors);
                    match input.parse::<ItemProperty>() {
                        Ok(mut p) => {
                            p.attrs = attrs;
                            child_properties.push(p);
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
                }
            } else if input.peek(keyword::remove) && input.peek2(syn::token::Brace) {
                let input = non_user_braced!(input, "remove");
                while !input.is_empty() {
                    if input.peek2(Token![::]) && input.peek(Ident::peek_any) {
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
            } else if input.peek(Ident::peek_any) {
                // peek ident or path (including keywords because of super:: and self::).
                match input.parse::<ItemProperty>() {
                    Ok(mut p) => {
                        p.attrs = attrs;
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
                errors.push("expected `when`, `child` or a property declaration", input.span());

                // suppress the "unexpected token" error from syn parse.
                let _ = input.parse::<TokenStream>();

                break;
            }
        }

        Ok(Properties {
            errors,
            child_properties,
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
    pub type_: Option<(Token![:], PropertyType)>,
    pub value: Option<(Token![=], PropertyValue)>,
    pub value_span: Span,
    pub semi: Option<Token![;]>,
}
impl Parse for ItemProperty {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;

        // as ident
        let alias = if input.peek(Token![as]) {
            let as_ = input.parse::<Token![as]>().unwrap();
            let name = input.parse::<Ident>()?;
            Some((as_, name))
        } else {
            None
        };

        // : Type [=|;]
        let mut type_error = None;
        let mut type_ = None;
        let mut type_terminator = None;
        if input.peek(Token![:]) {
            let colon: Token![:] = input.parse().unwrap();
            let type_stream = util::parse_soft_group(
                input,
                // terminates in the first `=` or `;`
                |input| PropertyTypeTerm::parse(input).ok(),
                |input| {
                    // TODO can we peek next property in some cases?
                    // we can't do `path = ` because path can be a type
                    let fork = input.fork();
                    let _ = util::parse_outer_attrs(&fork, &mut Errors::default());
                    fork.peek(keyword::when) || (fork.peek(keyword::child) || fork.peek(keyword::remove)) && fork.peek2(syn::token::Brace)
                },
                // skip generics (tokens within `< >`) because
                // `impl Iterator<Item=u32>` is a valid type.
                true,
            );

            let (r, term) = PropertyType::parse_soft_group(type_stream, colon.span());
            type_terminator = term;
            match r {
                Ok(t) => {
                    type_ = Some((colon, t));
                }
                Err(e) => {
                    // we don't return the error right now
                    // so that we can try to parse to the end of the property
                    // to make the error recoverable.
                    type_error = Some(e);
                }
            }
        }

        // = expr [;]
        let mut value_start_eq = None;
        let mut value = None;
        let mut value_span = Span::call_site();
        let mut semi = None;
        if let Some(term) = type_terminator {
            // if there was a property type, did it terminate
            // at the start of a value `=` or at the end of a property `;`?
            match term {
                PropertyTypeTerm::Eq(eq) => {
                    value_start_eq = Some(eq);
                }
                PropertyTypeTerm::Semi(s) => {
                    semi = Some(s);
                }
            }
        } else if input.peek(Token![=]) {
            // if there was no property type are we at the start of a value `=`?
            value_start_eq = Some(input.parse().unwrap());
        } else if input.peek(Token![;]) {
            semi = Some(input.parse::<Token![;]>().unwrap());
        } else if !input.is_empty() {
            // if we didn't have a type nor value but are also not at the
            // end of the stream a `;` was expected.

            let e = util::recoverable_err(input.span(), "expected `;`");
            return Err(if let Some(mut ty_err) = type_error.take() {
                ty_err.extend(e);
                ty_err
            } else {
                e
            });
        }
        if let Some(eq) = value_start_eq {
            // if we are after a value start `=`

            let value_stream = util::parse_soft_group(
                input,
                // terminates in the first `;` in the current level.
                |input| input.parse::<Option<Token![;]>>().unwrap_or_default(),
                |input| {
                    // checks if the next tokens in the stream look like the start
                    // of another ItemProperty.
                    // TODO can we anticipate `path: T = v;`
                    let fork = input.fork();
                    let _ = util::parse_outer_attrs(&fork, &mut Errors::default());
                    if fork.peek2(Token![=]) {
                        fork.peek(Ident)
                    } else if fork.peek2(Token![::]) {
                        fork.parse::<Path>().is_ok() && fork.peek(Token![=])
                    } else {
                        fork.peek(keyword::when)
                            || (fork.peek(keyword::child) || fork.peek(keyword::remove)) && fork.peek2(syn::token::Brace)
                    }
                },
                false,
            );

            let r = PropertyValue::parse_soft_group(value_stream, eq.span).map_err(|e| {
                if let Some(mut ty_err) = type_error.take() {
                    // we had a type error and now a value error.
                    ty_err.extend(e);
                    ty_err
                } else {
                    // we have just a value error.
                    e
                }
            })?;

            value = Some((eq, r.0));
            value_span = r.1;
            semi = r.2;
        }

        if let Some(e) = type_error {
            // we had a type error and no value or no value error.
            return Err(e);
        }

        let item_property = ItemProperty {
            attrs: vec![],
            path,
            alias,
            type_,
            value,
            value_span,
            semi,
        };

        Ok(item_property)
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
            Ok(PropertyType::Unnamed(Punctuated::parse_terminated(input)?))
        }
    }
}
impl PropertyType {
    fn fn_input_tokens(&self, property: &Ident) -> TokenStream {
        match self {
            PropertyType::Named(_, fields) => fields.to_token_stream(),
            PropertyType::Unnamed(unnamed) => {
                if unnamed.len() == 1 {
                    quote! { #property: #unnamed }
                } else {
                    let names = (0..unnamed.len()).map(|i| ident!("arg{}", i));
                    let unnamed = unnamed.iter();
                    quote! { #(#names: #unnamed),* }
                }
            }
        }
    }
    fn parse_soft_group(
        type_stream: Result<(TokenStream, Option<PropertyTypeTerm>), TokenStream>,
        group_start_span: Span,
    ) -> (syn::Result<Self>, Option<PropertyTypeTerm>) {
        match type_stream {
            Ok((type_stream, term)) => {
                if type_stream.is_empty() {
                    // no type tokens
                    (Err(util::recoverable_err(group_start_span, "expected property type")), term)
                } else {
                    (syn::parse2::<PropertyType>(type_stream).map_err(|e| {
                        if e.to_string() == "expected one of: `for`, parentheses, `fn`, `unsafe`, `extern`, identifier, `::`, `<`, square brackets, `*`, `&`, `!`, `impl`, `_`, lifetime" {
                            util::recoverable_err(e.span(), "expected property type")
                        } else {
                            e.set_recoverable()
                        }
                    }), term)
                }
            }
            Err(partial_ty) => {
                if partial_ty.is_empty() {
                    // no type tokens
                    (Err(util::recoverable_err(group_start_span, "expected property type")), None)
                } else {
                    // maybe missing next argument type (`,`) or terminator (`=`, `;`)
                    let last_tt = partial_ty.into_iter().last().unwrap();
                    let last_span = last_tt.span();
                    let mut msg = "expected `,`, `=` or `;`";
                    if let proc_macro2::TokenTree::Punct(p) = last_tt {
                        if p.as_char() == ',' {
                            msg = "expected another property arg type";
                        }
                    }
                    (Err(util::recoverable_err(last_span, msg)), None)
                }
            }
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
    syn::custom_keyword!(child);
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
