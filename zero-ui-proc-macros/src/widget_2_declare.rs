use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::Parse, Ident, LitBool};

use crate::{
    util::{self, parse_all, Errors},
    widget_0_attr::FnPriority,
    widget_new::{BuiltProperty, BuiltWhen, BuiltWhenAssign, PropertyCapture},
};

#[allow(unused_macros)]
macro_rules! quote {
    ($($tt:tt)*) => {
        compile_error!("don't use Span::call_site() in this file");

        // we don't use [`Span::call_site()`] here because some of the inherited data
        // span gets mixed with the call_site.
    };
}
#[allow(unused_macros)]
macro_rules! ident {
    ($($tt:tt)*) => {
        compile_error!("don't use Span::call_site() in this file");
        // see quote! above
    };
}

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Items { mut inherits, widget } = syn::parse(input).unwrap_or_else(|e| non_user_error!(e));
    //let enable_trace = widget.ident == "reset_wgt";
    let WidgetItem {
        call_site,
        module,
        ident,
        mixin,
        is_base,
        properties_remove,
        properties_declared,
        properties,
        whens,
        mut new_declarations,
        mut new_captures,
    } = widget;

    // same as args in widget_0_attr
    let uuid = util::uuid(&module.to_token_stream().into());

    macro_rules! quote {
        ($($tt:tt)*) => {
            quote::quote_spanned! {call_site.span=>
                $($tt)*
            }
        }
    }
    macro_rules! ident {
        ($($tt:tt)*) => {
            ident_spanned! {call_site.span=>
                $($tt)*
            }
        }
    }

    let properties_remove: HashSet<_> = properties_remove.into_iter().collect();
    let properties_declared: HashSet<_> = properties_declared.into_iter().collect();

    let mut errors = Errors::default();

    // validate inherits
    inherits.reverse();
    let mut invalid_inherits = false;
    if mixin {
        for inherit in inherits.iter() {
            if !inherit.mixin {
                errors.push(
                    format_args!(
                        "cannot inherit from `{}` because it is not a mix-in",
                        util::display_path(&inherit.inherit_use)
                    ),
                    util::path_span(&inherit.inherit_use),
                );
                invalid_inherits = true;
            }
        }
    } else if !is_base {
        debug_assert!(!inherits[0].mixin);

        let mut found_parent: Option<&InheritedItem> = None;
        for inherit in &inherits[1..] {
            if !inherit.mixin {
                if let Some(parent) = found_parent {
                    errors.push(
                        format_args!(
                            "cannot inherit from `{}` because is already inheriting from `{}`, can only inherit from a single full widget",
                            util::display_path(&inherit.inherit_use),
                            util::display_path(&parent.inherit_use)
                        ),
                        util::path_span(&inherit.inherit_use),
                    );
                    invalid_inherits = true;
                } else {
                    found_parent = Some(inherit);
                }
            }
        }
        if found_parent.is_some() {
            inherits.remove(0);
        }
    } else if !inherits.is_empty() {
        non_user_error!("inherit directive in base widget declaration");
    }
    if invalid_inherits {
        // returns early to avoid causing too many false positive errors.
        return errors.to_token_stream().into();
    }
    let inherits = inherits;

    // inherits new functions.
    let mut new_reexports = TokenStream::default();
    let mut inherited_new_sources = vec![];
    let mut inherited_caps = HashMap::new();
    if !mixin && !is_base {
        let parent = inherits
            .iter()
            .find(|i| !i.mixin)
            .unwrap_or_else(|| non_user_error!("expected a parent widget"));

        for (i, priority) in FnPriority::all().iter().enumerate() {
            if new_declarations[i].is_empty() {
                let source_mod = &parent.module;
                let new_ident = ident!("__{priority}");
                new_reexports.extend(quote! {
                    #[doc(hidden)]
                    pub use #source_mod::#new_ident;
                });
                {
                    let new_ident = ident!("__{priority}_inspect");
                    new_reexports.extend(quote! { #source_mod::__core::core_cfg_inspector! {
                        #[doc(hidden)]
                        pub use #source_mod::#new_ident;
                    }});
                }
                new_captures[i] = parent.new_captures[i].clone();
                for cap in &new_captures[i] {
                    inherited_caps.insert(cap.ident.clone(), *priority);
                }
                inherited_new_sources.push(Some(parent));
            } else {
                inherited_new_sources.push(None);
            }
        }

        // validate captures again, if there is an error here we assume is
        // because of inheritance, stage-0 already validates errors in the same widget.
        for (i, decl) in new_declarations.iter().enumerate() {
            if !decl.is_empty() {
                for p in &new_captures[i] {
                    if let Some(other_fn) = inherited_caps.get(&p.ident) {
                        errors.push(
                            format_args!("property `{}` is already captured in inherited fn `{other_fn}`", p.ident),
                            p.ident.span(),
                        );
                    }
                }
            }
        }
    }
    let new_captures = new_captures;

    // collect inherited properties. Late inherits of the same ident override early inherits.
    // [property_ident => inherit]
    let mut inherited_properties = HashMap::new();
    let mut inherited_props = vec![];
    for inherited in inherits.iter().rev() {
        for p in inherited.properties.iter().rev() {
            inherited_properties.entry(&p.ident).or_insert_with(|| {
                inherited_props.push(p);
                inherited
            });
        }
    }
    inherited_props.reverse();

    for (i, new_source) in inherited_new_sources.iter().enumerate() {
        if let Some(new_source) = new_source {
            for property in &new_captures[i] {
                if let Some(p) = inherited_properties.get_mut(&property.ident) {
                    if new_source.inherit_use != p.inherit_use {
                        errors.push(
                            format_args!(
                                "inherited property `{prop}` is captured in inherited fn `{fn_}` from `{fn_source}`, but the property is then overwritten in `{p_source}`\n\
                                a new `{fn_}` must be declared to resolve this conflict.",
                                prop = property.ident,
                                fn_ = FnPriority::all()[i],
                                fn_source = util::display_path(&new_source.inherit_use),
                                p_source = util::display_path(&p.inherit_use)
                            ),
                            util::path_span(&p.inherit_use)
                        );
                    }
                }
            }
        }
    }

    if invalid_inherits {
        // returns early to avoid causing too many false positive errors.
        return errors.to_token_stream().into();
    }

    // inherited properties that are required.
    let inherited_required: HashSet<_> = inherited_props.iter().filter(|p| p.required).map(|p| &p.ident).collect();

    // apply removes.
    for ident in &properties_remove {
        let cannot_remove_reason = if inherited_required.contains(ident) {
            Some("required".to_string())
        } else {
            new_captures
                .iter()
                .enumerate()
                .find_map(|(i, caps)| if caps.iter().any(|i| &i.ident == ident) { Some(i) } else { None })
                .map(|i| {
                    format!(
                        "captured in {}fn `{}`",
                        if inherited_new_sources[i].is_some() { "inherited " } else { "" },
                        FnPriority::all()[i]
                    )
                })
        };
        if let Some(reason) = cannot_remove_reason {
            // cannot remove
            errors.push(format_args!("cannot remove, property `{ident}` is {reason}"), ident.span());
        } else if inherited_properties.remove(ident).is_some() {
            // can remove
            if let Some(i) = inherited_props.iter().position(|p| &p.ident == ident) {
                inherited_props.remove(i);
            }
        } else {
            errors.push(format_args!("cannot remove, property `{ident}` is not inherited"), ident.span());
        }
    }

    // remove properties that are no longer captured.
    let captured_properties: HashSet<_> = new_captures.iter().flat_map(|c| c.iter().map(|c| &c.ident)).collect();
    for inherited in inherits.iter() {
        for cap in inherited.new_captures.iter().flatten() {
            if !captured_properties.contains(&cap.ident) {
                // if no longer captured
                if inherited_required.contains(&cap.ident) {
                    // but was explicitly marked required
                    errors.push(
                        format_args!(
                            "inherited widget `{}` requires property `{}` to be captured",
                            inherited.inherit_use.segments.last().map(|s| &s.ident).unwrap(),
                            &cap.ident
                        ),
                        util::path_span(&inherited.inherit_use),
                    );
                } else if inherited_properties.remove(&cap.ident).is_some() {
                    // remove property
                    if let Some(i) = inherited_props.iter().position(|p| p.ident == cap.ident) {
                        inherited_props.remove(i);
                    }
                }
            }
        }
    }

    let inherited_properties = inherited_properties;
    let inherited_props = inherited_props;

    // property docs info for inherited properties.
    let mut docs_required_inherited = vec![];
    let mut docs_normal_inherited = vec![];
    let mut docs_event_inherited = vec![];
    let mut docs_state_inherited = vec![];
    // final property docs info for properties, will be extended with
    // inherited after collecting newly declared properties, so that
    // the new properties show-up first in the docs page.
    let mut docs_required = vec![];
    let mut docs_normal = vec![];
    let mut docs_event = vec![];
    let mut docs_state = vec![];
    let mut docs_declared = vec![];
    let mut docs_whens = vec![]; // docs data about when blocks.

    // properties that are assigned (not in when blocks) or declared in the new widget.
    let wgt_used_properties: HashSet<_> = properties.iter().map(|p| &p.ident).collect();
    // properties data for widget macros.
    let mut wgt_properties = TokenStream::default();
    // property pub uses.
    let mut property_reexports = TokenStream::default();

    // when data for macros.
    let mut wgt_whens = TokenStream::default();

    // all properties used in when conditions and assigns.
    //
    // After property processing only properties not declared in the widget or inherited are left.
    //
    // the key is a property ident, the value is a cfg macro for use in-case the property is not
    // declared in the widget.
    let mut wgt_when_properties: HashMap<Ident, Option<Ident>> = HashMap::new();
    // Properties declared in the widget without default value and used in whens.
    let mut wgt_when_default_reqs: Vec<Ident> = vec![];

    // collect whens
    for bw in whens {
        let dbg_ident = &bw.dbg_ident;
        let BuiltWhen {
            ident,
            docs,
            cfg,
            inputs,
            assigns,
            expr_str,
            ..
        } = bw;

        docs_whens.push(WhenDocs {
            docs: docs.clone(),
            expr: expr_str.value(),
            affects: assigns.iter().map(|a| (a.property.clone(), a.cfg)).collect(),
        });

        let when_cfg = if cfg { Some(ident!("__cfg_{ident}")) } else { None };

        for property in inputs.iter().chain(assigns.iter().map(|a| &a.property)) {
            // set to the cfg of one of the whens that use it or `None` if any of the
            // whens that use don't have cfg.
            match wgt_when_properties.entry(property.clone()) {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(when_cfg.clone());
                }
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    if when_cfg.is_none() {
                        *e.get_mut() = None;
                    }
                }
            }
        }

        let mut assigns_tt = TokenStream::default();
        for BuiltWhenAssign { property, cfg, value_fn } in assigns {
            assigns_tt.extend(quote! {
                #property { cfg { #cfg } value_fn { #value_fn } }
            });
        }

        let dbg_ident = quote! {
            dbg_ident { #dbg_ident }
        };

        wgt_whens.extend(quote! {
            #ident {
                #dbg_ident
                docs { #docs }
                cfg { #cfg }
                inputs { #(#inputs)* }
                assigns {#assigns_tt }
                expr_str { #expr_str }
            }
        });
    }

    // collect inherited re-exports and property data for macros.
    for ip in &inherited_props {
        if wgt_used_properties.contains(&ip.ident) {
            // property was re-assigned in the widget, we will deal with then later.
            continue;
        }

        let &BuiltProperty {
            ident,
            docs,
            cfg,
            mut default,
            mut required,
        } = ip;

        required |= inherited_required.contains(ident);

        if let Some((k, _)) = wgt_when_properties.remove_entry(ident) {
            if !default && !required {
                // mark property to include default value, or compile error.
                wgt_when_default_reqs.push(k);
                default = true;
            }
        }

        // collect property documentation info.
        let docs_info = if required {
            &mut docs_required_inherited
        } else if ident.to_string().starts_with("is_") {
            &mut docs_state_inherited
        } else if ident.to_string().starts_with("on_") {
            &mut docs_event_inherited
        } else {
            &mut docs_normal_inherited
        };
        docs_info.push(PropertyDocs {
            ident: ident.to_string(),
            docs: docs.clone(),
            doc_hidden: util::is_doc_hidden_tt(docs.clone()),
            inherited: true,
            path: {
                let i = &inherited_properties[ident];
                i.inherit_use.to_token_stream()
            },
            assigned_by_wgt: default,
        });

        // collect property data for macros.
        wgt_properties.extend(quote! {
            #ident {
                docs { #docs }
                cfg { #cfg }
                default { #default }
                required { #required }
            }
        });

        // generate re-export.
        let path = &inherited_properties[&ip.ident].module;
        let p_ident = ident!("__p_{}", ip.ident);

        if *cfg {
            let cfg_ident = ident!("__cfg_{}", ip.ident);

            property_reexports.extend(quote! {
                #path::#cfg_ident! {
                    #[doc(hidden)]
                    pub use #path::{#p_ident, #cfg_ident};
                }
            });
        } else {
            property_reexports.extend(quote! {
                #[doc(hidden)]
                pub use #path::#p_ident;
            });
        }

        // generate values re-export.
        if ip.default {
            // default value.
            let d_ident = ident!("__d_{}", ip.ident);
            // source location reexport.
            let loc_ident = ident!("__loc_{}", ip.ident);

            let exp = quote! {
                #[doc(hidden)]
                pub use #path::#d_ident;

                #path::__core::core_cfg_inspector! {
                    #[doc(hidden)]
                    pub use #path::#loc_ident;
                }
            };

            if *cfg {
                let cfg_ident = ident!("__cfg_{}", ip.ident);

                property_reexports.extend(quote! {
                    #path::#cfg_ident! {
                        #exp
                    }
                });
            } else {
                property_reexports.extend(exp);
            }
        }
    }
    // collect property re-exports and data for macros.
    for p in &properties {
        let PropertyItem {
            ident,
            docs,
            cfg,
            mut default,
            required,
            declared,
            ..
        } = p;

        let required = *required || inherited_required.contains(ident);

        if wgt_when_properties.remove(ident).is_some() && !default && !required {
            wgt_when_default_reqs.push(ident.clone());
            default = true;
        }

        // collect property documentation info.
        let ident_str = ident.to_string();
        let docs_info = if required {
            &mut docs_required
        } else if ident_str.starts_with("is_") {
            &mut docs_state
        } else if ident_str.starts_with("on_") {
            &mut docs_event
        } else {
            &mut docs_normal
        };

        let inherited_p = match (&p.ident, &p.kind()) {
            (id, PropertyItemKind::Ident) | (_, PropertyItemKind::AliasedIdent(id)) => {
                inherited_properties.get(id).map(|i| i.module.clone())
            }
            (_, PropertyItemKind::Path) => None,
        };
        docs_info.push(PropertyDocs {
            ident: ident_str,
            docs: docs.clone(),
            doc_hidden: util::is_doc_hidden_tt(docs.clone()),
            inherited: inherited_p.is_some(),
            path: inherited_p.unwrap_or_else(|| p.path.clone()),
            assigned_by_wgt: default,
        });

        if *declared {
            docs_declared.push(ident!("__p_{ident}"));
        }

        // collect property data for macros.
        wgt_properties.extend(quote! {
            #ident {
                docs { #docs }
                cfg { #cfg }
                default { #default }
                required { #required }
            }
        });

        if properties_declared.contains(&p.ident) {
            // new capture_only property already is public in the `self` module.
            continue;
        }

        // re-export property
        let path = &p.path;
        let p_ident = ident!("__p_{}", p.ident);

        match p.kind() {
            PropertyItemKind::Ident => {
                if let Some(inherited) = inherited_properties.get(&p.ident) {
                    let inherited_source = &inherited.module;
                    // re-export inherited property.

                    if *cfg {
                        let cfg_ident = ident!("__cfg_{}", p.ident);
                        property_reexports.extend(quote! {
                            #inherited_source::#cfg_ident! {
                                #[doc(hidden)]
                                pub use #inherited_source::{#p_ident, #cfg_ident};
                            }
                        });
                    } else {
                        property_reexports.extend(quote! {
                            #[doc(hidden)]
                            pub use #inherited_source::#p_ident;
                        });
                    }
                    continue;
                }
            }
            PropertyItemKind::AliasedIdent(maybe_inherited) => {
                if let Some(inherited) = inherited_properties.get(&maybe_inherited) {
                    let inherited_source = &inherited.module;
                    // re-export inherited property as a new name.
                    let inherited_ident = ident!("__p_{maybe_inherited}");

                    if *cfg {
                        let cfg_ident = ident!("__cfg_{maybe_inherited}");
                        let new_cfg_ident = ident!("__cfg_{}", p.ident);
                        property_reexports.extend(quote! {
                            #inherited_source::#cfg_ident! {
                                #[doc(hidden)]
                                pub use #inherited_source::{#inherited_ident as #p_ident, #cfg_ident as #new_cfg_ident};
                            }

                        });
                    } else {
                        property_reexports.extend(quote! {
                            #[doc(hidden)]
                            pub use #inherited_source::#inherited_ident as #p_ident;
                        });
                    }
                    continue;
                }
            }
            PropertyItemKind::Path => {}
        }
        // not inherited.

        if *cfg {
            let cfg_ident = ident!("__cfg_{}", p.ident);
            property_reexports.extend(quote! {
                self::#cfg_ident! {
                    #[doc(hidden)]
                    pub use #path::export as #p_ident;
                }
            });
        } else {
            property_reexports.extend(quote! {
                #[doc(hidden)]
                pub use #path::export as #p_ident;
            });
        }
    }
    let property_reexports = property_reexports;
    let wgt_properties = wgt_properties;

    docs_required.extend(docs_required_inherited);
    docs_normal.extend(docs_normal_inherited);
    docs_event.extend(docs_event_inherited);
    docs_state.extend(docs_state_inherited);

    // inherited whens pub uses.
    let mut when_reexports = TokenStream::default();

    for inherited in &inherits {
        //inherited.module
        for bw in &inherited.whens {
            let dbg_ident = &bw.dbg_ident;
            let BuiltWhen {
                ident,
                docs,
                cfg,
                inputs,
                assigns,
                expr_str,
                ..
            } = bw;

            let module = &inherited.module;
            let module_id_str = util::tokens_to_ident_str(module);
            let new_ident = ident!("__{module_id_str}{ident}");

            let new_dbg_ident = ident!("__{module_id_str}{dbg_ident}");

            let mut assigns_tt = TokenStream::default();
            let mut defaults_tt = TokenStream::default();
            for BuiltWhenAssign { property, cfg, value_fn } in assigns {
                if properties_remove.contains(property) {
                    continue; // inherited was removed.
                }

                docs_whens.push(WhenDocs {
                    docs: docs.clone(),
                    expr: expr_str.value(),
                    affects: assigns.iter().map(|a| (a.property.clone(), a.cfg)).collect(),
                });

                let new_value_fn = ident!("__{module_id_str}{value_fn}");

                assigns_tt.extend(quote! {
                    #property { cfg { #cfg } value_fn { #new_value_fn } }
                });

                if *cfg {
                    let cfg_ident = ident!("__cfg_{ident}_{property}");
                    defaults_tt.extend(quote! {
                        #module::#cfg_ident! {
                            #[doc(hidden)]
                            pub use #module::#value_fn as #new_value_fn;
                        }
                    });
                } else {
                    defaults_tt.extend(quote! {
                        #[doc(hidden)]
                        pub use #module::#value_fn as #new_value_fn;
                    });
                }
            }
            if assigns_tt.is_empty() {
                continue; // all properties removed, remove when block.
            }

            let dbg_ident_value = quote! {
                dbg_ident { #new_dbg_ident }
            };

            wgt_whens.extend(quote! {
                #new_ident {
                    #dbg_ident_value
                    docs { #docs }
                    cfg { #cfg }
                    inputs { #(#inputs)* }
                    assigns { #assigns_tt }
                    expr_str { #expr_str }
                }
            });

            let rexp = quote! {
                #[doc(hidden)]
                pub use #module::#ident as #new_ident;
                #defaults_tt

                #module::__core::core_cfg_inspector! {
                    #[doc(hidden)]
                    pub use #module::#dbg_ident as #new_dbg_ident;
                }
            };

            if *cfg {
                let cfg_ident = ident!("__cfg_{ident}");
                let new_cfg_ident = ident!("__cfg_{new_ident}");

                when_reexports.extend(quote! {
                    #module::#cfg_ident! {
                        #rexp

                        #[doc(hidden)]
                        pub use #module::#cfg_ident as #new_cfg_ident;
                    }
                });
            } else {
                when_reexports.extend(rexp);
            }
        }
    }

    // all widget properties with and without values (excluding new when properties).
    let wgt_all_properties: HashSet<&Ident> = inherited_props
        .iter()
        .map(|p| &p.ident)
        .chain(properties.iter().map(|p| &p.ident))
        .collect();

    // validate captures exist.
    for (decl, caps) in new_declarations.iter_mut().zip(&new_captures) {
        if decl.is_empty() {
            continue;
        }
        let mut invalid = false;
        for capture in caps {
            if !wgt_all_properties.contains(&capture.ident) {
                errors.push(
                    format_args!("property `{}` is not inherited nor declared by the widget", capture.ident),
                    capture.ident.span(),
                );
                invalid = true;
            }
        }
        if invalid {
            *decl = TokenStream::default()
        }
    }

    // assert that properties not captured are not capture-only.
    let mut assert_not_captures = TokenStream::new();
    if mixin {
        for p in &properties {
            let msg = format!(
                "property `{}` is capture-only, only normal properties are allowed in mix-ins",
                p.ident
            );
            let p_mod = ident!("__p_{}", p.ident);

            let assert = quote_spanned!(p.ident.span()=>
                self::#p_mod::code_gen! {
                    if capture_only=> std::compile_error!{#msg}
                }
            );

            if p.cfg {
                let cfg_ident = ident!("__cfg_{}", p.ident);
                assert_not_captures.extend(quote! {
                    self::#cfg_ident! {
                        #assert
                    }
                });
            } else {
                assert_not_captures.extend(assert);
            }
        }
    } else {
        for p in &properties {
            if captured_properties.contains(&p.ident) {
                continue;
            }

            let msg = format!("property `{}` is capture-only, but is not captured by the widget", p.ident);
            let p_mod = ident!("__p_{}", p.ident);

            let assert = quote_spanned!(p.ident.span()=>
                self::#p_mod::code_gen! {
                    if capture_only=> std::compile_error!{#msg}
                }
            );
            if p.cfg {
                let cfg_ident = ident!("__cfg_{}", p.ident);
                assert_not_captures.extend(quote! {
                    self::#cfg_ident! {
                        #assert
                    }
                });
            } else {
                assert_not_captures.extend(assert);
            }
        }
    }
    let assert_not_captures = assert_not_captures;

    // properties that are only introduced in when conditions.
    // reexported if they have default values.
    let mut when_condition_default_props = TokenStream::default();

    let mut wgt_properties = wgt_properties;
    let crate_core = util::crate_core();

    for (w_prop, cfg) in wgt_when_properties {
        // property not declared in the widget first, or declared with no default, validate that it has a default value.

        let inherited = inherited_properties.get(&w_prop).map(|i| i.inherit_use.to_token_stream());

        // docs
        let w_prop_str = w_prop.to_string();
        let docs = if w_prop_str.starts_with("is_") {
            &mut docs_state
        } else if w_prop_str.starts_with("on_") {
            &mut docs_event
        } else {
            &mut docs_normal
        };
        docs.push(PropertyDocs {
            ident: w_prop_str,
            docs: TokenStream::default(),
            doc_hidden: false,
            inherited: inherited.is_some(),
            path: inherited.unwrap_or_else(|| w_prop.to_token_stream()),
            assigned_by_wgt: true,
        });

        let p_ident = ident!("__p_{w_prop}");
        let d_ident = ident!("__d_{w_prop}");

        // reexport property.

        when_condition_default_props.extend(quote! {
            #w_prop::code_gen! {
                if default=>

                #[doc(hidden)]
                pub use #w_prop::export as #p_ident;
            }
        });

        let loc_ident = ident!("__loc_{w_prop}");
        when_condition_default_props.extend(quote_spanned! {p_ident.span()=> #crate_core::core_cfg_inspector! {
            #w_prop::code_gen! {
                if default=>

                #[doc(hidden)]
                pub fn #loc_ident() -> #crate_core::inspector::SourceLocation {
                    #crate_core::inspector::source_location!()
                }
            }
        }});

        if let Some(cfg) = &cfg {
            let cfg_ident = ident!("__cfg_{w_prop}");
            when_condition_default_props.extend(quote! {
                #[doc(hidden)]
                pub use self::#cfg as #cfg_ident;
            });
        }

        let has_cfg = cfg.is_some();
        wgt_properties.extend(quote! {
            #w_prop {
                docs { }
                cfg { #has_cfg }
                default { true }
                required { false }
            }
        });

        // default value.
        when_condition_default_props.extend(quote! {
            #w_prop::code_gen! {
                if default=>

                #[doc(hidden)]
                pub fn #d_ident() -> impl self::#p_ident::Args {
                    self::#p_ident::default_args()
                }
            }
        });
        // OR compile error because the property has no default value.
        let msg = format!("property `{w_prop}` has no default value");
        when_condition_default_props.extend(quote_spanned! {w_prop.span()=>
            #w_prop::code_gen! {
                if !default=>

                std::compile_error! { #msg }
            }
        });
    }

    for w_prop in wgt_when_default_reqs {
        let p_ident = ident!("__p_{w_prop}");
        let d_ident = ident!("__d_{w_prop}");

        // default value.
        when_condition_default_props.extend(quote! {
            #p_ident::code_gen! {
                if default=>

                #[doc(hidden)]
                pub fn #d_ident() -> impl self::#p_ident::Args {
                    self::#p_ident::default_args()
                }
            }
        });
        // OR compile error because the property has no default value.
        let msg = format!("property `{w_prop}` has no default value");
        when_condition_default_props.extend(quote_spanned! {w_prop.span()=>
            #p_ident::code_gen! {
                if !default=>

                std::compile_error! { #msg }
            }
        });

        // location only added with default value.
        let loc_ident = ident!("__loc_{w_prop}");
        when_condition_default_props.extend(quote_spanned! {w_prop.span()=> #crate_core::core_cfg_inspector! {
            #p_ident::code_gen! {
                if default=>

                #[doc(hidden)]
                pub fn #loc_ident() -> #crate_core::inspector::SourceLocation {
                    #crate_core::inspector::source_location!()
                }
            }
        }});
    }

    let new_idents = FnPriority::all().iter().map(|p| ident!("{p}"));

    let new_captures_idents = new_captures.iter().map(|c| c.iter().map(|c| &c.ident).collect::<Vec<_>>());
    let new_captures_cfg = new_captures.iter().map(|c| c.iter().map(|c| &c.cfg).collect::<Vec<_>>());

    let built_data = quote! {
        module { #module }
        properties {
            #wgt_properties
        }
        whens {
            #wgt_whens
        }
        new_captures {
            #(
                #new_idents {
                    #(
                        #new_captures_idents {
                            cfg { #new_captures_cfg }
                        }
                    )*
                }
            )*
        }
    };

    let inherit_macro = quote! {
        (
            inherit=>
            cfg { $(#[$cfg:meta])? }
            not_cfg { #[$not_cfg:meta] }
            inherit_use { $inherit_use:path }
            inherit { $(
                $(#[$inh_cfg:meta])?
                $inherit:path
            )* }
            $($rest:tt)+
        ) => {
            $(#[$cfg])?
            #module::__core::widget_inherit! {
                inherit {
                    $(
                        $(#[$inh_cfg])?
                        $inherit
                    )*
                }
                inherited {
                    inherit_use { $inherit_use }
                    mixin { #mixin }

                    #built_data
                }
                $($rest)*
            }
            #[$not_cfg]
            #module::__core::widget_inherit! {
                inherit {
                    $(
                        $(#[$inh_cfg])?
                        $inherit
                    )*
                }
                $($rest)*
            }
        };
    };
    let new_macro = if mixin {
        quote! {
            ($($invalid:tt)*) => {
                std::compile_error!{"cannot instantiate widget mix-ins"}
            };
        }
    } else {
        quote! {
            ($($tt:tt)*) => {
                #module::__core::widget_new! {
                    widget {
                        #built_data
                    }
                    user {
                        $($tt)*
                    }
                }
            };
        }
    };

    let auto_docs = auto_docs(docs_required, docs_normal, docs_event, docs_state, docs_whens);

    let macro_ident = ident!("__{ident}_{}", uuid);

    let export_macro = if errors.is_empty() {
        quote! {
            #[doc(hidden)]
            pub use #macro_ident as __widget_macro;
        }
    } else {
        // in case there is an attempt to instantiate a widget
        // that is not compiling.
        TokenStream::new()
    };

    let r = quote! {
        #errors
        #assert_not_captures

        #(#new_declarations)*

        #property_reexports
        #when_reexports

        #new_reexports

        #when_condition_default_props

        #[doc(hidden)]
        #[macro_export]
        macro_rules! #macro_ident {
            #inherit_macro
            #new_macro
        }
        #export_macro

        /// Documentation of properties used or declared in the widget.
        ///
        /// This *const* is only visible if `zero-ui-docs` was not used to generate the docs.
        ///
        #auto_docs
        ///
        /// <script>
        /// var zero_ui_inner_docs = document.currentScript;
        /// document.addEventListener('DOMContentLoaded', function() {
        ///     let message = {
        ///         inner_docs: zero_ui_inner_docs.parentElement.innerHTML,
        ///     };
        ///     window.parent.postMessage(message, "*")
        /// });
        /// </script>
        pub mod __DOCS {
            #(
                /// Declared in widget.
                pub use super::#docs_declared::wgt_cap_export as #docs_declared;
            )*
        }
    };

    r.into()
}

struct PropertyDocs {
    doc_hidden: bool,
    docs: TokenStream,

    ident: String,
    /// Path to property or parent widget if `inherited`.
    path: TokenStream,
    inherited: bool,

    assigned_by_wgt: bool,
}
struct WhenDocs {
    docs: TokenStream,
    expr: String,
    // [(assigned_property, cfg)]
    affects: Vec<(Ident, bool)>,
}
fn auto_docs(
    required: Vec<PropertyDocs>,
    normal: Vec<PropertyDocs>,
    event: Vec<PropertyDocs>,
    state: Vec<PropertyDocs>,
    whens: Vec<WhenDocs>,
) -> TokenStream {
    #[allow(unused)]
    use util::is_doc_hidden;
    let mut r = TokenStream::default();

    docs_section(
        &mut r,
        required,
        "Required Properties\n\nProperties that must be set for the widget to compile.",
    );
    docs_section(&mut r, normal, "Normal Properties\n\nProperties that can be set without importing.");
    docs_section(
        &mut r,
        event,
        "Event Properties\n\nEvent properties that can set without importing.",
    );
    docs_section(
        &mut r,
        state,
        "State Properties\n\nProperties that can be used in when conditions without importing.",
    );

    if !whens.is_empty() {
        doc_extend!(
            r,
            "# When Conditions\n\nWhen conditions set by default, more expressions can be set during instantiation.\n\n<div style='overflow-x: visible;'>\n\n"
        );
        for w in whens {
            doc_extend!(r, "* **`when {}`**\n\n", w.expr);

            doc_extend!(r, "\n\n<div class='docblock'>\n\n");

            r.extend(w.docs);

            let mut comma = "";
            let mut affects = String::new();
            for (p, cfg) in w.affects {
                use std::fmt::Write;

                if !cfg {
                    let _ = write!(&mut affects, "{comma}[`{0}`](#wp-{0})", p);
                } else {
                    let _ = write!(&mut affects, "{comma}[`{0}`](#wp-{0} \"conditional\")", p,);
                }
                comma = ", ";
            }
            doc_extend!(r, "\n\n**Affects:** {affects}.");

            doc_extend!(r, "\n\n</div>\n\n");
        }
        doc_extend!(r, "\n\n</div>\n\n");
    }

    r
}
fn docs_section(r: &mut TokenStream, properties: Vec<PropertyDocs>, name: &str) {
    let mut properties: Vec<_> = properties.into_iter().filter(|p| !p.doc_hidden).collect();
    if properties.is_empty() {
        return;
    }

    properties.sort_by(|a, b| a.ident.cmp(&b.ident));

    doc_extend!(r, "# {}\n\n<div style='overflow-x: visible;'>\n\n", name);
    for p in properties {
        let path = p.path.to_string().replace(' ', "").replace('$', "");
        if p.inherited {
            doc_extend!(
                r,
                "* <span id='wp-{0}' class='wp-title'>**[`{0}`](mod@{path}#wp-{0})**</span>\n\n",
                p.ident,
            );
        } else {
            doc_extend!(r, "* <span id='wp-{0}' class='wp-title'>**[`{0}`](fn@{path})**</span>\n\n", p.ident);
        }

        doc_extend!(r, "<div class='docblock'>\n\n");

        if p.docs.is_empty() {
            doc_extend!(r, "<span data-fetch-docs='{}'></span>", p.ident);
        } else {
            r.extend(p.docs);
        }

        if p.assigned_by_wgt {
            doc_extend!(r, "\n*Set by default.*");
        }
        doc_extend!(r, "\n\n</div>");

        doc_extend!(r, "\n\n");
    }
    doc_extend!(r, "\n\n</div>\n\n");
}

struct Items {
    inherits: Vec<InheritedItem>,
    widget: WidgetItem,
}
impl Parse for Items {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut inherits = vec![];

        while !input.is_empty() {
            if input.peek(keyword::inherited) {
                inherits.push(non_user_braced!(input, "inherited").parse().unwrap_or_else(|e| non_user_error!(e)))
            } else if input.peek(keyword::widget) {
                let widget = non_user_braced!(input, "widget").parse().unwrap_or_else(|e| non_user_error!(e));

                if !input.is_empty() {
                    non_user_error!("expected `widget { .. }` to be the last item");
                }
                return Ok(Items { inherits, widget });
            } else {
                non_user_error!("expected `inherited { .. }` or `widget { .. }`")
            }
        }
        unreachable!("expected last item to be `new {{ .. }}`")
    }
}

/// Inherited widget or mixin data.
struct InheritedItem {
    inherit_use: syn::Path,
    mixin: bool,
    module: TokenStream,
    properties: Vec<BuiltProperty>,
    whens: Vec<BuiltWhen>,
    new_captures: Vec<Vec<PropertyCapture>>,
}
impl Parse for InheritedItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(InheritedItem {
            inherit_use: non_user_braced!(input, "inherit_use")
                .parse()
                .unwrap_or_else(|e| non_user_error!(e)),
            mixin: non_user_braced!(input, "mixin")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            module: non_user_braced!(input, "module").parse().unwrap(),
            properties: parse_all(&non_user_braced!(input, "properties")).unwrap_or_else(|e| non_user_error!(e)),
            whens: parse_all(&non_user_braced!(input, "whens")).unwrap_or_else(|e| non_user_error!(e)),
            new_captures: {
                let input = non_user_braced!(input, "new_captures");
                FnPriority::all()
                    .iter()
                    .map(|p| parse_all(&non_user_braced!(&input, p.to_string())).unwrap_or_else(|e| non_user_error!(e)))
                    .collect()
            },
        })
    }
}

/// New widget or mixin.
struct WidgetItem {
    call_site: keyword::call_site,
    module: TokenStream,
    ident: Ident,
    mixin: bool,
    is_base: bool,

    properties_remove: Vec<Ident>,
    properties_declared: Vec<Ident>,

    properties: Vec<PropertyItem>,
    whens: Vec<BuiltWhen>,

    new_declarations: Vec<TokenStream>,
    new_captures: Vec<Vec<PropertyCapture>>,
}
impl Parse for WidgetItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        macro_rules! named_braces {
            ($name:expr) => {
                non_user_braced!(input, $name)
            };
        }
        Ok(WidgetItem {
            call_site: input.parse::<keyword::call_site>().unwrap_or_else(|e| non_user_error!(e)),
            module: named_braces!("module").parse().unwrap(),
            ident: named_braces!("ident").parse().unwrap_or_else(|e| non_user_error!(e)),
            mixin: named_braces!("mixin")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            is_base: named_braces!("is_base")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,

            properties_remove: parse_all(&named_braces!("properties_remove")).unwrap_or_else(|e| non_user_error!(e)),
            properties_declared: parse_all(&named_braces!("properties_declared")).unwrap_or_else(|e| non_user_error!(e)),

            properties: parse_all(&named_braces!("properties")).unwrap_or_else(|e| non_user_error!(e)),
            whens: parse_all(&named_braces!("whens")).unwrap_or_else(|e| non_user_error!(e)),

            new_declarations: {
                let input = named_braces!("new_declarations");
                FnPriority::all()
                    .iter()
                    .map(|p| non_user_braced!(&input, p.to_string()).parse().unwrap())
                    .collect()
            },
            new_captures: {
                let input = named_braces!("new_captures");
                FnPriority::all()
                    .iter()
                    .map(|p| parse_all(&non_user_braced!(&input, p.to_string())).unwrap_or_else(|e| non_user_error!(e)))
                    .collect()
            },
        })
    }
}

/// A property declaration
struct PropertyItem {
    ident: Ident,
    docs: TokenStream,
    cfg: bool,
    path: TokenStream,
    default: bool,
    required: bool,
    declared: bool,
}
impl Parse for PropertyItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);
        macro_rules! named_braces {
            ($name:tt) => {
                non_user_braced!(&input, $name)
            };
        }
        let property_item = PropertyItem {
            ident,
            docs: named_braces!("docs").parse().unwrap(),
            cfg: named_braces!("cfg").parse::<LitBool>().unwrap_or_else(|e| non_user_error!(e)).value,
            path: named_braces!("path").parse().unwrap(),
            default: named_braces!("default")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            required: named_braces!("required")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            declared: named_braces!("declared")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
        };

        Ok(property_item)
    }
}
impl PropertyItem {
    /// Gets the kind of property reference.
    pub fn kind(&self) -> PropertyItemKind {
        if let Ok(ident) = syn::parse2::<Ident>(self.path.clone()) {
            if self.ident == ident {
                PropertyItemKind::Ident
            } else {
                PropertyItemKind::AliasedIdent(ident)
            }
        } else {
            PropertyItemKind::Path
        }
    }
}
/// Kind of property reference in [`PropertyItem`]
pub enum PropertyItemKind {
    /// Single property ident, maybe inherited.
    Ident,
    /// Single property ident as another ident, maybe inherited.
    AliasedIdent(Ident),
    /// Cannot be inherited, maybe aliased.
    Path,
}

mod keyword {
    syn::custom_keyword!(inherited);
    syn::custom_keyword!(widget);
    syn::custom_keyword!(call_site);
}
