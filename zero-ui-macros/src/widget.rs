use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
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

    let (export, pub_) = if input.export {
        (quote!(#[macro_export]), quote!(pub))
    } else {
        (quote!(), quote!())
    };

    docs.push(doc!("\n# Required Properties\n"));
    docs.push(doc!("Properties that must be set in this widget:"));

    docs.push(doc!("\n# Default Properties\n"));
    docs.push(doc!("Properties that are set by default in this widget:"));

    docs.push(doc!("\nYou can override any of this properties by setting then to another value."));
    docs.push(doc!("You can also unset this properties by setting then to `unset!`."));

    docs.push(doc!("\n# Other Properties\n"));
    docs.push(doc!("Properties that have special meaning in this widget:"));

    docs.push(doc!("\nAll widgets are open-ended and accept any property."));
    docs.push(doc!("See [zero_ui::properties] for more information."));

    let ident = input.ident;
    let mut imports = input.imports;
    // TODO change crate:: with $crate::

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
                    #(#imports)*
                    #default_child
                    #default_self
                    #(#whens)*
                    input:{$($tt)+}
                }
            };
        }

        #(#docs)*
        #pub_ mod #ident {
            use super::*;
            use zero_ui::core::UiNode;

            #[doc(hidden)]
            pub fn child(child: impl UiNode) -> impl UiNode {
                #child
            }

            //#[doc(hidden)]
            //#[allow(unused)]
            //fn test(child: impl UiNode) -> impl UiNode {
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

macro_rules! demo {
    ($($tt:tt)*) => {};
}

// Input:
demo! {
    /// Docs of widget macro named button.
    //`button` also becomes a mod.
    // `container` and `other` is another widget that button inherits from.
    pub button: container + other;

    // Uses inserted in the `button!` macro call.
    use crate::properties::{margin, align, Alignment, BorderStyle, on_click};
    use crate::core::types::{rgb, rgba};

    // Properties applied to the macro child.
    default(child) {
        // Property declaration without default value, if not set does not apply.
        // If set applies margin to child.
        padding -> margin;
        // Property declaration with default value, if not set still applies with
        // default value, only does not apply if set with `unset!`.
        content_align -> align: Alignment::CENTER;
        // Property declaration using that does not alias the property name.
        background_color: rgb(255, 255, 255);

        // to have a property apply to child and not `self` you can write:
        background_gradient -> background_gradient;
    }

    // Properties applied to the macro child properties.
    // Same sintax as `default(child)`.
    default(self) {
        border: 4., (rgba(0, 0, 0, 0.0), BorderStyle::Dashed);
        // When `required!` appears in the default values place the user
        // gets an error if the property is not set.
        on_click: required!;
    }

    // `when({bool expr})` blocks set properties given a condition. The
    // expression can contain `self.{property}` and `child.{property}` to reference
    // potentially live updating properties, every time this properties update the
    // expression is rechecked.
    when(self.is_mouse_over) {
        // Sets the properties when the expression is true.
        // the sintax in when blocks is like the sintax of properties
        // in the generated macro
        background_color: rgba(0, 0, 0, 0);
        background_gradient: {
            start: (0.0, 0.0),
            end: (1.0, 1.0),
            stops: vec![rgb(255, 0, 0), rgb(0, 255, 0), rgb(0, 0, 255)],
        };
    }

    /// Optionaly you can wrap the child into widgets, or do any custom code.
    ///
    /// This is evaluated after the `default(child)` and before the `default(self)`.
    => {
        let ct = container! {
            property: "";
            => child
        };
        println!("button created");
        ct
    }
}

// Output:
demo! {
    /// Docs generated by all the docs attributes and property names.
    #[other_name_attrs]
    #[macro_export]// if pub
    macro_rules! button {
        ($($tt::tt)+) => {
            widget_new! {
                mod button;

                // uses with `crate` converted to `$crate`
                use $crate::something;

                default(child) {
                    // all the default(child) blocks grouped or an empty block
                }
                default(self) {
                    // all the default(self) blocks grouped or an empty block
                }

                // all the when blocks
                when(expr) {}
                when(expr) {}

                // user args
                {
                    // zero or more property assigns; required! not allowed.
                    // => child
                    $($tt)+
                }
            }
        };
    }

    #[doc(hidden)]
    pub mod button {
        use super::*;
        use zero_ui::core::UiNode;

        // => { child }
        pub fn child(child: impl UiNode) -> impl UiNode {
            child
        }

        // compile test of the property declarations
        #[allow(unused)]
        fn test(child: impl UiNode) -> impl UiNode {
            button! {
                => child
            }
        }
    }
}
