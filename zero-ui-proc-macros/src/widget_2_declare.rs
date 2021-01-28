use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use syn::{parse::Parse, Ident, LitBool};

use crate::{
    util::{self, parse_all},
    widget_new2::{BuiltProperty, BuiltWhen, BuiltWhenAssign},
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Items { inherits, widget } = syn::parse(input).unwrap_or_else(|e| non_user_error!(e));
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
    let properties_unset: HashSet<_> = properties_unset.into_iter().collect();
    let properties_declared: HashSet<_> = properties_declared.into_iter().collect();

    let crate_core = util::crate_core();

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
                    pub use #crate_core::widget_base::default_widget_new_child as __new_child;
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
                    pub use #crate_core::widget_base::default_widget_new as __new;
                };
                new = vec![ident!("id")];
            }
        }
    }
    let new_child = new_child;
    let new = new;

    let captured_properties: HashSet<_> = new_child.iter().chain(&new).collect();

    // collect inherited properties. Late inherits of the same ident overrides early inherits.
    let mut inherited_properties = HashMap::new();
    let mut inherited_props_child = vec![];
    let mut inherited_props = vec![];
    for inherited in inherits.iter().rev() {
        for p_child in inherited.properties_child.iter().rev() {
            if !properties_unset.contains(&p_child.ident) && inherited_properties.insert(&p_child.ident, &inherited.module).is_none() {
                inherited_props_child.push(p_child);
            }
        }
        for p in inherited.properties.iter().rev() {
            if !properties_unset.contains(&p.ident) && inherited_properties.insert(&p.ident, &inherited.module).is_none() {
                inherited_props.push(p);
            }
        }
    }
    inherited_props_child.reverse();
    inherited_props.reverse();
    let inherited_properties = inherited_properties;
    let inherited_props_child = inherited_props_child;
    let inherited_props = inherited_props;

    // properties that are assigned (not in when blocks) or declared in the new widget.
    let wgt_used_properties: HashSet<_> = properties.iter().map(|p| &p.ident).collect();
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
            required,
        } = ip;

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
        let required = *required || captured_properties.contains(ident);

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

        // if property was declared `some_ident as new_ident;`.
        if let Some(maybe_inherited) = p.get_path_ident() {
            // if `some_ident` was inherited.
            if inherited_properties.contains_key(&maybe_inherited) {
                // re-exports: `pub use self::__p_some_ident as __p_new_ident;`
                let inherited_p_ident = ident!("__p_{}", maybe_inherited);
                property_reexports.extend(quote! {
                    #cfg
                    #[doc(inline)]
                    pub use self::#inherited_p_ident as #p_ident;
                });
                // done.
                continue;
            }
        }
        // else
        let path = inherited_properties.get(&p.ident).unwrap_or(&path);
        property_reexports.extend(quote! {
            #cfg
            #[doc(inline)]
            pub use #path::export as #p_ident;
        });
    }
    let property_reexports = property_reexports;
    let wgt_properties_child = wgt_properties_child;
    let wgt_properties = wgt_properties;

    // when data for macros.
    let mut wgt_whens = TokenStream::default();
    // inherited whens pub uses.
    let mut when_reexports = TokenStream::default();

    for inherited in &inherits {
        //inherited.module
        for BuiltWhen {
            ident,
            docs,
            cfg,
            inputs,
            assigns,
        } in &inherited.whens
        {
            let module = &inherited.module;
            let module_id_str = util::tokens_to_ident_str(module);
            let new_ident = ident!("__{}{}", module_id_str, ident);

            let mut assigns_tt = TokenStream::default();
            let mut defaults_tt = TokenStream::default();
            for BuiltWhenAssign { property, cfg, value_fn } in assigns {
                if properties_unset.contains(&property) {
                    continue; // inherited removed by unset!.
                }

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

    let wgt_captures: HashSet<_> = new_child.iter().chain(new.iter()).collect();
    let wgt_properties_with_value: HashSet<_> = inherited_props_child
        .iter()
        .chain(inherited_props.iter())
        .filter(|p| p.default || p.required || wgt_captures.contains(&p.ident))
        .map(|p| &p.ident)
        .chain(
            properties_child
                .iter()
                .chain(properties.iter())
                .filter(|p| p.default || p.required || wgt_captures.contains(&p.ident))
                .map(|p| &p.ident),
        )
        .collect();

    for BuiltWhen {
        ident,
        docs,
        cfg,
        inputs,
        assigns,
    } in whens
    {
        let mut assigns_tt = TokenStream::default();
        for BuiltWhenAssign { property, cfg, value_fn } in assigns {
            if wgt_properties_with_value.contains(&property) {
                assigns_tt.extend(quote! {
                    #property { cfg { #cfg } value_fn { #value_fn } }
                });
            } else {
                let p_ident = ident!("__p_{}", ident);
                let error = format!("property `{}` cannot be assigned in `when` because it has no default value", ident);
                assigns_tt.extend(quote_spanned! {ident.span()=>
                    self::#p_ident::code_gen! {
                        if !default=> std::compile_error!{ #error }
                    }
                });
            }
        }
        wgt_whens.extend(quote! {
            #ident {
                docs { #docs }
                cfg { #cfg }
                inputs { #(#inputs)* }
                assigns {#assigns_tt }
            }
        });
    }

    let gen_docs = TokenStream::default();

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

    let uuid = util::uuid();

    let inherit_macro_ident = ident!("inherit_{}_{}", ident, uuid);
    let inherit_macro = quote! {
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #inherit_macro_ident {
            (
                cfg { $(#[$cfg:meta])? }
                not_cfg { #[$not_cfg:meta] }
                inherit { $($inherit:path;)* }
                $($rest:tt)+
            ) => {
                $(#[$cfg])?
                #module::__core::widget_inherit! {
                    inherit { $($inherit;)* }
                    inherited {
                        mixin { #mixin }

                        #built_data
                    }
                    $($rest)*
                }
                #[$not_cfg]
                #module::__core::widget_inherit! {
                    inherit { $($inherit;)* }
                    $($rest)*
                }
            };
        }

        #[doc(hidden)]
        pub use #inherit_macro_ident as __inherit;
    };

    let (new_macro, new_macro_reexport) = if mixin {
        (TokenStream::default(), TokenStream::default())
    } else {
        let new_macro_ident = ident!("new_{}_{}", ident, uuid);

        let new = quote! {
            #[doc(hidden)]
            #[macro_export]
            macro_rules! #new_macro_ident {
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
            #[doc(hidden)]
            pub use #new_macro_ident as __new_macro;
        };
        let reexport = quote! {
            #cfg
            #[doc(hidden)]
            #vis use #ident::__new_macro as #ident;
        };

        (new, reexport)
    };

    let r = quote! {
        #attrs
        #gen_docs
        #cfg
        #vis mod #ident {
            #mod_items

            #property_reexports
            #when_reexports

            #new_child_reexport
            #new_reexport

            #new_macro

            #inherit_macro
        }
        #new_macro_reexport
    };

    r.into()
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

    properties_unset: Vec<Ident>,
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
    /// Gets `self.path` as [`Ident`] if it is a single ident.
    pub fn get_path_ident(&self) -> Option<Ident> {
        syn::parse2::<Ident>(self.path.clone()).ok()
    }
}

mod keyword {
    syn::custom_keyword!(inherited);
    syn::custom_keyword!(widget);
}
