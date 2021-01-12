use std::mem;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse2, parse_macro_input,
    spanned::Spanned,
    Ident, Item, ItemFn, ItemMacro, ItemMod, Path, Token,
};
use util::non_user_braced_id;

use crate::{
    util::{self, Attributes, Errors},
    widget_new2::{PropertyValue, When},
};

pub fn expand(mixin: bool, args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // the widget mod declaration.
    let mod_ = parse_macro_input!(input as ItemMod);
    if mod_.content.is_none() {
        return syn::Error::new(mod_.semi.span(), "only modules with inline content are supported")
            .to_compile_error()
            .to_token_stream()
            .into();
    }
    let (_, items) = mod_.content.unwrap();

    // accumulate the most errors as possible before returning.
    let mut errors = Errors::default();

    // a `$crate` path to the widget module.
    let mod_path = if mixin {
        TokenStream::new()
    } else {
        parse_mod_path(args.into(), &mut errors)
    };

    let Attributes {
        docs, cfg, others: attrs, ..
    } = Attributes::new(mod_.attrs);
    let vis = mod_.vis;
    let ident = mod_.ident;

    let WidgetItems {
        inherits,
        properties,
        mut new_child_fn,
        mut new_fn,
        others,
    } = WidgetItems::new(items, &mut errors);

    if mixin {
        if let Some(child_fn_) = new_child_fn.take() {
            errors.push("widget mixins do not have a `new_child` function", child_fn_.span())
        }

        if let Some(fn_) = new_fn.take() {
            errors.push("widget mixins do not have a `new` function", fn_.span())
        }
    }

    let mut inherits = inherits.into_iter().map(|i| i.path);
    let crate_core = util::crate_core();

    let stage_path = if mixin {
        if let Some(first) = inherits.next() {
            quote!(#first::__inherit!)
        } else {
            quote!(#crate_core::widget_declare!)
        }
    } else {
        // TODO change this back to implicit_mixin after testing
        quote!(#crate_core::widget_base::implicit_mixin2::__inherit!)
    };

    let r = quote! {
        // __inherit! will include an `inherited { .. }` block with the widget data after the
        // `inherit { .. }` block and take the next `inherit` path turn that into an `__inherit!` call.
        // This way we "eager" expand the inherited data recursively, when there no more path to inherit
        // a call to `widget_declare!` is made.
        #stage_path {
            mixin { #mixin }

            inherit { #(#inherits;)* }

            new {
                docs { #(#docs)* }
                ident { #ident }

                properties {

                }
                whens {

                }
                new_child { #new_child_fn }
                new { #new_fn }

                mod {
                    #(#attrs)*
                    #cfg
                    #vis mod #ident {
                        #(#others)*
                    }
                }
            }
        }
    };

    r.into()
}

fn parse_mod_path(args: TokenStream, errors: &mut Errors) -> TokenStream {
    let args_span = args.span();
    match syn::parse2::<Path>(args) {
        Ok(path) if path.segments.len() > 1 && path.segments[0].ident == "$crate" => path.to_token_stream(),
        _ => {
            errors.push("expected a macro_rules `$crate` path to this widget mod", args_span);
            quote! { $crate::missing_widget_mod_path }
        }
    }
}

struct WidgetItems {
    inherits: Vec<Inherit>,
    properties: Vec<Properties>,
    new_child_fn: Option<ItemFn>,
    new_fn: Option<ItemFn>,
    others: Vec<Item>,
}
impl WidgetItems {
    fn new(items: Vec<Item>, errors: &mut Errors) -> Self {
        let mut inherits = vec![];
        let mut properties = vec![];
        let mut new_child_fn = None;
        let mut new_fn = None;
        let mut others = vec![];

        for item in items {
            enum KnownMacro {
                Properties,
                Inherit,
            }
            let mut known_macro = None;
            enum KnownFn {
                New,
                NewChild,
            }
            let mut known_fn = None;
            match item {
                // match properties! or inherit!.
                Item::Macro(ItemMacro { mac, ident: None, .. })
                    if {
                        if let Some(ident) = mac.path.get_ident() {
                            if ident == "properties" {
                                known_macro = Some(KnownMacro::Properties);
                            } else if ident == "inherit" {
                                known_macro = Some(KnownMacro::Inherit);
                            }
                        }
                        known_macro.is_some()
                    } =>
                {
                    match known_macro {
                        Some(KnownMacro::Properties) => match syn::parse2::<Properties>(mac.tokens) {
                            Ok(mut p) => {
                                errors.extend(mem::take(&mut p.errors));
                                properties.push(p)
                            }
                            Err(e) => errors.push_syn(e),
                        },
                        Some(KnownMacro::Inherit) => match parse2::<Inherit>(mac.tokens) {
                            Ok(ps) => inherits.push(ps),
                            Err(e) => errors.push_syn(e),
                        },
                        None => unreachable!(),
                    }
                }
                // match fn new(..) or fn new_child(..).
                Item::Fn(fn_)
                    if {
                        if fn_.sig.ident == "new" {
                            known_fn = Some(KnownFn::New);
                        } else if fn_.sig.ident == "new_child" {
                            known_fn = Some(KnownFn::NewChild);
                        }
                        known_fn.is_some()
                    } =>
                {
                    match known_fn {
                        Some(KnownFn::New) => {
                            new_fn = Some(fn_);
                        }
                        Some(KnownFn::NewChild) => {
                            new_child_fn = Some(fn_);
                        }
                        None => unreachable!(),
                    }
                }
                // other user items.
                item => others.push(item),
            }
        }

        WidgetItems {
            inherits,
            properties,
            new_child_fn,
            new_fn,
            others,
        }
    }
}

struct Inherit {
    path: Path,
}
impl Parse for Inherit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Inherit { path: input.parse()? })
    }
}

struct Properties {
    errors: Errors,
    child_properties: Vec<ItemProperty>,
    properties: Vec<ItemProperty>,
    whens: Vec<When>,
}
impl Properties {
    fn flatten(self) -> (Vec<ItemProperty>, Vec<When>) {
        todo!(
            "flattening of multiple properties! \"macro calls\"\n\ngo to file:\n{}:{}\n(ctrl + e) (tripple click to select path)",
            file!(),
            line!()
        )
    }
}
impl Parse for Properties {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut errors = Errors::default();
        let mut child_properties = vec![];
        let mut properties = vec![];
        let mut whens = vec![];
        while !input.is_empty() {
            if input.peek(keyword::when) {
                if let Some(when) = When::parse(input, &mut errors) {
                    whens.push(when);
                }
            } else if input.peek(keyword::child) && input.peek2(syn::token::Brace) {
                let input = non_user_braced_id(input, "child");
                while !input.is_empty() {
                    match input.parse() {
                        Ok(p) => child_properties.push(p),
                        Err(e) => errors.push_syn(e),
                    }
                }
            } else if input.peek(Ident) {
                // peek ident or path.
                match input.parse() {
                    Ok(p) => properties.push(p),
                    Err(e) => errors.push_syn(e),
                }
            } else {
                errors.push("expected `when`, `child` or a property declaration", input.span());
                break;
            }
        }

        Ok(Properties {
            errors,
            child_properties,
            properties,
            whens,
        })
    }
}

struct ItemProperty {
    pub path: Path,
    pub alias: Option<(Token![as], Ident)>,
    pub type_: Option<(Token![:], PropertyType)>,
    pub value: Option<(Token![=], PropertyValue)>,
    pub semi: Option<Token![;]>,
}
impl Parse for ItemProperty {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        todo!(
            "parsing of widget properties\n\ngo to file:\n{}:{}\n(ctrl + e) (tripple click to select path)",
            file!(),
            line!()
        )
    }
}

enum PropertyType {
    Unamed,
    Named,
}

/// Property priority group in a widget.
enum PriorityGroup {
    Normal,
    Child,
}

struct Property {
    pub priority: PriorityGroup,
    pub ident: Ident,
}

mod keyword {
    pub use crate::widget_new2::keyword::when;
    syn::custom_keyword!(child);
}
