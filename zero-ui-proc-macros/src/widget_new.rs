use std::{collections::{HashMap, HashSet}, mem};

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    braced,
    ext::IdentExt,
    parse::{discouraged::Speculative, Parse, ParseStream},
    parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Expr, FieldValue, Ident, LitBool, Path, Token, AngleBracketedGenericArguments, PathArguments,
};

use crate::{
    util::{
        self, parse_all, parse_outer_attrs, parse_punct_terminated2, peek_any3, tokens_to_ident_str, Attributes, ErrorRecoverable, Errors,
    },
    widget_0_attr::FnPriority,
};

#[allow(unused_macros)]
macro_rules! quote {
    ($($tt:tt)*) => {
        compile_error!("don't use Span::call_site() in widget_new");

        // we don't use [`Span::call_site()`] in this widget because of a bug that highlights
        // more then the call_site span. Not sure what causes it but I think some of
        // the #[widget(..)] span gets used. Taking a direct sample of token inside
        // the the `<widget>!` macro solves the issue, this is done in [`WidgetData::call_site`].
    };
}
#[allow(unused_macros)]
macro_rules! ident {
    ($($tt:tt)*) => {
        compile_error!("don't use Span::call_site() in widget_new");
        // see quote! above
    };
}

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { widget_data, user_input } = match syn::parse::<Input>(input) {
        Ok(i) => i,
        Err(e) => non_user_error!(e),
    };

    let call_site = user_input.call_site;

    macro_rules! quote {
        ($($tt:tt)*) => {
            quote::quote_spanned! {call_site=>
                $($tt)*
            }
        }
    }
    macro_rules! ident {
        ($($tt:tt)*) => {
            ident_spanned! {call_site=>
                $($tt)*
            }
        }
    }

    let module = widget_data.module;

    let mut errors = user_input.errors;

    let inherited_properties: HashMap<_, _> = widget_data.properties.iter().map(|p| (&p.ident, p.cfg)).collect();

    // properties that must be assigned by the user.
    let required_properties: HashSet<_> = widget_data.properties.iter().filter(|p| p.required).map(|p| &p.ident).collect();
    // properties that have a default value.
    let default_properties: HashSet<_> = widget_data.properties.iter().filter(|p| p.default).map(|p| &p.ident).collect();
    // properties that are captured.
    let captured_properties: HashSet<_> = widget_data.new_captures.iter().flat_map(|c| c.iter().map(|c| &c.ident)).collect();

    // inherited properties unset by the user.
    let mut unset_properties = HashSet::new();

    let mut user_properties = HashSet::new();
    // user assigns with valid values.
    let user_properties: Vec<_> = user_input
        .properties
        .iter()
        .filter(|up| {
            // if already (un)set by the user.
            if !user_properties.insert(&up.path) {
                let p_name = util::display_path(&up.path);
                errors.push(format_args!("property `{p_name}` already set"), util::path_span(&up.path));
                return false;
            }

            if let PropertyValue::Special(sp, _) = &up.value {
                if sp == "unset" {
                    if let Some(maybe_inherited) = up.path.get_ident() {
                        if required_properties.contains(maybe_inherited) || captured_properties.contains(maybe_inherited) {
                            errors.push(format_args!("cannot unset required property `{maybe_inherited}`"), sp.span());
                        } else if !default_properties.contains(maybe_inherited) {
                            errors.push(
                                format_args!("cannot unset `{maybe_inherited}` because it is not set by the widget"),
                                sp.span(),
                            );
                        } else {
                            unset_properties.insert(maybe_inherited);
                        }
                    } else {
                        errors.push(
                            format_args!(
                                "cannot unset `{}` because it is not set by the widget",
                                util::display_path(&up.path)
                            ),
                            sp.span(),
                        );
                    }
                } else {
                    errors.push(format_args!("unknown value `{sp}!`"), sp.span());
                }

                false
            } else {
                true
            }
        })
        .collect();

    let unset_properties = unset_properties;

    // inherited properties that are set to a value or unset by the user.
    let overriden_properties: HashSet<_> = user_properties
        .iter()
        .filter_map(|p| p.path.get_ident())
        .filter(|p_id| inherited_properties.contains_key(p_id))
        .chain(unset_properties.iter().copied())
        .collect();

    // all widget properties that will be set (property_path, (Option<property_var|unreachable!>, property_cfg, user_cfg)).
    let mut wgt_properties = HashMap::<syn::Path, (Option<Ident>, Option<Ident>, TokenStream)>::new();

    let mut property_inits = TokenStream::default();
    let mut prop_set_calls = vec![];

    // for each inherited property that has a default value and is not overridden by the user:
    for ip in widget_data
        .properties
        .iter()
        .filter(|ip| ip.default && !overriden_properties.contains(&ip.ident))
    {
        let ident = &ip.ident;
        let p_default_fn_ident = ident!("__d_{ident}");
        let p_var_ident = ident!("__{ident}");
        let p_cfg = if ip.cfg { Some(ident!("__cfg_{ident}")) } else { None };

        wgt_properties.insert(
            parse_quote! { #ident },
            (Some(p_var_ident.clone()), p_cfg.clone(), TokenStream::new()),
        );

        // generate call to default args.
        if let Some(cfg) = &p_cfg {
            property_inits.extend(quote! {
                #module::#cfg! {
                    let #p_var_ident = #module::#p_default_fn_ident();
                }
            });
        } else {
            property_inits.extend(quote! {
                let #p_var_ident = #module::#p_default_fn_ident();
            });
        }

        if captured_properties.contains(ident) {
            continue; // we don't set captured properties.
        }

        let p_mod_ident = ident!("__p_{ident}");
        // register data for the set call generation.
        prop_set_calls.push((
            quote! { #module::#p_mod_ident },
            p_var_ident,
            ip.ident.to_string(),
            {
                let p_source_loc_ident = ident!("__loc_{}", ip.ident);
                quote! { #module::#p_source_loc_ident() }
            },
            p_cfg.clone(),
            /* user_cfg: */ TokenStream::new(),
            /* user_assigned: */ false,
            call_site,
            call_site,
        ));
    }

    // for each property assigned in the widget instantiation call (excluding when blocks and `special!` values).
    for (i, up) in user_properties.iter().enumerate() {
        let p_name = util::display_path(&up.path);

        let (p_mod, p_cfg) = match up.path.get_ident() {
            Some(maybe_inherited) if inherited_properties.contains_key(maybe_inherited) => {
                let p_ident = ident!("__p_{maybe_inherited}");

                let p_cfg = if *inherited_properties.get(maybe_inherited).unwrap() {
                    Some(ident!("__cfg_{maybe_inherited}"))
                } else {
                    None
                };

                (quote! { #module::#p_ident }, p_cfg)
            }
            _ => (up.path.to_token_stream(), None),
        };
        let p_var_ident = ident!("__u{}_{}", i, p_name.replace("::", "_"));
        let attrs = Attributes::new(up.attrs.clone());
        let cfg = attrs.cfg;
        let lints = attrs.lints;

        wgt_properties.insert(up.path.clone(), (Some(p_var_ident.clone()), p_cfg.clone(), cfg.to_token_stream()));

        let init_expr = up
            .value
            .expr_tokens(&p_mod, up.path.span(), up.value_span)
            .unwrap_or_else(|e| non_user_error!(e));

        if let Some(p_cfg) = &p_cfg {
            property_inits.extend(quote! {
                #module::#p_cfg! {
                    #cfg
                    #(#lints)*
                    let #p_var_ident = #init_expr;
                }
            });
        } else {
            property_inits.extend(quote! {
                #cfg
                #(#lints)*
                let #p_var_ident = #init_expr;
            });
        }

        if let Some(maybe_inherited) = up.path.get_ident() {
            if captured_properties.contains(maybe_inherited) {
                continue;
            }
        }
        // register data for the set call generation.
        prop_set_calls.push((
            p_mod.to_token_stream(),
            p_var_ident,
            p_name,
            quote_spanned! {up.path.span()=>
                #module::__core::source_location!()
            },
            p_cfg,
            cfg.to_token_stream(),
            /*user_assigned: */ true,
            up.path.span(),
            up.value_span,
        ));
    }

    // validate required properties.
    let mut missing_required = HashSet::new();
    for required in required_properties.into_iter().chain(captured_properties) {
        if !wgt_properties.contains_key(&parse_quote! { #required }) {
            missing_required.insert(required);
            errors.push(format!("missing required property `{required}`"), call_site);
        }
    }
    let missing_required = missing_required;

    // generate whens.
    let mut when_inits = TokenStream::default();

    when_inits.extend(quote! { #module::__core::core_cfg_inspector! {
        #[allow(unused_mut)]
        let mut when_infos__: std::vec::Vec<#module::__core::WhenInfoV1> = std::vec![];
    }});

    // properties in when condition expressions.
    let mut used_in_when_expr = HashSet::new();

    // map of { property => [(p_cfg, user_cfg, condition_var, when_value_ident, when_value_for_prop)] }
    #[allow(clippy::type_complexity)]
    let mut when_assigns: HashMap<Path, Vec<(Option<Ident>, TokenStream, Ident, Ident, TokenStream)>> = HashMap::new();
    for iw in widget_data.whens {
        if iw.inputs.iter().any(|p| unset_properties.contains(p)) {
            // deactivate when block because user unset one of the inputs.
            continue;
        }

        let assigns: Vec<_> = iw.assigns.into_iter().filter(|a| !unset_properties.contains(&a.property)).collect();
        if assigns.is_empty() {
            // deactivate when block because user unset all of the properties assigned.
            continue;
        }

        let ident = iw.ident;
        let dbg_ident = iw.dbg_ident;
        let cfg = if iw.cfg { Some(ident!("__cfg_{ident}")) } else { None };

        used_in_when_expr.extend(iw.inputs.iter().cloned());

        // arg variables for each input, they should all have a default value or be required (already deactivated if any unset).
        let len = iw.inputs.len();
        let inputs: Vec<_> = iw
            .inputs
            .into_iter()
            .filter_map(|id| {
                let r = wgt_properties.get(&parse_quote! { #id }).map(|(id, _, _)| id);
                if r.is_none() && !missing_required.contains(&id) {
                    non_user_error!("inherited when condition uses property not set, not required and not unset");
                }
                r
            })
            .collect();
        if inputs.len() != len {
            // one of the required properties was not set, an error for this is added elsewhere.
            continue;
        }
        let c_ident = ident!("__c_{ident}");

        let condition_call = quote! {
            {
                #module::__core::core_cfg_inspector! {
                    #module::#dbg_ident(#(&#inputs),* , &mut when_infos__)
                }
                #module::__core::core_cfg_inspector! {@NOT
                    #module::#ident(#(&#inputs),*)
                }
            }
        };

        let when_init = quote! {
            #[allow(non_snake_case)]
            #[allow(clippy::needless_late_init)]
            let #c_ident;
            #c_ident = #condition_call;
        };
        if let Some(cfg) = &cfg {
            when_inits.extend(quote! {
                #module::#cfg! {
                    #when_init
                }
            });
        } else {
            when_inits.extend(when_init);
        }

        // register when for each property assigned.
        for BuiltWhenAssign { property, cfg, value_fn } in assigns {
            let value = quote! { #module::#value_fn() };
            let p_whens = when_assigns.entry(parse_quote! { #property }).or_default();

            let cfg = if cfg { Some(ident!("__cfg_{value_fn}")) } else { None };

            p_whens.push((cfg, TokenStream::new(), c_ident.clone(), value_fn, value));
        }
    }

    // map of [property_without_value => (combined_cfg_for_default_init, unique_id)]
    let mut user_when_properties: HashMap<Path, (Option<TokenStream>, usize)> = HashMap::new();

    for (i, w) in user_input.whens.into_iter().enumerate() {
        // when condition with `self.property(.member)?` converted to `#(__property__member)` for the `expr_var` macro.
        let condition = match w.expand_condition() {
            Ok(c) => c,
            Err(e) => {
                errors.push_syn(e);
                continue;
            }
        };
        let inputs = condition.properties;
        let condition = condition.expr;

        used_in_when_expr.extend(inputs.iter().map(|(_, i)| i.clone()));

        // empty when blocks don't need to generate any code,
        // but we still want to run all validations possible.
        let validate_but_skip = w.assigns.is_empty();

        let ident = w.make_ident("uw", i, call_site);

        // validate/separate attributes
        let attrs = Attributes::new(w.attrs);
        for invalid_attr in attrs.others.into_iter().chain(attrs.inline).chain(attrs.docs) {
            errors.push("only `cfg` and lint attributes are allowed in when blocks", invalid_attr.span());
        }
        let cfg = attrs.cfg;
        let lints = attrs.lints;

        // for each property in inputs and assigns.
        for (property, p_attrs) in inputs
            .keys()
            .map(|(p, _)| (p, &[][..]))
            .chain(w.assigns.iter().map(|a| (&a.path, &a.attrs[..])))
        {
            // if property not set in the widget.
            if !wgt_properties.contains_key(property) {
                match property.get_ident() {
                    // if property was `unset!`.
                    Some(maybe_unset) if unset_properties.contains(maybe_unset) => {
                        errors.push(format!("cannot use unset property `{maybe_unset}` in when"), maybe_unset.span());
                    }
                    // if property maybe has a default value.
                    _ => {
                        let error = format!(
                            "property `{}` is not assigned and has no default value",
                            util::display_path(property)
                        );

                        let property_path = match property.get_ident() {
                            Some(maybe_inherited) if inherited_properties.contains_key(maybe_inherited) => {
                                let p_ident = ident!("__p_{maybe_inherited}");
                                quote! { #module::#p_ident }
                            }
                            _ => property.to_token_stream(),
                        };

                        property_inits.extend(quote_spanned! {util::path_span(property)=>
                            #property_path::code_gen!{
                                if !default=>
                                std::compile_error!{#error}
                            }
                        });

                        if !validate_but_skip {
                            let p_cfg = Attributes::new(p_attrs.to_vec()).cfg;
                            let cfg = util::cfg_attr_or(cfg.clone(), p_cfg);
                            let i = user_when_properties.len();
                            match user_when_properties.entry(property.clone()) {
                                std::collections::hash_map::Entry::Occupied(mut e) => {
                                    let prev = e.get().0.clone().map(|tt| util::parse_attr(tt).unwrap());
                                    e.get_mut().0 = util::cfg_attr_or(prev, cfg.map(|tt| util::parse_attr(tt).unwrap()));
                                }
                                std::collections::hash_map::Entry::Vacant(e) => {
                                    e.insert((cfg, i));
                                }
                            }
                        }
                    }
                }
            }
        }

        // generate let bindings for a clone var of each property.member.
        let mut member_vars = TokenStream::default();
        for ((property, member), var_ident) in inputs {
            let property_path = match property.get_ident() {
                Some(maybe_inherited) if inherited_properties.contains_key(maybe_inherited) => {
                    let p_ident = ident!("__p_{}", maybe_inherited);
                    quote! { #module::#p_ident }
                }
                _ => property.to_token_stream(),
            };

            let not_allowed_error = format!("property `{}` is not allowed in when", util::display_path(&property));
            when_inits.extend(quote_spanned! {util::path_span(&property)=>
                #property_path::code_gen!{ if !allowed_in_when=> std::compile_error!{ #not_allowed_error } }
            });

            if validate_but_skip {
                continue;
            }
            if let Some(maybe_unset) = property.get_ident() {
                if unset_properties.contains(maybe_unset) {
                    // also skip if unset, the error message is already added.
                    continue;
                }
            }

            let args_ident = wgt_properties.get(&property).map(|(id, _, _)| id.clone()).unwrap_or_else(|| {
                // if is not in `wgt_properties` it must be in `user_when_properties`
                // that will generate a __u_ variable before this binding in the final code.
                let (_, i) = user_when_properties.get(&property).unwrap_or_else(|| non_user_error!(""));
                Some(ident!("__ud{i}_{}", util::path_to_ident_str(&property)))
            });

            member_vars.extend(quote! {
                #[allow(non_snake_case)]
                #[allow(clippy::needless_late_init)]
                let #var_ident;
                #property_path::code_gen!{ if !allowed_in_when=>
                    #[allow(unreachable_code)] {
                        #var_ident = std::unreachable!{};
                    }
                }
                #property_path::code_gen!{ if allowed_in_when=>
                    // if you change this you need to change the allowed_in_when
                    // validation in ./property.rs
                    #var_ident =  #module::__core::var::IntoVar::into_var(
                        std::clone::Clone::clone(
                            #property_path::Args::#member(&#args_ident)
                        )
                    );
                }
            });
        }

        if validate_but_skip {
            continue;
        }

        // generate the condition var let binding.
        when_inits.extend(quote! {
            #[allow(non_snake_case)]
            #cfg
            let #ident;
            #cfg {
                #ident = {
                    #member_vars
                    #(#lints)*
                    #module::__core::var::expr_var!(#condition)
                };
            }
        });
        {
            let expr_str = util::format_rust_expr(w.condition_expr.to_string());
            let assign_names = w.assigns.iter().map(|a| util::display_path(&a.path));
            when_inits.extend(quote! { #module::__core::core_cfg_inspector! {
                #cfg
                when_infos__.push(#module::__core::WhenInfoV1 {
                    condition_expr: #expr_str,
                    condition_var: Some(#module::__core::var::Var::boxed(std::clone::Clone::clone(&#ident))),
                    properties: std::vec![
                        #(#assign_names),*
                    ],
                    decl_location: #module::__core::source_location!(),
                    user_declared: false,
                });
            }});
        }

        // init assign variables
        let mut assigns = HashSet::new();
        for (ai, assign) in w.assigns.into_iter().enumerate() {
            let attrs = Attributes::new(assign.attrs);
            for invalid_attr in attrs.others.into_iter().chain(attrs.inline).chain(attrs.docs) {
                errors.push("only `cfg` and lint attributes are allowed in property assign", invalid_attr.span());
            }

            let mut skip = false;

            if !assigns.insert(assign.path.clone()) {
                errors.push(
                    format_args!("property `{}` already set in this `when` block", util::display_path(&assign.path)),
                    assign.path.span(),
                );
                skip = true;
            }

            if let PropertyValue::Special(sp, _) = &assign.value {
                if sp == "unset" {
                    errors.push("cannot `unset!` properties in when blocks", sp.span());
                } else {
                    errors.push(format_args!("unknown value `{sp}!`"), sp.span());
                }
                skip = true;
            }

            if skip {
                continue;
            }
            if let Some(maybe_unset) = assign.path.get_ident() {
                if unset_properties.contains(maybe_unset) {
                    // also skip if unset, the error message is already added.
                    continue;
                }
            }

            let assign_val_id = ident!("__uwv{i}a{ai}_{}", util::display_path(&assign.path).replace("::", "_"));
            let cfg = util::cfg_attr_and(attrs.cfg, cfg.clone());
            let a_lints = attrs.lints;

            let (property_path, property_span, value_span) = match assign.path.get_ident() {
                Some(maybe_inherited) if inherited_properties.contains_key(maybe_inherited) => {
                    let p_ident = ident!("__p_{maybe_inherited}");
                    let span = maybe_inherited.span();
                    (quote_spanned! {span=> #module::#p_ident }, span, span)
                }
                _ => (assign.path.to_token_stream(), assign.path.span(), assign.value_span),
            };

            let not_allowed_error = format!("property `{}` is not allowed in when", util::display_path(&assign.path));

            let expr = assign
                .value
                .expr_tokens(&property_path, property_span, value_span)
                .unwrap_or_else(|e| non_user_error!(e));

            when_inits.extend(quote_spanned! {util::path_span(&assign.path)=>
                #property_path::code_gen!{ if !allowed_in_when=> std::compile_error!{ #not_allowed_error } }
            });
            when_inits.extend(quote! {
                #cfg
                #(#lints)*
                #(#a_lints)*
                let #assign_val_id = #expr;
            });

            // map of { property => [(cfg, condition_var, when_value_ident, when_value_for_prop)] }
            let p_whens = when_assigns.entry(assign.path).or_default();
            let val = assign_val_id.to_token_stream();
            p_whens.push((None, cfg.to_token_stream(), ident.clone(), assign_val_id, val));
        }
    }
    // properties that are only introduced in user when conditions.
    for (property, (cfg, i)) in user_when_properties {
        let args_ident = ident!("__ud{i}_{}", util::path_to_ident_str(&property));

        let property_path = match property.get_ident() {
            Some(maybe_inherited) if inherited_properties.contains_key(maybe_inherited) => {
                let p_ident = ident!("__p_{maybe_inherited}");
                parse_quote! { #module::#p_ident }
            }
            _ => property.clone(),
        };

        property_inits.extend(quote! {
            #cfg
            #property_path::code_gen! {
                if default=>

                #cfg
                #[allow(non_snake_case)]
                let #args_ident = #property_path::default_args();
            }
            #cfg
            #property_path::code_gen!{
                if !default=>

                #cfg
                #[allow(clippy::needless_late_init)]
                let #args_ident;

                // a compile error was already added for this case.
                #[allow(unreachable_code)] {
                    #cfg
                    #args_ident = std::unreachable!();
                }
            }
        });
        let p_span = property_path.span();

        // register data for the set call generation.
        prop_set_calls.push((
            property_path.to_token_stream(),
            args_ident.clone(),
            util::display_path(&property_path),
            {
                quote_spanned! {p_span=>
                    #module::__core::source_location!()
                }
            },
            None,
            cfg.to_token_stream(),
            /*user_assigned: */ true,
            p_span,
            /*val_span: */ call_site,
        ));

        wgt_properties.insert(property, (Some(args_ident), None, cfg.unwrap_or_default()));
    }

    // generate property assigns.
    let mut property_set_calls = TokenStream::default();

    // node__ @ call_site
    let node__ = ident!("node__");
    let dyn_node__ = ident!("dyn_node__");
    let caps: Vec<Vec<_>> = widget_data
        .new_captures
        .iter()
        .map(|c| {
            c.iter()
                .map(|p| {
                    wgt_properties
                        .get({
                            let ident = &p.ident;
                            &parse_quote!(#ident)
                        })
                        .cloned()
                        .unwrap_or_else(|| (None, None, quote!()))
                })
                .collect()
        })
        .collect();

    // generate capture_only asserts.
    for (p_mod, _, p_name, _, p_cfg, cfg, _, p_span, _) in &prop_set_calls {
        let capture_only_error =
            format!("property `{p_name}` cannot be set because it is capture-only, but is not captured by the widget",);
        let assert = quote_spanned! {*p_span=>
            #cfg
            #p_mod::code_gen!{
                if capture_only =>  std::compile_error!{#capture_only_error}
            }
        };
        if let Some(p_cfg) = p_cfg {
            property_set_calls.extend(quote! {
                #module::#p_cfg! {
                    #assert
                }
            })
        } else {
            property_set_calls.extend(assert)
        };
    }

    let make_cap_idents = |(ident, p_cfg, cfg): &(Option<Ident>, Option<Ident>, TokenStream)| {
        if let Some(ident) = ident {
            let ident = quote! {
                #cfg
                #ident
            };
            if let Some(p_cfg) = p_cfg {
                quote! {
                    #module::#p_cfg! {
                        #ident
                    }
                }
            } else {
                ident
            }
        } else {
            quote! { std::unreachable!() }
        }
    };
    let make_cap_user_set = |(ident, p_cfg, cfg): &(Option<Ident>, Option<Ident>, TokenStream)| {
        if let Some(ident) = ident {
            let user_set = overriden_properties.contains(&ident);
            let user_set = quote! {
                #cfg
                #user_set
            };
            if let Some(p_cfg) = p_cfg {
                quote! {
                    #module::#p_cfg! {
                        #user_set
                    }
                }
            } else {
                user_set
            }
        } else {
            quote! { std::unreachable!() }
        }
    };

    let dyn_wgt_part__ = ident!("dyn_wgt_part__");

    let settable_priorities = crate::property::Priority::all_settable();
    for (i, priority) in settable_priorities.iter().enumerate() {
        let caps_i = i + 1;
        let caps = &caps[caps_i];
        let dynamic = widget_data.new_dynamic[caps_i];

        if dynamic {
            property_set_calls.extend(quote! {
                #[allow(unreachable_code)]
                #[allow(unused_mut)]
                let mut #dyn_wgt_part__ = #module::__core::DynWidgetPart::new_v1();
            });
        }

        for (p_mod, p_var_ident, p_name, source_loc, p_cfg, cfg, user_assigned, p_span, val_span) in prop_set_calls.iter().rev() {
            // __set @ value span

            let child = if dynamic { &dyn_node__ } else { &node__ };

            let is_when_condition = dynamic && used_in_when_expr.contains(&ident!("{}", p_name.split(':').last().unwrap()));

            let set = ident_spanned!(*val_span=> "__set");
            let set_call = if dynamic {
                quote_spanned! {*p_span=>
                    #cfg
                    #p_mod::code_gen! {
                        set_dyn #priority, #child, #p_mod, #p_var_ident, #p_name, #source_loc, #user_assigned, #set,
                        #dyn_wgt_part__, #is_when_condition
                    }
                }
            } else {
                quote_spanned! {*p_span=>
                    #cfg
                    #p_mod::code_gen! {
                        set #priority, #child, #p_mod, #p_var_ident, #p_name, #source_loc, #user_assigned, #set
                    }
                }
            };

            if let Some(p_cfg) = p_cfg {
                property_set_calls.extend(quote! {
                    #module::#p_cfg! {
                        #set_call
                    }
                });
            } else {
                property_set_calls.extend(set_call);
            }
        }

        let cap_idents: Vec<_> = caps.iter().map(make_cap_idents).collect();
        let cap_user_set: Vec<_> = caps.iter().map(make_cap_user_set).collect();

        let (dyn_suffix, dyn_props__) = if dynamic {
            ("_dyn", quote! { #dyn_wgt_part__, })
        } else {
            ("", TokenStream::new())
        };

        let new_fn_ident = ident!("__new_{}_inspect{}", priority, dyn_suffix);
        let cap_idents2 = cap_idents.iter();

        property_set_calls.extend(quote! { #module::__core::core_cfg_inspector! {
            #[allow(unreachable_code)]
            let #node__ = #module::#new_fn_ident(#node__, #dyn_props__ #(#cap_idents2,)* #(#cap_user_set),*);
        }});

        let new_fn_ident = ident!("__new_{}{}", priority, dyn_suffix);
        property_set_calls.extend(quote! { #module::__core::core_cfg_inspector! {@NOT
            #[allow(unreachable_code)]
            let #node__ = #module::#new_fn_ident(#node__, #dyn_props__ #(#cap_idents),*);
        }});
    }
    let property_set_calls = property_set_calls;

    // apply the whens for each property.
    for (property, assigns) in when_assigns {
        let property_path = match property.get_ident() {
            Some(maybe_inherited) if inherited_properties.contains_key(maybe_inherited) => {
                let p_ident = ident!("__p_{maybe_inherited}");
                quote! { #module::#p_ident }
            }
            _ => property.to_token_stream(),
        };

        // collect assign items.
        let mut init_members = TokenStream::default();
        let mut conditions = Vec::with_capacity(assigns.len());
        for (p_cfg, w_cfg, condition_ident, value_ident, value) in assigns {
            if !util::token_stream_eq(value_ident.to_token_stream(), value.clone()) {
                let assign = quote! {
                    #[allow(non_snake_case)]
                    #w_cfg
                    let #value_ident;
                    #w_cfg {
                        #value_ident = #value;
                    }
                };

                if let Some(p_cfg) = &p_cfg {
                    init_members.extend(quote! {
                        #module::#p_cfg! {
                            #assign
                        }
                    });
                } else {
                    init_members.extend(assign);
                }
            }

            let p_cfg = if let Some(p_cfg) = p_cfg {
                quote! {
                    use(#module::#p_cfg)
                }
            } else {
                quote! {
                    use(#module::__core::core_cfg_ok)
                }
            };

            conditions.push(quote! {
                #w_cfg
                #[allow(non_snake_case)]
                #p_cfg #condition_ident => #value_ident,
            });
        }
        // later conditions have priority.
        conditions.reverse();

        let (default, p_cfg, cfg) = wgt_properties
            .get(&property)
            .unwrap_or_else(|| non_user_error!("property `{}` (introduced in when?) not found", quote!(#property)));

        let when_init = quote! {
            #property_path::code_gen! { if allowed_in_when=>
                #cfg
                #[allow(non_snake_case)]
                let #default = {
                    #init_members
                    #property_path::code_gen! {
                        when #property_path {
                            #(#conditions)*
                            _ => #default,
                        }
                    }
                };
            }
        };
        if let Some(p_cfg) = p_cfg {
            when_inits.extend(quote! {
                #module::#p_cfg! {
                    #when_init
                }
            });
        } else {
            when_inits.extend(when_init);
        }
    }

    // Generate new function calls:
    let new_child_call = {
        let new_child_caps = &caps[0];
        let ncc_idents: Vec<_> = new_child_caps.iter().map(make_cap_idents).collect();
        let ncc_idents2 = ncc_idents.iter();
        let ncc_cap_user_set: Vec<_> = new_child_caps.iter().map(make_cap_user_set).collect();

        quote! {
            #module::__core::core_cfg_inspector! {
                #[allow(unreachable_code)]
                let node__ = #module::__new_child_inspect(#(#ncc_idents2,)* #(#ncc_cap_user_set),*);
            }
            #module::__core::core_cfg_inspector! {@NOT
                #[allow(unreachable_code)]
                let node__ = #module::__new_child(#(#ncc_idents),*);
            }
        }
    };

    let new_call = {
        let new_caps = caps.last().unwrap();
        let nc_idents: Vec<_> = new_caps.iter().map(make_cap_idents).collect();
        let nc_idents2 = nc_idents.iter();
        let nc_cap_user_set: Vec<_> = new_caps.iter().map(make_cap_user_set).collect();
        quote! {
            #module::__core::core_cfg_inspector! {
                #[allow(unreachable_code)]
                #module::__new_inspect(
                    node__,
                    #(#nc_idents2,)*
                    #(#nc_cap_user_set,)*
                    #module::__widget_name(),
                    when_infos__,
                    #module::__decl_location(),
                    #module::__core::source_location!()
                )
            }
            #module::__core::core_cfg_inspector! {@NOT
                #[allow(unreachable_code)]
                #module::__new(node__, #(#nc_idents),*)
            }
        }
    };

    let r = quote! {
        {
            #errors
            #property_inits
            #when_inits
            #new_child_call
            #property_set_calls
            #new_call
        }
    };

    r.into()
}

struct Input {
    widget_data: WidgetData,
    user_input: UserInput,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Input {
            widget_data: input.parse().unwrap_or_else(|e| non_user_error!(e)),
            // user errors go into UserInput::errors field.
            user_input: input.parse().unwrap_or_else(|e| non_user_error!(e)),
        })
    }
}

struct WidgetData {
    module: TokenStream,
    properties: Vec<BuiltProperty>,
    whens: Vec<BuiltWhen>,
    new_captures: Vec<Vec<PropertyCapture>>,
    new_dynamic: Vec<bool>,
}
impl Parse for WidgetData {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = non_user_braced!(input, "widget");
        let r = Ok(Self {
            module: non_user_braced!(&input, "module").parse().unwrap(),
            properties: parse_all(&non_user_braced!(&input, "properties")).unwrap_or_else(|e| non_user_error!(e)),
            whens: parse_all(&non_user_braced!(&input, "whens")).unwrap_or_else(|e| non_user_error!(e)),
            new_captures: {
                let input = non_user_braced!(&input, "new_captures");
                FnPriority::all()
                    .iter()
                    .map(|p| parse_all(&non_user_braced!(&input, p.to_string())).unwrap_or_else(|e| non_user_error!(e)))
                    .collect()
            },
            new_dynamic: {
                let input = non_user_braced!(&input, "new_dynamic");
                FnPriority::all()
                    .iter()
                    .map(|p| {
                        let f: LitBool = non_user_braced!(&input, p.to_string()).parse().unwrap();
                        f.value
                    })
                    .collect()
            },
        });

        r
    }
}

pub struct BuiltProperty {
    pub ident: Ident,
    pub docs: TokenStream,
    pub cfg: bool,
    pub default: bool,
    pub required: bool,
}
impl Parse for BuiltProperty {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);

        let r = BuiltProperty {
            ident,
            docs: non_user_braced!(&input, "docs").parse().unwrap(),
            cfg: non_user_braced!(&input, "cfg")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            default: non_user_braced!(&input, "default")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            required: non_user_braced!(&input, "required")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
        };
        Ok(r)
    }
}

#[derive(Clone)]
pub struct PropertyCapture {
    pub ident: Ident,
    pub cfg: bool,
}
impl Parse for PropertyCapture {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);
        let r = PropertyCapture {
            ident,
            cfg: non_user_braced!(&input, "cfg")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
        };
        Ok(r)
    }
}

pub struct BuiltWhen {
    pub ident: Ident,
    pub dbg_ident: TokenStream,
    pub docs: TokenStream,
    pub cfg: bool,
    pub inputs: Vec<Ident>,
    pub assigns: Vec<BuiltWhenAssign>,
    pub expr_str: syn::LitStr,
}
impl Parse for BuiltWhen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);

        let r = BuiltWhen {
            ident,
            dbg_ident: non_user_braced!(&input, "dbg_ident").parse().unwrap(),
            docs: non_user_braced!(&input, "docs").parse().unwrap(),
            cfg: non_user_braced!(&input, "cfg")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            inputs: parse_all(&non_user_braced!(&input, "inputs")).unwrap_or_else(|e| non_user_error!(e)),
            assigns: parse_all(&non_user_braced!(&input, "assigns")).unwrap_or_else(|e| non_user_error!(e)),
            expr_str: non_user_braced!(&input, "expr_str").parse().unwrap_or_else(|e| non_user_error!(e)),
        };
        Ok(r)
    }
}

pub struct BuiltWhenAssign {
    pub property: Ident,
    pub cfg: bool,
    pub value_fn: Ident,
}
impl Parse for BuiltWhenAssign {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let property = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);

        let r = BuiltWhenAssign {
            property,
            cfg: non_user_braced!(&input, "cfg")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            value_fn: non_user_braced!(&input, "value_fn").parse().unwrap_or_else(|e| non_user_error!(e)),
        };
        Ok(r)
    }
}

/// The content of the widget macro call.
pub struct UserInput {
    call_site: Span,
    errors: Errors,
    pub properties: Vec<PropertyAssign>,
    pub whens: Vec<When>,
}
impl Parse for UserInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = non_user_braced!(input, "user");

        let call_site = non_user_braced!(&input, "call_site")
            .parse::<proc_macro2::TokenTree>()
            .unwrap()
            .span();

        let mut errors = Errors::default();
        let mut properties = vec![];
        let mut whens = vec![];

        while !input.is_empty() {
            let attrs = parse_outer_attrs(&input, &mut errors);

            if input.peek(keyword::when) {
                if let Some(mut when) = When::parse(&input, &mut errors) {
                    when.attrs = attrs;
                    whens.push(when);
                }
            } else if input.peek(Ident) || input.peek(Token![crate]) || input.peek(Token![super]) || input.peek(Token![self]) {
                // peek ident or path.
                match input.parse::<PropertyAssign>() {
                    Ok(mut assign) => {
                        assign.attrs = attrs;
                        if !input.is_empty() && assign.semi.is_none() {
                            errors.push("expected `;`", input.span());
                            while !(input.is_empty() || input.peek(Ident) || input.peek(Token![#]) && input.peek2(token::Bracket)) {
                                // skip to next property start or when
                                let _ = input.parse::<TokenTree>();
                            }
                        }
                        properties.push(assign);
                    }
                    Err(e) => {
                        let (recoverable, e) = e.recoverable();
                        errors.push_syn(e);
                        if !recoverable {
                            break;
                        }
                    }
                }
            } else {
                errors.push("expected `when` or a property path", input.span());
                break;
            }
        }

        if !input.is_empty() {
            if errors.is_empty() {
                errors.push("unexpected token", input.span());
            }
            // suppress the "unexpected token" error from syn parse.
            let _ = input.parse::<TokenStream>();
        }

        Ok(UserInput {
            call_site,
            errors,
            properties,
            whens,
        })
    }
}

/// Property assign in a widget instantiation or when block.
pub struct PropertyAssign {
    pub attrs: Vec<Attribute>,
    pub path: Path,
    pub path_args: Option<AngleBracketedGenericArguments>,
    pub eq: Token![=],
    pub value: PropertyValue,
    pub value_span: Span,
    pub semi: Option<Token![;]>,
}
impl Parse for PropertyAssign {
    /// Expects that outer attributes are already parsed and that ident, super or self was peeked.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut path = input.parse::<Path>()?;
        let path_is_ident = path.get_ident().is_some();

        if path_is_ident && (input.is_empty() || input.peek(Token![;])) {
            // shorthand assign
            let semi = input.parse().unwrap_or_default();
            let value_span = path.span();
            let eq = parse_quote_spanned! {value_span=> = };
            let value = parse_quote! { #path };
            return Ok(PropertyAssign {
                attrs: vec![],
                path,
                path_args: None,
                eq,
                value,
                value_span,
                semi,
            });
        }

        let peek_next_assign = |input: ParseStream| {
            // checks if the next tokens in the stream look like the start
            // of another property assign.
            let fork = input.fork();
            let _ = util::parse_outer_attrs(&fork, &mut Errors::default());
            if fork.peek2(Token![=]) {
                fork.peek(Ident)
            } else if fork.peek2(Token![::]) {
                fork.parse::<Path>().is_ok() && fork.peek(Token![=])
            } else {
                fork.peek(keyword::when)
            }
        };

        let eq = input.parse::<Token![=]>().map_err(|e| {
            if peek_next_assign(input) {
                let msg = if path_is_ident { "expected `=` or `;`" } else { "expected `=`" };
                util::recoverable_err(e.span(), msg)
            } else {
                syn::Error::new(e.span(), "expected `=`")
            }
        })?;

        let value_span = if input.is_empty() { eq.span() } else { input.span() };
        let value = input.parse::<PropertyValue>();
        let semi = if input.peek(Token![;]) {
            Some(input.parse().unwrap())
        } else {
            None
        };
        let value = value.map_err(|e| {
            let (recoverable, mut e) = e.recoverable();
            let mut msg = e.to_string();
            if msg.starts_with("unexpected end of input") {
                msg = "expected property value".to_owned();
            }
            if util::span_is_call_site(e.span()) {
                e = syn::Error::new(value_span, msg);
            }
            if recoverable {
                e = e.set_recoverable();
            }
            e
        })?;

        if let Some(t) = path.leading_colon.take() {
            if path.segments.is_empty() {
                return Err(syn::Error::new(t.span(), "expected property"));
            }
        }

        let mut path_args = None;
        let last = path.segments.len() - 1;
        for (i, seg) in path.segments.iter_mut().enumerate() {
            if seg.arguments.is_empty() {
                continue;
            }
            match mem::replace(&mut seg.arguments, PathArguments::None) {
                PathArguments::AngleBracketed(a) => {
                    if i == last {
                        path_args = Some(a);
                    } else {
                        return Err(syn::Error::new(a.span(), "unexpected in property path, type args are only allowed in the last segment"))
                    }
                },
                PathArguments::Parenthesized(a) => return Err(syn::Error::new(a.span(), "unexpected in property path")),
                PathArguments::None => unreachable!(),
            }
        }

        Ok(Self {
            attrs: vec![],
            path,
            path_args,
            eq,
            value,
            value_span,
            semi,
        })
    }
}

/// Value [assigned](PropertyAssign) to a property.
#[derive(Debug)]
pub enum PropertyValue {
    /// `unset!`. // TODO: rename Special to Unset.
    Special(Ident, Token![!]),
    /// `arg0, arg1,`
    Unnamed(Punctuated<Expr, Token![,]>),
    /// `{ field0: true, field1: false, }`
    Named(syn::token::Brace, Punctuated<FieldValue, Token![,]>),
}
impl PropertyValue {
    /// Convert this value to an expr. Panics if `self` is [`Special`].
    pub fn expr_tokens(&self, property_path: &TokenStream, span: Span, value_span: Span) -> Result<TokenStream, &'static str> {
        // property ArgsImpl alias with value span to show type errors involving generics in the
        // right place.

        if util::is_rust_analyzer() {
            // rust-analyzer can't find the `property_path`, because it is expanded after inheritance and it
            // can't resolve inheritance.

            use quote::quote as call_site_quote; // ra works better with the actual default call_site.

            let mut out = call_site_quote! {let _about = "This is a dummy expansion that only happens for rust-analyzer.";};

            match self {
                PropertyValue::Unnamed(args) => {
                    for arg in args.iter() {
                        out.extend(call_site_quote! {
                            drop(#arg);
                        });
                    }
                }
                PropertyValue::Named(_, fields) => {
                    for field in fields.iter() {
                        let value = &field.expr;
                        out.extend(call_site_quote! {
                            drop(#value);
                        });
                    }
                }
                PropertyValue::Special(_, _) => return Err("cannot expand special"),
            }

            Ok(out)
        } else {
            match self {
                PropertyValue::Unnamed(args) => {
                    let args_impl = quote_spanned!(value_span=> __ArgsImpl::new);
                    Ok(quote_spanned! {span=>
                        #property_path::code_gen! {if resolved=>{
                            use #property_path::{ArgsImpl as __ArgsImpl};
                            #args_impl(#args)
                        }}
                    })
                }
                PropertyValue::Named(brace, fields) => {
                    let args_impl = ident_spanned!(value_span=> "__ArgsImpl");
                    let fields = quote_spanned! { brace.span=> { #fields } };
                    Ok(quote_spanned! {span=>
                        #property_path::code_gen! { named_new #property_path, #args_impl #fields }
                    })
                }
                PropertyValue::Special(_, _) => Err("cannot expand special"),
            }
        }
    }
}
impl Parse for PropertyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) && input.peek2(Token![!]) && (input.peek3(Token![;]) || input.peek3(Ident::peek_any) || !peek_any3(input)) {
            let r = PropertyValue::Special(input.parse().unwrap(), input.parse().unwrap());
            return Ok(r);
        }

        if input.peek(token::Brace) && !input.peek2(Token![,]) {
            // Differentiating between a fields declaration and a single unnamed arg declaration gets tricky.
            //
            // This is a normal fields decl.: `{ field0: "value" }`
            // This is a block single argument decl.: `{ foo(); bar() }`
            //
            // Fields can use the shorthand field name only `{ field0 }`
            // witch is also a single arg block expression. In this case
            // we parse as Unnamed, if it was a field it will still work because
            // we only have one field.

            let maybe_fields = input.fork();
            let fields_input;
            let fields_brace = braced!(fields_input in maybe_fields);

            if fields_input.peek(Ident)
                && (
                    // ident:
                    (fields_input.peek2(Token![:]) && !fields_input.peek2(Token![::]))
                    // OR ident,
                    || fields_input.peek2(Token![,])
                )
            {
                // it is fields
                input.advance_to(&maybe_fields);

                // disconnect syn internal errors
                let fields_input = fields_input.parse::<TokenStream>().unwrap();
                let r = parse_punct_terminated2(fields_input).map_err(|e| {
                    if util::span_is_call_site(e.span()) {
                        util::recoverable_err(fields_brace.span, e)
                    } else {
                        e.set_recoverable()
                    }
                })?;
                return Ok(PropertyValue::Named(fields_brace, r));
            }
        }

        // only valid option left is a sequence of "{expr},", we want to parse
        // in a recoverable way, so first we take raw token trees until we find the
        // end "`;` | EOF" or we find the start of a new property, when or remove item.
        let mut args_input = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![;]) {
            if peek_next_wgt_item(&input.fork()) {
                break;
            }
            input.parse::<TokenTree>().unwrap().to_tokens(&mut args_input);
        }

        let r = util::parse_punct_terminated2(args_input).map_err(|e| e.set_recoverable())?;
        Ok(PropertyValue::Unnamed(r))
    }
}

fn peek_next_wgt_item(lookahead: ParseStream) -> bool {
    let has_attr = lookahead.peek(Token![#]) && lookahead.peek(token::Bracket);
    if has_attr {
        let _ = parse_outer_attrs(lookahead, &mut Errors::default());
    }
    if lookahead.peek(keyword::when) {
        return true; // when ..
    }
    if lookahead.peek(Ident) {
        if lookahead.peek2(Token![::]) {
            let _ = lookahead.parse::<Path>();
        } else {
            let ident = lookahead.parse::<Ident>().unwrap();

            if lookahead.peek(token::Brace) {
                return ident == "remove"; // remove { .. }
            }
        }

        return lookahead.peek(Token![=]) && !lookahead.peek(Token![==]);
    }

    false
}

/// When block in a widget instantiation or declaration.
pub struct When {
    pub attrs: Vec<Attribute>,
    pub when: keyword::when,
    pub condition_expr: TokenStream,
    pub brace_token: syn::token::Brace,
    pub assigns: Vec<PropertyAssign>,
}
impl When {
    /// Call only if peeked `when`. Parse outer attribute before calling.
    pub fn parse(input: ParseStream, errors: &mut Errors) -> Option<When> {
        let when = input.parse::<keyword::when>().unwrap_or_else(|e| non_user_error!(e));

        if input.is_empty() {
            errors.push("expected when expression", when.span());
            return None;
        }
        let condition_expr = crate::expr_var::parse_without_eager_brace(input);

        let (brace_token, assigns) = if input.peek(syn::token::Brace) {
            let (brace, inner) = util::parse_braces(input).unwrap();
            let mut assigns = vec![];
            while !inner.is_empty() {
                let attrs = parse_outer_attrs(&inner, errors);

                if !(inner.peek(Ident) || inner.peek(Token![super]) || inner.peek(Token![self])) {
                    errors.push("expected property path", if inner.is_empty() { brace.span } else { inner.span() });
                    while !(inner.is_empty()
                        || inner.peek(Ident)
                        || inner.peek(Token![super])
                        || inner.peek(Token![self])
                        || inner.peek(Token![#]) && inner.peek(token::Bracket))
                    {
                        // skip to next property.
                        let _ = inner.parse::<TokenTree>();
                    }
                }
                if inner.is_empty() {
                    break;
                }

                match inner.parse::<PropertyAssign>() {
                    Ok(mut p) => {
                        p.attrs = attrs;
                        if !inner.is_empty() && p.semi.is_none() {
                            errors.push("expected `,`", inner.span());
                            while !(inner.is_empty()
                                || input.peek(Ident)
                                || input.peek(Token![crate])
                                || input.peek(Token![super])
                                || input.peek(Token![self])
                                || inner.peek(Token![#]) && inner.peek(token::Bracket))
                            {
                                // skip to next property.
                                let _ = inner.parse::<TokenTree>();
                            }
                        }
                        assigns.push(p);
                    }
                    Err(e) => {
                        let (recoverable, e) = e.recoverable();
                        if util::span_is_call_site(e.span()) {
                            errors.push(e, brace.span);
                        } else {
                            errors.push_syn(e);
                        }
                        if !recoverable {
                            break;
                        }
                    }
                }
            }
            (brace, assigns)
        } else {
            errors.push("expected a block of property assigns", util::last_span(condition_expr));
            return None;
        };

        Some(When {
            attrs: vec![], // must be parsed before.
            when,
            condition_expr,
            brace_token,
            assigns,
        })
    }

    /// Returns an ident `__{prefix}{i}_{expr_to_str}`
    pub fn make_ident(&self, prefix: impl std::fmt::Display, i: usize, span: Span) -> Ident {
        ident_spanned!(span=> "__{prefix}{i}_{}", tokens_to_ident_str(&self.condition_expr.to_token_stream()))
    }

    /// Analyzes the [`Self::condition_expr`], collects all property member accesses and replaces then with `expr_var!` placeholders.
    pub fn expand_condition(&self) -> syn::Result<WhenExprToVar> {
        syn::parse2::<WhenExprToVar>(self.condition_expr.clone())
    }
}

pub mod keyword {
    syn::custom_keyword!(when);
}

/// See [`When::expand_condition`].
pub struct WhenExprToVar {
    /// Map of `(property_path, member_method) => var_name`, example: `(id, __0) => __id__0`
    pub properties: HashMap<(syn::Path, Ident), Ident>,
    ///The [input expression](When::condition_expr) with all properties replaced with `expr_var!` placeholders.
    pub expr: TokenStream,
}
impl WhenExprToVar {
    fn parse_inner(input: ParseStream) -> syn::Result<Self> {
        let mut properties = HashMap::new();
        let mut expr = TokenStream::default();

        while !input.is_empty() {
            // look for `self.property(.member)?` and replace with `#{__property__member}`
            if input.peek(Token![self]) && input.peek2(Token![.]) {
                input.parse::<Token![self]>().unwrap();
                let last_span = input.parse::<Token![.]>().unwrap().span();

                let property = input.parse::<Path>().map_err(|e| {
                    if util::span_is_call_site(e.span()) {
                        syn::Error::new(last_span, e)
                    } else {
                        e
                    }
                })?;
                let member_ident = if input.peek(Token![.]) && !input.peek2(Token![await]) && !input.peek3(token::Paren) {
                    input.parse::<Token![.]>().unwrap();
                    if input.peek(Ident) {
                        let member = input.parse::<Ident>().unwrap();
                        ident_spanned!(member.span()=> "__{member}")
                    } else {
                        let index = input.parse::<syn::Index>().map_err(|e| {
                            let span = if util::span_is_call_site(e.span()) { last_span } else { e.span() };

                            syn::Error::new(span, "expected identifier or index")
                        })?;
                        ident_spanned!(index.span()=> "__{}", index.index)
                    }
                } else {
                    ident_spanned!(property.span()=> "__0")
                };

                let member_span = member_ident.span();
                let var_ident = ident_spanned!(member_span=> "__{}{}", util::display_path(&property).replace("::", "_"), member_ident);

                expr.extend(quote_spanned! {member_span=>
                    (*#{#var_ident}) // deref here to simulate a `self.`
                });

                properties.insert((property, member_ident), var_ident);
            }
            // recursive parse groups:
            else if input.peek(token::Brace) {
                let inner = WhenExprToVar::parse_inner(&non_user_braced!(input))?;
                properties.extend(inner.properties);
                let inner = inner.expr;
                expr.extend(quote_spanned! {inner.span()=> { #inner } });
            } else if input.peek(token::Paren) {
                let inner = WhenExprToVar::parse_inner(&non_user_parenthesized!(input))?;
                properties.extend(inner.properties);
                let inner = inner.expr;
                expr.extend(quote_spanned! {inner.span()=> ( #inner ) });
            } else if input.peek(token::Bracket) {
                let inner = WhenExprToVar::parse_inner(&non_user_bracketed!(input))?;
                properties.extend(inner.properties);
                let inner = inner.expr;
                expr.extend(quote_spanned! {inner.span()=> [ #inner ] });
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
impl Parse for WhenExprToVar {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut r = WhenExprToVar::parse_inner(input)?;
        let expr = &mut r.expr;

        // assert expression type.
        *expr = quote_spanned! {expr.span()=>
            // TODO figure out a way to have this validation without causing
            // simple direct references to a boolean state generate a .map(..) var.
            let __result__: bool = { #expr };
            __result__
        };

        Ok(r)
    }
}
