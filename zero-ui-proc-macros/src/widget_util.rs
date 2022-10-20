use std::{collections::HashMap, fmt, mem};

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
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
    /// The ::<T> part of the path, if present it is removed from `path`.
    pub generics: TokenStream,
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

    /// Generate PropertyId init code.
    pub fn property_id(&self) -> TokenStream {
        let path = &self.path;
        let ident = self.ident();
        let ident_str = ident.to_string();
        quote! {
            #path::property::__id__(#ident_str)
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

    fn location_span(&self) -> Span {
        // if we just use the path span, go to rust-analyzer go-to-def gets confused.
        if let Some((eq, _)) = &self.value {
            eq.span()
        } else if let Some((as_, _)) = &self.rename {
            as_.span()
        } else if let Some(s) = &self.semi {
            s.span()
        } else {
            self.path.span()
        }
    }

    /// Gets the property args new code.
    pub fn args_new(&self, property_mod: TokenStream) -> TokenStream {
        let path = &self.path;
        let generics = &self.generics;
        let ident = self.ident();
        let ident_str = ident.to_string();
        let instance = quote_spanned! {self.location_span()=>
            #property_mod::PropertyInstInfo {
                name: #ident_str,
                location: #property_mod::source_location!(),
            }
        };
        if let Some((_, val)) = &self.value {
            match val {
                PropertyValue::Special(_, _) => quote!(),
                PropertyValue::Unnamed(args) => quote_spanned! {path.span()=>
                    #path::property #generics::__new__(#args).__build__(#instance)
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
                    let inputs_len = idents_sorted.len();

                    quote_spanned! {path.span()=>
                        {
                            #path::code_gen! {
                                if !inputs_len(#inputs_len) {
                                    std::compile_error!("incorrect inputs");
                                }
                            }
                            #(
                                #path::code_gen! {
                                    if input(#idents) {
                                        let #idents = #path::property #generics::#idents(#exprs);
                                    }
                                }
                                #path::code_gen! {
                                    if !input(#idents) {
                                        #errors
                                    }
                                }
                            )*

                            #path::code_gen! {
                                {#path::property #generics}::__new__(#(#idents_sorted),*)
                            }.__build__(#instance)
                        }
                    }
                }
            }
        } else {
            let ident = self.ident();
            quote! {
                #path::property #generics::__new__(#ident).__build__(#instance)
            }
        }
    }
}
impl Parse for WgtProperty {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let path: Path = input.parse()?;
        let (path, generics) = split_path_generics(path)?;

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
            generics,
            rename,
            value,
            semi: input.parse()?,
        })
    }
}

fn split_path_generics(mut path: Path) -> Result<(Path, TokenStream)> {
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
    pub colon: Token![:],
    pub expr: TokenStream,
}
impl Parse for PropertyField {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(PropertyField {
            ident: input.parse()?,
            colon: input.parse()?,
            expr: {
                let mut t = quote!();
                while !input.is_empty() {
                    if input.peek(Token![,]) {
                        break;
                    }
                    let tt = input.parse::<TokenTree>().unwrap();
                    tt.to_tokens(&mut t);
                }
                t
            },
        })
    }
}

// Value assigned in a [`PropertyAssign`].
pub enum PropertyValue {
    /// `unset!`.
    Special(Ident, Token![!]),
    /// `arg0, arg1,`
    Unnamed(TokenStream),
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

    /// Expand to a when struct
    pub fn when_new(&self, property_mod: TokenStream) -> TokenStream {
        let when_expr = match syn::parse2::<WhenExpr>(self.condition_expr.clone()) {
            Ok(w) => w,
            Err(e) => {
                let mut errors = Errors::default();
                errors.push_syn(e);
                return errors.to_token_stream();
            }
        };

        let mut var_decl = quote!();
        let mut inputs = quote!();

        for ((property, member), var) in when_expr.inputs {
            let (property, generics) = split_path_generics(property).unwrap();
            let var_input = ident!("{var}_in");

            let unknwon_member_err = format!("unknown member `{}`", member);
            let not_gettable_err = format!("member `{}` cannot be used in when", member);
            var_decl.extend(quote_spanned! {property.span()=>
                #property::code_gen! {
                    if input(#member) {
                        #property::code_gen! {
                            if !allowed_in_when(#member) {
                                std::compile_error!{ #not_gettable_err }
                            }
                        }

                        let (#var_input, #var) = #property::code_gen! {
                            {#property::property #generics}::when_input(#member)
                        };
                    }
                }
                #property::code_gen! {
                    if !input(#member) {
                        std::compile_error!{ #unknwon_member_err }
                    }
                }
            });

            let p_ident = &property.segments.last().unwrap().ident;
            let member = match member {
                WhenInputMember::Named(ident) => {
                    let ident_str = ident.to_string();
                    quote! {
                        Named(#ident_str)
                    }
                }
                WhenInputMember::Index(i) => quote! {
                    Index(#i)
                },
            };
            inputs.extend(quote! {
                #property_mod::WhenInput {
                    property: #property::property::__id__(std::stringify!(#p_ident)),
                    member: #property_mod::WhenInputMember::#member,
                    var: #var_input,
                },
            });
        }

        let mut assigns = quote!();
        for a in &self.assigns {
            let args = a.args_new(property_mod.clone());
            assigns.extend(quote! {
                #args,
            });
        }

        let expr = when_expr.expr;
        let expr_str = util::format_rust_expr(self.condition_expr.to_string());

        quote! {
            {
                #var_decl
                #property_mod::WhenInfo {
                    inputs: std::boxed::Box::new([
                        #inputs
                    ]),
                    state: #property_mod::when_condition_expr_var! { #expr },
                    assigns: std::boxed::Box::new([
                        #assigns
                    ]),
                    expr: #expr_str,
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum WhenInputMember {
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

struct WhenExpr {
    /// Map of `(property_path, member) => var_name`, example: `(id, 0) => __w_id__0`.
    pub inputs: HashMap<(syn::Path, WhenInputMember), Ident>,
    pub expr: TokenStream,
}
impl WhenExpr {
    fn parse_inner(input: parse::ParseStream) -> syn::Result<Self> {
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
                let mut var_ident = ident_spanned!(property.span()=> "w_{path_slug}_m_0");
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
                let inner = WhenExpr::parse_inner(&non_user_braced!(input))?;
                inputs.extend(inner.inputs);
                let inner = inner.expr;
                expr.extend(quote_spanned! {inner.span()=> { #inner } });
            } else if input.peek(token::Paren) {
                let inner = WhenExpr::parse_inner(&non_user_parenthesized!(input))?;
                inputs.extend(inner.inputs);
                let inner = inner.expr;
                expr.extend(quote_spanned! {inner.span()=> ( #inner ) });
            } else if input.peek(token::Bracket) {
                let inner = WhenExpr::parse_inner(&non_user_bracketed!(input))?;
                inputs.extend(inner.inputs);
                let inner = inner.expr;
                expr.extend(quote_spanned! {inner.span()=> [ #inner ] });
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
impl Parse for WhenExpr {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        let mut r = WhenExpr::parse_inner(input)?;
        let expr = &mut r.expr;

        // assert expression type.
        *expr = quote_spanned! {expr.span()=>
            let __result__: bool = { #expr };
            __result__
        };

        Ok(r)
    }
}
