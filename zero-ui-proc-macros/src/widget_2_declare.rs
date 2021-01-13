use syn::{parse::Parse, Attribute, Ident, LitBool};

use crate::{util, widget_new2::BuiltWhen};

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
            match input.parse::<DeclareItem>().unwrap_or_else(|e| non_user_error!(e)) {
                DeclareItem::Inherited(i) => inherited.push(i),
                DeclareItem::Widget(widget) => {
                    assert!(input.is_empty());
                    return Ok(Items {
                        inherited,
                        widget,
                    });
                }
            }
        }
        unreachable!("expected last item to be `new { .. }`")
    }
}

enum DeclareItem {
    Inherited(InheritedItem),
    Widget(WidgetItem),
}
impl Parse for DeclareItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(keyword::inherited) {
            let _ = input.parse::<keyword::inherited>();
            util::non_user_braced(input).parse::<InheritedItem>().map(DeclareItem::Inherited)
        } else if input.peek(keyword::widget) {
            let _ = input.parse::<keyword::widget>();
            util::non_user_braced(input).parse::<WidgetItem>().map(DeclareItem::Widget)
        } else {
            non_user_error!("expected `inherited { .. }` or `widget { .. }`")
        }
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
    docs: Vec<Attribute>,
    ident: Ident,
    mixin: bool,

    properties: Vec<PropertyItem>,
    whens: Vec<BuiltWhen>,
}
impl Parse for WidgetItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        todo!("WidgetItem")
    }
}

/// A property declaration
struct PropertyItem {}

mod keyword {
    syn::custom_keyword!(inherited);
    syn::custom_keyword!(widget);
}
