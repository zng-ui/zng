use std::{collections::HashMap, fmt, mem};

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    ext::IdentExt,
    parse::{Parse, discouraged::Speculative},
    punctuated::Punctuated,
    spanned::Spanned,
    *,
};

use crate::{
    util::{self, Attributes, ErrorRecoverable, Errors, parse_outer_attrs, parse_punct_terminated2, path_span, peek_any3},
    wgt_property_attrs::PropertyAttrData,
};

/// Represents a property assign or when block.
pub enum WgtItem {
    Property(WgtProperty),
    When(WgtWhen),
}

/// Represents a property assign.
pub struct WgtProperty {
    /// Attributes.
    pub attrs: Attributes,
    /// Path to property.
    pub path: Path,
    /// The ::<T> part of the path, if present it is removed from `path`.
    pub generics: TokenStream,
    /// Optional value, if not defined the property must be assigned to its own name.
    pub value: Option<(Token![=], PropertyValue)>,
    /// Optional terminator.
    pub semi: Option<Token![;]>,
}
impl WgtProperty {
    /// Gets the property name.
    pub fn ident(&self) -> &Ident {
        &self.path.segments.last().unwrap().ident
    }

    /// Gets if this property is assigned `unset!`.
    pub fn is_unset(&self) -> bool {
        if let Some((_, PropertyValue::Special(special, _))) = &self.value {
            special == "unset"
        } else {
            false
        }
    }

    /// If `custom_attrs_expand` is needed.
    pub fn has_custom_attrs(&self) -> bool {
        !self.attrs.others.is_empty()
    }

    /// Gets custom attributes expansion data if any was present in the property.
    pub fn custom_attrs_expand(&self, builder: Ident, is_when: bool) -> TokenStream {
        debug_assert!(self.has_custom_attrs());

        let attrs = &self.attrs.others;

        let path_slug = util::display_path(&self.path).replace("::", "_");
        PropertyAttrData {
            pending_attrs: attrs.clone(),
            data_ident: ident!("__property_custom_expand_data_p_{}__", path_slug),
            builder,
            is_unset: self.is_unset(),
            is_when,
            property: self.path.clone(),
        }
        .to_token_stream()
    }
}
impl Parse for WgtProperty {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let path: Path = input.parse()?;
        if input.peek(Token![!]) {
            // cause error.
            input.parse::<Token![=]>()?;
        }
        let (path, generics) = split_path_generics(path)?;

        let value = if input.peek(Token![=]) {
            Some((input.parse()?, input.parse()?))
        } else {
            None
        };

        Ok(WgtProperty {
            attrs: Attributes::new(attrs),
            path,
            generics,
            value,
            semi: input.parse()?,
        })
    }
}

pub(crate) fn split_path_generics(mut path: Path) -> Result<(Path, TokenStream)> {
    path.leading_colon = None;
    if let Some(s) = path.segments.last_mut() {
        let mut generics = quote!();
        match mem::replace(&mut s.arguments, PathArguments::None) {
            PathArguments::None => {}
            PathArguments::AngleBracketed(p) => {
                generics = p.to_token_stream();
            }
            PathArguments::Parenthesized(p) => return Err(syn::Error::new(p.span(), "expected property path or generics")),
        }
        Ok((path, generics))
    } else {
        Err(syn::Error::new(path.span(), "expected property ident in path"))
    }
}

pub struct PropertyField {
    pub ident: Ident,
    #[expect(dead_code)]
    pub colon: Token![:],
    pub expr: TokenStream,
}
impl Parse for PropertyField {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        let colon;
        let expr;
        if input.peek(Token![:]) {
            colon = input.parse()?;
            expr = {
                let mut t = quote!();
                while !input.is_empty() {
                    if input.peek(Token![,]) {
                        break;
                    }
                    let tt = input.parse::<TokenTree>().unwrap();
                    tt.to_tokens(&mut t);
                }
                t
            };
        } else {
            colon = parse_quote!(:);
            expr = quote!(#ident);
        };

        Ok(PropertyField { ident, colon, expr })
    }
}

// Value assigned in a [`PropertyAssign`].
pub enum PropertyValue {
    /// `unset!`.
    Special(Ident, #[expect(dead_code)] Token![!]),
    /// `arg0, arg1,`
    Unnamed(TokenStream),
    /// `{ field0: true, field1: false, }`
    Named(#[expect(dead_code)] syn::token::Brace, Punctuated<PropertyField, Token![,]>),
}
impl Parse for PropertyValue {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) && input.peek2(Token![!]) && (input.peek3(Token![;]) || input.peek3(Ident::peek_any) || !peek_any3(input)) {
            let ident: Ident = input.parse().unwrap();
            if ident != "unset" {
                return Err(Error::new(ident.span(), "unknown special value, expected `unset!`"));
            }
            let r = PropertyValue::Special(ident, input.parse().unwrap());
            return Ok(r);
        }

        if input.peek(token::Brace) && !input.peek2(Token![,]) {
            // Differentiating between a fields declaration and a single unnamed arg declaration gets tricky.
            //
            // This is a normal fields decl.: `{ field0: "value" }`
            // This is a block single argument decl.: `{ foo(); bar() }`
            //
            // Fields can use the shorthand field name only `{ field0 }`
            // witch is also a single arg block expression. In this case
            // we parse as Unnamed, if it was a field it will still work because
            // we only have one field.

            let maybe_fields = input.fork();
            let fields_input;
            let fields_brace = braced!(fields_input in maybe_fields);

            if fields_input.peek(Ident)
                && (
                    // ident:
                    (fields_input.peek2(Token![:]) && !fields_input.peek2(Token![::]))
                    // OR ident,
                    || fields_input.peek2(Token![,])
                )
            {
                // it is fields
                input.advance_to(&maybe_fields);

                // disconnect syn internal errors
                let fields_input = fields_input.parse::<TokenStream>().unwrap();
                let r = parse_punct_terminated2(fields_input).map_err(|e| {
                    if util::span_is_call_site(e.span()) {
                        util::recoverable_err(fields_brace.span.join(), e)
                    } else {
                        e.set_recoverable()
                    }
                })?;
                return Ok(PropertyValue::Named(fields_brace, r));
            }
        }

        // only valid option left is a sequence of "{expr},", we want to parse
        // in a recoverable way, so first we take raw token trees until we find the
        // end "`;` | EOF" or we find the start of a new property or when item.
        let mut args_input = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![;]) {
            if peek_next_wgt_item(&input.fork()) {
                break;
            }
            input.parse::<TokenTree>().unwrap().to_tokens(&mut args_input);
        }

        Ok(PropertyValue::Unnamed(args_input))
    }
}

fn peek_next_wgt_item(lookahead: parse::ParseStream) -> bool {
    let has_attr = lookahead.peek(Token![#]) && lookahead.peek(token::Bracket);
    if has_attr {
        let _ = parse_outer_attrs(lookahead, &mut Errors::default());
    }
    if lookahead.peek(keyword::when) {
        return true; // when ..
    }

    if lookahead.peek(Token![pub]) {
        let _ = lookahead.parse::<Visibility>();
    }
    if lookahead.peek(Ident) {
        if lookahead.peek2(Token![::]) {
            let _ = lookahead.parse::<Path>();
        } else {
            let _ = lookahead.parse::<Ident>().unwrap();
        }

        return lookahead.peek(Token![=]) && !lookahead.peek(Token![==]);
    }

    false
}

pub mod keyword {
    syn::custom_keyword!(when);
}

pub struct WgtWhen {
    pub attrs: Attributes,
    #[expect(dead_code)]
    pub when: keyword::when,
    pub condition_expr: TokenStream,
    pub condition_expr_str: String,
    #[expect(dead_code)]
    pub brace_token: syn::token::Brace,
    pub assigns: Vec<WgtProperty>,
}
impl WgtWhen {
    /// Call only if peeked `when`. Parse outer attribute before calling.
    pub fn parse(input: parse::ParseStream, errors: &mut Errors) -> Option<WgtWhen> {
        let when = input.parse::<keyword::when>().unwrap_or_else(|e| non_user_error!(e));

        if input.is_empty() {
            errors.push("expected when expression", when.span());
            return None;
        }
        let condition_expr = parse_without_eager_brace(input);

        let (brace_token, assigns) = if input.peek(syn::token::Brace) {
            let (brace, inner) = util::parse_braces(input).unwrap();
            let mut assigns = vec![];
            while !inner.is_empty() {
                let attrs = parse_outer_attrs(&inner, errors);

                if !(inner.peek(Ident::peek_any) || inner.peek(Token![super]) || inner.peek(Token![self])) {
                    errors.push(
                        "expected property path",
                        if inner.is_empty() { brace.span.join() } else { inner.span() },
                    );
                    while !(inner.is_empty()
                        || inner.peek(Ident::peek_any)
                        || inner.peek(Token![super])
                        || inner.peek(Token![self])
                        || inner.peek(Token![#]) && inner.peek(token::Bracket))
                    {
                        // skip to next property.
                        let _ = inner.parse::<TokenTree>();
                    }
                }
                if inner.is_empty() {
                    break;
                }

                match inner.parse::<WgtProperty>() {
                    Ok(mut p) => {
                        p.attrs = Attributes::new(attrs);
                        if !inner.is_empty() && p.semi.is_none() {
                            errors.push("expected `,`", inner.span());
                            while !(inner.is_empty()
                                || input.peek(Ident::peek_any)
                                || input.peek(Token![crate])
                                || input.peek(Token![super])
                                || input.peek(Token![self])
                                || inner.peek(Token![#]) && inner.peek(token::Bracket))
                            {
                                // skip to next property.
                                let _ = inner.parse::<TokenTree>();
                            }
                        }

                        if let Some((_, PropertyValue::Special(s, _))) = &p.value {
                            errors.push(format!("cannot {s} in when assign"), s.span());
                        }

                        assigns.push(p);
                    }
                    Err(e) => {
                        let (recoverable, e) = e.recoverable();
                        if util::span_is_call_site(e.span()) {
                            errors.push(e, brace.span.join());
                        } else {
                            errors.push_syn(e);
                        }
                        if !recoverable {
                            break;
                        }
                    }
                }
            }
            (brace, assigns)
        } else {
            errors.push("expected a when block expr and properties", util::last_span(condition_expr));
            return None;
        };

        let expr_str = condition_expr.to_string();
        // normalize #
        let expr_str = expr_str.replace(" # ", "#").replace(" #", "#").replace("# ", "#");
        // convert to valid Rust code
        let expr_str = expr_str.replace("#{", "__pound__!{").replace("#", "__pound__");
        // format
        let expr_str = util::format_rust_expr(expr_str);
        // convert back to special syntax
        let expr_str = expr_str.replace("__pound__! {", "#{").replace("__pound__", "#");
        // collapse to single line
        let expr_str = util::undo_line_wrap(&expr_str);
        // prettyplease error
        let expr_str = expr_str.replace("crate ::", "crate::");

        Some(WgtWhen {
            attrs: Attributes::new(vec![]), // must be parsed before.
            when,
            condition_expr_str: expr_str,
            condition_expr,
            brace_token,
            assigns,
        })
    }
}

/// Like [`syn::Expr::parse_without_eager_brace`] but does not actually parse anything and includes
/// the braces of interpolation.
fn parse_without_eager_brace(input: parse::ParseStream) -> TokenStream {
    let mut r = TokenStream::default();
    let mut is_start = true;
    while !input.is_empty() {
        if input.peek(Token![match]) || input.peek(Token![while]) {
            // keyword
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            // expr
            r.extend(parse_without_eager_brace(input));
            // block
            if input.peek(token::Brace) {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            }
        } else if input.peek(Token![if]) {
            // keyword
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            // expr
            r.extend(parse_without_eager_brace(input));
            // block
            if input.peek(token::Brace) {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);

                if input.peek(Token![else]) {
                    input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                    if input.peek(token::Brace) {
                        // else { }
                        input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                    } else {
                        // maybe another if
                        continue;
                    }
                }
            }
        } else if input.peek(Token![loop]) {
            // keyword
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            // block
            if input.peek(token::Brace) {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            }
        } else if input.peek(Token![for]) {
            // keyword (for)
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            while !input.is_empty() && !input.peek(Token![in]) {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            }
            if !input.is_empty() {
                // keyword (in)
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                //expr
                r.extend(parse_without_eager_brace(input));
                // block
                if input.peek(token::Brace) {
                    input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                }
            }
        } else if input.peek2(token::Brace) {
            if input.peek(Token![#]) {
                // #
                let tt = input.parse::<TokenTree>().unwrap();
                tt.to_tokens(&mut r);
                // { .. }
                let tt = input.parse::<TokenTree>().unwrap();
                tt.to_tokens(&mut r);

                if input.peek(token::Brace) {
                    break; // found { } after expr or Struct #{ }
                }
            } else {
                // item before brace
                let tt = input.parse::<TokenTree>().unwrap();
                tt.to_tokens(&mut r);
                break;
            }
        } else if !is_start && input.peek(token::Brace) {
            break; // found { } after expr
        } else {
            let tt = input.parse::<TokenTree>().unwrap();
            tt.to_tokens(&mut r);
        }
        is_start = false;
    }
    r
}

#[derive(PartialEq, Eq, Hash)]
pub(crate) enum WhenInputMember {
    Named(Ident),
    Index(usize),
}
impl fmt::Display for WhenInputMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhenInputMember::Named(n) => write!(f, "{n}"),
            WhenInputMember::Index(i) => write!(f, "{i}"),
        }
    }
}
impl ToTokens for WhenInputMember {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            WhenInputMember::Named(ident) => ident.to_tokens(tokens),
            WhenInputMember::Index(i) => i.to_tokens(tokens),
        }
    }
}

pub(crate) struct WhenExpr {
    /// Map of `(property_path, member) => var_name`, example: `(id, 0) => __w_id__0`.
    pub inputs: HashMap<(syn::Path, WhenInputMember), Ident>,
    pub expr: TokenStream,
}
impl Parse for WhenExpr {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        let mut inputs = HashMap::new();
        let mut expr = TokenStream::default();

        while !input.is_empty() {
            if input.peek(Token![#]) && input.peek2(Ident) {
                let tt = input.parse::<Token![#]>().unwrap();
                let last_span = tt.span();

                let property = input.parse::<Path>().map_err(|e| {
                    if util::span_is_call_site(e.span()) {
                        syn::Error::new(last_span, e)
                    } else {
                        e
                    }
                })?;

                let path_slug = util::display_path(&property).replace("::", "_");

                let mut member = WhenInputMember::Index(0);
                let mut var_ident = ident_spanned!(path_span(&property)=> "w_{path_slug}_m_0");
                if input.peek(Token![.]) && !input.peek2(Token![await]) && !input.peek3(token::Paren) {
                    let _: Token![.] = input.parse()?;
                    if input.peek(Ident) {
                        let m = input.parse::<Ident>().unwrap();
                        var_ident = ident_spanned!(m.span()=> "w_{path_slug}_m_{m}");
                        member = WhenInputMember::Named(m);
                    } else {
                        let index = input.parse::<syn::Index>().map_err(|e| {
                            let span = if util::span_is_call_site(e.span()) { last_span } else { e.span() };

                            syn::Error::new(span, "expected identifier or index")
                        })?;
                        member = WhenInputMember::Index(index.index as usize);
                        var_ident = ident_spanned!(index.span()=> "w_{path_slug}_m_{}", index.index);
                    }
                }

                expr.extend(quote_spanned! {var_ident.span()=>
                    #{ #var_ident }
                });

                inputs.insert((property, member), var_ident);
            }
            // recursive parse groups:
            else if input.peek(token::Brace) {
                let inner;
                let group = syn::braced!(inner in input);
                let inner = WhenExpr::parse(&inner)?;
                inputs.extend(inner.inputs);
                group.surround(&mut expr, |e| e.extend(inner.expr));
            } else if input.peek(token::Paren) {
                let inner;
                let group = syn::parenthesized!(inner in input);
                let inner = WhenExpr::parse(&inner)?;
                inputs.extend(inner.inputs);
                group.surround(&mut expr, |e| e.extend(inner.expr));
            } else if input.peek(token::Bracket) {
                let inner;
                let group = syn::bracketed!(inner in input);
                let inner = WhenExpr::parse(&inner)?;
                inputs.extend(inner.inputs);
                group.surround(&mut expr, |e| e.extend(inner.expr));
            }
            // keep other tokens the same:
            else {
                let tt = input.parse::<TokenTree>().unwrap();
                tt.to_tokens(&mut expr)
            }
        }

        Ok(WhenExpr { inputs, expr })
    }
}

pub struct WidgetCustomRules {
    pub rules: Vec<WidgetCustomRule>,
}
impl Parse for WidgetCustomRules {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let inner;
        braced!(inner in input);
        let mut rules = vec![];
        while !inner.is_empty() {
            rules.push(inner.parse()?);
        }
        Ok(WidgetCustomRules { rules })
    }
}

/// Represents a custom widget macro rule.
pub struct WidgetCustomRule {
    /// Rule tokens, `(<rule>) => { .. };`.
    pub rule: TokenStream,
    /// Init tokens, `(..) => { <init> };`
    pub init: TokenStream,
}
impl Parse for WidgetCustomRule {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let rule;
        parenthesized!(rule in input);
        let rule = rule.parse()?;

        let _ = input.parse::<Token![=>]>()?;

        let init;
        braced!(init in input);
        let init = init.parse()?;

        if input.peek(Token![;]) {
            let _ = input.parse::<Token![;]>();
        };

        Ok(WidgetCustomRule { rule, init })
    }
}

// expansion of `macro_rules! source_location`
pub fn source_location(crate_core: &TokenStream, location: Span) -> TokenStream {
    let source_location = quote_spanned! {location=>
        #crate_core::widget::builder::SourceLocation::new(
            std::file!(), std::line!(), std::column!(),
        )
    };
    util::set_stream_span(source_location, location)
}
