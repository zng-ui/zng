use proc_macro2::TokenStream;
use syn::{parse::Parse, Attribute, Ident, ItemMod, LitBool};

use crate::{
    util::{self, parse_all},
    widget_new2::BuiltWhen,
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Items { inherited, widget } = syn::parse(input).unwrap_or_else(|e| non_user_error!(e));

    for inherited in inherited {}

    todo!(
        "`widget_declare` macro expansion\ngo to file:\n{}:{}\n(ctrl + e) (tripple click to select path)",
        file!(),
        line!()
    )
}

struct Items {
    inherited: Vec<InheritedItem>,
    widget: WidgetItem,
}
impl Parse for Items {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut inherited = vec![];
        assert!(util::non_user_braced_id(input, "inherit").is_empty());

        while !input.is_empty() {
            if input.peek(keyword::inherited) {
                inherited.push(
                    util::non_user_braced_id(input, "inherited")
                        .parse()
                        .unwrap_or_else(|e| non_user_error!(e)),
                )
            } else if input.peek(keyword::widget) {
                let widget = util::non_user_braced_id(input, "widget")
                    .parse()
                    .unwrap_or_else(|e| non_user_error!(e));

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
struct InheritedItem {}
impl Parse for InheritedItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        todo!("InheritedItem")
    }
}

/// New widget or mixin.
struct WidgetItem {
    module: TokenStream,
    docs: TokenStream,
    cfg: TokenStream,
    ident: Ident,
    mixin: bool,

    properties_unset: Vec<Ident>,
    properties_declared: Vec<Ident>,

    properties: Vec<PropertyItem>,
    whens: Vec<BuiltWhen>,

    new_child: Vec<Ident>,
    new: Vec<Ident>,

    mod_: ItemMod,
}
impl Parse for WidgetItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let named_braces = |name| util::non_user_braced_id(input, name);
        Ok(WidgetItem {
            module: named_braces("module").parse().unwrap(),
            docs: named_braces("docs").parse().unwrap(),
            cfg: named_braces("cfg").parse().unwrap(),
            ident: named_braces("ident").parse().unwrap_or_else(|e| non_user_error!(e)),
            mixin: named_braces("mixin")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,

            properties_unset: parse_all(&named_braces("properties_unset")).unwrap_or_else(|e| non_user_error!(e)),
            properties_declared: parse_all(&named_braces("properties_declared")).unwrap_or_else(|e| non_user_error!(e)),

            properties: parse_all(&named_braces("properties")).unwrap_or_else(|e| non_user_error!(e)),
            whens: parse_all(&named_braces("whens")).unwrap_or_else(|e| non_user_error!(e)),

            new_child: parse_all(&named_braces("new_child")).unwrap_or_else(|e| non_user_error!(e)),
            new: parse_all(&named_braces("new")).unwrap_or_else(|e| non_user_error!(e)),

            mod_: named_braces("mod").parse().unwrap_or_else(|e| non_user_error!(e)),
        })
    }
}

/// A property declaration
struct PropertyItem {
    ident: Ident,
    docs: TokenStream,
    cfg: TokenStream,
    default: bool,
    required: bool,
}
impl Parse for PropertyItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = util::non_user_braced(input);
        let named_braces = |name| util::non_user_braced_id(&input, name);
        let property_item = PropertyItem {
            ident,
            docs: named_braces("docs").parse().unwrap(),
            cfg: named_braces("cfg").parse().unwrap(),
            default: named_braces("default")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            required: named_braces("required")
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
