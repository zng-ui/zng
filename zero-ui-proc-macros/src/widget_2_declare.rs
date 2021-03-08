use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use regex::Regex;
use syn::{parse::Parse, spanned::Spanned, Ident, LitBool};

use crate::{
    util::{self, parse_all, Errors},
    widget_new2::{BuiltProperty, BuiltWhen, BuiltWhenAssign},
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Items { inherits, widget } = syn::parse(input).unwrap_or_else(|e| non_user_error!(e));
    //let enable_trace = widget.ident == "reset_wgt";
    let WidgetItem {
        module,
        attrs,
        cfg,
        vis,
        ident,
        mixin,
        properties_unset,
        properties_declared,
        properties_child,
        properties,
        whens,
        new_child_declared,
        mut new_child,
        new_declared,
        mut new,
        mod_items,
    } = widget;
    let properties_unset: HashMap<_, _> = properties_unset.into_iter().map(|u| (u.property, u.unset.span())).collect();
    let properties_declared: HashSet<_> = properties_declared.into_iter().collect();

    let crate_core = util::crate_core();
    let mut errors = Errors::default();

    // inherits `new_child` and `new`.
    let mut new_child_reexport = TokenStream::default();
    let mut new_reexport = TokenStream::default();
    if !mixin {
        let last_not_mixin = inherits.iter().filter(|i| !i.mixin).last();
        if !new_child_declared {
            if let Some(source) = last_not_mixin {
                let source_mod = &source.module;
                new_child_reexport = quote! {
                    #[doc(hidden)]
                    pub use #source_mod::__new_child;
                };
                new_child = source.new_child.clone();
            } else {
                // zero_ui::core::widget_base::default_widget_new_child()
                new_child_reexport = quote! {
                    #[doc(hidden)]
                    #[inline]
                    pub fn __new_child() -> impl #crate_core::UiNode {
                        #crate_core::widget_base::default_widget_new_child()
                    }
                };
                assert!(new_child.is_empty());
            }
        }
        if !new_declared {
            if let Some(source) = last_not_mixin {
                let source_mod = &source.module;
                new_reexport = quote! {
                    #[doc(hidden)]
                    pub use #source_mod::__new;
                };
                new = source.new.clone();
            } else {
                // zero_ui::core::widget_base::default_widget_new(id)
                new_reexport = quote! {
                    #[doc(hidden)]
                    #[inline]
                    pub fn __new(child: impl #crate_core::UiNode, id: impl self::__p_id::Args) -> impl #crate_core::Widget {
                        // TODO remove the "2" when we convert all to the new macro.
                        #crate_core::widget_base::default_widget_new2(child, self::__p_id::Args::unwrap(id))
                    }
                };
                new = vec![ident!("id")];
            }
        }
    }
    let new_child = new_child;
    let new = new;

    // collect inherited properties. Late inherits of the same ident overrides early inherits.
    let mut inherited_properties = HashMap::new();
    let mut inherited_props_child = vec![];
    let mut inherited_props = vec![];
    for inherited in inherits.iter().rev() {
        for p_child in inherited.properties_child.iter().rev() {
            if inherited_properties.insert(&p_child.ident, &inherited.module).is_none() {
                inherited_props_child.push(p_child);
            }
        }
        for p in inherited.properties.iter().rev() {
            if inherited_properties.insert(&p.ident, &inherited.module).is_none() {
                inherited_props.push(p);
            }
        }
    }
    inherited_props_child.reverse();
    inherited_props.reverse();

    // inherited properties that are required!
    let inherited_required: HashSet<_> = inherited_props_child
        .iter()
        .chain(inherited_props.iter())
        .filter(|p| p.required)
        .map(|p| &p.ident)
        .collect();

    // apply unsets.
    for (unset, &unset_span) in &properties_unset {
        if inherited_required.contains(unset) {
            // cannot unset
            errors.push(format_args!("property `{}` is required", unset), unset_span);
        } else if inherited_properties.remove(unset).is_some() {
            // can unset
            if let Some(i) = inherited_props_child.iter().position(|p| &p.ident == unset) {
                inherited_props_child.remove(i);
            } else if let Some(i) = inherited_props.iter().position(|p| &p.ident == unset) {
                inherited_props.remove(i);
            }
        }
        // else was unset in a new property that must be a warning when that is stable
    }

    let inherited_properties = inherited_properties;
    let inherited_props_child = inherited_props_child;
    let inherited_props = inherited_props;

    // property docs info for inherited properties.
    let mut docs_required_inherited = vec![];
    let mut docs_default_inherited = vec![];
    let mut docs_state_inherited = vec![];
    let mut docs_other_inherited = vec![];
    // final property docs info for properties, will be extended with
    // inherited after collecting newly declared properties, so that
    // the new properties show-up first in the docs page.
    let mut docs_required = vec![];
    let mut docs_default = vec![];
    let mut docs_state = vec![];
    let mut docs_other = vec![];

    // properties that are assigned (not in when blocks) or declared in the new widget.
    let wgt_used_properties: HashSet<_> = properties_child.iter().chain(properties.iter()).map(|p| &p.ident).collect();
    // properties data for widget macros.
    let mut wgt_properties_child = TokenStream::default();
    let mut wgt_properties = TokenStream::default();
    // property pub uses.
    let mut property_reexports = TokenStream::default();

    // collect inherited re-exports and property data for macros.
    for (ip, is_child) in inherited_props_child
        .iter()
        .map(|ip| (ip, true))
        .chain(inherited_props.iter().map(|ip| (ip, false)))
    {
        if wgt_used_properties.contains(&ip.ident) {
            // property was re-assigned in the widget, we will deal with then later.
            continue;
        }

        let &BuiltProperty {
            ident,
            docs,
            cfg,
            default,
            mut required,
        } = ip;

        required |= inherited_required.contains(ident);

        // collect property documentation info.
        let docs_info = if required {
            &mut docs_required_inherited
        } else if ident.to_string().starts_with("is_") {
            &mut docs_state_inherited
        } else if *default {
            &mut docs_default_inherited
        } else {
            &mut docs_other_inherited
        };
        docs_info.push(PropertyDocs {
            ident: ident.to_string(),
            docs: docs.clone(),
            doc_hidden: util::is_doc_hidden_tt(docs.clone()),
            inherited_from_path: Some(inherited_properties[ident].clone()),
            has_default: *default,
        });

        // collect property data for macros.
        let wgt_props = if is_child { &mut wgt_properties_child } else { &mut wgt_properties };
        wgt_props.extend(quote! {
            #ident {
                docs { #docs }
                cfg { #cfg }
                default { #default }
                required { #required }
            }
        });

        // generate re-export.
        let path = inherited_properties.get(&ip.ident).unwrap();
        let p_ident = ident!("__p_{}", ip.ident);
        property_reexports.extend(quote! {
            #cfg
            #[doc(inline)]
            pub use #path::#p_ident;
        });

        // generate values re-export.
        if ip.default {
            // default value.
            let d_ident = ident!("__d_{}", ip.ident);
            property_reexports.extend(quote! {
                #cfg
                #[doc(hidden)]
                pub use #path::#d_ident;
            });

            // source location reexport.
            #[cfg(debug_assertions)]
            {
                let loc_ident = ident!("__loc_{}", ip.ident);
                property_reexports.extend(quote! {
                    #cfg
                    #[doc(hidden)]
                    pub use #path::#loc_ident;
                });
            }
        }
    }
    // collect property re-exports and data for macros.
    for (p, is_child) in properties_child
        .iter()
        .map(|p| (p, true))
        .chain(properties.iter().map(|p| (p, false)))
    {
        let PropertyItem {
            ident,
            docs,
            cfg,
            default,
            required,
            ..
        } = p;
        let required = *required || inherited_required.contains(ident);

        // collect property documentation info.
        let ident_str = ident.to_string();
        let docs_info = if required {
            &mut docs_required
        } else if ident_str.starts_with("is_") {
            &mut docs_state
        } else if *default {
            &mut docs_default
        } else {
            &mut docs_other
        };
        docs_info.push(PropertyDocs {
            ident: ident_str,
            docs: docs.clone(),
            doc_hidden: util::is_doc_hidden_tt(docs.clone()),
            inherited_from_path: None,
            has_default: *default,
        });

        // collect property data for macros.
        let wgt_props = if is_child { &mut wgt_properties_child } else { &mut wgt_properties };
        wgt_props.extend(quote! {
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
                if let Some(inherited_source) = inherited_properties.get(&p.ident) {
                    // re-export inherited property.
                    property_reexports.extend(quote! {
                        #cfg
                        #[doc(inline)]
                        pub use #inherited_source::#p_ident;
                    });
                    continue;
                }
            }
            PropertyItemKind::AliasedIdent(maybe_inherited) => {
                if let Some(inherited_source) = inherited_properties.get(&maybe_inherited) {
                    // re-export inherited property as a new name.
                    let inherited_ident = ident!("__p_{}", maybe_inherited);
                    property_reexports.extend(quote! {
                        #cfg
                        #[doc(inline)]
                        pub use #inherited_source::#inherited_ident as #p_ident;
                    });
                    continue;
                }
            }
            PropertyItemKind::Path => {}
        }
        // not inherited.
        property_reexports.extend(quote! {
            #cfg
            #[doc(inline)]
            pub use #path::export as #p_ident;
        });
    }
    let property_reexports = property_reexports;
    let wgt_properties_child = wgt_properties_child;
    let wgt_properties = wgt_properties;

    docs_required.extend(docs_required_inherited);
    docs_default.extend(docs_default_inherited);
    docs_state.extend(docs_state_inherited);
    docs_other.extend(docs_other_inherited);
    let docs_required = docs_required;
    let docs_other = docs_other;

    // when data for macros.
    let mut wgt_whens = TokenStream::default();
    // inherited whens pub uses.
    let mut when_reexports = TokenStream::default();

    // docs data bout when blocks.
    let mut docs_whens = vec![];

    for inherited in &inherits {
        //inherited.module
        for BuiltWhen {
            ident,
            docs,
            cfg,
            inputs,
            assigns,
            expr_str,
        } in &inherited.whens
        {
            let module = &inherited.module;
            let module_id_str = util::tokens_to_ident_str(module);
            let new_ident = ident!("__{}{}", module_id_str, ident);

            let mut assigns_tt = TokenStream::default();
            let mut defaults_tt = TokenStream::default();
            for BuiltWhenAssign { property, cfg, value_fn } in assigns {
                if properties_unset.contains_key(&property) {
                    continue; // inherited removed by unset!.
                }

                docs_whens.push(WhenDocs {
                    docs: docs.clone(),
                    expr: expr_str.value(),
                    affects: assigns.iter().map(|a| (a.property.clone(), a.cfg.clone())).collect(),
                });

                let new_value_fn = ident!("__{}{}", module_id_str, value_fn);

                assigns_tt.extend(quote! {
                    #property { cfg { #cfg } value_fn { #new_value_fn } }
                });

                defaults_tt.extend(quote! {
                    #[doc(hidden)]
                    #cfg
                    pub use #module::#value_fn as #new_value_fn;
                });
            }
            if assigns_tt.is_empty() {
                continue; // all properties unset!, remove when block.
            }
            wgt_whens.extend(quote! {
                #new_ident {
                    docs { #docs }
                    cfg { #cfg }
                    inputs { #(#inputs)* }
                    assigns { #assigns_tt }
                    expr_str { #expr_str }
                }
            });
            when_reexports.extend(quote! {

                #[doc(hidden)]
                #cfg
                pub use #module::#ident as #new_ident;
                #defaults_tt
            });
        }
    }

    // all widget properties with and without values (excluding new when properties).
    let wgt_all_properties: HashSet<_> = inherited_props_child
        .iter()
        .chain(inherited_props.iter())
        .map(|p| &p.ident)
        .chain(properties_child.iter().chain(properties.iter()).map(|p| &p.ident))
        .collect();
    // widget properties introduced first by use in when blocks, we validate for default value.
    // map of [property_without_value => combined_cfg_for_default_init]
    let mut wgt_when_properties: HashMap<Ident, Option<TokenStream>> = HashMap::new();

    for BuiltWhen {
        ident,
        docs,
        cfg,
        inputs,
        assigns,
        expr_str,
    } in whens
    {
        docs_whens.push(WhenDocs {
            docs: docs.clone(),
            expr: expr_str.value(),
            affects: assigns.iter().map(|a| (a.property.clone(), a.cfg.clone())).collect(),
        });

        let bw_cfg = if cfg.is_empty() {
            None
        } else {
            Some(util::parse_attr(cfg.clone()).unwrap())
        };

        // for each property in inputs and assigns that are not declared in widget or inherited.
        for (property, p_cfg) in inputs.iter().map(|i| (i, None)).chain(
            assigns
                .iter()
                .map(|a| (&a.property, if a.cfg.is_empty() { None } else { Some(a.cfg.clone()) }))
                .filter(|(p, _)| !wgt_all_properties.contains(p)),
        ) {
            let cfg = util::cfg_attr_or(bw_cfg.clone(), p_cfg.map(|tt| util::parse_attr(tt).unwrap()));
            match wgt_when_properties.entry(property.clone()) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    let prev = e.get().clone().map(|tt| util::parse_attr(tt).unwrap());
                    *e.get_mut() = util::cfg_attr_or(prev, cfg.map(|tt| util::parse_attr(tt).unwrap()));
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(cfg);
                }
            }
        }

        let mut assigns_tt = TokenStream::default();
        for BuiltWhenAssign { property, cfg, value_fn } in assigns {
            assigns_tt.extend(quote! {
                #property { cfg { #cfg } value_fn { #value_fn } }
            });
        }
        wgt_whens.extend(quote! {
            #ident {
                docs { #docs }
                cfg { #cfg }
                inputs { #(#inputs)* }
                assigns {#assigns_tt }
                expr_str { #expr_str }
            }
        });
    }

    // properties that are only introduced in when conditions.
    // reexported if they have default values.
    let mut when_condition_default_props = TokenStream::default();
    let mut wgt_properties = wgt_properties;
    for (w_prop, cfg) in &wgt_when_properties {
        // property not introduced in the widget first, validate that it has a default value.

        let w_prop_str = w_prop.to_string();
        let docs = if w_prop_str.starts_with("is_") {
            &mut docs_state
        } else {
            &mut docs_default
        };
        docs.push(PropertyDocs {
            ident: w_prop_str,
            docs: TokenStream::default(),
            doc_hidden: false,
            inherited_from_path: inherited_properties.get(w_prop).map(|&tt| tt.clone()),
            has_default: true,
        });

        let p_ident = ident!("__p_{}", w_prop);
        let d_ident = ident!("__d_{}", w_prop);

        // reexport property and default value.
        when_condition_default_props.extend(quote! {
            #w_prop::code_gen! {
                if default=>

                #[doc(inline)]
                pub use #w_prop::export as #p_ident;

                #[doc(hidden)]
                pub fn #d_ident() -> impl self::#p_ident::Args {
                    self::#p_ident::ArgsImpl::default()
                }
            }
        });
        #[cfg(debug_assertions)]
        {
            let loc_ident = ident!("__loc_{}", w_prop);
            when_condition_default_props.extend(quote_spanned! {p_ident.span()=>
                #w_prop::code_gen! {
                    if default=>

                    #[doc(hidden)]
                    pub fn #loc_ident() -> #crate_core::debug::SourceLocation {
                        #crate_core::debug::source_location!()
                    }
                }
            });
        }

        // OR compile error because the property has no default value.
        let msg = format!("property `{}` is not declared in the widget and has no default value", w_prop);
        when_condition_default_props.extend(quote_spanned! {w_prop.span()=>
            #w_prop::code_gen! {
                if !default=>

                std::compile_error! { #msg }
            }
        });

        wgt_properties.extend(quote! {
            #w_prop {
                docs { }
                cfg { #cfg }
                default { true }
                required { false }
            }
        });
    }

    let built_data = quote! {
        module { #module }
        properties_child {
            #wgt_properties_child
        }
        properties {
            #wgt_properties
        }
        whens {
            #wgt_whens
        }
        new_child {
            #(#new_child)*
        }
        new {
            #(#new)*
        }
    };

    let macro_ident = ident!("{}_{}", ident, util::uuid());
    let inherit_macro = quote! {
        (
            inherit=>
            cfg { $(#[$cfg:meta])? }
            not_cfg { #[$not_cfg:meta] }
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
        TokenStream::default()
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

    let auto_docs = auto_docs(docs_required, docs_default, docs_state, docs_other, docs_whens);

    let r = quote! {
        #errors

        #attrs
        #auto_docs
        #cfg
        #vis mod #ident {
            #mod_items

            #property_reexports
            #when_reexports

            #new_child_reexport
            #new_reexport

            #when_condition_default_props
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #macro_ident {
            (reexport=> $as_ident:ident $(#[$cfg:meta])?) => {
                $(#[$cfg])?
                pub use #module as $as_ident;
            };
            #inherit_macro
            #new_macro
        }
        #[doc(hidden)]
        pub use #macro_ident as #ident;
    };

    r.into()
}
struct PropertyDocs {
    ident: String,
    docs: TokenStream,
    doc_hidden: bool,
    inherited_from_path: Option<TokenStream>,
    has_default: bool,
}
struct WhenDocs {
    docs: TokenStream,
    expr: String,
    // [(assigned_property, cfg)]
    affects: Vec<(Ident, TokenStream)>,
}
fn auto_docs(
    required: Vec<PropertyDocs>,
    default: Vec<PropertyDocs>,
    state: Vec<PropertyDocs>,
    mut other: Vec<PropertyDocs>,
    whens: Vec<WhenDocs>,
) -> TokenStream {
    #[allow(unused)]
    use util::is_doc_hidden;
    let mut r = TokenStream::default();
    doc_extend!(r, js_tag!("widget_full.js"));

    doc_extend!(r, "\n</div>");

    docs_section(
        &mut r,
        required,
        "Required Properties",
        "required-properties",
        "Properties that must be set.",
    );
    docs_section(
        &mut r,
        default,
        "Default Properties",
        "default-properties",
        "Properties that have a default value.",
    );
    docs_section(
        &mut r,
        state,
        "State Properties",
        "state-properties",
        "Properties that probe the widget state.",
    );

    other.push(PropertyDocs {
        ident: "*".to_owned(),
        docs: quote! {
            /// Widgets are open ended, all property functions can be used in any widget.
        },
        doc_hidden: false,
        inherited_from_path: None,
        has_default: false,
    });
    docs_section(
        &mut r,
        other,
        "Other Properties",
        "other-properties",
        "Other properties declared in the widget.",
    );

    if !whens.is_empty() {
        docs_section_header(&mut r, "When Conditions", "whens", "When conditions and what properties they set.");
        for (i, when) in whens.into_iter().enumerate() {
            let pattern = Regex::new(r#"self\.(\w+)"#).unwrap();
            let expr = util::format_rust_expr(when.expr);
            let expr = pattern.replace_all(&expr, r#"<span class='keyword'>self</span>.<a href='#wp-$1' class='fnname'>$1</a>"#);
            doc_extend!(r, "\n\n");
            doc_extend!(
                r,
                r##"<h3 id="ww-{0}" class="impl in-band"><code style="margin-left:10px">impl {1}</code><a href="#{0}" class="anchor" style="left:-10px"></a></h3><div class="docblock">"##,
                i,
                expr,
            );
            doc_extend!(r, "\n\n");
            r.extend(when.docs);

            let mut affected = String::new();
            for (assigned, cfg) in when.affects {
                use std::fmt::Write;
                if !affected.is_empty() {
                    affected.push_str(", ");
                }
                let mut cfg = util::html_title_cfg(cfg);
                if !cfg.is_empty() {
                    cfg = format!("title='{}'", cfg);
                }
                write!(&mut affected, "<a href='#wp-{0}' {1}><code>{0}</code></a>", assigned, cfg).unwrap();
            }
            doc_extend!(r, "\n\n*Affects {}.*", affected);
            doc_extend!(r, "\n\n</div>\n\n");
        }
        docs_close_section(&mut r);
    }

    r
}
fn docs_section(docs: &mut TokenStream, properties: Vec<PropertyDocs>, title: &'static str, id: &'static str, tool_tip: &'static str) {
    if properties.is_empty() {
        return;
    }
    docs_section_header(docs, title, id, tool_tip);
    for property in properties {
        if property.doc_hidden {
            // TODO handle doc generation that includes doc_hidden items
            continue;
        }
        docs_property_header(
            docs,
            &format!("wp-{}", property.ident),
            &property.ident.to_string(),
            &format!("fn.__p_{}.html", property.ident),
        );

        if property.docs.is_empty() {
            doc_extend!(docs, "<script>default_help('{}')</script>", property.ident);
        } else {
            docs.extend(property.docs);
        }

        if property.has_default {
            doc_extend!(docs, "\n\n*This property has a default value set by the widget.*");
        }
        if let Some(widget_path) = property.inherited_from_path {
            let widget_path = widget_path.to_string().replace(" ", "");
            let widget_name = if let Some(i) = widget_path.rfind(':') {
                &widget_path[i + 1..]
            } else {
                &widget_path
            };
            doc_extend!(
                docs,
                "\n\n*Inherited from [`{}`]({}#wp-{}).*",
                widget_name,
                widget_path,
                property.ident
            );
        }
        docs_close_property(docs);
    }
    docs_close_section(docs);
}
fn docs_section_header(docs: &mut TokenStream, title: &'static str, id: &'static str, tool_tip: &'static str) {
    doc_extend!(
        docs,
        r##"<h2 id="{0}" class="small-section-header" title="{2}">{1}<a href="#{0}" class="anchor"></a></h2><div class="methods" style="display: block;">"##,
        id,
        title,
        tool_tip,
    )
}
fn docs_close_section(docs: &mut TokenStream) {
    doc_extend!(docs, "</div>");
}
fn docs_property_header(docs: &mut TokenStream, id: &str, property: &str, url: &str) {
    doc_extend!(docs, "\n\n");
    doc_extend!(
        docs,
        r##"<h3 id="{0}" class="method in-band"><code><a style="margin-left:10px" href="{2}" class="fnname">{1}</a></code><a href="#{0}" class="anchor" style="left:-10px"></a></h3><div class="docblock">"##,
        id,
        property,
        url
    );
    doc_extend!(docs, "\n\n");
}
fn docs_close_property(docs: &mut TokenStream) {
    doc_extend!(docs, "\n\n</div>\n\n");
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
        unreachable!("expected last item to be `new { .. }`")
    }
}

/// Inherited widget or mixin data.
struct InheritedItem {
    mixin: bool,
    module: TokenStream,
    properties_child: Vec<BuiltProperty>,
    properties: Vec<BuiltProperty>,
    whens: Vec<BuiltWhen>,
    new_child: Vec<Ident>,
    new: Vec<Ident>,
}
impl Parse for InheritedItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(InheritedItem {
            mixin: non_user_braced!(input, "mixin")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            module: non_user_braced!(input, "module").parse().unwrap(),
            properties_child: parse_all(&non_user_braced!(input, "properties_child")).unwrap_or_else(|e| non_user_error!(e)),
            properties: parse_all(&non_user_braced!(input, "properties")).unwrap_or_else(|e| non_user_error!(e)),
            whens: parse_all(&non_user_braced!(input, "whens")).unwrap_or_else(|e| non_user_error!(e)),
            new_child: parse_all(&non_user_braced!(input, "new_child")).unwrap_or_else(|e| non_user_error!(e)),
            new: parse_all(&non_user_braced!(input, "new")).unwrap_or_else(|e| non_user_error!(e)),
        })
    }
}

/// New widget or mixin.
struct WidgetItem {
    module: TokenStream,
    attrs: TokenStream,
    cfg: TokenStream,
    vis: TokenStream,
    ident: Ident,
    mixin: bool,

    properties_unset: Vec<UnsetItem>,
    properties_declared: Vec<Ident>,

    properties_child: Vec<PropertyItem>,
    properties: Vec<PropertyItem>,
    whens: Vec<BuiltWhen>,

    new_child_declared: bool,
    new_child: Vec<Ident>,
    new_declared: bool,
    new: Vec<Ident>,

    mod_items: TokenStream,
}
impl Parse for WidgetItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        macro_rules! named_braces {
            ($name:tt) => {
                non_user_braced!(input, $name)
            };
        }
        Ok(WidgetItem {
            module: named_braces!("module").parse().unwrap(),
            attrs: named_braces!("attrs").parse().unwrap(),
            cfg: named_braces!("cfg").parse().unwrap(),
            vis: named_braces!("vis").parse().unwrap(),
            ident: named_braces!("ident").parse().unwrap_or_else(|e| non_user_error!(e)),
            mixin: named_braces!("mixin")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,

            properties_unset: parse_all(&named_braces!("properties_unset")).unwrap_or_else(|e| non_user_error!(e)),
            properties_declared: parse_all(&named_braces!("properties_declared")).unwrap_or_else(|e| non_user_error!(e)),

            properties_child: parse_all(&named_braces!("properties_child")).unwrap_or_else(|e| non_user_error!(e)),
            properties: parse_all(&named_braces!("properties")).unwrap_or_else(|e| non_user_error!(e)),
            whens: parse_all(&named_braces!("whens")).unwrap_or_else(|e| non_user_error!(e)),

            new_child_declared: named_braces!("new_child_declared")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            new_child: parse_all(&named_braces!("new_child")).unwrap_or_else(|e| non_user_error!(e)),
            new_declared: named_braces!("new_declared")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            new: parse_all(&named_braces!("new")).unwrap_or_else(|e| non_user_error!(e)),

            mod_items: named_braces!("mod_items").parse().unwrap(),
        })
    }
}
struct UnsetItem {
    property: Ident,
    /// for the span of the unset keyword.
    unset: TokenStream,
}
impl Parse for UnsetItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            property: input.parse().unwrap_or_else(|e| non_user_error!(e)),
            unset: non_user_braced!(input).parse().unwrap(),
        })
    }
}

/// A property declaration
struct PropertyItem {
    ident: Ident,
    docs: TokenStream,
    cfg: TokenStream,
    path: TokenStream,
    default: bool,
    required: bool,
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
            cfg: named_braces!("cfg").parse().unwrap(),
            path: named_braces!("path").parse().unwrap(),
            default: named_braces!("default")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            required: named_braces!("required")
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
    /// Cannot ne inherited, maybe aliased.
    Path,
}

mod keyword {
    syn::custom_keyword!(inherited);
    syn::custom_keyword!(widget);
}
