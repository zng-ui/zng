use quote::ToTokens;
use syn::{parse::Parse, *};

use crate::util::crate_core;

pub(crate) fn expand_easing(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as PropertyAttrData);
    let args = parse_macro_input!(args as Args);
    let PropertyAttrData {
        builder,
        is_unset: property_is_unset,
        property,
        importance,
        when,
        ..
    } = &data;
    let Args { duration, easing } = &args;

    if *property_is_unset {
        return quote! {
            compile_error!{"cannot set `easing` in unset assign\n\n   note: you can use `#[easing(unset)]` to unset in a normal assign to unset easing"}
            #data
        }.into();
    }

    let is_unset = args.is_unset();

    let core = crate_core();
    let name = "zero_ui::core::var::easing";

    if let Some(when) = when {
        if is_unset {
            return quote! {
                compile_error!{"cannot unset `easing` in when assign, try `#[easing(0.ms())]`"}
                #data
            }
            .into();
        }

        return quote! {
            {
                let __data__ = #core::var::types::easing_property::easing_when_data(
                    #core::widget_builder::property_input_types!(#property),
                    {
                        use #core::units::TimeUnits as _;
                        #duration
                    },
                    std::sync::Arc::new({
                        use #core::var::easing::*;
                        #easing
                    }),
                );
                #when.build_action_data.push((
                    (
                        #core::widget_builder::property_id!(#property),
                        #name,
                    ),
                    __data__,
                ));
            }
            #data
        }
        .into();
    }

    let r = if is_unset {
        quote! {
            #core::var::types::easing_property::easing_property_unset(
                #core::widget_builder::property_input_types!(#property)
            );
            #builder.push_unset_property_build_action(
                #core::widget_builder::property_id!(#property),
                #name,
                #importance,
            );
            #data
        }
    } else {
        quote! {
            {
                let __actions__ = #core::var::types::easing_property::easing_property(
                    #core::widget_builder::property_input_types!(#property),
                    {
                        use #core::units::TimeUnits as _;
                        #duration
                    },
                    std::sync::Arc::new({
                        use #core::var::easing::*;
                        #easing
                    }),
                );
                if !__actions__.is_empty() {
                    #builder.push_property_build_action(
                        #core::widget_builder::property_id!(#property),
                        #name,
                        #importance,
                        __actions__
                    );
                }
            }
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
    /// Other custom property attributes that must be expanded.
    ///
    /// `PropertyAttrData::to_tokens` only generates tokens if there are pending attrs.
    pub pending_attrs: Vec<Attribute>,
    /// ident of the "fake" data module used to expand this attribute, is unique in scope.
    pub data_ident: Ident,

    /// mut local: WidgetBuilder.
    pub builder: Ident,
    /// If is property `unset!`.
    pub is_unset: bool,
    /// path to the property function/struct.
    pub property: Path,
    /// mut local: Importance. Only set if is property assign (has value and not in when).
    pub importance: Option<Ident>,
    /// mut local: WhenInfo. Only set if is when assign.
    pub when: Option<Ident>,
}
impl Parse for PropertyAttrData {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let item_mod: ItemMod = input.parse()?;

        let mut builder = None;
        let mut is_unset = false;
        let mut property = None;
        let mut importance = None;
        let mut when = None;

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
            } else if f.sig.ident == ident!("importance_ident") {
                let path = match expr {
                    Expr::Path(p) => p.path,
                    _ => non_user_error!(""),
                };

                let ident = match path.get_ident() {
                    Some(i) => i.clone(),
                    None => non_user_error!(""),
                };

                importance = Some(ident);
            } else if f.sig.ident == ident!("when_ident") {
                let path = match expr {
                    Expr::Path(p) => p.path,
                    _ => non_user_error!(""),
                };

                let ident = match path.get_ident() {
                    Some(i) => i.clone(),
                    None => non_user_error!(""),
                };

                when = Some(ident);
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
            } else {
                non_user_error!("")
            }
        }

        Ok(Self {
            pending_attrs: item_mod.attrs,
            data_ident: item_mod.ident,
            builder: builder.unwrap_or_else(|| non_user_error!("")),
            is_unset,
            property: property.unwrap_or_else(|| non_user_error!("")),
            importance,
            when,
        })
    }
}
impl ToTokens for PropertyAttrData {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if self.pending_attrs.is_empty() {
            return;
        }

        let span = self.pending_attrs[0].pound_token.span;

        let Self {
            pending_attrs,
            data_ident,
            builder,
            is_unset,
            property,
            importance,
            when,
        } = self;

        let when = if let Some(when) = when {
            quote_spanned! {span=>
                fn when_ident() {
                    #when
                }
            }
        } else {
            quote!()
        };
        let importance = if let Some(importance) = importance {
            quote_spanned! {span=>
                fn importance_ident() {
                    #importance
                }
            }
        } else {
            quote!()
        };

        tokens.extend(quote_spanned! {span=>
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

                #when

                #importance
            }
        })
    }
}
