use quote::ToTokens;
use syn::parse_macro_input;

/// `widget!` actual expansion.
///
/// ## In Stage 3
///
/// We have all the inherited tokens to work with. This module is divided in 3,
/// * [input] - Defines the [`input::WidgetDeclaration`] type and parsers for it.
/// * [output] -> Defines the [`output::WidgetOutput`] type and code generation.
/// * [analysis] - Defines the [`analysis::generate`] that converts input into output.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as input::WidgetDeclaration);
    let output = analysis::generate(input);
    let output_stream = output.to_token_stream();
    output_stream.into()
}

/// Widget Stage 3 AST definition and parsing.
///
/// The root type is [`input::WidgetDeclaration`].
pub mod input {
    #![allow(unused)]

    use crate::util::{non_user_braced, non_user_parenthesized};
    use crate::widget_stage1::WidgetHeader;
    use proc_macro2::{TokenStream, TokenTree};
    use quote::ToTokens;
    use syn::parse::discouraged::Speculative;
    use syn::spanned::Spanned;
    use syn::{parse::*, punctuated::Punctuated, *};

    pub mod keyword {
        syn::custom_keyword!(default_child);
        syn::custom_keyword!(required);
        syn::custom_keyword!(unset);
        syn::custom_keyword!(when);
        syn::custom_keyword!(mixin);
        syn::custom_keyword!(new);
        syn::custom_keyword!(new_child);
        syn::custom_keyword!(inherit);
        syn::custom_keyword!(inherited_tokens);
        syn::custom_keyword!(whens);
        syn::custom_keyword!(local);
    }
    pub struct PropertyBlock<P> {
        /// { .. }
        pub brace_token: token::Brace,
        // ..
        pub properties: Vec<P>,
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

    /// Represents a stage 3 widget declaration.
    pub struct WidgetDeclaration {
        /// The included inheritance data.
        ///
        /// This was added to the input in Stage 1 and 2.
        pub inherits: Vec<InheritItem>,

        /// If we are declaring a mix-in.
        ///
        /// This was added to the input in Stage 1.
        pub mixin_signal: MixinSignal,

        /// The widget header.
        ///
        /// The inherits in here should already be included in `inherits` by Stage 1 and 2.
        pub header: WidgetHeader,

        /// The widget declaration items as defined by the developer.
        pub items: Vec<WidgetItem>,
    }
    impl Parse for WidgetDeclaration {
        /// ```text
        /// $(=> inherited_tokens $inherits: InheritItem)*  
        /// $mixin_signal: MixinSignal
        /// $header: WidgetHeader
        /// $($items:WidgetItem)*
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            let mut inherits = Vec::new();
            while input.peek(Token![=>]) && input.peek3(keyword::inherited_tokens) {
                inherits.push(input.parse().unwrap_or_else(|e| non_user_error!(e)));
            }

            let mixin_signal = input.parse().unwrap_or_else(|e| non_user_error!(e));

            let header = input.parse().unwrap_or_else(|e| non_user_error!(e));

            let mut items = Vec::new();
            while !input.is_empty() {
                items.push(input.parse()?);
            }
            Ok(WidgetDeclaration {
                inherits,
                mixin_signal,
                header,
                items,
            })
        }
    }

    /// All the data included by a single inherited widget or mix-in.
    ///
    /// This data was included during Stage 1 and 2.
    pub struct InheritItem {
        /// Ident of widget or mix-in inherited.
        pub ident: Ident,
        /// The path to the inherited widget as typed by the new widget developer.
        pub inherit_path: Path,
        /// If is inheriting a mix-in.
        pub mixin_signal: MixinSignal,

        /// Inherited properties that apply to the widget.
        pub default: Punctuated<InheritedProperty, Token![,]>,
        /// Inherited properties that apply to the widget child.
        pub default_child: Punctuated<InheritedProperty, Token![,]>,
        /// Inherited when conditions.
        pub whens: Punctuated<InheritedWhen, Token![,]>,

        /// Properties captured in the inherited widget `new` function.
        pub new: Punctuated<Ident, Token![,]>,
        /// Properties captured in the inherited widget `new_child` function.
        pub new_child: Punctuated<Ident, Token![,]>,
    }
    impl Parse for InheritItem {
        /// ```text
        /// =>inherited_tokens
        /// $ident:ident $inherit_path:path
        /// $mixin_signal: MixinSignal
        /// { $($default:InheritedProperty),* }
        /// { $($default_child:InheritedProperty),* }
        /// { $($whens:InheritedWhen),* }
        /// { $($new:ident),* }
        /// { $($new_child:ident),* }
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            fn parse_block<T: Parse, R: Parse>(input: ParseStream) -> Punctuated<R, Token![,]> {
                input.parse::<T>().unwrap_or_else(|e| non_user_error!(e));
                let inner = non_user_braced(input);
                Punctuated::parse_terminated(&inner).unwrap_or_else(|e| non_user_error!(e))
            }

            input.parse::<Token![=>]>().unwrap_or_else(|e| non_user_error!(e));
            input.parse::<keyword::inherited_tokens>().unwrap_or_else(|e| non_user_error!(e));

            let input = non_user_braced(input);

            Ok(InheritItem {
                ident: input.parse().unwrap_or_else(|e| non_user_error!(e)),
                inherit_path: input.parse().unwrap_or_else(|e| non_user_error!(e)),
                mixin_signal: input.parse().unwrap_or_else(|e| non_user_error!(e)),
                default: parse_block::<Token![default], InheritedProperty>(&input),
                default_child: parse_block::<keyword::default_child, InheritedProperty>(&input),
                whens: parse_block::<keyword::whens, InheritedWhen>(&input),
                new: parse_block::<keyword::new, Ident>(&input),
                new_child: parse_block::<keyword::new_child, Ident>(&input),
            })
        }
    }

    /// An inherited property in [`InheritItem`].
    pub struct InheritedProperty {
        /// Property documentation.
        ///
        /// We can assume only `#[doc=".."]` attributes are here.
        pub docs: Vec<Attribute>,

        pub kind: BuiltPropertyKind,
        pub ident: Ident,
    }
    impl Parse for InheritedProperty {
        /// ```text
        /// $(#[$docs:Attribute])* $kind:BuiltPropertyKind $ident:Ident
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(InheritedProperty {
                docs: Attribute::parse_outer(input).unwrap_or_else(|e| non_user_error!(e)),
                kind: input.parse().unwrap_or_else(|e| non_user_error!(e)),
                ident: input.parse().unwrap_or_else(|e| non_user_error!(e)),
            })
        }
    }

    /// The kind of [`InheritedProperty`].
    #[derive(PartialEq, Eq)]
    pub enum BuiltPropertyKind {
        /// Property required by the inherited widget, `property: required!;`.
        ///
        /// Required inherited properties cannot be unset by the new widget.
        Required,
        /// Property provided by the inherited widget without default value, `property;`.
        Local,
        /// Property provided by the inherited widget with default value, `property: "..";`.
        Default,
    }
    impl Parse for BuiltPropertyKind {
        /// ```text
        /// (default) => Default;
        /// (local) => Local;
        /// (required) => Required;
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            if input.peek(Token![default]) {
                input.parse::<Token![default]>().unwrap_or_else(|e| non_user_error!(e));
                Ok(BuiltPropertyKind::Default)
            } else if input.peek(keyword::local) {
                input.parse::<keyword::local>().unwrap_or_else(|e| non_user_error!(e));
                Ok(BuiltPropertyKind::Local)
            } else if input.peek(keyword::required) {
                input.parse::<keyword::required>().unwrap_or_else(|e| non_user_error!(e));
                Ok(BuiltPropertyKind::Required)
            } else {
                non_user_error!("expected one of: required, default, local")
            }
        }
    }

    /// A when block inherited in a [`InheritItem`].
    pub struct InheritedWhen {
        /// When documentation.
        ///
        /// We can assume only `#[doc=".."]` attributes are here.
        pub docs: Vec<Attribute>,

        /// Properties used in the expression.
        pub args: Punctuated<Ident, Token![,]>,
        /// Properties set when the expression is true.
        pub sets: Punctuated<Ident, Token![,]>,
    }
    impl Parse for InheritedWhen {
        /// ```text
        /// $(#[$docs:Attribute])*
        /// ( $($args:ident),* )
        /// ( $($sets:ident),* )
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(InheritedWhen {
                docs: Attribute::parse_outer(input).unwrap_or_else(|e| non_user_error!(e)),
                args: Punctuated::parse_terminated(&non_user_parenthesized(input)).unwrap_or_else(|e| non_user_error!(e)),
                sets: Punctuated::parse_terminated(&non_user_braced(input)).unwrap_or_else(|e| non_user_error!(e)),
            })
        }
    }

    /// Flag that indicates if we dealing with a full widget or a mix-in.
    ///
    /// This is used in [`InheritItem`] and [`WidgetDeclaration`].
    pub struct MixinSignal {
        pub mixin_token: keyword::mixin,
        pub colon: Token![:],
        pub value: LitBool,
    }
    impl Parse for MixinSignal {
        /// ```text
        /// mixin: $value:bool
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(MixinSignal {
                mixin_token: input.parse()?,
                colon: input.parse()?,
                value: input.parse()?,
            })
        }
    }

    /// A item in a [`WidgetDeclaration`].
    pub enum WidgetItem {
        /// `default { .. }` or `default_child { .. }`
        Default(WgtItemDefault),
        /// `fn new(..) -> W { .. }` or `fn new_child(..) -> impl UiNode { .. }`
        New(WgtItemNew),
        /// `when .. { .. }`
        When(WgtItemWhen),
    }
    impl Parse for WidgetItem {
        /// ```text
        /// ($default:WgtItemDefault) => Default;
        /// ($(#[$docs:Attribute]*) => {
        ///     ($new:WgtItemNew) => New;
        ///     ($When:WgtItemWhen) => When;
        /// }
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            if input.peek(Token![default]) || input.peek(keyword::default_child) {
                input.parse().map(WidgetItem::Default)
            } else {
                // both new and when can have outer docs.
                let attrs = Attribute::parse_outer(input)?;

                let lookahead = input.lookahead1();
                if attrs.is_empty() {
                    // add Default to the expected tokens message
                    /// that will show if we don't find any match.
                    lookahead.peek(Token![default]);
                    lookahead.peek(keyword::default_child);
                }

                if lookahead.peek(keyword::when) {
                    let mut when: WgtItemWhen = input.parse()?;
                    when.attrs = attrs;
                    Ok(WidgetItem::When(when))
                } else if lookahead.peek(Token![fn]) {
                    let mut new: WgtItemNew = input.parse()?;
                    new.attrs = attrs;
                    Ok(WidgetItem::New(new))
                } else {
                    Err(lookahead.error())
                }
            }
        }
    }

    /// Targeted default properties.
    ///
    /// This is one of the [items](WidgetItem) in a [`WidgetDeclaration`].
    pub struct WgtItemDefault {
        /// If the properties are applied to the widget or the widget child.
        pub target: DefaultTarget,
        pub block: DefaultBlock,
    }

    impl Parse for WgtItemDefault {
        /// ```text
        /// $target:DefaultTarget $block:DefaultBlock
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(WgtItemDefault {
                target: input.parse()?,
                block: input.parse()?,
            })
        }
    }

    /// Target of a [`WgtItemDefault`].
    pub enum DefaultTarget {
        /// Properties apply after the child properties.
        Default(Token![default]),
        /// Properties apply before the widget properties.
        DefaultChild(keyword::default_child),
    }
    impl Parse for DefaultTarget {
        /// ```text
        /// (default) => Default;
        /// (default_child) => DefaultChild;
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            if input.peek(Token![default]) {
                Ok(DefaultTarget::Default(input.parse().unwrap()))
            } else {
                Ok(DefaultTarget::DefaultChild(input.parse()?))
            }
        }
    }

    /// Properties in a [`WgtItemDefault`].
    pub type DefaultBlock = PropertyBlock<PropertyDeclaration>;

    /// Property declared in a [`WgtItemDefault`].
    pub struct PropertyDeclaration {
        /// Outer attributes applied to the property.
        ///
        /// Attribute type is not validated here.
        pub attrs: Vec<Attribute>,

        /// Property name.
        pub ident: Ident,

        /// Actual name of property that is used when `ident` is set.
        ///
        /// If `None` `ident` must be a property module visible for
        /// the widget declaration or one of the inherited properties.
        ///
        /// This is not validated during parsing.
        pub maps_to: Option<MappedProperty>,

        /// Default value for the property.
        pub default_value: Option<(Token![:], PropertyDefaultValue)>,

        /// Terminator.
        pub semi_token: Token![;],
    }
    impl Parse for PropertyDeclaration {
        /// ```text
        /// $(#[$attrs:Attribute])*
        /// $ident:ident $(-> $maps_to:MappedProperty)? $(: $default_value:PropertyDefaultValue)?;
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyDeclaration {
                attrs: Attribute::parse_outer(input)?,
                ident: input.parse()?,
                maps_to: if input.peek(Token![->]) { Some(input.parse()?) } else { None },
                default_value: if input.peek(Token![:]) {
                    Some((input.parse().unwrap(), input.parse()?))
                } else {
                    None
                },
                semi_token: input.parse()?,
            })
        }
    }

    /// Actual property a [`PropertyDeclaration`] uses.
    pub struct MappedProperty {
        pub r_arrow_token: Token![->],
        pub ident: Ident,
    }
    impl Parse for MappedProperty {
        /// ```text
        /// -> $ident:ident
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(MappedProperty {
                r_arrow_token: input.parse()?,
                ident: input.parse()?,
            })
        }
    }

    /// Default value of a [`PropertyDeclaration`].
    pub enum PropertyDefaultValue {
        /// Named arguments, `{ arg0: v0, arg1: v1 }`.
        Fields(PropertyFields),
        /// Unnamed arguments, `v0, v1`.
        Args(PropertyArgs),
        /// `unset!`.
        Unset(PropertyUnset),
        /// `required!`.
        Required(PropertyRequired),
    }
    impl Parse for PropertyDefaultValue {
        fn parse(input: ParseStream) -> Result<Self> {
            parse_property_value(input, true)
        }
    }

    /// ```text
    /// ( $($arg:expr),+ ) => Args;
    /// ( { $($arg:ident : $value:expr) } ) => Fields;
    /// ( unset! ) => Unset;
    /// ( required!) => Required;
    /// ```
    fn parse_property_value(input: ParseStream, allow_required: bool) -> Result<PropertyDefaultValue> {
        // Differentiating between a fields declaration and a single args declaration gets tricky.
        //
        // This is a normal fields decl.: `{ field0: "value" }`
        // This is a block single argument decl.: `{ foo(); bar() }`
        //
        // Fields can use the shorthand field name only `{ field0 }`
        // witch is also a single arg block expression. In this case
        // we parse as Args, if it was a field it will still work because
        // we only have one field.

        // first we buffer ahead to the end of all property declarations `;`.
        let ahead = input.fork();
        let mut buffer = TokenStream::new();
        while !ahead.is_empty() && !ahead.peek(Token![;]) {
            let tt: TokenTree = ahead.parse().unwrap();
            tt.to_tokens(&mut buffer);
        }
        input.advance_to(&ahead);
        // now we have only the property value in `buffer`.

        if let Ok(fields) = syn::parse2::<PropertyFields>(buffer.clone()) {
            if fields.fields.len() == 1 && fields.fields[0].colon_token.is_none() {
                // we parsed `{ ident }` witch can also be a single arg expression
                // and we don't known if ident is a field name, so we parse as args.
                Ok(PropertyDefaultValue::Args(syn::parse2(buffer).expect("{ arg0 }")))
            } else {
                Ok(PropertyDefaultValue::Fields(fields))
            }
        } else if let Ok(args) = syn::parse2(buffer.clone()) {
            Ok(PropertyDefaultValue::Args(args))
        } else if let Ok(unset) = syn::parse2(buffer.clone()) {
            Ok(PropertyDefaultValue::Unset(unset))
        } else if let (true, Ok(required)) = (allow_required, syn::parse2(buffer.clone())) {
            Ok(PropertyDefaultValue::Required(required))
        } else if allow_required {
            Err(Error::new(
                buffer.span(),
                "expected one of: args, named args, `unset!`, `required!`",
            ))
        } else {
            Err(Error::new(buffer.span(), "expected one of: args, named args, `unset!`"))
        }
    }

    /// Named args default value of [`PropertyDeclaration`].
    #[derive(Debug)]
    pub struct PropertyFields {
        /// { .. }
        pub brace_token: token::Brace,
        /// ..
        pub fields: Punctuated<FieldValue, Token![,]>,
    }
    impl Parse for PropertyFields {
        /// ```text
        /// { $($arg:ident : $value:expr),* }
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            let fields;
            Ok(PropertyFields {
                brace_token: braced!(fields in input),
                fields: Punctuated::parse_terminated(&fields)?,
            })
        }
    }

    /// Unnamed args default value of [`PropertyDeclaration`].
    #[derive(Debug)]
    pub struct PropertyArgs(pub Punctuated<Expr, Token![,]>);
    impl Parse for PropertyArgs {
        /// ```text
        /// $($arg:ident),*
        /// ```       
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyArgs(Punctuated::parse_terminated(input)?))
        }
    }

    /// Special value that indicates the property must be removed.
    pub struct PropertyUnset {
        pub unset_token: keyword::unset,
        pub bang_token: Token![!],
    }
    impl Parse for PropertyUnset {
        /// ```text
        /// unset!
        /// ```        
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyUnset {
                unset_token: input.parse()?,
                bang_token: input.parse()?,
            })
        }
    }
    impl Spanned for PropertyUnset {
        fn span(&self) -> proc_macro2::Span {
            let unset_span = self.unset_token.span();
            unset_span.span().join(self.bang_token.span()).unwrap_or(unset_span)
        }
    }

    /// Special value that indicates the property must be set by of the
    /// widget and cannot be unset.
    pub struct PropertyRequired {
        pub required_token: keyword::required,
        pub bang_token: Token![!],
    }
    impl Parse for PropertyRequired {
        /// ```text
        /// required!
        /// ```        
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyRequired {
                required_token: input.parse()?,
                bang_token: input.parse()?,
            })
        }
    }

    /// A `fn new(..) -> W { .. }` or `fn new_child(..) -> impl UiNode { .. }`
    /// in a [`PropertyDeclaration`].
    #[derive(Clone)]
    pub struct WgtItemNew {
        /// Outer attributes applied to the function.
        ///
        /// Attribute type is not validated here.
        pub attrs: Vec<Attribute>,
        pub fn_token: Token![fn],
        /// `new` or `new_child`.
        pub target: NewTarget,

        /// ( .. )
        pub paren_token: token::Paren,
        /// ..
        pub inputs: Punctuated<Ident, Token![,]>,

        pub r_arrow_token: Token![->],
        pub return_type: Box<Type>,

        pub block: Block,
    }

    impl Parse for WgtItemNew {
        /// ```text
        /// $(#[$attrs:meta])*
        /// fn $target:NewTarget ( $($inputs:property_ident),* ) -> $returnType:ty
        /// $block: Block
        /// ```        
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

    /// Witch of the two widget functions is a [`WgtItemNew`].
    #[derive(Clone)]
    pub enum NewTarget {
        /// `new`
        New(keyword::new),
        /// `new_child`
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

    /// A `when` block in a [`WidgetDeclaration`].
    pub struct WgtItemWhen {
        /// Outer attributes applied to the function.
        ///
        /// Attribute type is not validated here.
        pub attrs: Vec<Attribute>,
        pub when_token: keyword::when,
        pub condition: Box<Expr>,
        pub block: WhenBlock,
    }

    impl Parse for WgtItemWhen {
        /// ```text
        /// $(#[$attrs:meta])*
        /// when $condition:expr $block:WhenBlock
        /// ```
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(WgtItemWhen {
                attrs: Attribute::parse_outer(input)?,
                when_token: input.parse()?,
                condition: Box::new(Expr::parse_without_eager_brace(input)?),
                block: input.parse()?,
            })
        }
    }

    /// A block of property assigns in a [`WgtItemWhen`].
    pub type WhenBlock = PropertyBlock<PropertyAssign>;

    /// A property assign in a [`WgtItemWhen`].
    pub struct PropertyAssign {
        pub ident: Ident,
        pub colon_token: Token![:],
        pub value: PropertyValue,
        pub semi_token: Token![;],
    }
    impl Parse for PropertyAssign {
        /// $ident:ident : $value:PropertyValue ;
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyAssign {
                ident: input.parse()?,
                colon_token: input.parse()?,
                value: input.parse()?,
                semi_token: input.parse()?,
            })
        }
    }

    /// A property value is a [`PropertyDeclaration`] or [`PropertyAssign`].
    pub enum PropertyValue {
        /// Named arguments. prop1: { arg0: "value", arg1: "other value" };
        Fields(PropertyFields),
        /// Unnamed arguments. prop1: {"value"}, "other value";
        Args(PropertyArgs),
        /// unset. prop1: unset!;
        Unset(PropertyUnset),
    }
    impl Parse for PropertyValue {
        /// see [`parse_property_value`].
        fn parse(input: ParseStream) -> Result<Self> {
            parse_property_value(input, false).map(|s| match s {
                PropertyDefaultValue::Fields(f) => PropertyValue::Fields(f),
                PropertyDefaultValue::Args(a) => PropertyValue::Args(a),
                PropertyDefaultValue::Unset(u) => PropertyValue::Unset(u),
                PropertyDefaultValue::Required(_) => unreachable!(),
            })
        }
    }
}

/// The meat of widget generation without the distractions of
/// parsing and quoting.
///
/// [`analysis::generate`] is the entry-point.
pub mod analysis {
    use super::input::{self, BuiltPropertyKind, DefaultTarget, NewTarget, PropertyDefaultValue, WidgetDeclaration, WidgetItem};
    use super::output::*;
    use crate::{
        property::input::Prefix as PropertyPrefix,
        util::{Attributes, Errors, PatchSuperPath},
    };
    use input::{PropertyAssign, PropertyValue, WgtItemWhen};
    use proc_macro2::Ident;
    use std::collections::{HashMap, HashSet};
    use std::fmt;
    use syn::{
        parse::{Error, Result},
        parse_quote,
        punctuated::Punctuated,
        spanned::Spanned,
        visit_mut::{self, VisitMut},
        Expr, ExprPath, Member, Visibility,
    };

    pub(super) fn generate(mut input: WidgetDeclaration) -> WidgetOutput {
        // check if all inherits where properly included before moving
        // to Stage 3.
        debug_assert!(
            input
                .header
                .inherits
                .iter()
                .zip(input.inherits.iter().map(|i| &i.inherit_path))
                .all(|(header, included)| header == included),
            "inherits don't match inherited tokens"
        );

        // property resolution order is left-to-right, here we setup inherits
        // so that the left most inherit is the last item, that causes it to
        // override properties from the others.
        input.inherits.reverse();

        // #[macro_export] if `pub` or `pub(crate)`
        let macro_export = match &input.header.vis {
            Visibility::Public(_) => true,
            Visibility::Restricted(r) => r.path.get_ident().map(|i| i == &ident!("crate")).unwrap_or_default(),
            Visibility::Crate(_) | Visibility::Inherited => false,
        };

        // unwrap items
        #[derive(Eq, PartialEq)]
        enum PropertyTarget {
            Default,
            DefaultChild,
        }
        let mut properties = vec![];
        let mut new_fns = vec![];
        let mut new_child_fns = vec![];
        let mut whens = vec![];
        for item in input.items {
            match item {
                WidgetItem::Default(d) => match d.target {
                    DefaultTarget::Default(_) => properties.extend(d.block.properties.into_iter().map(|p| (PropertyTarget::Default, p))),
                    DefaultTarget::DefaultChild(_) => {
                        properties.extend(d.block.properties.into_iter().map(|p| (PropertyTarget::DefaultChild, p)))
                    }
                },
                WidgetItem::New(n) => match &n.target {
                    NewTarget::New(_) => new_fns.push(n),
                    NewTarget::NewChild(_) => new_child_fns.push(n),
                },
                WidgetItem::When(w) => whens.push(w),
            }
        }

        //validate items
        let mut errors = Errors::default();
        let mut new_properties = HashSet::new();
        properties.retain(|(_, property)| {
            let inserted = new_properties.insert(property.ident.clone());
            if !inserted {
                errors.push(format!("property `{}` already declared", property.ident), property.ident.span());
            }
            inserted
        });
        for extra_new in new_fns.iter().skip(1).chain(new_child_fns.iter().skip(1)) {
            errors.push(format!("function `{}` already declared", extra_new.target), extra_new.target.span())
        }
        validate_whens(&mut whens, &mut errors);

        // map that defines each property origin.
        // widgets override properties with the same name when inheriting,
        // the map is (property: Ident, widget: Path), the widget is `Self` for
        // properties declared locally.
        let mut inheritance_map = HashMap::new();
        for inherit in &input.inherits {
            for property in inherit.default_child.iter().chain(inherit.default.iter()) {
                inheritance_map.insert(
                    property.ident.clone(),
                    PropertyOrigin::Inherited(inherit.inherit_path.clone(), false),
                );
            }
        }
        for property in properties.iter() {
            if let Some(PropertyOrigin::Inherited(_, has_new_value)) = inheritance_map.get_mut(&property.1.ident) {
                *has_new_value = true;
            } else {
                let old_value = inheritance_map.insert(property.1.ident.clone(), PropertyOrigin::New);
                debug_assert!(old_value.is_none())
            }
        }

        enum PropertyOrigin {
            Inherited(syn::Path, bool),
            New,
        }

        impl PropertyOrigin {
            fn get_path(&self, if_has_new_value: bool) -> Option<&syn::Path> {
                use PropertyOrigin::*;
                match self {
                    Inherited(path, has_new_value) if *has_new_value == if_has_new_value => Some(path),
                    _ => None,
                }
            }

            pub fn inherited_path(&self) -> Option<&syn::Path> {
                self.get_path(false)
            }
            pub fn setted_path(&self) -> Option<&syn::Path> {
                self.get_path(true)
            }
        }

        // all `when` for the macro
        let mut macro_whens = vec![];
        // all inherited `when` for the mod.
        let mut mod_whens = vec![];
        // next available index for when function names.
        let mut when_index = 0;
        //all properties that have a initial value
        let mut inited_properties = HashSet::new();

        // all properties for the macro
        let mut macro_default = vec![];
        let mut macro_default_child = vec![];

        // all properties for the mod
        let mut mod_properties = WidgetProperties::default();
        let mut mod_defaults = WidgetDefaults::default();

        // all property docs
        let mut docs_required = vec![];
        let mut docs_provided = vec![];
        let mut docs_state = vec![];
        let mut docs_other = vec![];

        let mut inherited_fns = None;

        // process inherited properties and when blocks.
        for inherit in input.inherits {
            let inherit_path = inherit.inherit_path;

            // collects all inherited property information.
            let mut process_properties =
                |target: PropertyTarget, properties: Punctuated<input::InheritedProperty, _>, macro_defaults: &mut Vec<BuiltProperty>| {
                    for property in properties {
                        if inheritance_map[&property.ident].inherited_path() != Some(&inherit_path) {
                            continue;
                        }
                        // if property is not overridden:

                        // if inherited required property or property with default value
                        // it is one of the always initialized properties.
                        if property.kind != BuiltPropertyKind::Local {
                            assert!(inited_properties.insert(property.ident.clone()));
                        }

                        // widget mod need to re-export from inherited widget mod.
                        mod_properties.props.push(WidgetPropertyUse::Inherited {
                            widget: inherit_path.clone(),
                            ident: property.ident.clone(),
                        });

                        // if the inherited property has a default value, generate a default function
                        // that calls the function from the inherited widget mod.
                        if property.kind == BuiltPropertyKind::Default {
                            mod_defaults.defaults.push(WidgetDefault {
                                property: property.ident.clone(),
                                default: FinalPropertyDefaultValue::Inherited(inherit_path.clone()),
                            });
                        }

                        let docs = match property.kind {
                            BuiltPropertyKind::Default => &mut docs_provided,
                            BuiltPropertyKind::Required => &mut docs_required,
                            BuiltPropertyKind::Local => {
                                if PropertyPrefix::is_state(&property.ident) {
                                    &mut docs_state
                                } else {
                                    &mut docs_other
                                }
                            }
                        };
                        docs.push(PropertyDocs {
                            docs: property.docs.clone(),
                            target_child: target == PropertyTarget::DefaultChild,
                            ident: property.ident.clone(),
                            property_source: PropertySource::Widget(inherit_path.clone(), property.ident.clone()),
                            is_required_provided: false,
                        });

                        // InheritedProperty is BuiltProperty already.
                        macro_defaults.push(property);
                    }
                };
            process_properties(PropertyTarget::Default, inherit.default, &mut macro_default);
            process_properties(PropertyTarget::DefaultChild, inherit.default_child, &mut macro_default_child);

            for (inherited_index, when) in inherit.whens.into_iter().enumerate() {
                mod_whens.push(WhenCondition {
                    index: when_index,
                    properties: when.args.iter().cloned().collect(),
                    expr: WhenConditionExpr::Inherited(InheritedWhen {
                        widget: inherit_path.clone(),
                        when_name: when_fn_name(when_index),
                        properties: when.args.iter().cloned().collect(),
                    }),
                    #[cfg(debug_assertions)]
                    expr_str: None,
                    #[cfg(debug_assertions)]
                    property_sets: vec![],
                });

                mod_defaults.when_defaults.push(WhenDefaults {
                    index: when_index,
                    defaults: when
                        .sets
                        .iter()
                        .map(|p| WidgetDefault {
                            property: p.clone(),
                            default: FinalPropertyDefaultValue::WhenInherited(inherit_path.clone(), inherited_index as u32),
                        })
                        .collect(),
                });

                // mod_properties.props not needed, compiled widgets include when condition properties as local.

                macro_whens.push(when);

                when_index += 1;
            }

            if !inherit.mixin_signal.value.value {
                inherited_fns = Some((inherit_path, inherit.new, inherit.new_child));
            }
        }
        // process newly declared properties
        // widget_mod { defaults { <moved here> } }
        let mut patch_super = PatchSuperPath::new(2);
        for (target, property) in properties {
            let mut has_value = true;
            let mut is_required = false;
            let mut default_value = None;

            match property.default_value {
                Some((_, value)) => match value {
                    PropertyDefaultValue::Fields(mut fields) => {
                        for field in fields.fields.iter_mut() {
                            patch_super.visit_field_value_mut(field);
                        }
                        default_value = Some(FinalPropertyDefaultValue::Fields(fields))
                    }
                    PropertyDefaultValue::Args(mut args) => {
                        for expr in args.0.iter_mut() {
                            patch_super.visit_expr_mut(expr);
                        }
                        default_value = Some(FinalPropertyDefaultValue::Args(args))
                    }

                    PropertyDefaultValue::Unset(_) => continue,

                    PropertyDefaultValue::Required(_) => is_required = true,
                },
                None => has_value = false,
            }

            if has_value {
                assert!(inited_properties.insert(property.ident.clone()));
            }

            let docs = if is_required {
                &mut docs_required
            } else if has_value {
                &mut docs_provided
            } else if PropertyPrefix::is_state(&property.ident) {
                &mut docs_state
            } else {
                &mut docs_other
            };
            let attrs = Attributes::new(property.attrs);

            if let Some(default) = default_value {
                mod_defaults.defaults.push(WidgetDefault {
                    property: property.ident.clone(),
                    default,
                })
            }

            let macro_properties = match target {
                PropertyTarget::Default => &mut macro_default,
                PropertyTarget::DefaultChild => &mut macro_default_child,
            };
            macro_properties.push(BuiltProperty {
                docs: attrs.docs.clone(),
                kind: if is_required {
                    BuiltPropertyKind::Required
                } else if has_value {
                    BuiltPropertyKind::Default
                } else {
                    BuiltPropertyKind::Local
                },
                ident: property.ident.clone(),
            });

            if let Some(maps_to) = property.maps_to {
                // property maps to another, re-export with new property name.

                if let Some(widget) = inheritance_map.get(&maps_to.ident).and_then(|o| o.setted_path()) {
                    // property maps to another inherited property
                    docs.push(PropertyDocs {
                        docs: attrs.docs,
                        target_child: target == PropertyTarget::DefaultChild,
                        ident: property.ident.clone(),
                        property_source: PropertySource::Widget(widget.clone(), maps_to.ident.clone()),
                        is_required_provided: false,
                    });
                    mod_properties.props.push(WidgetPropertyUse::AliasInherited {
                        ident: property.ident,
                        widget: widget.clone(),
                        original: maps_to.ident,
                    });
                } else {
                    // property maps to a new property
                    docs.push(PropertyDocs {
                        docs: attrs.docs,
                        target_child: target == PropertyTarget::DefaultChild,
                        ident: property.ident.clone(),
                        property_source: PropertySource::Property(maps_to.ident.clone()),
                        is_required_provided: false,
                    });
                    mod_properties.props.push(WidgetPropertyUse::Alias {
                        ident: property.ident,
                        original: maps_to.ident,
                    });
                }
            } else {
                // property does not map to another, re-export the property mod.

                if let Some(widget) = inheritance_map[&property.ident].setted_path() {
                    // property sets inherited property
                    docs.push(PropertyDocs {
                        docs: attrs.docs,
                        target_child: target == PropertyTarget::DefaultChild,
                        ident: property.ident.clone(),
                        property_source: PropertySource::Widget(widget.clone(), property.ident.clone()),
                        is_required_provided: false,
                    });
                    mod_properties.props.push(WidgetPropertyUse::Inherited {
                        widget: widget.clone(),
                        ident: property.ident,
                    });
                } else {
                    // property is new
                    docs.push(PropertyDocs {
                        docs: attrs.docs,
                        target_child: target == PropertyTarget::DefaultChild,
                        ident: property.ident.clone(),
                        property_source: PropertySource::Property(property.ident.clone()),
                        is_required_provided: false,
                    });
                    mod_properties.props.push(WidgetPropertyUse::Mod(property.ident));
                }
            }
        }

        // process newly declared whens
        validate_whens_with_default(&mut whens, &mut errors, inited_properties);
        for when in whens {
            #[cfg(debug_assertions)]
            let expr_str = {
                let c = &when.condition;
                Some(quote!(#c).to_string())
            };

            let when_analysis = match WhenConditionAnalysis::new(when.condition) {
                Ok(r) => r,
                Err(e) => {
                    errors.push_syn(e);
                    continue;
                }
            };

            for p in when_analysis.properties.iter() {
                if !inheritance_map.contains_key(&p.property) {
                    inheritance_map.insert(p.property.clone(), PropertyOrigin::New);

                    mod_properties.props.push(WidgetPropertyUse::Mod(p.property.clone()));

                    let docs = if PropertyPrefix::is_state(&p.property.clone()) {
                        &mut docs_state
                    } else {
                        &mut docs_other
                    };

                    let property_docs = vec![]; // TODO import property doc first line.

                    docs.push(PropertyDocs {
                        docs: property_docs.clone(),
                        target_child: false,
                        ident: p.property.clone(),
                        property_source: PropertySource::Property(p.property.clone()),
                        is_required_provided: false,
                    });

                    macro_default.push(BuiltProperty {
                        docs: property_docs,
                        kind: BuiltPropertyKind::Local,
                        ident: p.property.clone(),
                    });
                } else {
                }
            }

            // widget_mod { whens {  <moved here> } }
            let mut patch_super = PatchSuperPath::new(2);
            let mut expr = when_analysis.expr;
            match &mut expr {
                WhenConditionExpr::Map(_, expr) | WhenConditionExpr::Merge(_, expr) => {
                    patch_super.visit_expr_mut(expr);
                }
                _ => {}
            }
            mod_whens.push(WhenCondition {
                index: when_index,
                properties: when_analysis.properties.iter().map(|p| &p.property).cloned().collect(),
                expr,
                #[cfg(debug_assertions)]
                expr_str,
                #[cfg(debug_assertions)]
                property_sets: when.block.properties.iter().map(|p| p.ident.clone()).collect(),
            });

            let attributes = Attributes::new(when.attrs);
            macro_whens.push(input::InheritedWhen {
                docs: attributes.docs,
                args: when_analysis.properties.into_iter().map(|p| p.property).collect(),
                sets: when.block.properties.iter().map(|p| p.ident.clone()).collect(),
            });

            // widget_mod { when_defaults { w1 { <moved here> } } }
            let mut patch_super = PatchSuperPath::new(3);
            mod_defaults.when_defaults.push(WhenDefaults {
                index: when_index,
                defaults: when
                    .block
                    .properties
                    .into_iter()
                    .map(|p| WidgetDefault {
                        property: p.ident,
                        default: match p.value {
                            PropertyValue::Fields(mut fields) => {
                                for field in fields.fields.iter_mut() {
                                    patch_super.visit_field_value_mut(field);
                                }
                                FinalPropertyDefaultValue::Fields(fields)
                            }
                            PropertyValue::Args(mut args) => {
                                for expr in args.0.iter_mut() {
                                    patch_super.visit_expr_mut(expr);
                                }
                                FinalPropertyDefaultValue::Args(args)
                            }
                            PropertyValue::Unset(_) => unreachable!("error case removed early"),
                        },
                    })
                    .collect(),
            });

            when_index += 1;
        }

        debug_assert_eq!(when_index, macro_whens.len());
        debug_assert_eq!(when_index, mod_whens.len());

        let macro_new;
        let macro_new_child;
        let new;
        let new_child;
        if let Some(mut fn_) = new_fns.drain(..).next() {
            macro_new = BuiltNew {
                properties: fn_.inputs.iter().skip(1).cloned().collect(),
            };
            let mut patch_super = PatchSuperPath::new(1);
            patch_super.visit_block_mut(&mut fn_.block);
            patch_super.visit_type_mut(&mut *fn_.return_type);
            for attr in fn_.attrs.iter_mut() {
                patch_super.visit_attribute_mut(attr);
            }
            new = NewFn::New(fn_);
        } else if let Some((inherited, fn_, _)) = &inherited_fns {
            macro_new = BuiltNew {
                properties: fn_.iter().cloned().collect(),
            };
            new = NewFn::Inherited(inherited.clone());
        } else {
            macro_new = BuiltNew {
                properties: vec![ident!("id")],
            };
            new = NewFn::None;
        }
        if let Some(mut fn_) = new_child_fns.drain(..).next() {
            macro_new_child = BuiltNew {
                properties: fn_.inputs.iter().cloned().collect(),
            };
            let mut patch_super = PatchSuperPath::new(1);
            patch_super.visit_block_mut(&mut fn_.block);
            patch_super.visit_type_mut(&mut *fn_.return_type);
            for attr in fn_.attrs.iter_mut() {
                patch_super.visit_attribute_mut(attr);
            }
            new_child = NewFn::New(fn_);
        } else if let Some((inherited, _, fn_)) = inherited_fns {
            macro_new_child = BuiltNew {
                properties: fn_.into_iter().collect(),
            };
            new_child = NewFn::Inherited(inherited);
        } else {
            macro_new_child = BuiltNew::default();
            new_child = NewFn::None;
        }

        let mut captured_properties = HashSet::new();
        for captured in macro_new_child.properties.iter().chain(macro_new.properties.iter()) {
            if !captured_properties.insert(captured) {
                errors.push(format! {"property `{}` already captured", captured}, captured.span())
            }
        }
        //docs_other.drain_filter()
        let mut i = 0;
        while i != docs_other.len() {
            if captured_properties.contains(&docs_other[i].ident) {
                let doc = docs_other.remove(i);
                docs_required.push(doc)
            } else {
                i += 1;
            }
        }
        for doc in &mut docs_provided {
            doc.is_required_provided = captured_properties.contains(&doc.ident);
        }

        let Attributes {
            docs,
            cfg,
            others: mut attrs,
            ..
        } = Attributes::new(input.header.attrs);

        if let Some(cfg) = &cfg {
            attrs.push(cfg.clone());
        }

        let is_mixin = input.mixin_signal.value.value;

        WidgetOutput {
            macro_: WidgetMacro {
                cfg,
                widget_name: input.header.name.clone(),
                vis: input.header.vis.clone(),
                export: macro_export,
                is_mixin,
                default: macro_default,
                default_child: macro_default_child,
                whens: macro_whens,
                new: macro_new,
                new_child: macro_new_child,
            },
            mod_: WidgetMod {
                docs: WidgetDocs {
                    docs,
                    is_mixin,
                    required: docs_required,
                    provided: docs_provided,
                    other: docs_other,
                    state: docs_state,
                },
                attrs,
                vis: input.header.vis,
                widget_name: input.header.name,
                is_mixin,
                new,
                new_child,
                properties: mod_properties,
                defaults: mod_defaults,
                whens: WidgetWhens { conditions: mod_whens },
            },
            errors,
        }
    }

    /// Validates when assigns, removes duplicates and invalid values.
    pub fn validate_whens(whens: &mut [WgtItemWhen], errors: &mut Errors) {
        for when in whens {
            validate_property_assigns_impl(&mut when.block.properties, errors, true);
        }
    }
    /// Validate when assigns in the context of what properties have a default state value.
    pub fn validate_whens_with_default(whens: &mut [WgtItemWhen], errors: &mut Errors, defaults: HashSet<Ident>) {
        for when in whens {
            // only supports properties that have a default value.
            when.block.properties.retain(|property| {
                let used = defaults.contains(&property.ident);
                if !used {
                    errors.push(
                        format!("property `{}` is not used in this widget", property.ident),
                        property.ident.span(),
                    );
                }
                used
            });
        }
    }
    /// Validates property assigns, removes duplicates.
    pub fn validate_property_assigns(properties: &mut Vec<PropertyAssign>, errors: &mut Errors) {
        validate_property_assigns_impl(properties, errors, false)
    }
    fn validate_property_assigns_impl(properties: &mut Vec<PropertyAssign>, errors: &mut Errors, is_in_when: bool) {
        let mut property_names = HashSet::new();
        properties.retain(|property| {
            let inserted = property_names.insert(property.ident.clone());
            if !inserted {
                errors.push(format!("property `{}` already set", property.ident), property.ident.span());
            }
            let mut retain = inserted;
            if is_in_when {
                if let PropertyValue::Unset(unset) = &property.value {
                    retain = false;
                    errors.push("cannot unset property in when blocks", unset.span());
                }
            }
            retain
        })
    }

    impl fmt::Display for NewTarget {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", quote!(#self))
        }
    }

    /// Find properties referenced in the condition expression and patches the
    /// expression so it can be used inside the transformed condition var.
    pub struct WhenConditionAnalysis {
        /// All property refs
        pub properties: HashSet<WhenPropertyRef>,
        pub expr: WhenConditionExpr,
    }
    impl WhenConditionAnalysis {
        pub fn new(mut condition: Box<Expr>) -> Result<Self> {
            let when_condition_span = condition.span();

            let mut visitor = WhenConditionVisitor::default();
            visitor.visit_expr_mut(&mut condition);

            // when expressions must have at least one 'self.property'.
            if visitor.properties.is_empty() {
                Err(Error::new(when_condition_span, "when condition does not reference any property"))
            } else {
                Ok(WhenConditionAnalysis {
                    expr: if visitor.found_mult_exprs {
                        if visitor.properties.len() == 1 {
                            WhenConditionExpr::Map(visitor.properties.iter().next().unwrap().clone(), condition)
                        } else {
                            WhenConditionExpr::Merge(visitor.properties.clone(), condition)
                        }
                    } else {
                        WhenConditionExpr::Ref(visitor.properties.iter().next().unwrap().clone())
                    },
                    properties: visitor.properties,
                })
            }
        }
    }

    #[derive(Default)]
    struct WhenConditionVisitor {
        properties: HashSet<WhenPropertyRef>,
        found_mult_exprs: bool,
    }
    impl VisitMut for WhenConditionVisitor {
        //visit expressions like:
        // self.is_hovered
        // self.is_hovered.0
        // self.is_hovered.state
        fn visit_expr_mut(&mut self, expr: &mut Expr) {
            //get self or child
            fn is_self(expr_path: &ExprPath) -> bool {
                if let Some(ident) = expr_path.path.get_ident() {
                    return ident == &ident!("self");
                }
                false
            }

            let mut found = None;

            if let Expr::Field(expr_field) = expr {
                match &mut *expr_field.base {
                    // self.is_hovered
                    Expr::Path(expr_path) => {
                        if let (true, Member::Named(property)) = (is_self(expr_path), expr_field.member.clone()) {
                            found = Some(WhenPropertyRef {
                                property,
                                arg: WhenPropertyRefArg::Index(0),
                            })
                        }
                    }
                    // self.is_hovered.0
                    // self.is_hovered.state
                    Expr::Field(i_expr_field) => {
                        if let Expr::Path(expr_path) = &mut *i_expr_field.base {
                            if let (true, Member::Named(property)) = (is_self(expr_path), i_expr_field.member.clone()) {
                                found = Some(WhenPropertyRef {
                                    property,
                                    arg: expr_field.member.clone().into(),
                                })
                            }
                        }
                    }
                    _ => {}
                }
            }

            if let Some(p) = found {
                let replacement = p.name();
                *expr = parse_quote!((*#replacement));
                self.properties.insert(p);
            } else {
                self.found_mult_exprs = true;
                visit_mut::visit_expr_mut(self, expr);
            }
        }
    }

    impl From<Member> for WhenPropertyRefArg {
        fn from(member: Member) -> Self {
            match member {
                Member::Named(ident) => WhenPropertyRefArg::Named(ident),
                Member::Unnamed(i) => WhenPropertyRefArg::Index(i.index),
            }
        }
    }
}

/// Widget output AST and code generation.
///
/// The root type is [`output::WidgetOutput`].
pub mod output {
    use super::input::{keyword, BuiltPropertyKind, NewTarget, PropertyArgs, PropertyFields, WgtItemNew};
    use crate::util::{crate_core, docs_with_first_line_js, uuid, Errors};
    use proc_macro2::{Ident, TokenStream};
    use quote::ToTokens;
    use std::{collections::HashSet, fmt};
    use syn::spanned::Spanned;
    use syn::{Attribute, Expr, Path, Token, Visibility};

    pub use super::input::{InheritedProperty as BuiltProperty, InheritedWhen as BuiltWhen};

    /// All the data needed to generate a widget module and macro.
    pub struct WidgetOutput {
        pub macro_: WidgetMacro,
        pub mod_: WidgetMod,
        pub errors: Errors,
    }
    impl ToTokens for WidgetOutput {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.errors.to_tokens(tokens);
            self.macro_.to_tokens(tokens);
            self.mod_.to_tokens(tokens);
        }
    }

    /// The widget macro.
    pub struct WidgetMacro {
        pub cfg: Option<Attribute>,
        pub widget_name: Ident,
        pub vis: Visibility,
        pub export: bool,
        pub is_mixin: bool,
        pub default: Vec<BuiltProperty>,
        pub default_child: Vec<BuiltProperty>,
        pub whens: Vec<BuiltWhen>,
        pub new: BuiltNew,
        pub new_child: BuiltNew,
    }
    impl ToTokens for WidgetMacro {
        /// ```text
        /// #[macro_export]
        /// macro_rules! #widget_name_GUID {
        ///
        ///     // inherit branch, present in widget and mix-ins, calls the [`widget_stage2!`]
        ///     // including this widgets information so it can be inherited.
        ///     (-> inherit { $stage3_entry:ident; $named_as:path; $($inherit_next:tt)* } $($rest:tt)*) => {
        ///         zero_ui::core::widget_stage2! {
        ///             // continuation of the inheritance recursive calls.
        ///             => {
        ///                 $stage3_entry;
        ///                 $($inherit_next)*        
        ///             }
        ///
        ///             // data to inherit from this widget.
        ///             => inherited_tokens {
        ///                 #widget_name
        ///                 $named_as
        ///                 mixin: #is_mixin
        ///                 default { #(#default),* } // see [`BuiltProperty`]
        ///                 default_child { #(#default_child),* } // see [`BuiltProperty`]
        ///                 whens { #(#whens),* } // see [`BuiltWhen`]
        ///                 new { #new } // see [`BuiltNew`]
        ///                 new_child { #new_child } // see [`BuiltNew`]
        ///             }
        ///         }
        ///     };
        ///
        ///     // instantiate branch, present only for full widgets, not mix-ins. Calls [`widget_new!`]
        ///     // including all the information needed to instantiate the widget
        ///     ($($input:tt)*) => {
        ///         zero_ui::widget_new! {
        ///             #widget_name
        ///             
        ///             // these are the same types used in inherited_tokens
        ///             // except this time the #[doc=".."] attributes are not included.
        ///             default { #(#default),* }
        ///             default_child { #(#default_child),* }
        ///             whens { #(#whens),* }
        ///             new { #new }
        ///             new_child { #new_child }
        ///
        ///             // the widget user input.
        ///             user_input { $($input)* }
        ///         }
        ///     };
        /// }
        ///
        /// // Using a random name and then reexporting is a trick to scope the macro in its parent
        /// // module. Also allows more then one widget with the same name in different modules in the same crate.
        ///
        /// // When macro 2.0 is stable we can remove this.
        ///
        /// #[cfg(..)]// if set in the declaration header.
        /// #[doc(hidden)]
        /// pub use #widget_name_GUID as #widget_name;
        /// ```
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let crate_ = crate_core();
            let name = &self.widget_name;
            let is_mixin = self.is_mixin;

            let default = &self.default;
            let default_child = &self.default_child;
            let whens = &self.whens;
            let new = &self.new;
            let new_child = &self.new_child;

            let inherit_arm = quote! {
                (-> inherit { $stage3_entry:ident; $named_as:path; $($inherit_next:tt)* } $($rest:tt)*) => {
                    #crate_::widget_stage2! {
                        => {
                            $stage3_entry;
                            $($inherit_next)*
                        }

                        => inherited_tokens {
                            #name
                            $named_as
                            mixin: #is_mixin
                            default { #(#default),* }
                            default_child { #(#default_child),* }
                            whens { #(#whens),* }
                            new { #new }
                            new_child { #new_child }
                        }

                        $($rest)*
                    }
                };
            };

            let new_arm = if self.is_mixin {
                None
            } else {
                let default = self.default.iter().map(|p| p.tokens(false));
                let default_child = self.default_child.iter().map(|p| p.tokens(false));
                let whens = self.whens.iter().map(|p| p.tokens(false));

                Some(quote! {
                    ($($input:tt)*) => {
                        #crate_::widget_new! {
                            #name
                            default { #(#default),* }
                            default_child { #(#default_child),* }
                            whens { #(#whens),* }
                            new { #new }
                            new_child { #new_child }
                            user_input { $($input)* }
                        }
                    };
                })
            };

            let unique_name = ident!("{}_{}", self.widget_name, uuid());
            let cfg = &self.cfg;
            let vis = &self.vis;

            tokens.extend(quote!( #cfg #[doc(hidden)] ));
            if self.export {
                tokens.extend(quote!( #[macro_export] ));
            }
            tokens.extend(quote! {
                macro_rules! #unique_name {
                    #inherit_arm
                    #new_arm
                }

                #cfg
                #[doc(hidden)]
                #vis use #unique_name as #name;
            });
        }
    }
    impl BuiltProperty {
        /// ```text
        /// #(#docs)* // if `include_docs`
        /// #kind #ident
        /// ```
        fn tokens(&self, include_docs: bool) -> TokenStream {
            let mut r = TokenStream::new();
            if include_docs {
                let docs = &self.docs;
                r.extend(quote!( #(#docs)* ));
            }
            self.kind.to_tokens(&mut r);
            self.ident.to_tokens(&mut r);
            r
        }
    }
    impl ToTokens for BuiltProperty {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.extend(self.tokens(true))
        }
    }
    impl ToTokens for BuiltPropertyKind {
        /// Writes the keyword for each kind.
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                BuiltPropertyKind::Required => keyword::required::default().to_tokens(tokens),
                BuiltPropertyKind::Local => keyword::local::default().to_tokens(tokens),
                BuiltPropertyKind::Default => <Token![default]>::default().to_tokens(tokens),
            }
        }
    }
    impl BuiltWhen {
        /// ```text
        /// #(#docs)* // if `include_docs`
        /// (#(#args),*) { #(#sets),* }
        /// ```
        fn tokens(&self, include_docs: bool) -> TokenStream {
            let mut r = TokenStream::new();
            if include_docs {
                let docs = &self.docs;
                r.extend(quote!( #(#docs)* ));
            }
            let args = &self.args;
            let sets = &self.sets;
            r.extend(quote! {
                 (#args) { #sets }
            });
            r
        }
    }
    impl ToTokens for BuiltWhen {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.extend(self.tokens(true))
        }
    }

    #[derive(Default)]
    pub struct BuiltNew {
        pub properties: Vec<Ident>,
    }
    impl ToTokens for BuiltNew {
        /// ```text
        /// #(#properties),*
        /// ```
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let props = &self.properties;
            tokens.extend(quote!( #(#props),* ))
        }
    }

    /// The widget module, contains all the types and function required
    /// for instantiating the widget. This is also the public face of the widget
    /// the macro is hidden.
    pub struct WidgetMod {
        pub docs: WidgetDocs,
        pub attrs: Vec<Attribute>,
        pub vis: Visibility,
        pub widget_name: Ident,
        pub is_mixin: bool,
        pub new: NewFn,
        pub new_child: NewFn,
        pub properties: WidgetProperties,
        pub defaults: WidgetDefaults,
        pub whens: WidgetWhens,
    }
    impl ToTokens for WidgetMod {
        /// ```text
        /// #(#attrs)* // all attributes that are not docs
        /// #docs // see [`WidgetDocs`], includes header docs
        /// #vis mod #widget_name {
        ///     use super::*;
        ///     use zero_ui::core::widget_base::implicit_mixin;
        ///
        ///     #new // see [`NewFn`]
        ///     #new_child
        ///     
        ///     #properties // see [`WidgetProperties`]
        ///
        ///     #defaults // see [`WidgetDefaults`]
        ///
        ///     #whens // see [`WidgetWhens`]
        /// }
        /// ```
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let attrs = self.attrs.iter();
            let docs = &self.docs;
            let vis = &self.vis;
            let widget_name = &self.widget_name;
            let crate_ = crate_core();

            let some_mixin = if self.is_mixin { None } else { Some(()) };
            let use_implicit_mixin = some_mixin.map(|_| quote!( use #crate_::widget_base::implicit_mixin; ));
            let new = some_mixin.map(|_| self.new.new_tokens(widget_name));
            let new_child = some_mixin.map(|_| self.new_child.new_child_tokens(widget_name));

            let properties = &self.properties;
            let defaults = &self.defaults;
            let whens = &self.whens;

            let doc_helper_mod = docs.helper_mod_tokens();

            tokens.extend(quote! {
                #(#attrs)*
                #docs
                #vis mod #widget_name {
                    use super::*;
                    #use_implicit_mixin

                    // new functions.
                    #new
                    #new_child

                    // properties re-export mod.
                    #properties

                    // property default values mod.
                    #defaults

                    // when condition var init fns mod.
                    #whens

                    #doc_helper_mod
                }
            })
        }
    }

    /// Widget module documentation generation.
    ///
    /// Makes extensive use of HTML, CSS and JS inlining to present the concept
    /// of widgets as a first class item.
    pub struct WidgetDocs {
        pub docs: Vec<Attribute>,
        pub is_mixin: bool,
        ///required properties
        pub required: Vec<PropertyDocs>,
        ///properties with provided default value
        pub provided: Vec<PropertyDocs>,
        ///properties that are defined in the widget, but have no default value and are not required
        pub other: Vec<PropertyDocs>,
        ///state properties that are defined in the widget
        pub state: Vec<PropertyDocs>,
    }
    impl WidgetDocs {
        /// Generates a module with documentation links for rust-doc to expand.
        ///
        /// The module is hidden by CSS from the module list, when loaded in an IFRAME sends
        /// a message back using `postMessage`.
        fn helper_mod_tokens(&self) -> Option<TokenStream> {
            if self.state.is_empty() {
                None
            } else {
                let properties = self
                    .required
                    .iter()
                    .chain(&self.provided)
                    .chain(&self.other)
                    .chain(&self.state)
                    .filter(|p| p.docs.is_empty())
                    .map(|p| &p.ident);

                let callback = js_tag!("widget_doc_helper_ext.js");

                Some(quote! {
                    /// <style>#modules, a[href="doc_helper/index.html"], a[href="#modules"] { display: none; }</style>
                    ///
                    #[doc=#callback]
                    pub mod doc_helper {
                        #(
                            #[doc(inline)]
                            pub use super::properties::#properties;
                        )*
                    }
                })
            }
        }
    }
    impl ToTokens for WidgetDocs {
        /// The widget header docs are inserted first, but spliced in the first line with JS that
        /// moves the module to a new section called "Widget Modules". This is because rust-doc includes
        /// the first line in the module list section of the parent module and we need our JS to affect that list.
        ///
        /// The header docs just looks like a normal module header docs in the full page. After the header docs
        /// we insert a `</DIV>` and go rogue. This causes a small HTML parsing error at the end of the file, because
        /// rust-doc will still close a DIV after our custom stuff.
        ///
        /// After the header we create sections for properties using the `<H2 class="small-section-header">` that is
        /// used by other rust-docs generated content. The properties are divided in *required*, *provided*, *state* and *other*.
        ///
        /// *Required properties* are those marked `required!`, *provided properties* are the properties that have a default value,
        /// *state properties* are the `is_state` properties that are defined in the widget and *other properties* are the properties
        /// defined in the widget without any value set.
        ///
        /// The documentation for each property is included, see [`PropertyDocs`] for details.
        fn to_tokens(&self, tokens: &mut TokenStream) {
            docs_with_first_line_js(tokens, &self.docs, js_tag!("widget_mods_ext.js"));

            doc_extend!(
                tokens,
                "\n</div><style>span.wgprop p {{ display: inline; margin-left:-1ch; }}</style>{}",
                js_tag!("widget_docs_ext.js")
            );

            fn open_section(tokens: &mut TokenStream, id: &str, title: &str) {
                doc_extend!(
                    tokens,
                    r##"<h2 id="{0}" class="small-section-header">{1}<a href="#{0}" class="anchor"></a></h2><div class="methods" style="display: block;">"##,
                    id,
                    title
                )
            }
            fn close_section(tokens: &mut TokenStream) {
                doc_extend!(tokens, "</div>")
            }

            if !self.required.is_empty() {
                open_section(tokens, "required-properties", "Required properties");
                for doc in &self.required {
                    doc.to_tokens(tokens);
                }
                close_section(tokens);
            }
            if !self.provided.is_empty() {
                open_section(tokens, "provided-properties", "Provided properties");
                for doc in &self.provided {
                    doc.to_tokens(tokens);
                }
                close_section(tokens);
            }
            if !self.state.is_empty() {
                open_section(tokens, "state-properties", "State properties");
                for doc in &self.state {
                    doc.to_tokens(tokens);
                }
                close_section(tokens);
            }

            if !self.is_mixin || !self.other.is_empty() {
                open_section(tokens, "other-properties", "Other properties");
                for doc in &self.other {
                    doc.to_tokens(tokens)
                }
                if !self.is_mixin {
                    doc_extend!(
                        tokens,
                        r##"<h3 id="wgall" class="method"><code><a href="#wgall" class="fnname">*</a> -> <span title="applied to self">self</span>.<span class='wgprop'>"##
                    );
                    //generate link to properties module (needs to be separate and in between \n)
                    doc_extend!(tokens, "\n[<span class='mod'>*</span>](zero_ui::properties)\n");
                    doc_extend!(
                        tokens,
                        r##"<ul style='display:none;'></ul></span></code></h3><div class="docblock">Widgets are open-ended, all properties are accepted.</div>"##
                    );
                }
                close_section(tokens);
            }
        }
    }

    /// Generator of documentation for a single property in a [`WidgetDocs`] section.
    pub struct PropertyDocs {
        pub docs: Vec<Attribute>,
        pub target_child: bool,
        pub ident: Ident,
        pub property_source: PropertySource,
        pub is_required_provided: bool,
    }
    impl ToTokens for PropertyDocs {
        /// We reuse the `<H3 class="method">` to create each property section.
        ///
        ///
        fn to_tokens(&self, tokens: &mut TokenStream) {
            doc_extend!(
                tokens,
                r##"<h3 id="wgproperty.{0}" class="method"><code id='{0}.v'><a href='#wgproperty.{0}' class='fnname'>{0}</a> -> <span title="applied to {1}">{1}</span>.<span class='wgprop'>"##,
                self.ident,
                if self.target_child { "child" } else { "self" },
            );

            let mut is_inherited = false;
            let mut source_widget = String::new();

            match &self.property_source {
                PropertySource::Property(p) => {
                    doc_extend!(tokens, "\n[<span class='mod'>{0}</span>]({0})\n", p);
                }
                PropertySource::Widget(w, p) => {
                    is_inherited = true;
                    source_widget = w.to_token_stream().to_string().replace(" :: ", "::");
                    doc_extend!(
                        tokens,
                        "\n[<span class='mod' data-inherited>{0}</span>](module@{1}#wgproperty.{0})\n",
                        p,
                        source_widget
                    );
                }
            }

            doc_extend!(tokens, "<ul style='display:none;'></ul></span></code></h3>");

            doc_extend!(tokens, "<div class='docblock'>\n\n");
            for doc in &self.docs {
                doc.to_tokens(tokens)
            }
            if self.docs.is_empty() {
                doc_extend!(
                    tokens,
                    "<span class='load-property-help' data-property='{}'>Loading content..</span>",
                    self.ident
                );
            }
            if is_inherited {
                let name_start = source_widget.rfind(':').map(|i| i + 1).unwrap_or_default();
                doc_extend!(
                    tokens,
                    "\n*Inherited from [`{}`](module@{}).*",
                    &source_widget[name_start..],
                    source_widget
                );
            }
            if self.is_required_provided {
                doc_extend!(tokens, "\n*This property is required, cannot be `unset!`.*")
            }
            doc_extend!(tokens, "\n</div>");
        }
    }

    pub enum PropertySource {
        Property(Ident),
        Widget(Path, Ident),
    }

    #[derive(Clone)]
    pub enum NewFn {
        None,
        Inherited(Path),
        New(WgtItemNew),
    }
    impl NewFn {
        fn new_tokens(&self, widget_name: &Ident) -> TokenStream {
            let r = match self {
                NewFn::None => {
                    let crate_ = crate_core();
                    let fn_doc = format!(
                        "Initializes a new [`{}`](self).\n\nThis calls the [`default_widget_new`]({}::core::default_widget_new) function.",
                        widget_name, crate_
                    );
                    let fn_docs = fn_doc.lines();
                    quote!(
                        #(#[doc=#fn_docs])*
                        #[inline]
                        pub fn new(child: impl #crate_::UiNode, id: impl properties::id::Args) -> impl #crate_::Widget {
                            #crate_::widget_base::default_widget_new(child, id)
                        }
                    )
                }
                NewFn::Inherited(super_widget) => {
                    quote! {
                        pub use #super_widget::new;
                    }
                }
                NewFn::New(new) => new.to_token_stream(),
            };

            #[cfg(debug_assertions)]
            {
                let mut r = r;
                let crate_ = crate_core();

                r.extend(quote! {
                    #[doc(hidden)]
                    #[cfg(debug_assertions)]
                    pub fn decl_location() -> #crate_::debug::SourceLocation {
                        #crate_::debug::source_location!()
                    }
                });

                r
            }
            #[cfg(not(debug_assertions))]
            r
        }

        fn new_child_tokens(&self, widget_name: &Ident) -> TokenStream {
            match self {
                NewFn::None => {
                    let crate_ = crate_core();
                    let fn_doc = format!(
                        "Initializes a new [`{}`](self) content.\n\n[`default_widget_new_child`]({}::core::default_widget_new_child) function.",
                        widget_name, crate_
                    );
                    let fn_docs = fn_doc.lines();
                    quote!(
                        #(#[doc=#fn_docs])*
                        #[inline]
                        pub fn new_child() -> impl #crate_::UiNode {
                            #crate_::widget_base::default_widget_new_child()
                        }
                    )
                }
                NewFn::Inherited(super_widget) => {
                    quote! {
                        pub use #super_widget::new_child;
                    }
                }
                NewFn::New(new_child) => new_child.to_token_stream(),
            }
        }
    }

    impl ToTokens for WgtItemNew {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            //#(#[..])*
            for attr in &self.attrs {
                attr.to_tokens(tokens)
            }

            // pub
            tokens.extend(quote_spanned!(self.fn_token.span()=> pub));

            // fn
            self.fn_token.to_tokens(tokens);

            // #fn_name
            self.target.to_tokens(tokens);

            // (#child, #(#args),*)
            // or
            // ( #(#args),*)
            match &self.target {
                NewTarget::New(_) => {
                    let child = self.inputs.first().unwrap();
                    let crate_ = crate_core();
                    let child = quote_spanned! {child.span()=> #child: impl #crate_::UiNode};
                    let args = self
                        .inputs
                        .iter()
                        .skip(1)
                        .map(|a| quote_spanned! {a.span()=> #a: impl properties::#a::Args});

                    tokens.extend(quote_spanned! {self.paren_token.span=> (#child, #(#args),*) });
                }
                NewTarget::NewChild(_) => {
                    let args = self
                        .inputs
                        .iter()
                        .map(|a| quote_spanned! {a.span()=> #a: impl properties::#a::Args });
                    tokens.extend(quote_spanned! {self.paren_token.span=> (#(#args),*) });
                }
            }

            // ->
            self.r_arrow_token.to_tokens(tokens);

            // #Output
            self.return_type.to_tokens(tokens);

            // {..}
            self.block.to_tokens(tokens);
        }
    }

    impl ToTokens for NewTarget {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                NewTarget::New(new) => new.to_tokens(tokens),
                NewTarget::NewChild(new_child) => new_child.to_tokens(tokens),
            }
        }
    }

    /// Properties used in a widget.
    #[derive(Default)]
    pub struct WidgetProperties {
        pub props: Vec<WidgetPropertyUse>,
    }

    impl ToTokens for WidgetProperties {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let props = &self.props;
            tokens.extend(quote! {
                #[doc(hidden)]
                pub mod properties {
                    use super::*;
                    #(#props)*
                }
            })
        }
    }

    pub enum WidgetPropertyUse {
        Mod(Ident),
        Alias { ident: Ident, original: Ident },
        Inherited { widget: Path, ident: Ident },
        AliasInherited { ident: Ident, widget: Path, original: Ident },
    }

    impl ToTokens for WidgetPropertyUse {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let tt = match self {
                WidgetPropertyUse::Mod(ident) => quote! {
                    #ident::if_export!(pub use #ident::export as #ident;);
                },
                WidgetPropertyUse::Alias { ident, original } => quote! {
                    #original::if_export!(pub use #original::export as #ident;);
                },
                WidgetPropertyUse::Inherited { widget, ident } => quote! {
                    #widget::properties::#ident::if_export!(pub use #widget::properties::#ident::export as #ident;);
                },
                WidgetPropertyUse::AliasInherited { ident, widget, original } => quote! {
                    #widget::properties::#original::if_export!(pub use #widget::properties::#original::export as #ident;);
                },
            };
            tokens.extend(tt);
        }
    }

    #[derive(Default)]
    pub struct WidgetDefaults {
        pub defaults: Vec<WidgetDefault>,
        pub when_defaults: Vec<WhenDefaults>,
    }
    impl ToTokens for WidgetDefaults {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let defaults = &self.defaults;
            let when_defaults = &self.when_defaults;
            if !defaults.is_empty() {
                tokens.extend(quote! {
                    #[doc(hidden)]
                    pub mod defaults {
                        use super::*;
                        #(#defaults)*
                    }
                });
            }
            if !when_defaults.is_empty() {
                tokens.extend(quote! {
                    #[doc(hidden)]
                    pub mod when_defaults {
                        use super::*;
                        #(#when_defaults)*
                    }
                });
            }
        }
    }

    pub struct WidgetDefault {
        pub property: Ident,
        pub default: FinalPropertyDefaultValue,
    }
    impl ToTokens for WidgetDefault {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let property = &self.property;
            let tt = match &self.default {
                FinalPropertyDefaultValue::Fields(f) => {
                    let fields = &f.fields;
                    quote! {
                        properties::#property::named_args! {
                            properties::#property: {
                                #fields
                            }
                        }
                    }
                }
                FinalPropertyDefaultValue::Args(a) => {
                    let args = &a.0;
                    quote!(properties::#property::args(#args))
                }
                FinalPropertyDefaultValue::Inherited(widget) => quote!(#widget::defaults::#property()),
                FinalPropertyDefaultValue::WhenInherited(widget, index) => {
                    let mod_name = ident!("w{}", index);
                    quote!(#widget::when_defaults::#mod_name::#property())
                }
            };
            tokens.extend(quote! {
                #[inline]
                pub fn #property() -> impl properties::#property::Args {
                    #tt
                }
            });
        }
    }

    pub struct WhenDefaults {
        pub index: usize,
        pub defaults: Vec<WidgetDefault>,
    }

    impl ToTokens for WhenDefaults {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let mod_name = ident!("w{}", self.index);
            let defaults = &self.defaults;
            tokens.extend(quote! { pub mod #mod_name {
                use super::*;
                #(#defaults)*
            }})
        }
    }

    pub enum FinalPropertyDefaultValue {
        Fields(PropertyFields),
        Args(PropertyArgs),
        Inherited(Path),
        WhenInherited(Path, u32),
    }

    pub struct WidgetWhens {
        pub conditions: Vec<WhenCondition>,
    }

    impl ToTokens for WidgetWhens {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let conditions = &self.conditions;
            if !conditions.is_empty() {
                let tt = quote! {
                    #[doc(hidden)]
                    pub mod whens {
                        use super::*;

                        #(#conditions)*
                    }
                };
                tokens.extend(tt)
            }
        }
    }

    pub struct WhenCondition {
        pub index: usize,
        pub properties: Vec<Ident>,
        pub expr: WhenConditionExpr,
        #[cfg(debug_assertions)]
        pub expr_str: Option<String>,
        #[cfg(debug_assertions)]
        pub property_sets: Vec<Ident>,
    }

    pub fn when_fn_name(index: usize) -> Ident {
        ident!("w{}", index)
    }

    impl ToTokens for WhenCondition {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let fn_ident = when_fn_name(self.index);
            let crate_ = crate_core();
            let p = &self.properties;
            let expr = &self.expr;
            let not_allowed_msg = p.iter().map(|p| format!("property `{}` is not allowed in when condition", p));

            tokens.extend(quote! {
                #(properties::#p::assert!(allowed_in_when, #not_allowed_msg);)*

                #[inline]
                pub fn #fn_ident(#(#p: &impl properties::#p::Args),*) -> impl #crate_::var::Var<bool> {
                    #expr
                }
            });

            #[cfg(debug_assertions)]
            {
                let fn_info_ident = ident!("{}_info", fn_ident);
                let info = if let Some(expr_str) = &self.expr_str {
                    let props_str = self.property_sets.iter().map(|p| p.to_string());
                    quote! {
                        #crate_::debug::WhenInfoV1 {
                            condition_expr: #expr_str,
                            condition_var: Some(condition_var),
                            properties:  vec![#(#props_str),*],
                            decl_location: #crate_::debug::source_location!(),
                            instance_location,
                            user_declared: false,
                        }
                    }
                } else {
                    self.expr.debug_info_tokens()
                };

                tokens.extend(quote_spanned! {fn_ident.span()=>
                    #[doc(hidden)]
                    #[cfg(debug_assertions)]
                    pub fn #fn_info_ident(
                        condition_var: #crate_::var::BoxedVar<bool>,
                        instance_location: #crate_::debug::SourceLocation)
                    -> #crate_::debug::WhenInfoV1 {
                        #info
                    }
                });
            }
        }
    }

    pub enum WhenConditionExpr {
        Ref(WhenPropertyRef),
        Map(WhenPropertyRef, Box<Expr>),
        Merge(HashSet<WhenPropertyRef>, Box<Expr>),
        Inherited(InheritedWhen),
    }
    impl WhenConditionExpr {
        #[cfg(debug_assertions)]
        pub fn debug_info_tokens(&self) -> TokenStream {
            if let WhenConditionExpr::Inherited(iw) = self {
                iw.debug_info_tokens()
            } else {
                panic!("expected WhenConditionExpr::Inherited")
            }
        }
    }
    impl ToTokens for WhenConditionExpr {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                WhenConditionExpr::Ref(let_name) => {
                    let name = let_name.name();
                    tokens.extend(quote! {
                        #[allow(clippy::let_and_return)]
                        #let_name
                        #name
                    })
                }
                WhenConditionExpr::Map(let_name, expr) => {
                    let name = let_name.name();
                    let crate_ = crate_core();
                    tokens.extend(quote! {
                        #let_name
                        #crate_::var::Var::into_map(#name, |#name|{#expr})
                    })
                }
                WhenConditionExpr::Merge(let_names, expr) => {
                    let names: Vec<_> = let_names.iter().map(|n| n.name()).collect();
                    let crate_ = crate_core();
                    let let_names = let_names.iter();
                    tokens.extend(quote! {
                        #(#let_names)*
                        #crate_::var::merge_var!(#(#names, )* |#(#names),*|{
                            #expr
                        })
                    })
                }
                WhenConditionExpr::Inherited(inh) => inh.to_tokens(tokens),
            }
        }
    }

    pub struct InheritedWhen {
        pub widget: Path,
        pub when_name: Ident,
        pub properties: Vec<Ident>,
    }
    impl InheritedWhen {
        #[cfg(debug_assertions)]
        pub fn debug_info_tokens(&self) -> TokenStream {
            let widget = &self.widget;
            let fn_ = ident!("{}_info", self.when_name);
            quote! {
                #widget::whens::#fn_(condition_var, instance_location)
            }
        }
    }
    impl ToTokens for InheritedWhen {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let widget = &self.widget;
            let properties = &self.properties;
            let fn_ = &self.when_name;
            tokens.extend(quote! { #widget::whens::#fn_(#(#properties),*) });
        }
    }

    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct WhenPropertyRef {
        pub property: Ident,
        pub arg: WhenPropertyRefArg,
    }

    impl WhenPropertyRef {
        pub fn name(&self) -> Ident {
            // property_0
            // property_named
            ident!("{}_{}", self.property, self.arg)
        }
    }

    impl ToTokens for WhenPropertyRef {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let crate_ = crate_core();
            let property = &self.property;
            let arg = &self.arg;
            let name = self.name();
            tokens.extend(quote! {
                let #name = #crate_::var::IntoVar::into_var(std::clone::Clone::clone(properties::#property::#arg(#property)));
            });
        }
    }

    #[derive(Clone, PartialEq, Eq, Hash)]
    pub enum WhenPropertyRefArg {
        Index(u32),
        Named(Ident),
    }

    impl fmt::Display for WhenPropertyRefArg {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                WhenPropertyRefArg::Index(idx) => idx.fmt(f),
                WhenPropertyRefArg::Named(ident) => ident.fmt(f),
            }
        }
    }

    impl ToTokens for WhenPropertyRefArg {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                WhenPropertyRefArg::Index(idx) => {
                    let ident = ident!("arg{}", idx);
                    tokens.extend(quote! {ArgsNumbered::#ident})
                }
                WhenPropertyRefArg::Named(ident) => tokens.extend(quote! {ArgsNamed::#ident}),
            }
        }
    }
}
