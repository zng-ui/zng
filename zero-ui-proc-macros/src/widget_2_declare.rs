#![allow(unused)] // TODO remove after expand is called in lib.rs.

use syn::parse::Parse;

use crate::util;

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Items { inherited, new } = syn::parse(input).unwrap_or_else(|e| non_user_error!(e));

    for inherited in inherited {}

    todo!("`widget_declare` macro expansion\ngo to file:\n{}:{}\n(ctrl + e) (tripple click to select path)", file!(), line!())
}

struct Items {
    inherited: Vec<InheritedItem>,
    new: NewItem,
}
impl Parse for Items {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut inherited = vec![];

        assert!(util::non_user_braced_id(input, "inherit").is_empty());

        while !input.is_empty() {
            match input.parse::<DeclareItem>().unwrap_or_else(|e| non_user_error!(e)) {
                DeclareItem::Inherited(i) => inherited.push(i),
                DeclareItem::New(new) => {
                    assert!(input.is_empty());
                    return Ok(Items { inherited, new });
                }
            }
        }
        unreachable!("expected last item to be `new { .. }`")
    }
}

enum DeclareItem {
    Inherited(InheritedItem),
    New(NewItem),
}
impl Parse for DeclareItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(keyword::inherited) {
            let _ = input.parse::<keyword::inherited>();
            util::non_user_braced(input).parse::<InheritedItem>().map(DeclareItem::Inherited)
        } else if input.peek(keyword::new) {
            let _ = input.parse::<keyword::new>();
            util::non_user_braced(input).parse::<NewItem>().map(DeclareItem::New)
        } else {
            non_user_error!("expected `inherited { .. }` or `new { .. }`")
        }
    }
}

/// Inherited widget or mixin data.
struct InheritedItem {}
impl Parse for InheritedItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        todo!()
    }
}

/// New widget or mixin data.
struct NewItem {}
impl Parse for NewItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        todo!()
    }
}

mod keyword {
    syn::custom_keyword!(inherited);
    syn::custom_keyword!(new);
}
