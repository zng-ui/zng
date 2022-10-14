use std::mem;

use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{parse::Parse, spanned::Spanned, *};

use crate::{
    util::{self, parse_outer_attrs, Attributes, ErrorRecoverable, Errors},
    widget_util::{self, parse_remove, WgtProperty, WgtWhen},
};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // the widget mod declaration.
    let mod_ = parse_macro_input!(input as ItemMod);

    let uuid = util::uuid(&args); // full path to widget must be unique.

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
    let attrs = mod_.attrs;
    let wgt_cfg = Attributes::new(attrs.clone()).cfg;

    // a `$crate` path to the widget module.
    let mod_path = match syn::parse::<ArgPath>(args) {
        Ok(a) => a.path,
        Err(e) => {
            errors.push_syn(e);
            quote! { $crate::missing_widget_path}
        }
    };

    let WidgetItems {
        uses,
        inherits,
        properties,
        intrinsic_fn,
        build_fn,
        others,
    } = WidgetItems::new(items, &mut errors);

    let mut intrinsic = quote!();

    for Inherit { attrs, path } in &inherits {
        intrinsic.extend(quote_spanned! {path.span()=>
            #(#attrs)*
            #path::__intrinsic__(__wgt__);
        });
    }

    if let Some(int) = &intrinsic_fn {
        intrinsic.extend(quote_spanned! {int.span()=>
            self::intrinsic(__wgt__);
        })
    }

    for prop in properties.iter().flat_map(|i| i.properties.iter()) {
        if prop.is_unset() {
            let id = prop.property_id();
            intrinsic.extend(quote! {
                __wgt__.insert_unset(#crate_core::property::Importance::WIDGET, #id);
            });
        } else if prop.value.is_some() {
            let args = prop.args_new();
            intrinsic.extend(quote! {
                __wgt__.insert_property(#crate_core::property::Importance::WIDGET, #args);
            });
        }
    }

    let build = if let Some(build) = &build_fn {
        let out = &build.sig.output;
        let ident = &build.sig.ident;
        quote_spanned! {build.span()=>
            #[doc(hidden)]
            pub fn __build__(__wgt__: #crate_core::property::WidgetBuilder) #out {
                self::#ident(__wgt__)
            }
        }
    } else if let Some(inh) = inherits.last() {
        let path = &inh.path;
        quote! {
            #[doc(hidden)]
            pub use #path::__build__;
        }
    } else {
        errors.push(
            "missing `build(WidgetBuilder) -> T` function, must be provided or inherited",
            ident.span(),
        );
        quote! {
            #[doc(hidden)]
            pub fn __build__(_: #crate_core::property::WidgetBuilder) -> #crate_core::NilUiNode {
                #crate_core::NilUiNode
            }
        }
    };

    let mut inherits_reexport = quote!();
    if let Some(Inherit { attrs, path }) = inherits.first() {
        let mut instructions = quote!();
        for p in &properties {
            for p in &p.removes {
                instructions.extend(quote! {
                    remove { #p }
                });
            }
        }
        for Inherit { attrs, path } in &inherits[1..] {
            instructions.extend(quote! {
                inherit { #path }
            });
        }

        inherits_reexport = quote! {
            #path! {
                >> reexport {
                    #instructions
                }
            }
        }
    }

    let mut widget_macro_intrinsic = quote!();
    for p in properties.iter().flat_map(|p| p.properties.iter()) {
        let cfg = Attributes::new(p.attrs.clone()).cfg;
        let ident = p.property_ident();
        let required = &p.is_required();
        let default = p.has_default();
        widget_macro_intrinsic.extend(quote! {
            property {
                cfg { #cfg }
                ident { #ident }
                required { #required }
                default { #default }
            }
        });
    }

    let macro_ident = ident!("{ident}_{uuid}");

    let r = quote! {
        #(#attrs)*
        #vis mod #ident {
            // custom items
            #(#others)*

            // use items (after custom items in case of custom macro_rules re-export)
            #(#uses)*

            #inherits_reexport

            #intrinsic_fn

            #[doc(hidden)]
            pub fn __intrinsic__(__wgt__: &mut #crate_core::property::WidgetBuilder) {
                #intrinsic
            }

            #build_fn
            #build

            #[doc(hidden)]
            pub mod __core__ {
                pub use #crate_core::{widget_new, widget_reexport};
            }
        }

        #wgt_cfg
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #macro_ident {
            (>> reexport {
                $($instructions:tt)*
            }) => {
                #mod_path::__core__::widget_reexport! {
                    intrinsic {
                        widget { #mod_path }
                        #widget_macro_intrinsic
                    }
                    $($instructions)*
                }
            };
            ($($tt:tt)*) => {
                #mod_path::__core__::widget_new! {
                    intrinsic {
                        widget { #mod_path }
                        #widget_macro_intrinsic
                    }
                    instance {
                        $($tt)*
                    }
                }
            };
        }
        #[doc(hidden)]
        #vis use #macro_ident as #ident;

        #errors
    };
    r.into()
}

struct ArgPath {
    path: TokenStream,
}
impl Parse for ArgPath {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
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

struct WidgetItems {
    uses: Vec<ItemUse>,
    inherits: Vec<Inherit>,
    properties: Vec<Properties>,
    intrinsic_fn: Option<ItemFn>,
    build_fn: Option<ItemFn>,
    others: Vec<Item>,
}
impl WidgetItems {
    fn new(items: Vec<Item>, errors: &mut Errors) -> Self {
        let mut uses = vec![];
        let mut inherits = vec![];
        let mut properties = vec![];
        let mut intrinsic_fn = None;
        let mut build_fn = None;
        let mut others = vec![];

        for item in items {
            match item {
                Item::Use(use_) => {
                    uses.push(use_);
                }
                // match properties!
                Item::Macro(ItemMacro { mac, ident: None, .. }) if mac.path.get_ident().map(|i| i == "properties").unwrap_or(false) => {
                    match syn::parse2::<Properties>(mac.tokens) {
                        Ok(mut p) => {
                            errors.extend(mem::take(&mut p.errors));
                            properties.push(p)
                        }
                        Err(e) => errors.push_syn(e),
                    }
                }
                // match inherit!
                Item::Macro(ItemMacro {
                    mac, attrs, ident: None, ..
                }) if mac.path.get_ident().map(|i| i == "inherit").unwrap_or(false) => match parse2::<Inherit>(mac.tokens) {
                    Ok(mut ps) => {
                        ps.attrs.extend(attrs);
                        inherits.push(ps)
                    }
                    Err(e) => errors.push_syn(e),
                },

                // match fn intrinsic(..)
                Item::Fn(fn_) if fn_.sig.ident == "intrinsic" => {
                    intrinsic_fn = Some(fn_);
                }
                // match fn build(..)
                Item::Fn(fn_) if fn_.sig.ident == "build" => {
                    build_fn = Some(fn_);
                }
                // other user items.
                item => others.push(item),
            }
        }

        WidgetItems {
            uses,
            inherits,
            properties,
            intrinsic_fn,
            build_fn,
            others,
        }
    }
}

struct Inherit {
    attrs: Vec<Attribute>,
    path: Path,
}
impl Parse for Inherit {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        Ok(Inherit {
            attrs: vec![],
            path: input.parse()?,
        })
    }
}

struct Properties {
    errors: Errors,
    properties: Vec<WgtProperty>,
    removes: Vec<Ident>,
    whens: Vec<WgtWhen>,
}

impl Parse for Properties {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut errors = Errors::default();
        let mut properties = vec![];
        let mut removes = vec![];
        let mut whens = vec![];

        while input.is_empty() {
            let attrs = parse_outer_attrs(input, &mut errors);

            if input.peek(widget_util::keyword::when) {
                if let Some(mut when) = WgtWhen::parse(input, &mut errors) {
                    when.attrs = attrs;
                    whens.push(when);
                }
            } else if input.peek(widget_util::keyword::remove) && input.peek2(syn::token::Brace) {
                parse_remove(input, &mut removes, &mut errors);
            } else if input.peek(Ident) || input.peek(Token![crate]) || input.peek(Token![super]) || input.peek(Token![self]) {
                // peek ident or path (including keywords because of super:: and self::). {
                match input.parse::<WgtProperty>() {
                    Ok(mut p) => {
                        p.attrs = attrs;
                        if !input.is_empty() && p.semi.is_none() {
                            errors.push("expected `;`", input.span());
                            while !(input.is_empty()
                                || input.peek(Ident)
                                || input.peek(Token![crate])
                                || input.peek(Token![super])
                                || input.peek(Token![self])
                                || input.peek(Token![#]) && input.peek(token::Bracket))
                            {
                                // skip to next value item.
                                let _ = input.parse::<TokenTree>();
                            }
                        }
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
                errors.push("expected `when`, `child`, `remove` or a property declaration", input.span());

                // suppress the "unexpected token" error from syn parse.
                let _ = input.parse::<TokenStream>();

                break;
            }
        }

        Ok(Properties {
            errors,
            properties,
            removes,
            whens,
        })
    }
}

/*
    REEXPORT
*/

pub fn expand_reexport(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as ReexportArgs);

    let r = quote! {};
    r.into()
}

struct ReexportArgs {
    widget: TokenStream,
    intrinsic: Vec<IntrinsicItem>,
    instructions: Vec<ReexportInstruction>,
}
impl Parse for ReexportArgs {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let intr = non_user_braced!(input, "intrinsic");
        let widget = non_user_braced!(&intr, "widget").parse().unwrap();
        let mut intrinsic = vec![];
        while !intr.is_empty() {
            intrinsic.push(intr.parse().unwrap());
        }

        let inst = non_user_braced!(input, "instructions");
        let mut instructions = vec![];
        while !inst.is_empty() {
            instructions.push(inst.parse().unwrap());
        }

        Ok(ReexportArgs {
            widget,
            intrinsic,
            instructions,
        })
    }
}

enum IntrinsicItem {
    Property {
        cfg: TokenStream,
        ident: Ident,
        required: bool,
        default: bool,
    },
}
impl Parse for IntrinsicItem {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let input = non_user_braced!(input, "property");
        let r = IntrinsicItem::Property {
            cfg: non_user_braced!(&input, "cfg").parse().unwrap(),
            ident: non_user_braced!(&input, "ident").parse().unwrap_or_else(|e| non_user_error!(e)),
            required: non_user_braced!(&input, "required").parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            default: non_user_braced!(&input, "default").parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
        };
        Ok(r)
    }
}

enum ReexportInstruction {
    Remove(Ident),
    Inherit(TokenStream),
}
impl Parse for ReexportInstruction {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let tag: Ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        if tag == "remove" {
            Ok(ReexportInstruction::Remove(
                non_user_braced!(input).parse().unwrap_or_else(|e| non_user_error!(e)),
            ))
        } else if tag == "inherit" {
            Ok(ReexportInstruction::Inherit(non_user_braced!(input).parse().unwrap()))
        } else {
            non_user_error!("{}", tag)
        }
    }
}
