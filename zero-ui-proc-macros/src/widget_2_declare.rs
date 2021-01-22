use proc_macro2::TokenStream;
use syn::{parse::Parse, Attribute, Ident, ItemMod, LitBool, Visibility};

use crate::{
    util::{self, parse_all},
    widget_new2::{BuiltProperty, BuiltWhen},
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Items { inherited, widget } = syn::parse(input).unwrap_or_else(|e| non_user_error!(e));
    let WidgetItem {
        module,
        attrs,
        cfg,
        vis,
        ident,
        mixin,
        properties_unset,
        properties_declared,
        properties,
        whens,
        new_child_declared,
        mut new_child,
        new_declared,
        mut new,
        mod_items,
    } = widget;

    // inherits `new_child` and `new`.
    let last_not_mixin = inherited.iter().filter(|i| !i.mixin).last();
    let mut new_child_reexport = TokenStream::default();
    let mut new_reexport = TokenStream::default();
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
                pub use #module::__core::widget_base::default_widget_new_child as __new_child;
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
            new_child_reexport = quote! {
                #[doc(hidden)]
                pub use #module::__core::widget_base::default_widget_new as __new;
            };
            new = vec![ident!("id")];
        }
    }
    let _ = last_not_mixin;

    let mut property_reexports = TokenStream::default();
    for p in properties {
        let cfg = p.cfg;
        let path = p.path;
        let p_ident = ident!("__p_{}", p.ident);

        property_reexports.extend(quote! {
            #cfg
            #[doc(inline)]
            pub use #path::export as #p_ident;
        });
    }

    let gen_docs = TokenStream::default();

    let uuid = util::uuid();

    let inherit_macro_ident = ident!("inherit_{}_{}", ident, uuid);
    let inherit_macro = quote! {
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #inherit_macro_ident {
            (
                inherit { $($inherit:path;)* }
                $($rest:tt)+
            ) => {
                #module::__core::widget_inherit! {
                    inherit { $($inherit;)* }
                    inherited {
                        module { #module }
                        mixin { #mixin }
                        properties_child {
                            // TODO
                        }
                        properties {
                            // TODO
                        }
                        whens {
                            // TODO
                        }
                        new_child {
                            #(#new_child)*
                        }
                        new {
                            #(#new)*
                        }
                    }
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
                        // TODO
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
    inherited: Vec<InheritedItem>,
    widget: WidgetItem,
}
impl Parse for Items {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut inherited = vec![];
        assert!(non_user_braced!(input, "inherit").is_empty());

        while !input.is_empty() {
            if input.peek(keyword::inherited) {
                inherited.push(non_user_braced!(input, "inherited").parse().unwrap_or_else(|e| non_user_error!(e)))
            } else if input.peek(keyword::widget) {
                let widget = non_user_braced!(input, "widget").parse().unwrap_or_else(|e| non_user_error!(e));

                if !input.is_empty() {
                    non_user_error!("expected `widget { .. }` to be the last item");
                }
                return Ok(Items { inherited, widget });
            } else {
                non_user_error!("expected `inherited { .. }` or `widget { .. }`")
            }
        }
        unreachable!("expected last item to be `new { .. }`")
    }
}

/// Inherited widget or mixin data.
struct InheritedItem {
    module: TokenStream,
    mixin: bool,
    properties_child: Vec<BuiltProperty>,
    properties: Vec<BuiltProperty>,
    whens: Vec<BuiltWhen>,
    new_child: Vec<Ident>,
    new: Vec<Ident>,
}
impl Parse for InheritedItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(InheritedItem {
            module: non_user_braced!(input, "module").parse().unwrap(),
            mixin: non_user_braced!(input, "mixin")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
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

mod keyword {
    syn::custom_keyword!(inherited);
    syn::custom_keyword!(widget);
}
