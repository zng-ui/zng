use std::collections::{HashMap, HashSet};

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    parse::{discouraged::Speculative, Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Expr, FieldValue, Ident, LitBool, Path, Token,
};

use crate::util::{self, parse_all, parse_outer_attrs, tokens_to_ident_str, Attributes, ErrorRecoverable, Errors};

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

    let call_site = widget_data.call_site;
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

    let child_properties: HashSet<_> = widget_data.properties_child.iter().map(|p| &p.ident).collect();

    let inherited_properties: HashSet<_> = widget_data
        .properties_child
        .iter()
        .chain(widget_data.properties.iter())
        .map(|p| &p.ident)
        .collect();

    // properties that must be assigned by the user.
    let required_properties: HashSet<_> = widget_data
        .properties_child
        .iter()
        .chain(widget_data.properties.iter())
        .filter(|p| p.required)
        .map(|p| &p.ident)
        .collect();
    // properties that have a default value.
    let default_properties: HashSet<_> = widget_data
        .properties_child
        .iter()
        .chain(widget_data.properties.iter())
        .filter(|p| p.default)
        .map(|p| &p.ident)
        .collect();
    // properties that are captured in new_child or new.
    let captured_properties: HashSet<_> = widget_data.new_child.iter().chain(widget_data.new.iter()).collect();

    // inherited properties unset by the user.
    let mut unset_properties = HashSet::new();

    // properties user assigned with `special!` values (valid and invalid).
    let mut user_properties = HashSet::new();

    // user assigns with valid values.
    let user_properties: Vec<_> = user_input
        .properties
        .iter()
        .filter(|up| {
            // if already (un)set by the user.
            if !user_properties.insert(&up.path) {
                let p_name = util::display_path(&up.path);
                errors.push(format_args!("property `{}` already set", p_name), util::path_span(&up.path));
                return false;
            }

            if let PropertyValue::Special(sp, _) = &up.value {
                if sp == "unset" {
                    if let Some(maybe_inherited) = up.path.get_ident() {
                        if required_properties.contains(maybe_inherited) || captured_properties.contains(maybe_inherited) {
                            errors.push(format_args!("cannot unset required property `{}`", maybe_inherited), sp.span());
                        } else if !default_properties.contains(maybe_inherited) {
                            errors.push(
                                format_args!("cannot unset `{}` because it is not set by the widget", maybe_inherited),
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
                    errors.push(format_args!("unknown value `{}!`", sp), sp.span());
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
        .filter(|p_id| inherited_properties.contains(p_id))
        .chain(unset_properties.iter().copied())
        .collect();

    // all widget properties that will be set (property_path, (property_var, cfg)).
    let mut wgt_properties = HashMap::<syn::Path, (Ident, TokenStream)>::new();

    let mut property_inits = TokenStream::default();
    let mut child_prop_set_calls = vec![];
    let mut prop_set_calls = vec![];

    // for each inherited property that has a default value and is not overridden by the user:
    for (ip, is_child) in widget_data
        .properties_child
        .iter()
        .map(|ip| (ip, true))
        .chain(widget_data.properties.iter().map(|ip| (ip, false)))
        .filter(|(ip, _)| ip.default && !overriden_properties.contains(&ip.ident))
    {
        let ident = &ip.ident;
        let p_default_fn_ident = ident!("__d_{}", ident);
        let p_var_ident = ident!("__{}", ident);
        let cfg = &ip.cfg;

        wgt_properties.insert(parse_quote! { #ident }, (p_var_ident.clone(), cfg.clone()));

        // generate call to default args.
        property_inits.extend(quote! {
            #cfg
            let #p_var_ident = #module::#p_default_fn_ident();
        });

        if captured_properties.contains(ident) {
            continue; // we don't set captured properties.
        }

        let p_mod_ident = ident!("__p_{}", ident);
        // register data for the set call generation.
        let prop_set_calls = if is_child { &mut child_prop_set_calls } else { &mut prop_set_calls };
        #[cfg(debug_assertions)]
        prop_set_calls.push((
            quote! { #module::#p_mod_ident },
            p_var_ident,
            ip.ident.to_string(),
            {
                let p_source_loc_ident = ident!("__loc_{}", ip.ident);
                quote! { #module::#p_source_loc_ident() }
            },
            cfg.clone(),
            /*user_assigned: */ false,
            call_site,
            call_site,
        ));
        #[cfg(not(debug_assertions))]
        prop_set_calls.push((
            quote! { #module::#p_mod_ident },
            p_var_ident,
            ip.ident.to_string(),
            cfg.clone(),
            call_site,
            call_site,
        ));
    }

    // for each property assigned in the widget instantiation call (excluding when blocks and `special!` values).
    for up in &user_properties {
        let p_name = util::display_path(&up.path);

        let p_mod = match up.path.get_ident() {
            Some(maybe_inherited) if inherited_properties.contains(maybe_inherited) => {
                let p_ident = ident!("__p_{}", maybe_inherited);
                quote! { #module::#p_ident }
            }
            _ => up.path.to_token_stream(),
        };
        let p_var_ident = ident!("__u_{}", p_name.replace("::", "_"));
        let attrs = Attributes::new(up.attrs.clone());
        let cfg = attrs.cfg;
        let lints = attrs.lints;

        wgt_properties.insert(up.path.clone(), (p_var_ident.clone(), cfg.to_token_stream()));

        let init_expr = up
            .value
            .expr_tokens(&p_mod, up.path.span(), up.value_span)
            .unwrap_or_else(|e| non_user_error!(e));
        property_inits.extend(quote! {
            #cfg
            #(#lints)*
            let #p_var_ident = #init_expr;
        });

        if let Some(maybe_inherited) = up.path.get_ident() {
            if captured_properties.contains(maybe_inherited) {
                continue;
            }
        }
        let prop_set_calls = match up.path.get_ident() {
            Some(maybe_child) if child_properties.contains(maybe_child) => &mut child_prop_set_calls,
            _ => &mut prop_set_calls,
        };
        // register data for the set call generation.
        #[cfg(debug_assertions)]
        prop_set_calls.push((
            p_mod.to_token_stream(),
            p_var_ident,
            p_name,
            quote_spanned! {up.path.span()=>
                #module::__core::source_location!()
            },
            cfg.to_token_stream(),
            /*user_assigned: */ true,
            up.path.span(),
            up.value_span,
        ));
        #[cfg(not(debug_assertions))]
        prop_set_calls.push((
            p_mod.to_token_stream(),
            p_var_ident,
            p_name,
            cfg.to_token_stream(),
            up.path.span(),
            up.value_span,
        ));
    }

    // validate required properties.
    let mut missing_required = HashSet::new();
    for required in required_properties.into_iter().chain(captured_properties) {
        if !wgt_properties.contains_key(&parse_quote! { #required }) {
            missing_required.insert(required);
            errors.push(format!("missing required property `{}`", required), call_site);
        }
    }
    let missing_required = missing_required;

    // generate whens.
    let mut when_inits = TokenStream::default();

    #[cfg(debug_assertions)]
    when_inits.extend(quote! {
        #[allow(unused_mut)]
        let mut when_infos__: std::vec::Vec<#module::__core::WhenInfoV1> = std::vec![];
    });

    // map of { property => [(cfg, condition_var, when_value_ident, when_value_for_prop)] }
    let mut when_assigns: HashMap<Path, Vec<(TokenStream, Ident, Ident, TokenStream)>> = HashMap::new();
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
        #[cfg(debug_assertions)]
        let dbg_ident = iw.dbg_ident;
        let cfg = iw.cfg;

        // arg variables for each input, they should all have a default value or be required (already deactivated if any unset).
        let len = iw.inputs.len();
        let inputs: Vec<_> = iw
            .inputs
            .into_iter()
            .filter_map(|id| {
                let r = wgt_properties.get(&parse_quote! { #id }).map(|(id, _)| id);
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
        let c_ident = ident!("__c_{}", ident);

        #[cfg(debug_assertions)]
        let condition_call = quote! {
            #module::#dbg_ident(#(&#inputs),* , &mut when_infos__)
        };
        #[cfg(not(debug_assertions))]
        let condition_call = quote! {
            #module::#ident(#(&#inputs),*)
        };

        when_inits.extend(quote! {
            #cfg
            #[allow(non_snake_case)]
            let #c_ident;
            #cfg { #c_ident = #condition_call; }
        });

        // register when for each property assigned.
        for BuiltWhenAssign { property, cfg, value_fn } in assigns {
            let value = quote! { #module::#value_fn() };
            let p_whens = when_assigns.entry(parse_quote! { #property }).or_default();
            p_whens.push((cfg, c_ident.clone(), value_fn, value));
        }
    }

    // map of [property_without_value => combined_cfg_for_default_init]
    let mut user_when_properties: HashMap<Path, Option<TokenStream>> = HashMap::new();

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
                        errors.push(format!("cannot use unset property `{}` in when", maybe_unset), maybe_unset.span());
                    }
                    // if property maybe has a default value.
                    _ => {
                        let error = format!(
                            "property `{}` is not assigned and has no default value",
                            util::display_path(&property)
                        );
                        property_inits.extend(quote_spanned! {util::path_span(&property)=>
                            #property::code_gen!{
                                if !default=>
                                std::compile_error!{#error}
                            }
                        });

                        if !validate_but_skip {
                            let p_cfg = Attributes::new(p_attrs.to_vec()).cfg;
                            let cfg = util::cfg_attr_or(cfg.clone(), p_cfg);
                            match user_when_properties.entry(property.clone()) {
                                std::collections::hash_map::Entry::Occupied(mut e) => {
                                    let prev = e.get().clone().map(|tt| util::parse_attr(tt).unwrap());
                                    *e.get_mut() = util::cfg_attr_or(prev, cfg.map(|tt| util::parse_attr(tt).unwrap()));
                                }
                                std::collections::hash_map::Entry::Vacant(e) => {
                                    e.insert(cfg);
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
                Some(maybe_inherited) if inherited_properties.contains(maybe_inherited) => {
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

            let args_ident = wgt_properties.get(&property).map(|(id, _)| id.clone()).unwrap_or_else(|| {
                // if is not in `wgt_properties` it must be in `user_when_properties`
                // that will generate a __u_ variable before this binding in the final code.
                #[cfg(debug_assertions)]
                if !user_when_properties.contains_key(&property) {
                    non_user_error!("");
                }
                ident!("__u_{}", util::path_to_ident_str(&property))
            });

            member_vars.extend(quote! {
                #[allow(non_snake_case)]
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
        #[cfg(debug_assertions)]
        {
            let expr_str = util::format_rust_expr(w.condition_expr.to_string());
            let assign_names = w.assigns.iter().map(|a| util::display_path(&a.path));
            when_inits.extend(quote! {
                #cfg
                when_infos__.push(#module::__core::WhenInfoV1 {
                    condition_expr: #expr_str,
                    condition_var: Some(#module::__core::var::VarObj::boxed(std::clone::Clone::clone(&#ident))),
                    properties: std::vec![
                        #(#assign_names),*
                    ],
                    decl_location: #module::__core::source_location!(),
                    user_declared: false,
                });
            });
        }

        // init assign variables
        let mut assigns = HashSet::new();
        for assign in w.assigns {
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
                    errors.push(format_args!("unknown value `{}!`", sp), sp.span());
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

            let assign_val_id = ident!("__uwv_{}", util::display_path(&assign.path).replace("::", "_"));
            let cfg = util::cfg_attr_and(attrs.cfg, cfg.clone());
            let a_lints = attrs.lints;

            let (property_path, property_span, value_span) = match assign.path.get_ident() {
                Some(maybe_inherited) if inherited_properties.contains(maybe_inherited) => {
                    let p_ident = ident!("__p_{}", maybe_inherited);
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
            p_whens.push((cfg.to_token_stream(), ident.clone(), assign_val_id, val));
        }
    }
    // properties that are only introduced in user when conditions.
    for (p, cfg) in user_when_properties {
        let args_ident = ident!("__u_{}", util::path_to_ident_str(&p));

        property_inits.extend(quote! {
            #cfg
            #p::code_gen! {
                if default=>

                #cfg
                let #args_ident = #p::ArgsImpl::default();
            }
            #cfg
            #p::code_gen!{
                if !default=>

                #cfg
                let #args_ident;

                // a compile error was already added for this case.
                #[allow(unreachable_code)] {
                    #cfg
                    #args_ident = std::unreachable!();
                }
            }
        });
        let p_span = p.span();

        // register data for the set call generation.
        #[cfg(debug_assertions)]
        prop_set_calls.push((
            p.to_token_stream(),
            args_ident.clone(),
            util::display_path(&p),
            {
                quote_spanned! {p_span=>
                    #module::__core::source_location!()
                }
            },
            cfg.to_token_stream(),
            /*user_assigned: */ true,
            p_span,
            /*val_span: */ call_site,
        ));
        #[cfg(not(debug_assertions))]
        prop_set_calls.push((
            p.to_token_stream(),
            args_ident.clone(),
            util::display_path(&p),
            cfg.to_token_stream(),
            p_span,
            call_site,
        ));

        wgt_properties.insert(p, (args_ident, cfg.unwrap_or_default()));
    }

    // generate property assigns.
    let mut property_set_calls = TokenStream::default();

    // node__ @ call_site
    let node__ = ident!("node__");

    for set_calls in vec![child_prop_set_calls, prop_set_calls] {
        for set_call in &set_calls {
            #[cfg(debug_assertions)]
            let (p_mod, _, p_name, _, cfg, _, p_span, _) = set_call;
            #[cfg(not(debug_assertions))]
            let (p_mod, _, p_name, cfg, p_span, _) = set_call;
            let capture_only_error = format!(
                "property `{}` cannot be set because it is capture-only, but is not captured by the widget",
                p_name
            );
            property_set_calls.extend(quote_spanned! {*p_span=>
                #cfg
                #p_mod::code_gen!{
                    if capture_only =>  std::compile_error!{#capture_only_error}
                }
            })
        }
        for priority in &crate::property::Priority::all_settable() {
            #[cfg(debug_assertions)]
            for (p_mod, p_var_ident, p_name, source_loc, cfg, user_assigned, p_span, val_span) in &set_calls {
                // __set @ value span
                let set = ident_spanned!(*val_span=> "__set");
                property_set_calls.extend(quote_spanned! {*p_span=>
                    #cfg
                    #p_mod::code_gen! {
                        set #priority, #node__, #p_mod, #p_var_ident, #p_name, #source_loc, #user_assigned, #set
                    }
                });
            }
            #[cfg(not(debug_assertions))]
            for (p_mod, p_var_ident, _, cfg, p_span, val_span) in &set_calls {
                // __set @ value span
                let set = ident_spanned!(*val_span=> "__set");
                property_set_calls.extend(quote_spanned! {*p_span=>
                    #cfg
                    #p_mod::code_gen! {
                        set #priority, #node__, #p_mod, #p_var_ident, #set
                    }
                });
            }
        }
    }
    let property_set_calls = property_set_calls;

    // apply the whens for each property.
    for (property, assigns) in when_assigns {
        let property_path = match property.get_ident() {
            Some(maybe_inherited) if inherited_properties.contains(maybe_inherited) => {
                let p_ident = ident!("__p_{}", maybe_inherited);
                quote! { #module::#p_ident }
            }
            _ => property.to_token_stream(),
        };

        // collect assign items.
        let mut init_members = TokenStream::default();
        let mut conditions = Vec::with_capacity(assigns.len());
        for (w_cfg, condition_ident, value_ident, value) in assigns {
            if !util::token_stream_eq(value_ident.to_token_stream(), value.clone()) {
                init_members.extend(quote! {
                    #[allow(non_snake_case)]
                    #w_cfg
                    let #value_ident;
                    #w_cfg {
                        #value_ident = #value;
                    }
                });
            }
            conditions.push(quote! {
                #w_cfg
                #[allow(non_snake_case)]
                #condition_ident => #value_ident,
            });
        }
        // later conditions have priority.
        conditions.reverse();

        let (default, cfg) = wgt_properties
            .get(&property)
            .unwrap_or_else(|| non_user_error!("property(introduced in when?) not found"));
        when_inits.extend(quote! {
            #property_path::code_gen! { if allowed_in_when=>
                #cfg
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
        });
    }

    // Generate new function calls:
    let mut allow_unreachable = false;

    let new_child_caps: Vec<_> = widget_data
        .new_child
        .iter()
        .map(|p| {
            wgt_properties
                .get(&parse_quote! {#p})
                .map(|(id, _)| id.to_token_stream())
                .unwrap_or_else(|| {
                    allow_unreachable = true;
                    quote! { std::unreachable!() }
                })
        })
        .collect();

    let new_caps: Vec<_> = widget_data
        .new
        .iter()
        .map(|p| {
            wgt_properties
                .get(&parse_quote! {#p})
                .map(|(id, _)| id.to_token_stream())
                .unwrap_or_else(|| {
                    allow_unreachable = true;
                    quote! { std::unreachable!() }
                })
        })
        .collect();

    let allow_unreachable = if allow_unreachable {
        // we have filled-in missing captured with a panic, because of this we need
        // too suppress the `unreachable_code` lint. Missing captured already generated
        // a compile error so the panic cannot actually execute.
        quote! { #[allow(unreachable_code)] }
    } else {
        TokenStream::default()
    };

    #[cfg(debug_assertions)]
    let new_child_call = {
        let cap_user_set = widget_data.new_child.iter().map(|id| overriden_properties.contains(id));
        quote! {
            #[allow(unused_mut)]
            let mut captured_new_child__ = std::vec![];
            #allow_unreachable
            let node__ = #module::__new_child_debug(#(#new_child_caps,)* #(#cap_user_set,)* &mut captured_new_child__);
        }
    };
    #[cfg(not(debug_assertions))]
    let new_child_call = quote! {
        #allow_unreachable
        let node__ = #module::__new_child(#(#new_child_caps),*);
    };
    #[cfg(debug_assertions)]
    let new_call = {
        let cap_user_set = widget_data.new.iter().map(|id| overriden_properties.contains(id));
        quote! {
            #allow_unreachable
            #module::__new_debug(
                node__,
                #(#new_caps,)*
                #(#cap_user_set,)*
                captured_new_child__,
                when_infos__,
                #module::__core::source_location!()
            )
        }
    };
    #[cfg(not(debug_assertions))]
    let new_call = quote! {
        #allow_unreachable
        #module::__new(node__, #(#new_caps),*)
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
    call_site: Span,
    module: TokenStream,
    properties_child: Vec<BuiltProperty>,
    properties: Vec<BuiltProperty>,
    whens: Vec<BuiltWhen>,
    new_child: Vec<Ident>,
    new: Vec<Ident>,
}
impl Parse for WidgetData {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let call_site = input.span();
        let input = non_user_braced!(input, "widget");
        let r = Ok(Self {
            call_site,
            module: non_user_braced!(&input, "module").parse().unwrap(),
            properties_child: parse_all(&non_user_braced!(&input, "properties_child")).unwrap_or_else(|e| non_user_error!(e)),
            properties: parse_all(&non_user_braced!(&input, "properties")).unwrap_or_else(|e| non_user_error!(e)),
            whens: parse_all(&non_user_braced!(&input, "whens")).unwrap_or_else(|e| non_user_error!(e)),
            new_child: parse_all(&non_user_braced!(&input, "new_child")).unwrap_or_else(|e| non_user_error!(e)),
            new: parse_all(&non_user_braced!(&input, "new")).unwrap_or_else(|e| non_user_error!(e)),
        });

        r
    }
}

pub struct BuiltProperty {
    pub ident: Ident,
    pub docs: TokenStream,
    pub cfg: TokenStream,
    pub default: bool,
    pub required: bool,
}
impl Parse for BuiltProperty {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);

        let r = Ok(BuiltProperty {
            ident,
            docs: non_user_braced!(&input, "docs").parse().unwrap(),
            cfg: non_user_braced!(&input, "cfg").parse().unwrap(),
            default: non_user_braced!(&input, "default")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            required: non_user_braced!(&input, "required")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
        });
        r
    }
}

pub struct BuiltWhen {
    pub ident: Ident,
    #[cfg(debug_assertions)]
    pub dbg_ident: TokenStream,
    pub docs: TokenStream,
    pub cfg: TokenStream,
    pub inputs: Vec<Ident>,
    pub assigns: Vec<BuiltWhenAssign>,
    pub expr_str: syn::LitStr,
}
impl Parse for BuiltWhen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);

        let r = Ok(BuiltWhen {
            ident,
            #[cfg(debug_assertions)]
            dbg_ident: non_user_braced!(&input, "dbg_ident").parse().unwrap(),
            docs: non_user_braced!(&input, "docs").parse().unwrap(),
            cfg: non_user_braced!(&input, "cfg").parse().unwrap(),
            inputs: parse_all(&non_user_braced!(&input, "inputs")).unwrap_or_else(|e| non_user_error!(e)),
            assigns: parse_all(&non_user_braced!(&input, "assigns")).unwrap_or_else(|e| non_user_error!(e)),
            expr_str: non_user_braced!(&input, "expr_str").parse().unwrap_or_else(|e| non_user_error!(e)),
        });
        r
    }
}

pub struct BuiltWhenAssign {
    pub property: Ident,
    pub cfg: TokenStream,
    pub value_fn: Ident,
}
impl Parse for BuiltWhenAssign {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let property = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);
        let r = Ok(BuiltWhenAssign {
            property,
            cfg: non_user_braced!(&input, "cfg").parse().unwrap(),
            value_fn: non_user_braced!(&input, "value_fn").parse().unwrap_or_else(|e| non_user_error!(e)),
        });
        r
    }
}

/// The content of the widget macro call.
struct UserInput {
    errors: Errors,
    properties: Vec<PropertyAssign>,
    whens: Vec<When>,
}
impl Parse for UserInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = non_user_braced!(input, "user");

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
            } else if input.peek(Ident) || input.peek(Token![super]) || input.peek(Token![self]) {
                // peek ident or path.
                match input.parse::<PropertyAssign>() {
                    Ok(mut assign) => {
                        assign.attrs = attrs;
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

        Ok(UserInput { errors, properties, whens })
    }
}

/// Property assign in a widget instantiation or when block.
pub struct PropertyAssign {
    pub attrs: Vec<Attribute>,
    pub path: Path,
    pub eq: Token![=],
    pub value: PropertyValue,
    pub value_span: Span,
    pub semi: Option<Token![;]>,
}
impl Parse for PropertyAssign {
    /// Expects that outer attributes are already parsed and that ident, super or self was peeked.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<Path>()?;
        let path_is_ident = path.get_ident().is_some();

        // TODO don't allow shorthand in widget declaration.
        if path_is_ident && (input.is_empty() || input.peek(Token![;])) {
            // shorthand assign
            let semi = input.parse().unwrap_or_default();
            let value_span = path.span();
            let eq = parse_quote_spanned! {value_span=> = };
            let value = parse_quote! { #path };
            return Ok(PropertyAssign {
                attrs: vec![],
                path,
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

        let value_stream = util::parse_soft_group(
            input,
            // terminates in the first `;` in the current level.
            |input| input.parse::<Option<Token![;]>>().unwrap_or_default(),
            // next item is found after optional outer attributes.
            // then is an `ident =` OR a `when` OR a `property::path =`
            peek_next_assign,
        );

        let (value, value_span, semi) = PropertyValue::parse_soft_group(value_stream, eq.span)?;

        Ok(PropertyAssign {
            attrs: vec![],
            path,
            eq,
            value_span,
            value,
            semi,
        })
    }
}

/// Value [assigned](PropertyAssign) to a property.
#[derive(Debug)]
pub enum PropertyValue {
    /// `unset!` or `required!`.
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

        match self {
            PropertyValue::Unnamed(args) => {
                let args_impl = quote_spanned!(value_span=> __ArgsImpl::new);
                Ok(quote_spanned! {span=>
                    {
                        use #property_path::{ArgsImpl as __ArgsImpl};
                        #args_impl(#args)
                    }
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

    pub fn parse_soft_group(
        value_stream: Result<(TokenStream, Option<Token![;]>), TokenStream>,
        group_start_span: Span,
    ) -> syn::Result<(Self, Span, Option<Token![;]>)> {
        let value;
        let value_span;
        let semi;

        match value_stream {
            Ok((value_stream, s)) => {
                semi = s;
                if value_stream.is_empty() {
                    // no value tokens
                    let span = semi.as_ref().map(|s| s.span()).unwrap_or(group_start_span);
                    return Err(util::recoverable_err(span, "expected property value"));
                }
                value_span = value_stream.span();
                match syn::parse2::<PropertyValue>(value_stream) {
                    Ok(v) => {
                        value = v;
                        Ok((value, value_span, semi))
                    }
                    Err(e) => Err(e),
                }
            }
            Err(partial_value) => {
                if partial_value.is_empty() {
                    // no value tokens
                    Err(util::recoverable_err(group_start_span, "expected property value"))
                } else {
                    // maybe missing next argument (`,`) or terminator (`;`)
                    let last_tt = partial_value.into_iter().last().unwrap();
                    let last_span = last_tt.span();
                    let mut msg = "expected `,` or `;`";
                    if let proc_macro2::TokenTree::Punct(p) = last_tt {
                        if p.as_char() == ',' {
                            msg = "expected another property arg";
                        }
                    }
                    Err(util::recoverable_err(last_span, msg))
                }
            }
        }
    }
}
impl Parse for PropertyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) && input.peek2(Token![!]) {
            // input stream can be `unset!` with no third token.
            let unset = input.fork();
            let r = PropertyValue::Special(unset.parse().unwrap(), unset.parse().unwrap());
            if unset.is_empty() {
                input.advance_to(&unset);
                return Ok(r);
            }
        }

        fn map_unnamed_err(e: syn::Error) -> syn::Error {
            if e.to_string() == "expected `,`" {
                // We expect a `;` in here also, if there was one `input` would have terminated.
                syn::Error::new(e.span(), "expected `,` or `;`")
            } else {
                e
            }
        }

        if input.peek(syn::token::Brace) {
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

            if maybe_fields.is_empty() {
                // is only block in assign, still can be a block expression.
                if fields_input.peek(Ident) && (fields_input.peek2(Token![:]) || fields_input.peek2(Token![,])) {
                    // is named fields, { field: .. } or { field, .. }.
                    input.advance_to(&maybe_fields);
                    Ok(PropertyValue::Named(fields_brace, Punctuated::parse_terminated(&fields_input)?))
                } else {
                    // is an unnamed block expression or { field } that works as an expression.
                    Ok(PropertyValue::Unnamed(
                        Punctuated::parse_terminated(input).map_err(map_unnamed_err)?,
                    ))
                }
            } else {
                // first arg is a block expression but has other arg expression e.g: `{ <expr> }, ..`
                Ok(PropertyValue::Unnamed(
                    Punctuated::parse_terminated(input).map_err(map_unnamed_err)?,
                ))
            }
        } else {
            Ok(PropertyValue::Unnamed(
                Punctuated::parse_terminated(input).map_err(map_unnamed_err)?,
            ))
        }
    }
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
            let brace = syn::group::parse_braces(input).unwrap();
            let mut assigns = vec![];
            while !brace.content.is_empty() {
                match brace.content.parse() {
                    Ok(p) => assigns.push(p),
                    Err(e) => {
                        let (recoverable, e) = e.recoverable();
                        errors.push_syn(e);
                        if !recoverable {
                            break;
                        }
                    }
                }
            }
            (brace.token, assigns)
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
        ident_spanned!(span=> "__{}{}_{}", prefix, i, tokens_to_ident_str(&self.condition_expr.to_token_stream()))
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
                        ident_spanned!(member.span()=> "__{}", member)
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
