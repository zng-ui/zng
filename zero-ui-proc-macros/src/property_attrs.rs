use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse::Parse, spanned::Spanned, *};

use crate::util::crate_core;

pub(crate) fn expand_easing(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let target = parse_macro_input!(input as PropertyAttrTarget);

    let target = match target {
        PropertyAttrTarget::PushProperty(p) => p,
        t => {
            // ignore other targets
            return t.to_token_stream().into();
        }
    };

    let args = parse_macro_input!(args as Args);
    let is_unset = args.is_unset();
    let Args { duration, easing } = args;

    let PropertyPushInfo {
        builder,
        importance,
        property,
    } = match PropertyPushInfo::new(&target) {
        Ok(r) => r,
        Err(e) => {
            let e = e.to_string();
            return quote! {
                compile_error!{#e}
            }
            .into();
        }
    };
    let core = crate_core();
    let name = "zero_ui::core::var::easing";

    let r = if is_unset {
        quote! {
            #builder.push_unset_property_build_action(
                #core::widget_builder::property_id!(#property),
                #name,
                #importance,
            );
            #target
        }
    } else {
        quote! {
            #builder.push_property_build_action(
                #core::widget_builder::property_id!(#property),
                #name,
                #importance,
                #core::var::types::easing_property_build_action(#duration, #easing), // !!: TODO
            );
            #target
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

enum PropertyAttrTarget {
    ValueInit(Local),
    PushProperty(ExprMethodCall),
    PushUnset(ExprMethodCall),
    CaptureOnlyDeclaration(ItemFn),
    Reexport(ItemUse),
}
impl Parse for PropertyAttrTarget {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let mut r = if input.peek(Token![let]) {
            let stmt = input.parse::<Stmt>()?;
            match stmt {
                Stmt::Local(l) => Ok(Self::ValueInit(l)),
                s => Err(syn::Error::new(s.span(), "expected property value init")),
            }
        } else if input.peek2(Token![.]) {
            let mtd_call = input.parse::<ExprMethodCall>()?;
            if mtd_call.method == ident!("push_property") {
                Ok(Self::PushProperty(mtd_call))
            } else if mtd_call.method == ident!("push_unset") {
                Ok(Self::PushUnset(mtd_call))
            } else {
                Err(Error::new(mtd_call.method.span(), "expected widget property"))
            }
        } else {
            let vis = input.parse::<Visibility>()?;

            if input.peek(Token![use]) {
                let mut item_use = input.parse::<ItemUse>()?;
                item_use.vis = vis;
                Ok(Self::Reexport(item_use))
            } else if input.peek(Token![fn]) {
                let mut item_fn = input.parse::<ItemFn>()?;
                item_fn.vis = vis;
                Ok(Self::CaptureOnlyDeclaration(item_fn))
            } else {
                Err(syn::Error::new(Span::call_site(), "expected widget property"))
            }
        }?;

        match &mut r {
            Self::ValueInit(init) => init.attrs = attrs,
            Self::PushProperty(push) | Self::PushUnset(push) => push.attrs = attrs,
            Self::CaptureOnlyDeclaration(cap) => cap.attrs = attrs,
            Self::Reexport(reex) => reex.attrs = attrs,
        }

        Ok(r)
    }
}
impl ToTokens for PropertyAttrTarget {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::ValueInit(it) => it.to_tokens(tokens),
            Self::PushProperty(it) | Self::PushUnset(it) => it.to_tokens(tokens),
            Self::CaptureOnlyDeclaration(it) => it.to_tokens(tokens),
            Self::Reexport(it) => it.to_tokens(tokens),
        }
    }
}

struct PropertyPushInfo {
    builder: Ident,
    importance: Expr,
    property: Path,
}
impl PropertyPushInfo {
    fn new(mtd_call: &ExprMethodCall) -> Result<Self> {
        let builder = match &*mtd_call.receiver {
            Expr::Path(p) if p.path.get_ident().is_some() => p.path.get_ident().unwrap().clone(),
            p => return Err(syn::Error::new(p.span(), "expected widget property")),
        };

        if mtd_call.args.len() != 2 {
            return Err(syn::Error::new(mtd_call.args.span(), "expected widget property"));
        }

        Ok(Self {
            builder,
            importance: mtd_call.args[0].clone(),
            property: match &mtd_call.args[1] {
                Expr::MethodCall(ExprMethodCall { receiver, .. }) => match &**receiver {
                    Expr::Path(p) if p.path.segments.last().map(|s| s.ident == ident!("__new__")).unwrap_or(false) => {
                        let mut p = p.path.clone();
                        p.segments.pop();
                        p.leading_colon = None;
                        p
                    }
                    t => return Err(syn::Error::new(t.span(), "expected widget property")),
                },
                e => return Err(syn::Error::new(e.span(), "expected widget property")),
            },
        })
    }
}
