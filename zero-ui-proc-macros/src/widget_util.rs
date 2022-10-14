use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    ext::IdentExt,
    parse::{discouraged::Speculative, Parse},
    punctuated::Punctuated,
    spanned::Spanned,
    *,
};

use crate::util::{self, parse_outer_attrs, parse_punct_terminated2, peek_any3, ErrorRecoverable, Errors};

/// Represents a property assign.
pub struct WgtProperty {
    /// Attributes.
    pub attrs: Vec<Attribute>,
    /// Path to property.
    pub path: Path,
    /// Optional property rename.
    pub rename: Option<(Token![as], Ident)>,
    /// Optional value, if not defined the property must be assigned to its own name.
    pub value: Option<(Token![=], PropertyValue)>,
    /// Optional terminator.
    pub semi: Option<Token![;]>,
}
impl WgtProperty {
    /// Gets the property name.
    pub fn ident(&self) -> &Ident {
        if let Some((_, id)) = &self.rename {
            id
        } else {
            &self.path.segments.last().unwrap().ident
        }
    }

    /// Returns `true` if the property does not rename and the path a a single ident.
    pub(crate) fn is_ident(&self) -> bool {
        self.rename.is_none() && self.path.get_ident().is_some()
    }

    /// Generate PropertyId init code.
    pub fn property_id(&self) -> TokenStream {
        let path = &self.path;
        let ident = self.ident();
        quote! {
            #path::Args::__id__(stringify!(#ident))
        }
    }

    /// Gets if this property is assigned `unset!`.
    pub fn is_unset(&self) -> bool {
        if let Some((_, PropertyValue::Special(special, _))) = &self.value {
            special == "unset"
        } else {
            false
        }
    }

    /// Gets if this property has args.
    pub fn has_default(&self) -> bool {
        matches!(&self.value, Some((_, PropertyValue::Unnamed(_) | PropertyValue::Named(_, _))))
    }

    /// Gets if this property is marked `#[required]`.
    pub fn is_required(&self) -> bool {
        for attr in &self.attrs {
            if let Some(id) = attr.path.get_ident() {
                if id == "required" {
                    return true;
                }
            }
        }
        false
    }

    /// Gets the property args new code.
    pub fn args_new(&self, property_mod: TokenStream) -> TokenStream {
        let path = &self.path;
        let ident = self.ident();
        let instance = quote_spanned! {property_mod.span()=>
            #property_mod::PropertyInstInfo {
                name: stringify!(#ident),
                location: #property_mod::source_location!(),
            }
        };
        if let Some((_, val)) = &self.value {
            match val {
                PropertyValue::Special(_, _) => quote!(),
                PropertyValue::Unnamed(args) => quote! {
                    #path::Args::__new__(#instance, #args)
                },
                PropertyValue::Named(_, args) => {
                    let mut idents_sorted: Vec<_> = args.iter().map(|f| &f.ident).collect();
                    idents_sorted.sort();
                    let idents = args.iter().map(|f| &f.ident);
                    let exprs = args.iter().map(|f| &f.expr);
                    let errors = args.iter().map(|f| {
                        let msg = format!("unknown input `{}`", f.ident);
                        quote_spanned! {f.ident.span()=>
                            std::compile_error!(#msg);
                        }
                    });
                    quote! {
                        {
                            #(
                                #path::code_gen! {
                                    if #idents {
                                        let #idents = #path::Args::#idents(#exprs);
                                    }
                                }
                                #path::code_gen! {
                                    if !#idents {
                                        #errors
                                    }
                                }
                            )*

                            #path::code_gen! {
                                <#path>::__new__(#instance, #(#idents_sorted),*)
                            }
                        }
                    }
                }
            }
        } else {
            let ident = self.ident();
            quote! {
                #path::Args::__new__(#instance, #ident)
            }
        }
    }
}
impl Parse for WgtProperty {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
        let path = input.parse()?;
        let rename = if input.peek(Token![as]) {
            Some((input.parse()?, input.parse()?))
        } else {
            None
        };
        let value = if input.peek(Token![=]) {
            Some((input.parse()?, input.parse()?))
        } else {
            None
        };
        Ok(WgtProperty {
            attrs,
            path,
            rename,
            value,
            semi: input.parse()?,
        })
    }
}

pub struct PropertyField {
    pub ident: Ident,
    pub colon: Token![:],
    pub expr: Expr,
}
impl Parse for PropertyField {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(PropertyField {
            ident: input.parse()?,
            colon: input.parse()?,
            expr: input.parse()?,
        })
    }
}

// Value assigned in a [`PropertyAssign`].
pub enum PropertyValue {
    /// `unset!`.
    Special(Ident, Token![!]),
    /// `arg0, arg1,`
    Unnamed(Punctuated<Expr, Token![,]>),
    /// `{ field0: true, field1: false, }`
    Named(syn::token::Brace, Punctuated<PropertyField, Token![,]>),
}
impl Parse for PropertyValue {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) && input.peek2(Token![!]) && (input.peek3(Token![;]) || input.peek3(Ident::peek_any) || !peek_any3(input)) {
            let r = PropertyValue::Special(input.parse().unwrap(), input.parse().unwrap());
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
                        util::recoverable_err(fields_brace.span, e)
                    } else {
                        e.set_recoverable()
                    }
                })?;
                return Ok(PropertyValue::Named(fields_brace, r));
            }
        }

        // only valid option left is a sequence of "{expr},", we want to parse
        // in a recoverable way, so first we take raw token trees until we find the
        // end "`;` | EOF" or we find the start of a new property, when or remove item.
        let mut args_input = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![;]) {
            if peek_next_wgt_item(&input.fork()) {
                break;
            }
            input.parse::<TokenTree>().unwrap().to_tokens(&mut args_input);
        }

        let r = util::parse_punct_terminated2(args_input).map_err(|e| e.set_recoverable())?;
        Ok(PropertyValue::Unnamed(r))
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
    if lookahead.peek(Ident) {
        if lookahead.peek2(Token![::]) {
            let _ = lookahead.parse::<Path>();
        } else {
            let ident = lookahead.parse::<Ident>().unwrap();

            if lookahead.peek(token::Brace) {
                return ident == "remove"; // remove { .. }
            }
        }

        return lookahead.peek(Token![=]) && !lookahead.peek(Token![==]);
    }

    false
}

pub mod keyword {
    syn::custom_keyword!(when);
    syn::custom_keyword!(remove);
}

pub struct WgtWhen {
    pub attrs: Vec<Attribute>,
    pub when: keyword::when,
    pub condition_expr: TokenStream,
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
        let condition_expr = crate::expr_var::parse_without_eager_brace(input);

        let (brace_token, assigns) = if input.peek(syn::token::Brace) {
            let (brace, inner) = util::parse_braces(input).unwrap();
            let mut assigns = vec![];
            while !inner.is_empty() {
                let attrs = parse_outer_attrs(&inner, errors);

                if !(inner.peek(Ident) || inner.peek(Token![super]) || inner.peek(Token![self])) {
                    errors.push("expected property path", if inner.is_empty() { brace.span } else { inner.span() });
                    while !(inner.is_empty()
                        || inner.peek(Ident)
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
                        p.attrs = attrs;
                        if !inner.is_empty() && p.semi.is_none() {
                            errors.push("expected `,`", inner.span());
                            while !(inner.is_empty()
                                || input.peek(Ident)
                                || input.peek(Token![crate])
                                || input.peek(Token![super])
                                || input.peek(Token![self])
                                || inner.peek(Token![#]) && inner.peek(token::Bracket))
                            {
                                // skip to next property.
                                let _ = inner.parse::<TokenTree>();
                            }
                        }
                        assigns.push(p);
                    }
                    Err(e) => {
                        let (recoverable, e) = e.recoverable();
                        if util::span_is_call_site(e.span()) {
                            errors.push(e, brace.span);
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
            errors.push("expected a block of properties", util::last_span(condition_expr));
            return None;
        };

        Some(WgtWhen {
            attrs: vec![], // must be parsed before.
            when,
            condition_expr,
            brace_token,
            assigns,
        })
    }
}

pub fn parse_remove(input: parse::ParseStream, removes: &mut Vec<Ident>, errors: &mut Errors) {
    let input = non_user_braced!(input, "remove");
    while !input.is_empty() {
        if input.peek2(Token![::])
            && (input.peek(Ident) || input.peek(Token![crate]) || input.peek(Token![super]) || input.peek(Token![self]))
        {
            if let Ok(p) = input.parse::<Path>() {
                errors.push("expected inherited property ident, found path", p.span());
                let _ = input.parse::<Token![;]>();
            }
        }
        match input.parse::<Ident>() {
            Ok(ident) => {
                if input.is_empty() {
                    // found valid last item
                    removes.push(ident);
                    break;
                } else {
                    match input.parse::<Token![;]>() {
                        Ok(_) => {
                            // found valid item
                            removes.push(ident);
                            continue;
                        }
                        Err(e) => errors.push_syn(e),
                    }
                }
            }
            Err(e) => errors.push("expected inherited property ident", e.span()),
        }

        // seek next valid item
        while !(input.is_empty() || input.peek(Ident) && input.peek2(Token![;])) {
            input.parse::<TokenTree>().unwrap();
        }
    }
}
