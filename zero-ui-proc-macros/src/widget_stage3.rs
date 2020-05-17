use crate::util;
use crate::{widget_new::BuiltPropertyKind, widget_stage1::WidgetHeader};
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{parse::*, punctuated::Punctuated, *};
use uuid::Uuid;

/// `widget!` actual expansion, in stage3 we have all the inherited tokens to work with.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}

pub mod keyword {
    syn::custom_keyword!(default_child);
    syn::custom_keyword!(required);
    syn::custom_keyword!(unset);
    syn::custom_keyword!(when);
    syn::custom_keyword!(new);
    syn::custom_keyword!(new_child);
    syn::custom_keyword!(inherit);
    syn::custom_keyword!(inherit_next);
}

struct PropertyBlock<P> {
    brace_token: token::Brace,
    properties: Vec<P>,
}
impl<P: Parse> Parse for PropertyBlock<P> {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        let brace_token = braced!(inner in input);
        let mut properties = Vec::new();
        while !inner.is_empty() {
            properties.push(inner.parse()?);
        }
        Ok(PropertyBlock { brace_token, properties })
    }
}

struct WidgetDeclaration {
    header: WidgetHeader,
    items: Vec<WgtItem>,
}

impl Parse for WidgetDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        let header = input.parse().expect(util::NON_USER_ERROR);
        let mut items = Vec::new();
        while !input.is_empty() {
            todo!()
        }
        Ok(WidgetDeclaration { header, items })
    }
}

enum WgtItem {
    Default(WgtItemDefault),
    New(WgtItemNew),
}

struct WgtItemDefault {
    target: DefaultTarget,
    block: DefaultBlock,
}

impl Parse for WgtItemDefault {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(WgtItemDefault {
            target: input.parse()?,
            block: input.parse()?,
        })
    }
}

enum DefaultTarget {
    Default(Token![default]),
    DefaultChild(keyword::default_child),
}
impl Parse for DefaultTarget {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![default]) {
            Ok(DefaultTarget::Default(input.parse().unwrap()))
        } else {
            Ok(DefaultTarget::DefaultChild(input.parse()?))
        }
    }
}

type DefaultBlock = PropertyBlock<PropertyDeclaration>;

struct PropertyDeclaration;
impl Parse for PropertyDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        todo!()
    }
}

struct WgtItemNew {
    attrs: Vec<Attribute>,
    fn_token: Token![fn],
    target: NewTarget,
    paren_token: token::Paren,
    inputs: Punctuated<Ident, Token![,]>,
    r_arrow_token: Token![->],
    return_type: Type,
    block: Block,
}

impl Parse for WgtItemNew {
    fn parse(input: ParseStream) -> Result<Self> {
        let inputs;
        Ok(WgtItemNew {
            attrs: Attribute::parse_outer(input)?,
            fn_token: input.parse()?,
            target: input.parse()?,
            paren_token: parenthesized!(inputs in input),
            inputs: Punctuated::parse_terminated(&inputs)?,
            r_arrow_token: input.parse()?,
            return_type: input.parse()?,
            block: input.parse()?,
        })
    }
}

enum NewTarget {
    New(keyword::new),
    NewChild(keyword::new_child),
}

impl Parse for NewTarget {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(keyword::new) {
            Ok(NewTarget::New(input.parse().unwrap()))
        } else {
            Ok(NewTarget::NewChild(input.parse()?))
        }
    }
}

struct WgtItemWhen {
    attrs: Vec<Attribute>,
    when_token: keyword::when,
    condition: Expr,
    block: WhenBlock,
}

impl Parse for WgtItemWhen {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
        todo!()
    }
}

type WhenBlock = PropertyBlock<PropertyAssign>;

struct PropertyAssign;
impl Parse for PropertyAssign {
    fn parse(input: ParseStream) -> Result<Self> {
        todo!()
    }
}