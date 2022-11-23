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

use crate::{
    util::{self, parse_outer_attrs, parse_punct_terminated2, path_span, peek_any3, Attributes, ErrorRecoverable, Errors},
    wgt_property_attrs::PropertyAttrData,
};

/// Represents a property assign.
pub struct WgtProperty {
    /// Attributes.
    pub attrs: Attributes,
    /// Reexport visibility.
    pub vis: Visibility,
    /// Path to property.
    pub path: Path,
    pub capture_decl: Option<CaptureDeclaration>,
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
        quote_spanned! {path_span(path)=>
            #path::__id__(#ident_str)
        }
    }

    pub fn is_private(&self) -> bool {
        matches!(&self.vis, Visibility::Inherited)
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
    pub fn has_args(&self) -> bool {
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

    /// Converts values to `let` bindings that are returned.
    pub fn pre_bind_args(&mut self, shorthand_init_enabled: bool, extra_attrs: Option<&Attributes>, extra_prefix: &str) -> TokenStream {
        let prefix = if let Some((_, id)) = &self.rename {
            format!("__{extra_prefix}as_{id}_")
        } else {
            let path_str = self.path.to_token_stream().to_string().replace(' ', "").replace("::", "_i_");
            format!("__{extra_prefix}p_{path_str}_")
        };

        let mut attrs = self.attrs.cfg_and_lints();
        if let Some(extra) = extra_attrs {
            attrs.extend(extra.cfg_and_lints())
        }

        let mut r = quote!();
        if let Some((eq, val)) = &mut self.value {
            match val {
                PropertyValue::Unnamed(args) => {
                    let args_exprs = mem::replace(args, quote!());
                    match syn::parse2::<UnamedArgs>(args_exprs.clone()) {
                        Ok(a) => {
                            for (i, arg) in a.args.into_iter().enumerate() {
                                let ident = ident_spanned!(eq.span()=> "{prefix}{i}__");
                                args.extend(quote!(#ident,));
                                r.extend(quote! {
                                    #attrs
                                    let #ident = {#arg};
                                });
                            }
                        }
                        Err(_) => {
                            // let natural error happen, this helps Rust-Analyzer auto-complete.
                            *args = args_exprs;
                        }
                    };
                }
                PropertyValue::Named(_, args) => {
                    for arg in args {
                        let expr = mem::replace(&mut arg.expr, quote!());
                        let ident = ident_spanned!(eq.span()=> "{prefix}{}__", arg.ident);
                        arg.expr = quote!(#ident);
                        r.extend(quote! {
                            #attrs
                            let #ident = {#expr};
                        });
                    }
                }
                PropertyValue::Special(_, _) => {}
            }
        } else if shorthand_init_enabled && (self.rename.is_some() || self.path.get_ident().is_some()) {
            let ident = self.ident().clone();
            let let_ident = ident!("{prefix}0__");
            self.value = Some((parse_quote!(=), PropertyValue::Unnamed(quote!(#let_ident))));
            r.extend(quote! {
                #attrs
                let #let_ident = #ident;
            });
        }
        r
    }

    pub fn reexport(&self) -> TokenStream {
        if self.capture_decl.is_some() {
            return quote!();
        }

        let vis = match self.vis.clone() {
            Visibility::Inherited => {
                if self.rename.is_some() || self.path.get_ident().is_none() {
                    // so at least the widget can use it after pre-bind.
                    Visibility::Inherited
                } else {
                    // already in context
                    return quote!();
                }
            }
            vis => vis,
        };

        let path = &self.path;

        let name = match &self.rename {
            Some((as_, id_)) => quote!(#as_ #id_),
            None => {
                let id_ = self.ident();
                quote_spanned!(id_.span()=> as #id_)
            }
        };
        let attrs = self.attrs.cfg_and_lints();
        let clippy_nag = quote!(#[allow(clippy::useless_attribute)]);

        quote_spanned! {self.path.span()=>
            #clippy_nag
            #attrs
            #[allow(unused_imports)]
            #[doc(inline)]
            #vis use #path #name;
        }
    }

    /// Declares capture property if it is one.
    pub fn declare_capture(&self) -> TokenStream {
        if let Some(decl) = &self.capture_decl {
            let mut errors = Errors::default();
            if !self.generics.is_empty() {
                errors.push("new capture shorthand cannot have explicit generics", self.generics.span());
            }
            if let Some((as_, _)) = &self.rename {
                errors.push("new capture properties cannot be renamed", as_.span());
            }
            if self.path.get_ident().is_none() {
                errors.push("new capture properties must have a single ident", self.path.span());
            }
            let default_args = match &self.value {
                Some((_, val)) => match val {
                    PropertyValue::Unnamed(val) => Some(val),
                    PropertyValue::Special(id, _) => {
                        errors.push("cannot `{id}` new capture property", id.span());
                        None
                    }
                    PropertyValue::Named(brace, _) => {
                        errors.push("expected unnamed default", brace.span);
                        None
                    }
                },
                None => None,
            };

            if errors.is_empty() {
                let ident = self.path.get_ident().unwrap().clone();

                let ty = &decl.ty;
                let vis = match self.vis.clone() {
                    Visibility::Inherited => {
                        // so at least the widget can get the `property_id!`.
                        parse_quote!(pub(super))
                    }
                    vis => vis,
                };
                let core = util::crate_core();

                let default = if let Some(default_args) = default_args {
                    quote! {
                        , default(#default_args)
                    }
                } else {
                    quote!()
                };

                let attrs = &self.attrs;
                quote_spanned! {ident.span()=>
                    #attrs
                    #[#core::property(CONTEXT, capture #default)]
                    #vis fn #ident(__child__: impl #core::widget_instance::UiNode, #ident: #ty) -> impl #core::widget_instance::UiNode {
                        __child__
                    }
                }
            } else {
                errors.to_token_stream()
            }
        } else {
            quote!()
        }
    }

    /// Gets the property args new code.
    pub fn args_new(&self, wgt_builder_mod: TokenStream) -> TokenStream {
        let path = &self.path;
        let generics = &self.generics;
        let ident = self.ident();
        let ident_str = ident.to_string();
        let instance = quote_spanned! {self.location_span()=>
            #wgt_builder_mod::PropertyInstInfo {
                name: #ident_str,
                location: #wgt_builder_mod::source_location!(),
            }
        };
        if let Some((_, val)) = &self.value {
            match val {
                PropertyValue::Special(_, _) => non_user_error!("no args for special value"),
                PropertyValue::Unnamed(args) => quote_spanned! {path_span(path)=>
                    #path #generics::__new__(#args).__build__(#instance)
                },
                PropertyValue::Named(_, args) => {
                    let mut idents_sorted: Vec<_> = args.iter().map(|f| &f.ident).collect();
                    idents_sorted.sort();
                    let idents = args.iter().map(|f| &f.ident);
                    let exprs = args.iter().map(|f| &f.expr);
                    quote_spanned! {path_span(path)=>
                        {
                            #(
                                let #idents = #path #generics::#idents(#exprs);
                            )*

                            #path #generics::__new_sorted__(#(#idents_sorted),*).__build__(#instance)
                        }
                    }
                }
            }
        } else {
            let ident = self.ident();
            quote! {
                #path #generics::__new__(#ident).__build__(#instance)
            }
        }
    }

    /// If `custom_attrs_expand` is needed.
    pub fn has_custom_attrs(&self) -> bool {
        !self.attrs.others.is_empty()
    }

    /// Gets custom attributes expansion data if any was present in the property.
    pub fn custom_attrs_expand(&self, builder: Ident, when: Option<Ident>, importance: Option<Ident>) -> TokenStream {
        debug_assert!(self.has_custom_attrs());

        let attrs = &self.attrs.others;

        let path_slug = util::display_path(&self.path).replace("::", "_");
        PropertyAttrData {
            pending_attrs: attrs.clone(),
            data_ident: ident!("__property_custom_expand_data_p{}__", path_slug),
            builder,
            is_unset: self.is_unset(),
            when,
            property: self.path.clone(),
            importance,
        }
        .to_token_stream()
    }
}
impl Parse for WgtProperty {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let vis = input.parse()?;
        let path: Path = input.parse()?;
        if input.peek(Token![!]) {
            // cause error.
            input.parse::<Token![=]>()?;
        }
        let (path, generics) = split_path_generics(path)?;

        let capture_decl = if input.peek(token::Paren) { Some(input.parse()?) } else { None };

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
            attrs: Attributes::new(attrs),
            vis,
            path,
            capture_decl,
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

/// Property assign declares a new capture-only.
pub struct CaptureDeclaration {
    ty: Type,
}
impl Parse for CaptureDeclaration {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let inner;
        parenthesized!(inner in input);
        Ok(Self { ty: inner.parse()? })
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
            let ident: Ident = input.parse().unwrap();
            if ident != "unset" {
                return Err(Error::new(ident.span(), "unknown special value,  expected `unset!`"));
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
    pub when: keyword::when,
    pub condition_expr: TokenStream,
    pub condition_expr_str: String,
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
                        p.attrs = Attributes::new(attrs);
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
                        if !matches!(p.vis, Visibility::Inherited) {
                            errors.push("cannot reexport property from when assign", p.vis.span());
                        }
                        if let Some(decl) = &p.capture_decl {
                            errors.push("cannot declare capture property in when assign", decl.ty.span());
                        }

                        if let Some((_, PropertyValue::Special(s, _))) = &p.value {
                            errors.push(format!("cannot {} in when assign", s), s.span());
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
            errors.push("expected a when block expr and properties", util::last_span(condition_expr));
            return None;
        };

        let expr_str = condition_expr.to_string().replace(" # ", "__pound__");
        let expr_str = util::format_rust_expr(expr_str).replace("__pound__", "#");

        Some(WgtWhen {
            attrs: Attributes::new(vec![]), // must be parsed before.
            when,
            condition_expr_str: expr_str,
            condition_expr,
            brace_token,
            assigns,
        })
    }

    pub fn pre_bind(&mut self, shorthand_init_enabled: bool, when_index: usize) -> TokenStream {
        let prefix = format!("w{}_", when_index);
        let mut r = quote!();
        for p in &mut self.assigns {
            r.extend(p.pre_bind_args(shorthand_init_enabled, Some(&self.attrs), &prefix));
        }
        r
    }

    /// Expand to a init, expects pre-bind variables.
    pub fn when_new(&self, wgt_builder_mod: TokenStream) -> TokenStream {
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

            let path_span = path_span(&property);
            let member_ident = ident_spanned!(path_span=> "__w_{member}__");
            var_decl.extend(quote_spanned! {path_span=>
                let (#var_input, #var) = #property #generics::#member_ident();
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
            let p_ident_str = p_ident.to_string();
            let error = format!("property `{p_ident_str}` cannot be read in when expr");
            inputs.extend(quote! {
                {
                    const _: () = if !#property #generics::ALLOWED_IN_WHEN_EXPR {
                        panic!(#error)
                    };

                    #wgt_builder_mod::WhenInput {
                        property: #property #generics::__id__(#p_ident_str),
                        member: #wgt_builder_mod::WhenInputMember::#member,
                        var: #var_input,
                        property_default: #property #generics::__default_fn__(),
                    }
                },
            });
        }

        let mut assigns = quote!();
        let mut assigns_error = quote!();
        for a in &self.assigns {
            if !a.has_args() {
                continue;
            }

            let args = a.args_new(wgt_builder_mod.clone());
            let generics = &a.generics;
            let cfg = &a.attrs.cfg;
            let attrs = a.attrs.cfg_and_lints();
            assigns.extend(quote! {
                #attrs
                #args,
            });

            let path = &a.path;
            let error = format!("property `{}` cannot be assigned in when", a.ident());
            assigns_error.extend(quote_spanned! {path_span(path)=>
                #cfg
                const _: () = if !#path #generics::ALLOWED_IN_WHEN_ASSIGN {
                    panic!(#error);
                };
            });
        }

        let expr = when_expr.expr;
        let expr_str = &self.condition_expr_str;

        quote! {
            {
                #var_decl
                #assigns_error
                #wgt_builder_mod::WhenInfo {
                    inputs: std::boxed::Box::new([
                        #inputs
                    ]),
                    state: #wgt_builder_mod::when_condition_expr_var! { #expr },
                    assigns: std::vec![
                        #assigns
                    ],
                    build_action_data: std::vec![],
                    expr: #expr_str,
                    location: #wgt_builder_mod::source_location!(),
                }
            }
        }
    }

    pub fn custom_assign_expand(&self, builder: &Ident, when: &Ident) -> TokenStream {
        let mut r = quote!();
        for a in &self.assigns {
            if a.has_custom_attrs() {
                r.extend(a.custom_attrs_expand(builder.clone(), Some(when.clone()), None));
            }
        }
        r
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

struct UnamedArgs {
    args: Punctuated<Expr, Token![,]>,
}
impl Parse for UnamedArgs {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Self {
            args: Punctuated::parse_terminated(input)?,
        })
    }
}
