use crate::util;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::{parse::*, punctuated::Punctuated, *};

pub mod keyword {
    syn::custom_keyword!(child);
    syn::custom_keyword!(required);
    syn::custom_keyword!(unset);
    syn::custom_keyword!(when);
    syn::custom_keyword!(input);
    syn::custom_keyword!(new_child);
    syn::custom_keyword!(new);
    syn::custom_keyword!(inherit);
}

/// `widget!` implementation
#[allow(clippy::cognitive_complexity)]
pub fn expand_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // arguments can be in two states:
    let args = parse_macro_input!(input as WidgetArgs);
    let mut input = match args {
        // 1 - Is in a recursive expansion that is including the widget_new! info
        // from the inherited widgets.
        WidgetArgs::IncludeInherits { inherits_todo, rest } => {
            let inherits: Vec<_> = inherits_todo.into_iter().map(|i| ident!("__{}", i)).collect();
            let inherit = &inherits[0];
            let r = quote! { #inherit! { inherit #rest } };
            return r.into();
        }
        // 2 - Has collected the inherited widget_new! info or does not have any
        // the rest of this function expands the widget.
        WidgetArgs::Declare(input) => *input,
    };

    // we get the item level docs
    let (mut docs, attrs) = util::split_doc_other(&mut input.attrs);
    // end insert the header termination html because we will
    // generate custom sections to the item docs page.
    finish_docs_header(&mut docs);

    let (export, pub_) = if input.export {
        (quote!(#[macro_export]), quote!(pub))
    } else {
        (quote!(), quote!())
    };

    let ident = input.ident;

    // Collect `new_child` and what properties are required by it.
    let mut new_child_properties = vec![];
    let new_child;
    if let Some(mut c) = input.new_child {
        for input in c.sig.inputs.iter().skip(1) {
            match input {
                FnArg::Typed(input) => {
                    if let Pat::Ident(pat) = &*input.pat {
                        new_child_properties.push(pat.ident.clone());
                    } else {
                        abort!(input.pat.span(), "new_child must only use simple argument names")
                    }
                }
                // can this even happen? we parsed as ItemFn
                FnArg::Receiver(self_) => abort!(self_.span(), "new_child must be stand-alone fn"),
            }
        }
        c.vis = util::pub_vis();
        new_child = quote!(#c);
    } else {
        let fn_doc = doc!(
            "Manually initializes the `{0}` widget content.\n\nSee [the module level documentation](super) for more.",
            ident
        );
        new_child = quote!(
            #fn_doc
            pub fn new_child<C: zero_ui::core::UiNode>(child: C) -> C {
                zero_ui::core::default_new_widget_child(child)
            }
        );
    };

    // Collect `new` and what properties are required by it.
    let mut new_properties = vec![];
    let new;
    if let Some(mut n) = input.new {
        for input in n.sig.inputs.iter().skip(1) {
            match input {
                FnArg::Typed(input) => {
                    if let Pat::Ident(pat) = &*input.pat {
                        new_properties.push(pat.ident.clone());
                    } else {
                        abort!(input.pat.span(), "new must only use simple argument names")
                    }
                }
                // can this even happen? we parsed as ItemFn
                FnArg::Receiver(self_) => abort!(self_.span(), "new must be stand-alone fn"),
            }
        }
        n.vis = util::pub_vis();
        new = quote!(#n);
    } else {
        new_properties.push(ident!["id"]);
        let fn_doc = doc!(
            "Manually initializes the `{0}` widget.\n\nSee [the module level documentation](super) for more.",
            ident
        );
        new = quote!(
            #fn_doc
            pub fn new(child: impl zero_ui::core::UiNode, id: impl zero_ui::properties::id::Args) -> impl zero_ui::core::UiNode {
                zero_ui::core::default_new_widget(child, id)
            }
        );
    };

    // Group all child properties in one DefaultBlock that will be send to widget_new!.
    let mut default_child: Vec<_> = input.default_child.into_iter().flat_map(|d| d.properties).collect();

    // Group all self properties in one DefaultBlock that will be send to widget_new!.
    let mut default_self: Vec<_> = input.default_self.into_iter().flat_map(|d| d.properties).collect();

    // add missing requiried properties from new_child function.
    for p in new_child_properties.iter() {
        if !default_child.iter().any(|c| &c.ident == p) {
            default_child.push(parse_quote!(#p: required!;));
        }
    }

    let id_ident = ident!("id");

    // add missing required properties from new function.
    for p in new_properties.iter() {
        if !default_self.iter().any(|s| &s.ident == p) {
            if p == &id_ident {
                // id is provided if missing.
                default_self.push(parse_quote!(id: zero_ui::core::types::WidgetId::new_unique();));
            } else {
                default_self.push(parse_quote!(#p: required!;));
            }
        }
    }

    // Collect some property info:
    // 1 - Property use clauses and defaults.
    let mut use_props = vec![];
    let mut fn_prop_dfts = vec![];
    // 2 - Generate widget_new property metadata.
    let mut built_child = vec![];
    let mut built_self = vec![];
    // 3 - Separate the property documentation. Each vec contains (DefaultBlockTarget, &mut PropertyDeclaration).
    let mut required_docs = vec![];
    let mut default_docs = vec![];
    let mut other_docs = vec![];
    let mut default_blocks = [
        (DefaultBlockTarget::Child, &mut default_child),
        (DefaultBlockTarget::Self_, &mut default_self),
    ];
    for (target, properties) in &mut default_blocks {
        for p in properties.iter_mut() {
            let (prop_docs, other_attrs) = util::split_doc_other(&mut p.attrs);

            if let Some(invalid) = other_attrs.into_iter().next() {
                abort!(invalid.span(), "only #[doc] attributes are allowed here")
            }

            p.attrs = prop_docs;

            let is_required = p.is_required();

            // 1
            let ident = &p.ident;
            if let Some(maps_to) = &p.maps_to {
                use_props.push(quote!(pub use super::#maps_to as #ident;))
            } else if ident == &id_ident {
                use_props.push(quote!(
                    pub use zero_ui::properties::id;
                ))
            } else {
                use_props.push(quote!(pub use super::#ident;))
            }
            if !is_required {
                if let Some(dft) = &p.default_value {
                    fn_prop_dfts.push(quote! {
                        pub fn #ident() -> impl ps::#ident::Args {
                            ps::#ident::args(#dft)
                        }
                    });
                }
            }

            // 2
            let built = match target {
                DefaultBlockTarget::Child => &mut built_child,
                DefaultBlockTarget::Self_ => &mut built_self,
            };
            if is_required {
                built.push(quote!(r #ident));
            } else if p.default_value.is_some() {
                built.push(quote!(d #ident));
            } else {
                built.push(quote!(l #ident));
            }

            // 3
            if is_required {
                required_docs.push((*target, p));
            } else if p.default_value.is_some() {
                default_docs.push((*target, p));
            } else {
                other_docs.push((*target, p));
            }
        }
    }

    // Pushes the custom documentation sections.
    print_required_section(&mut docs, required_docs);
    print_provided_section(&mut docs, default_docs);
    print_aliases_section(&mut docs, other_docs);
    print_whens(&mut docs, &mut input.whens);

    // ident of a doc(hidden) macro that is the actual macro implementation.
    // This macro is needed because we want to have multiple match arms, but
    // the widget macro needs to take $($tt:tt)*.
    let inner_ident = ident!("__{}", ident);

    let widget_new_tokens = quote! {
        m #ident
        c { #(#built_child),* }
        s { #(#built_self),* }
        n (#(#new_child_properties),*) (#(#new_properties),*)
    };

    let r = quote! {
        // widget macro. Is hidden until macro 2.0 comes around.
        // For now this simulates name-spaced macros because we
        // require a use widget_mod for this to work.
        #[doc(hidden)]
        #(#attrs)*
        #export
        macro_rules! #ident {
            ($($input:tt)*) => {
                // call the new variant.
                #inner_ident!{new $($input)*}
            };
        }

        #[doc(hidden)]
        #export
        macro_rules! #inner_ident {
            // new widget instance.
            (new $($input:tt)*) => {
                widget_new! {
                    #widget_new_tokens
                    i { $($input)* }
                }
            };
            // recursive callback to widget! but this time including
            // the widget_new! info from this widget in an inherit block.
            (inherit $($rest:tt)*) => {
                widget! {
                    inherit {
                        #widget_new_tokens
                        i { =>{} }
                    }

                    $($rest)*
                }
            };
        }

        // the widget module, also the public face of the widget in the documentation.
        #(#docs)*
        #pub_ mod #ident {
            #[doc(hidden)]
            pub use super::*;

            #new_child
            #new

            // Properties used in self.
            #[doc(hidden)]
            pub mod ps {
                #(#use_props)*
            }

            // Default values from the widget.
            #[doc(hidden)]
            pub mod df {
                use super::*;

                #(#fn_prop_dfts)*
            }
        }
    };

    //panic!("{}", r);

    r.into()
}

enum WidgetArgs {
    IncludeInherits {
        inherits_todo: Punctuated<Ident, Token![+]>,
        rest: TokenStream,
    },
    Declare(Box<WidgetInput>),
}
impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut inherits = vec![];
        while input.peek(keyword::inherit) {
            input.parse::<keyword::inherit>().expect(util::NON_USER_ERROR);
            let inner = util::non_user_braced(input);
            inherits.push(inner.parse().expect(util::NON_USER_ERROR));
        }

        let attrs = Attribute::parse_outer(input)?;

        let export = input.peek(Token![pub]);
        if export {
            input.parse::<Token![pub]>()?;
        }

        let ident = input.parse()?;
        let inherits_todo = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Punctuated::parse_separated_nonempty(input)?
        } else {
            Punctuated::new()
        };
        input.parse::<Token![;]>()?;

        if !inherits_todo.is_empty() {
            let rest: TokenStream = input.parse().unwrap();
            let pub_ = if export { quote!(pub) } else { quote!() };
            return Ok(WidgetArgs::IncludeInherits {
                inherits_todo,
                rest: quote! {
                    #(#attrs)*
                    #pub_ #ident;
                    #rest
                },
            });
        }

        let mut default_child = vec![];
        let mut default_self = vec![];
        let mut whens = vec![];
        let mut new_child = None;
        let mut new = None;
        while !input.is_empty() {
            let mut attrs = Attribute::parse_outer(input)?;

            let lookahead = input.lookahead1();

            if attrs.is_empty() && lookahead.peek(Token![default]) {
                let block: DefaultBlock = input.parse()?;
                match block.target {
                    DefaultBlockTarget::Self_ => {
                        default_self.push(block);
                    }
                    DefaultBlockTarget::Child => {
                        default_child.push(block);
                    }
                }
            } else if lookahead.peek(keyword::when) {
                let mut when: WhenBlock = input.parse()?;
                // extend outer with inner
                attrs.extend(when.attrs.drain(..));
                when.attrs = attrs;

                whens.push(when);
            } else if lookahead.peek(Token![fn]) {
                let mut fn_: ItemFn = input.parse()?;
                attrs.extend(fn_.attrs.drain(..));
                fn_.attrs = attrs;

                if ident!("new") == fn_.sig.ident {
                    if new.is_some() {
                        return Err(Error::new(fn_.sig.ident.span(), "function `new` can only be defined once"));
                    }
                    new = Some(fn_);
                } else if ident!("new_child") == fn_.sig.ident {
                    if new_child.is_some() {
                        return Err(Error::new(fn_.sig.ident.span(), "function `new_child` can only be defined once"));
                    }
                    new_child = Some(fn_);
                } else {
                    return Err(Error::new(fn_.sig.ident.span(), "expected one of: new, new_child"));
                }
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(WidgetArgs::Declare(Box::new(WidgetInput {
            attrs,
            export,
            ident,
            inherits,
            default_child,
            default_self,
            whens,
            new_child,
            new,
        })))
    }
}

struct WidgetInput {
    attrs: Vec<Attribute>,
    export: bool,
    ident: Ident,
    inherits: Vec<crate::widget_new::WidgetNewInput>,
    default_child: Vec<DefaultBlock>,
    default_self: Vec<DefaultBlock>,
    whens: Vec<WhenBlock>,
    new_child: Option<ItemFn>,
    new: Option<ItemFn>,
}

#[derive(Debug)]
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
        input.parse::<keyword::when>()?;

        let condition = input.parse()?;

        let inner;
        braced!(inner in input);

        let attrs = Attribute::parse_inner(input)?;

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

#[derive(Debug)]
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
    pub ident: Ident,
    pub value: PropertyValue,
}
impl Parse for PropertyAssign {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;
        Ok(PropertyAssign { ident, value })
    }
}
impl ToTokens for PropertyAssign {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let value = &self.value;
        tokens.extend(quote!(#ident: #value;))
    }
}

#[derive(Debug)]
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
            if let Ok(fields) = Punctuated::parse_terminated(&inner) {
                input.advance_to(&fields_fork);
                input.parse::<Token![;]>()?;

                Ok(PropertyDefaultValue::Fields(fields))
            } else if let Ok(args) = Punctuated::parse_separated_nonempty(&input) {
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                input.parse::<Token![;]>()?;

                Ok(PropertyDefaultValue::Args(args))
            } else {
                Err(Error::new(inner.span(), "expected one of: args, named args"))
            }
        } else if input.peek(keyword::unset) && input.peek2(Token![!]) {
            input.parse::<keyword::unset>()?;
            input.parse::<Token![!]>()?;
            input.parse::<Token![;]>()?;
            Ok(PropertyDefaultValue::Unset)
        } else if input.peek(keyword::required) && input.peek2(Token![!]) {
            input.parse::<keyword::required>()?;
            input.parse::<Token![!]>()?;
            input.parse::<Token![;]>()?;
            Ok(PropertyDefaultValue::Required)
        } else if let Ok(args) = Punctuated::parse_separated_nonempty(input) {
            input.parse::<Token![;]>()?;
            Ok(PropertyDefaultValue::Args(args))
        } else {
            Err(Error::new(input.span(), "expected one of: args, named args, `required!`, `unset!`"))
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
    /// Named arguments.
    Fields(Punctuated<FieldValue, Token![,]>),
    /// Unamed arguments.
    Args(Punctuated<Expr, Token![,]>),
    /// unset!.
    Unset,
}
impl Parse for PropertyValue {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Brace) {
            use syn::parse::discouraged::Speculative;

            let fields_fork = input.fork();
            let inner;
            braced!(inner in fields_fork);
            if let Ok(fields) = Punctuated::parse_terminated(&inner) {
                input.advance_to(&fields_fork);
                input.parse::<Token![;]>()?;

                Ok(PropertyValue::Fields(fields))
            } else if let Ok(args) = Punctuated::parse_separated_nonempty(&input) {
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                input.parse::<Token![;]>()?;

                Ok(PropertyValue::Args(args))
            } else {
                Err(Error::new(inner.span(), "expected one of: args, named args"))
            }
        } else if input.peek(keyword::unset) && input.peek2(Token![!]) {
            input.parse::<keyword::unset>()?;
            input.parse::<Token![!]>()?;
            input.parse::<Token![;]>()?;
            Ok(PropertyValue::Unset)
        } else if let Ok(args) = Punctuated::parse_separated_nonempty(input) {
            input.parse::<Token![;]>()?;
            Ok(PropertyValue::Args(args))
        } else {
            Err(Error::new(input.span(), "expected one of: args, named args, `unset!`, `todo!(..)`"))
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
impl PropertyValue {
    pub fn is_unset(&self) -> bool {
        match self {
            PropertyValue::Unset => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

fn print_required_section(docs: &mut Vec<Attribute>, required_docs: Vec<(DefaultBlockTarget, &mut PropertyDeclaration)>) {
    print_section(docs, "required-properties", "Required properties", required_docs);
}

fn print_provided_section(docs: &mut Vec<Attribute>, default_docs: Vec<(DefaultBlockTarget, &mut PropertyDeclaration)>) {
    print_section(docs, "provided-properties", "Provided properties", default_docs);
}

fn print_aliases_section(docs: &mut Vec<Attribute>, other_docs: Vec<(DefaultBlockTarget, &mut PropertyDeclaration)>) {
    print_section_header(docs, "other-properties", "Other properties");
    for p in other_docs {
        print_property(docs, p);
    }
    docs.push(doc!(r##"<h3 id="wgall" class="method"><code><a href="#wgall" class="fnname">*</a> -> <span title="applied to self">self</span>.<span class='wgprop'>"##));
    docs.push(doc!("\n[<span class='mod'>*</span>](zero_ui::properties)\n"));
    docs.push(doc!(r##"<ul style='display:none;'></ul></span></code></h3><div class="docblock">Widgets are open-ended, all properties are accepted.</div>"##));
    print_section_footer(docs);
}

fn print_section(docs: &mut Vec<Attribute>, id: &str, title: &str, properties: Vec<(DefaultBlockTarget, &mut PropertyDeclaration)>) {
    if properties.is_empty() {
        return;
    }

    print_section_header(docs, id, title);
    for p in properties {
        print_property(docs, p);
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

fn print_property(docs: &mut Vec<Attribute>, (t, p): (DefaultBlockTarget, &mut PropertyDeclaration)) {
    docs.push(doc!(
        r##"<h3 id="wgproperty.{0}" class="method"><code id='{0}.v'><a href='#wgproperty.{0}' class='fnname'>{0}</a> -> <span title="applied to {1}">{1}</span>.<span class='wgprop'>"##,
        p.ident,
        match t {
            DefaultBlockTarget::Self_ => "self",
            DefaultBlockTarget::Child => "child",
        },
    ));
    docs.push(doc!(
        "\n[<span class='mod'>{0}</span>]({0})\n",
        p.maps_to.as_ref().unwrap_or(&p.ident),
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

fn print_whens(docs: &mut Vec<Attribute>, whens: &mut [WhenBlock]) {
    let mut whens: Vec<_> = whens.iter_mut().filter(|w| !w.properties.is_empty()).collect();
    if whens.is_empty() {
        return;
    }

    print_section_header(docs, "conditional-assigns", "Conditional assigns");

    let mut used_when_ids = HashSet::with_capacity(whens.len());

    for when in whens.iter_mut() {
        let condition = &when.condition;
        let condition = quote!(#condition).to_string();

        let mut in_in_replace = false;
        let mut when_id: String = condition
            .chars()
            .filter_map(|c| {
                if c.is_alphanumeric() {
                    in_in_replace = false;
                    Some(c)
                } else if !in_in_replace {
                    in_in_replace = true;
                    Some('_')
                } else {
                    None
                }
            })
            .collect();

        let mut i = 0;
        let when_id_len = when_id.len();
        while used_when_ids.contains(&when_id) {
            when_id.truncate(when_id_len);
            when_id.push_str(&i.to_string());
            i += 1;
        }
        used_when_ids.insert(when_id.clone());

        docs.push(doc!(
            r##"<h3 id="wgwhen.{0}" class="method"><code id='{0}.v'><a href='#wgwhen.{0}' class='fnname'>when</a> "##,
            when_id
        ));

        // TODO actual formatting.
        let mut next_is_property = false;
        let mut condition_span = String::new();
        let mut prev_point = false;
        for w in condition.split_whitespace() {
            if next_is_property {
                if w == "." {
                    condition_span.push('.');
                } else {
                    next_is_property = false;
                    docs.push(doc!("{}", condition_span.trim()));
                    condition_span = String::new();

                    docs.push(doc!("<span class='wgprop'>"));
                    docs.push(doc!("\n[<span class='mod'>{0}</span>]({0})\n", w));
                    docs.push(doc!("<ul style='display:none;'></ul></span>"));
                }
            } else if w == "self" || w == "child" {
                next_is_property = true;
                if !prev_point {
                    condition_span.push(' ');
                }
                condition_span.push_str(w)
            } else if w.chars().all(|c| c.is_alphanumeric()) {
                if !prev_point {
                    condition_span.push(' ');
                }
                condition_span.push_str(w);
            } else {
                condition_span.push_str(w);
            }

            prev_point = w == "." || w == "!";
        }

        docs.push(doc!("<ul style='display:none;'></ul></code></h3>"));

        if !when.attrs.is_empty() {
            docs.push(doc!("<div class='docblock'>\n"));
            docs.extend(when.attrs.drain(..));
            docs.push(doc!("\n</div>"));
        }
    }

    print_section_footer(docs);
}

// #endregion docs hack
