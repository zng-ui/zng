use quote::ToTokens;
use syn::{parse::Parse, *};

use crate::util::crate_core;

pub(crate) fn expand_easing(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as PropertyAttrData);
    let args = parse_macro_input!(args as Args);
    let PropertyAttrData {
        builder,
        is_unset,
        is_when_assign,
        property,
        importance,
        ..
    } = &data;
    let Args { duration, easing } = &args;

    if *is_unset {
        return quote! {
            compile_error!{"cannot set `easing` in unset assign\n\n   note: you can use `#[easing(unset)]` to unset in a normal assign to unset easing"}
            #data
        }.into();
    }

    if *is_when_assign {
        return quote! {
            compile_error!{"cannot set `easing` in when assign"}
            #data
        }
        .into();
    }

    let core = crate_core();
    let name = "zero_ui::core::var::easing";

    let r = if args.is_unset() {
        quote! {
            #builder.push_unset_property_build_action(
                #core::widget_builder::property_id!(#property),
                #name,
                #importance,
            );
            #data
        }
    } else {
        quote! {
            #builder.push_property_build_action(
                #core::widget_builder::property_id!(#property),
                #name,
                #importance,
                #core::var::types::easing_property_build_action(
                    {
                        use #core::units::TimeUnits as _;
                        #duration
                    },
                    {
                        use #core::var::easing::*;
                        #easing
                    }
                ), // !!: TODO, pass the property types to the build action
            );
            #data
        }
    };
    r.into()
}

struct Args {
    duration: Expr,
    easing: Expr,
}
impl Args {
    fn is_unset(&self) -> bool {
        match &self.duration {
            Expr::Path(p) => p.path.get_ident().map(|id| id == &ident!("unset")).unwrap_or(false),
            _ => false,
        }
    }
}
impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Args {
            duration: input.parse()?,
            easing: {
                if input.peek(Token![,]) {
                    let _ = input.parse::<Token![,]>();
                    input.parse()?
                } else {
                    parse_quote!(linear)
                }
            },
        })
    }
}

pub(crate) struct PropertyAttrData {
    pub pending_attrs: Vec<Attribute>,
    pub data_ident: Ident,
    pub builder: Ident,
    pub is_unset: bool,
    pub is_when_assign: bool,
    pub property: Path,
    pub importance: Expr,
}
impl Parse for PropertyAttrData {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let item_mod: ItemMod = input.parse()?;

        let mut builder = None;
        let mut is_unset = false;
        let mut is_when_assign = false;
        let mut property = None;
        let mut importance = None;

        for item in item_mod.content.unwrap_or_else(|| non_user_error!("")).1 {
            let mut f = match item {
                Item::Fn(f) => f,
                _ => non_user_error!(""),
            };

            let stmt = f.block.stmts.pop().unwrap_or_else(|| non_user_error!(""));
            let expr = match stmt {
                Stmt::Expr(e) => e,
                _ => non_user_error!(""),
            };

            if f.sig.ident == ident!("builder_ident") {
                let path = match expr {
                    Expr::Path(p) => p.path,
                    _ => non_user_error!(""),
                };

                let ident = match path.get_ident() {
                    Some(i) => i.clone(),
                    None => non_user_error!(""),
                };

                builder = Some(ident);
            } else if f.sig.ident == ident!("property_path") {
                let path = match expr {
                    Expr::Path(p) => p.path,
                    _ => non_user_error!(""),
                };

                property = Some(path);
            } else if f.sig.ident == ident!("importance") {
                importance = Some(expr);
            } else if f.sig.ident == ident!("is_unset") {
                let lit = match expr {
                    Expr::Lit(l) => l,
                    _ => non_user_error!(""),
                };

                let lit_bool = match lit.lit {
                    Lit::Bool(b) => b,
                    _ => non_user_error!(""),
                };

                is_unset = lit_bool.value();
            } else if f.sig.ident == ident!("is_when_assign") {
                let lit = match expr {
                    Expr::Lit(l) => l,
                    _ => non_user_error!(""),
                };

                let lit_bool = match lit.lit {
                    Lit::Bool(b) => b,
                    _ => non_user_error!(""),
                };

                is_when_assign = lit_bool.value();
            } else {
                non_user_error!("")
            }
        }

        Ok(Self {
            pending_attrs: item_mod.attrs,
            data_ident: item_mod.ident,
            builder: builder.unwrap_or_else(|| non_user_error!("")),
            is_unset,
            is_when_assign,
            property: property.unwrap_or_else(|| non_user_error!("")),
            importance: importance.unwrap_or_else(|| {
                let core = crate_core();
                parse_quote! {
                    #core::widget_base::Importance::WIDGET
                }
            }),
        })
    }
}
impl ToTokens for PropertyAttrData {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if self.pending_attrs.is_empty() {
            return;
        }

        let Self {
            pending_attrs,
            data_ident,
            builder,
            is_unset,
            is_when_assign,
            property,
            importance,
        } = self;

        tokens.extend(quote! {
            #(#pending_attrs)*
            mod #data_ident {
                fn builder_ident() {
                    #builder
                }

                fn property_path() {
                    #property
                }

                fn is_unset() {
                    #is_unset
                }

                fn is_when_assign() {
                    #is_when_assign
                }

                fn importance() {
                    #importance
                }
            }
        })
    }
}
