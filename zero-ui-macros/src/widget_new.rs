use crate::util;
use crate::widget::{self, PropertyAssign, WhenBlock};
use proc_macro2::{Span, TokenStream};
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::{parse::*, *};

pub mod keyword {
    syn::custom_keyword!(m);
    syn::custom_keyword!(c);
    syn::custom_keyword!(s);
    syn::custom_keyword!(n);
    syn::custom_keyword!(i);

    syn::custom_keyword!(r);
    syn::custom_keyword!(l);
    syn::custom_keyword!(d);
}

/// `widget_new!` implementation
#[allow(clippy::cognitive_complexity)]
pub fn expand_widget_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as WidgetNewInput);

    // declarations of property arguments in the user written order.
    //let mut let_args = vec![];

    // widget child expression `=> {this}`
    let child = input.input.child_expr;

    // declarations of self property arguments.

    let r = quote! {{
        //#(#let_args)*

        let __node = #child;

        // apply child properties
        //#(#set_child_props_ctx)*
        //#(#set_child_props_event)*
        //#(#set_child_props_outer)*
        //#(#set_child_props_inner)*

        //let __node = #ident::new_child(__node, #(#new_child_args),*);

        // apply self properties
        //#(#set_self_props_ctx)*
        //#(#set_self_props_event)*
        //#(#set_self_props_outer)*
        //#(#set_self_props_inner)*

        //#ident::new(__node, #(#new_args),*)
        __node
    }};

    r.into()
}

pub struct WidgetNewInput {
    pub ident: Ident,
    default_child: BuiltDefaultBlock,
    default_self: BuiltDefaultBlock,
    new_child: Punctuated<Ident, Token![,]>,
    new: Punctuated<Ident, Token![,]>,
    input: NewWidgetInput,
}

impl Parse for WidgetNewInput {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<keyword::m>().expect(util::NON_USER_ERROR);
        let ident = input.parse().expect(util::NON_USER_ERROR);

        input.parse::<keyword::c>().expect(util::NON_USER_ERROR);
        let default_child = input.parse().expect(util::NON_USER_ERROR);

        input.parse::<keyword::s>().expect(util::NON_USER_ERROR);
        let default_self = input.parse().expect(util::NON_USER_ERROR);

        input.parse::<keyword::n>().expect(util::NON_USER_ERROR);
        let inner = util::non_user_parenthesized(input);
        let new_child = Punctuated::parse_terminated(&inner).expect(util::NON_USER_ERROR);
        let inner = util::non_user_parenthesized(input);
        let new = Punctuated::parse_terminated(&inner).expect(util::NON_USER_ERROR);

        input.parse::<keyword::i>().expect(util::NON_USER_ERROR);
        let input = input.parse()?;

        Ok(WidgetNewInput {
            ident,
            default_child,
            default_self,
            new_child,
            new,
            input,
        })
    }
}

struct BuiltDefaultBlock {
    properties: Punctuated<BuiltProperty, Token![,]>,
}

impl Parse for BuiltDefaultBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        braced!(inner in input);
        let properties = Punctuated::parse_terminated(&inner)?;
        Ok(BuiltDefaultBlock { properties })
    }
}

struct BuiltProperty {
    kind: BuiltPropertyKind,
    ident: Ident,
}

impl Parse for BuiltProperty {
    fn parse(input: ParseStream) -> Result<Self> {
        let kind = input.parse()?;
        let ident = input.parse()?;
        Ok(BuiltProperty { kind, ident })
    }
}

enum BuiltPropertyKind {
    /// Required property.
    Required,
    /// Property is provided by the widget.
    Local,
    /// Property and default is provided by the widget.
    Default,
}

impl Parse for BuiltPropertyKind {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(keyword::d) {
            input.parse::<keyword::d>()?;
            Ok(BuiltPropertyKind::Default)
        } else if input.peek(keyword::l) {
            input.parse::<keyword::l>()?;
            Ok(BuiltPropertyKind::Local)
        } else if input.peek(keyword::r) {
            input.parse::<keyword::r>()?;
            Ok(BuiltPropertyKind::Required)
        } else {
            Err(Error::new(input.span(), "expected one of: r, d, l"))
        }
    }
}

struct NewWidgetInput {
    sets: Vec<PropertyAssign>,
    whens: Vec<WhenBlock>,
    child_expr: Expr,
}

impl Parse for NewWidgetInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner = util::non_user_braced(input);

        let mut sets = vec![];
        let mut whens = vec![];

        while !inner.is_empty() {
            let lookahead = inner.lookahead1();

            // expect `when` at start or after `property:`
            if lookahead.peek(widget::keyword::when) {
                whens.push(inner.parse()?);
            }
            // expect `property:` only before `when` blocks.
            else if whens.is_empty() && lookahead.peek(Ident) {
                sets.push(inner.parse()?);
            }
            // expect `=>` to be the last item.
            else if lookahead.peek(Token![=>]) {
                inner.parse::<Token![=>]>()?;
                let child_expr = inner.parse()?;

                return Ok(NewWidgetInput { sets, whens, child_expr });
            } else {
                return Err(lookahead.error());
            }
        }

        todo!()
    }
}
