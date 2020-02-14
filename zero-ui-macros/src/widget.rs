use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{parse::*, punctuated::Punctuated, *};

include!("util.rs");

pub mod keyword {
    syn::custom_keyword!(child);
    syn::custom_keyword!(required);
    syn::custom_keyword!(unset);
    syn::custom_keyword!(when);
    syn::custom_keyword!(input);
}

/// `widget!` implementation
pub fn expand_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as WidgetInput);

    let (mut docs, attrs) = split_doc_other(&mut input.attrs);
    finish_docs_header(&mut docs);

    let (export, pub_) = if input.export {
        (quote!(#[macro_export]), quote!(pub))
    } else {
        (quote!(), quote!())
    };

    let mut required_docs = vec![];
    let mut default_docs = vec![];
    let mut other_docs = vec![];

    let ident = input.ident;
    let imports = input.imports;

    let macro_imports = imports.clone(); //TODO $crate

    let mut redirect_imports = imports;
    for use_ in redirect_imports.iter_mut() {
        use_.vis = self::pub_vis();
    }

    let redirect_ident = self::ident(&format!("__{}_redirect", ident));

    for b in input.default_child.iter_mut().chain(input.default_self.iter_mut()) {
        for p in b.properties.iter_mut() {
            let (prop_docs, other_attrs) = split_doc_other(&mut p.attrs);

            if let Some(invalid) = other_attrs.into_iter().next() {
                abort!(invalid.span(), "only #[doc] attributes are allowed here")
            }

            p.attrs = prop_docs;

            if p.is_required() {
                required_docs.push((b.target, p));
            } else if p.default_value.is_some() {
                default_docs.push((b.target, p));
            } else {
                other_docs.push((b.target, p));
            }
        }
    }

    print_required_section(&mut docs, &redirect_ident, required_docs);
    print_provided_section(&mut docs, &redirect_ident, default_docs);
    print_aliases_section(&mut docs, &redirect_ident, other_docs);

    let default_child = input.default_child.into_iter().flat_map(|d| d.properties);
    let default_child = quote! {
        default(child) {
            #(#default_child)*
        }
    };

    let default_self = input.default_self.into_iter().flat_map(|d| d.properties);
    let default_self = quote! {
        default(self) {
            #(#default_self)*
        }
    };

    let whens = input.whens;

    let child = if let Some(c) = input.child_expr {
        quote!(#c)
    } else {
        quote!(child)
    };

    // rust-doc includes the macro arm pattern in documentation.
    let macro_arm = quote_spanned! {ident.span()=>
        ($($tt:tt)+)
    };

    let r = quote! {
        #[doc(hidden)]
        #(#attrs)*
        #export
        macro_rules! #ident {
            #macro_arm => {
                widget_new! {
                    mod #ident;
                    #(#macro_imports)*
                    #default_child
                    #default_self
                    #(#whens)*
                    input:{$($tt)+}
                }
            };
        }

        #[doc(hidden)]
        mod #redirect_ident {
            #(#redirect_imports)*
        }

        #(#docs)*
        #pub_ mod #ident {
            use super::*;
            use #redirect_ident::*;

            #[doc(hidden)]
            pub fn __child(child: impl zero_ui::core::UiNode) -> impl zero_ui::core::UiNode {
                #child
            }

            //#[doc(hidden)]
            //#[allow(unused)]
            //fn __test(child: impl zero_ui::core::UiNode) -> impl zero_ui::core::UiNode {
            //    #ident! {
            //        => child
            //    }
            //}
        }
    };

    r.into()
}

struct WidgetInput {
    attrs: Vec<Attribute>,
    export: bool,
    ident: Ident,
    inherits: Punctuated<Ident, Token![+]>,
    imports: Vec<ItemUse>,
    default_child: Vec<DefaultBlock>,
    default_self: Vec<DefaultBlock>,
    whens: Vec<WhenBlock>,
    child_expr: Option<Expr>,
}
impl Parse for WidgetInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let export = input.peek(Token![pub]);
        if export {
            input.parse::<Token![pub]>()?;
        }

        let ident = input.parse()?;
        let inherits = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Punctuated::parse_separated_nonempty(input)?
        } else {
            Punctuated::new()
        };
        input.parse::<Token![;]>()?;

        let mut imports = vec![];
        while input.peek(Token![use]) {
            imports.push(input.parse()?);
        }

        let mut default_child = vec![];
        let mut default_self = vec![];
        let mut whens = vec![];
        let mut child_expr = None;
        while !input.is_empty() {
            let lookahead = input.lookahead1();

            if lookahead.peek(Token![default]) {
                let block: DefaultBlock = input.parse()?;
                match block.target {
                    DefaultBlockTarget::Self_ => {
                        default_child.push(block);
                    }
                    DefaultBlockTarget::Child => {
                        default_self.push(block);
                    }
                }
            } else if lookahead.peek(keyword::when) {
                whens.push(input.parse()?);
            } else if lookahead.peek(Token![=>]) {
                input.parse::<Token![=>]>()?;
                child_expr = Some(input.parse()?);
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(WidgetInput {
            attrs,
            export,
            ident,
            inherits,
            imports,
            default_child,
            default_self,
            whens,
            child_expr,
        })
    }
}

pub struct DefaultBlock {
    pub target: DefaultBlockTarget,
    pub properties: Vec<PropertyDeclaration>,
}
impl Parse for DefaultBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![default]>()?;

        let inner;
        parenthesized!(inner in input);
        let target = inner.parse()?;

        let inner;
        braced!(inner in input);
        let mut properties = vec![];
        while !inner.is_empty() {
            properties.push(inner.parse()?);
        }

        Ok(DefaultBlock { target, properties })
    }
}

pub struct WhenBlock {
    attrs: Vec<Attribute>,
    pub condition: Expr,
    pub properties: Vec<PropertyAssign>,
}
impl Parse for WhenBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let inner;
        parenthesized!(inner in input);
        let condition = inner.parse()?;

        let inner;
        braced!(inner in input);
        let mut properties = vec![];
        while !inner.is_empty() {
            properties.push(inner.parse()?);
        }

        Ok(WhenBlock {
            attrs,
            condition,
            properties,
        })
    }
}
impl ToTokens for WhenBlock {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let condition = &self.condition;
        let properties = &self.properties;

        tokens.extend(quote! {
            when(#condition) {
                #(#properties)*
            }
        })
    }
}

pub struct PropertyDeclaration {
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub maps_to: Option<Ident>,
    pub default_value: Option<PropertyDefaultValue>,
}
impl Parse for PropertyDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let ident = input.parse()?;
        let mut maps_to = None;
        let mut default_value = None;

        let lookahead = input.lookahead1();
        if lookahead.peek(Token![->]) {
            // is property alias.
            input.parse::<Token![->]>()?;
            maps_to = Some(input.parse()?);

            let lookahead = input.lookahead1();
            if lookahead.peek(Token![:]) {
                // alias does not need default value but this one has it too.
                input.parse::<Token![:]>()?;
                default_value = Some(input.parse()?);
            } else if lookahead.peek(Token![;]) {
                // no value and added the required ;.
                input.parse::<Token![;]>()?;
            } else {
                // invalid did not finish the declaration with ;.
                return Err(lookahead.error());
            }
        } else if lookahead.peek(Token![:]) {
            // is not property alias but has default value.
            input.parse::<Token![:]>()?;
            default_value = Some(input.parse()?);
        } else {
            // invalid, no alias and no value.
            return Err(lookahead.error());
        }

        Ok(PropertyDeclaration {
            attrs,
            ident,
            maps_to,
            default_value,
        })
    }
}
impl ToTokens for PropertyDeclaration {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ts = match (&self.ident, &self.maps_to, &self.default_value) {
            (ident, None, Some(default_value)) => quote!(#ident: #default_value;),
            (ident, Some(maps_to), Some(default_value)) => quote!(#ident -> #maps_to: #default_value;),
            (ident, Some(maps_to), None) => quote!(#ident -> #maps_to;),
            _ => unreachable!(),
        };
        tokens.extend(ts)
    }
}
impl PropertyDeclaration {
    pub fn is_required(&self) -> bool {
        self.default_value.as_ref().map(|v| v.is_required()).unwrap_or_default()
    }
}

pub struct PropertyAssign {
    attrs: Vec<Attribute>,
    pub ident: Ident,
    pub value: PropertyValue,
}
impl Parse for PropertyAssign {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(PropertyAssign {
            attrs: Attribute::parse_outer(input)?,
            ident: input.parse()?,
            value: input.parse()?,
        })
    }
}
impl ToTokens for PropertyAssign {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let value = &self.value;
        tokens.extend(quote!(#ident: #value))
    }
}

pub enum PropertyDefaultValue {
    Fields(Punctuated<FieldValue, Token![,]>),
    Args(Punctuated<Expr, Token![,]>),
    Unset,
    Required,
}
impl Parse for PropertyDefaultValue {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Brace) {
            use syn::parse::discouraged::Speculative;

            let fields_fork = input.fork();
            let inner;
            braced!(inner in fields_fork);
            if let Ok(fields) = Punctuated::parse_separated_nonempty(&inner) {
                input.advance_to(&fields_fork);
                input.parse::<Token![;]>()?;

                Ok(PropertyDefaultValue::Fields(fields))
            } else if let Ok(args) = Punctuated::parse_separated_nonempty(&input) {
                input.parse::<Token![;]>()?;

                Ok(PropertyDefaultValue::Args(args))
            } else {
                Err(Error::new(
                    inner.span(),
                    "expected named args block or expression block for the first arg",
                ))
            }
        } else if input.peek2(Token![!]) {
            let lookahead = input.lookahead1();
            if lookahead.peek(keyword::unset) {
                input.parse::<keyword::required>()?;
                input.parse::<Token![!]>()?;
                input.parse::<Token![;]>()?;

                Ok(PropertyDefaultValue::Unset)
            } else if lookahead.peek(keyword::required) {
                input.parse::<keyword::required>()?;
                input.parse::<Token![!]>()?;
                input.parse::<Token![;]>()?;

                Ok(PropertyDefaultValue::Required)
            } else {
                Err(lookahead.error())
            }
        } else {
            let args = Punctuated::parse_separated_nonempty(input)?;
            input.parse::<Token![;]>()?;

            Ok(PropertyDefaultValue::Args(args))
        }
    }
}
impl ToTokens for PropertyDefaultValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            PropertyDefaultValue::Fields(f) => tokens.extend(quote!({#f})),
            PropertyDefaultValue::Args(a) => a.to_tokens(tokens),
            PropertyDefaultValue::Unset => tokens.extend(quote!(unset!)),
            PropertyDefaultValue::Required => tokens.extend(quote!(required!)),
        }
    }
}
impl PropertyDefaultValue {
    pub fn is_required(&self) -> bool {
        match self {
            PropertyDefaultValue::Required => true,
            _ => false,
        }
    }
}

pub enum PropertyValue {
    Fields(Punctuated<FieldValue, Token![,]>),
    Args(Punctuated<Expr, Token![,]>),
    Unset,
}
impl Parse for PropertyValue {
    fn parse(input: ParseStream) -> Result<Self> {
        let p: PropertyDefaultValue = input.parse()?;

        match p {
            PropertyDefaultValue::Fields(f) => Ok(PropertyValue::Fields(f)),
            PropertyDefaultValue::Args(a) => Ok(PropertyValue::Args(a)),
            PropertyDefaultValue::Unset => Ok(PropertyValue::Unset),
            PropertyDefaultValue::Required => Err(Error::new(input.span(), "cannot assign `required!`")),
        }
    }
}
impl ToTokens for PropertyValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            PropertyValue::Fields(f) => tokens.extend(quote!({#f})),
            PropertyValue::Args(a) => a.to_tokens(tokens),
            PropertyValue::Unset => tokens.extend(quote!(unset!)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DefaultBlockTarget {
    Self_,
    Child,
}
impl Parse for DefaultBlockTarget {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![self]) {
            input.parse::<Token![self]>()?;

            Ok(DefaultBlockTarget::Self_)
        } else if lookahead.peek(keyword::child) {
            input.parse::<keyword::child>()?;

            Ok(DefaultBlockTarget::Child)
        } else {
            Err(lookahead.error())
        }
    }
}
impl ToTokens for DefaultBlockTarget {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            DefaultBlockTarget::Self_ => tokens.extend(quote!(self)),
            DefaultBlockTarget::Child => tokens.extend(quote!(child)),
        }
    }
}

/* #region docs hack
*
* We emit inline html that reuses the rustdoc style to insert custom sections to a Module page.
*
* The result looks professional, unlike the code that makes it happen.
*/

fn finish_docs_header(docs: &mut Vec<Attribute>) {
    docs.push(doc!(
        "\n</div><style>span.wgprop p {{ display: inline; margin-left:-1ch; }}</style>"
    )); // finish item level docs.
}

fn print_required_section(
    docs: &mut Vec<Attribute>,
    redirect_ident: &Ident,
    required_docs: Vec<(DefaultBlockTarget, &mut PropertyDeclaration)>,
) {
    print_section(docs, redirect_ident, "required-properties", "Required properties", required_docs);
}

fn print_provided_section(
    docs: &mut Vec<Attribute>,
    redirect_ident: &Ident,
    default_docs: Vec<(DefaultBlockTarget, &mut PropertyDeclaration)>,
) {
    print_section(docs, redirect_ident, "provided-properties", "Provided properties", default_docs);
}

fn print_aliases_section(
    docs: &mut Vec<Attribute>,
    redirect_ident: &Ident,
    other_docs: Vec<(DefaultBlockTarget, &mut PropertyDeclaration)>,
) {
    print_section_header(docs, "other-properties", "Other properties");
    for p in other_docs {
        print_property(docs, redirect_ident, p);
    }
    docs.push(doc!(r##"<h3 id="wgall" class="method"><code><a href="#wgall" class="fnname">*</a> -> <span title="applied to self">self</span>.<span class='wgprop'>"##));
    docs.push(doc!("\n[<span class='mod'>*</span>](zero_ui::properties)\n"));
    docs.push(doc!(r##"<ul style='display:none;'></ul></span></code></h3><div class="docblock">Widgets are open-ended, all properties are accepted.</div>"##));
    print_section_footer(docs);
}

fn print_section(
    docs: &mut Vec<Attribute>,
    redirect_ident: &Ident,
    id: &str,
    title: &str,
    properties: Vec<(DefaultBlockTarget, &mut PropertyDeclaration)>,
) {
    if properties.is_empty() {
        return;
    }

    print_section_header(docs, id, title);
    for p in properties {
        print_property(docs, redirect_ident, p);
    }
    print_section_footer(docs);
}

fn print_section_header(docs: &mut Vec<Attribute>, id: &str, title: &str) {
    docs.push(doc!(
        r##"<h2 id="{0}" class="small-section-header">{1}<a href="#{0}" class="anchor"></a></h2>
        <div class="methods" style="display: block;">"##,
        id,
        title
    ));
}

fn print_property(docs: &mut Vec<Attribute>, redirect_ident: &Ident, (t, p): (DefaultBlockTarget, &mut PropertyDeclaration)) {
    docs.push(doc!(
        r##"<h3 id="wgproperty.{0}" class="method"><code id='{0}.v'><a href='#wgproperty.{0}' class='fnname'>{0}</a> -> <span title="applied to {1}">{1}</span>.<span class='wgprop'>"##,
        p.ident,
        match t {
            DefaultBlockTarget::Self_ => "self",
            DefaultBlockTarget::Child => "child",
        },
    ));
    docs.push(doc!(
        "\n[<span class='mod'>{0}</span>]({1}::{0})\n",
        p.maps_to.as_ref().unwrap_or(&p.ident),
        redirect_ident
    ));
    docs.push(doc!("<ul style='display:none;'></ul></span></code></h3>"));

    if !p.attrs.is_empty() {
        docs.push(doc!("<div class='docblock'>\n"));
        docs.extend(p.attrs.drain(..));
        docs.push(doc!("\n</div>"));
    }
}

fn print_section_footer(docs: &mut Vec<Attribute>) {
    docs.push(doc!("</div>"));
}

// #endregion docs hack
