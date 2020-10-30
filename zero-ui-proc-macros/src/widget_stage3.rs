use quote::ToTokens;
use syn::parse_macro_input;

/// `widget!` actual expansion, in stage3 we have all the inherited tokens to work with.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as input::WidgetDeclaration);
    let output = analysis::generate(input);
    let output_stream = output.to_token_stream();
    output_stream.into()
}

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
        pub brace_token: token::Brace,
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

    pub struct WidgetDeclaration {
        pub inherits: Vec<InheritItem>,
        pub mixin_signal: MixinSignal,
        pub header: WidgetHeader,
        pub items: Vec<WgtItem>,
    }

    impl Parse for WidgetDeclaration {
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

    pub struct InheritItem {
        pub ident: Ident,
        pub inherit_path: Path,
        pub mixin_signal: MixinSignal,
        pub default: Punctuated<InheritedProperty, Token![,]>,
        pub default_child: Punctuated<InheritedProperty, Token![,]>,
        pub whens: Punctuated<InheritedWhen, Token![,]>,
        pub new: Punctuated<Ident, Token![,]>,
        pub new_child: Punctuated<Ident, Token![,]>,
    }

    impl Parse for InheritItem {
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

    pub struct InheritedProperty {
        pub docs: Vec<Attribute>,
        pub kind: BuiltPropertyKind,
        pub ident: Ident,
    }
    impl Parse for InheritedProperty {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(InheritedProperty {
                docs: Attribute::parse_outer(input).unwrap_or_else(|e| non_user_error!(e)),
                kind: input.parse().unwrap_or_else(|e| non_user_error!(e)),
                ident: input.parse().unwrap_or_else(|e| non_user_error!(e)),
            })
        }
    }

    #[derive(PartialEq, Eq)]
    pub enum BuiltPropertyKind {
        /// Required property.
        Required,
        /// Property is provided by the widget.
        Local,
        /// Property and default is provided by the widget.
        Default,
    }

    impl Parse for BuiltPropertyKind {
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

    pub struct InheritedWhen {
        pub docs: Vec<Attribute>,
        pub args: Punctuated<Ident, Token![,]>,
        pub sets: Punctuated<Ident, Token![,]>,
    }
    impl Parse for InheritedWhen {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(InheritedWhen {
                docs: Attribute::parse_outer(input).unwrap_or_else(|e| non_user_error!(e)),
                args: Punctuated::parse_terminated(&non_user_parenthesized(input)).unwrap_or_else(|e| non_user_error!(e)),
                sets: Punctuated::parse_terminated(&non_user_braced(input)).unwrap_or_else(|e| non_user_error!(e)),
            })
        }
    }

    pub struct MixinSignal {
        pub mixin_token: keyword::mixin,
        pub colon: Token![:],
        pub value: LitBool,
    }
    impl Parse for MixinSignal {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(MixinSignal {
                mixin_token: input.parse()?,
                colon: input.parse()?,
                value: input.parse()?,
            })
        }
    }

    pub enum WgtItem {
        Default(WgtItemDefault),
        New(WgtItemNew),
        When(WgtItemWhen),
    }
    impl Parse for WgtItem {
        fn parse(input: ParseStream) -> Result<Self> {
            if input.peek(Token![default]) || input.peek(keyword::default_child) {
                input.parse().map(WgtItem::Default)
            } else {
                let attrs = Attribute::parse_outer(input)?;

                let lookahead = input.lookahead1();
                if attrs.is_empty() {
                    // add to error message.
                    lookahead.peek(Token![default]);
                    lookahead.peek(keyword::default_child);
                }

                if lookahead.peek(keyword::when) {
                    let mut when: WgtItemWhen = input.parse()?;
                    when.attrs = attrs;
                    Ok(WgtItem::When(when))
                } else if lookahead.peek(Token![fn]) {
                    let mut new: WgtItemNew = input.parse()?;
                    new.attrs = attrs;
                    Ok(WgtItem::New(new))
                } else {
                    Err(lookahead.error())
                }
            }
        }
    }

    pub struct WgtItemDefault {
        pub target: DefaultTarget,
        pub block: DefaultBlock,
    }

    impl Parse for WgtItemDefault {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(WgtItemDefault {
                target: input.parse()?,
                block: input.parse()?,
            })
        }
    }

    pub enum DefaultTarget {
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

    pub type DefaultBlock = PropertyBlock<PropertyDeclaration>;

    pub struct PropertyDeclaration {
        pub attrs: Vec<Attribute>,
        pub ident: Ident,
        pub maps_to: Option<MappedProperty>,
        pub default_value: Option<(Token![:], PropertyDefaultValue)>,
        pub semi_token: Token![;],
    }
    impl Parse for PropertyDeclaration {
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

    pub struct MappedProperty {
        pub r_arrow_token: Token![->],
        pub ident: Ident,
    }
    impl Parse for MappedProperty {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(MappedProperty {
                r_arrow_token: input.parse()?,
                ident: input.parse()?,
            })
        }
    }

    pub enum PropertyDefaultValue {
        /// Named arguments.
        Fields(PropertyFields),
        /// Unnamed arguments.
        Args(PropertyArgs),
        /// unset.
        Unset(PropertyUnset),
        /// required.
        Required(PropertyRequired),
    }

    impl Parse for PropertyDefaultValue {
        fn parse(input: ParseStream) -> Result<Self> {
            parse_property_value(input, true)
        }
    }

    fn parse_property_value(input: ParseStream, allow_required: bool) -> Result<PropertyDefaultValue> {
        let ahead = input.fork();
        let mut buffer = TokenStream::new();
        while !ahead.is_empty() && !ahead.peek(Token![;]) {
            let tt: TokenTree = ahead.parse().unwrap();
            tt.to_tokens(&mut buffer);
        }
        input.advance_to(&ahead);

        if let Ok(fields) = syn::parse2(buffer.clone()) {
            Ok(PropertyDefaultValue::Fields(fields))
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

    #[derive(Debug)]
    pub struct PropertyFields {
        pub brace_token: token::Brace,
        pub fields: Punctuated<FieldValue, Token![,]>,
    }

    impl Parse for PropertyFields {
        fn parse(input: ParseStream) -> Result<Self> {
            let fields;
            Ok(PropertyFields {
                brace_token: braced!(fields in input),
                fields: Punctuated::parse_terminated(&fields)?,
            })
        }
    }

    #[derive(Debug)]
    pub struct PropertyArgs(pub Punctuated<Expr, Token![,]>);

    impl Parse for PropertyArgs {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyArgs(Punctuated::parse_terminated(input)?))
        }
    }

    pub struct PropertyUnset {
        pub unset_token: keyword::unset,
        pub bang_token: Token![!],
    }
    impl Parse for PropertyUnset {
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

    pub struct PropertyRequired {
        pub required_token: keyword::required,
        pub bang_token: Token![!],
    }

    impl Parse for PropertyRequired {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyRequired {
                required_token: input.parse()?,
                bang_token: input.parse()?,
            })
        }
    }

    #[derive(Clone)]
    pub struct WgtItemNew {
        pub attrs: Vec<Attribute>,
        pub fn_token: Token![fn],
        pub target: NewTarget,
        pub paren_token: token::Paren,
        pub inputs: Punctuated<Ident, Token![,]>,
        pub r_arrow_token: Token![->],
        pub return_type: Box<Type>,
        pub block: Block,
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

    #[derive(Clone)]
    pub enum NewTarget {
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

    pub struct WgtItemWhen {
        pub attrs: Vec<Attribute>,
        pub when_token: keyword::when,
        pub condition: Box<Expr>,
        pub block: WhenBlock,
    }

    impl Parse for WgtItemWhen {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(WgtItemWhen {
                attrs: Attribute::parse_outer(input)?,
                when_token: input.parse()?,
                condition: Box::new(Expr::parse_without_eager_brace(input)?),
                block: input.parse()?,
            })
        }
    }

    pub type WhenBlock = PropertyBlock<PropertyAssign>;

    pub struct PropertyAssign {
        pub ident: Ident,
        pub colon_token: Token![:],
        pub value: PropertyValue,
        pub semi_token: Token![;],
    }

    impl Parse for PropertyAssign {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyAssign {
                ident: input.parse()?,
                colon_token: input.parse()?,
                value: input.parse()?,
                semi_token: input.parse()?,
            })
        }
    }

    pub enum PropertyValue {
        /// Named arguments. prop1: { arg0: "value", arg1: "other value" };
        Fields(PropertyFields),
        /// Unnamed arguments. prop1: {"value"}, "other value";
        Args(PropertyArgs),
        /// unset. prop1: unset!;
        Unset(PropertyUnset),
    }

    impl Parse for PropertyValue {
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

pub mod analysis {
    use super::input::{self, BuiltPropertyKind, DefaultTarget, NewTarget, PropertyDefaultValue, WgtItem, WidgetDeclaration};
    use super::output::*;
    use crate::{
        property::input::Prefix as PropertyPrefix,
        util::{Attributes, Errors},
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
        // check if included all inherits in the recursive call.
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
        // so that the left most inherit is the last item, that caused it to
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
                WgtItem::Default(d) => match d.target {
                    DefaultTarget::Default(_) => properties.extend(d.block.properties.into_iter().map(|p| (PropertyTarget::Default, p))),
                    DefaultTarget::DefaultChild(_) => {
                        properties.extend(d.block.properties.into_iter().map(|p| (PropertyTarget::DefaultChild, p)))
                    }
                },
                WgtItem::New(n) => match &n.target {
                    NewTarget::New(_) => new_fns.push(n),
                    NewTarget::NewChild(_) => new_child_fns.push(n),
                },
                WgtItem::When(w) => whens.push(w),
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
                            property_source: PropertySource::Widget(inherit_path.clone()),
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
        for (target, property) in properties {
            let mut has_value = true;
            let mut is_required = false;
            let mut default_value = None;

            match property.default_value {
                Some((_, value)) => match value {
                    PropertyDefaultValue::Fields(fields) => default_value = Some(FinalPropertyDefaultValue::Fields(fields)),
                    PropertyDefaultValue::Args(args) => default_value = Some(FinalPropertyDefaultValue::Args(args)),

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
            docs.push(PropertyDocs {
                docs: attrs.docs.clone(),
                target_child: target == PropertyTarget::DefaultChild,
                ident: property.ident.clone(),
                property_source: PropertySource::Property(property.maps_to.as_ref().map(|m| &m.ident).unwrap_or(&property.ident).clone()),
                is_required_provided: false,
            });

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
                docs: attrs.docs,
                kind: if is_required {
                    BuiltPropertyKind::Required
                } else if has_value {
                    BuiltPropertyKind::Default
                } else {
                    BuiltPropertyKind::Local
                },
                ident: property.ident.clone(),
            });

            mod_properties.props.push(if let Some(maps_to) = property.maps_to {
                // property maps to another, re-export with new property name.
                if let Some(widget) = inheritance_map.get(&maps_to.ident).and_then(|o| o.setted_path()) {
                    WidgetPropertyUse::AliasInherited {
                        ident: property.ident,
                        widget: widget.clone(),
                        original: maps_to.ident,
                    }
                } else {
                    WidgetPropertyUse::Alias {
                        ident: property.ident,
                        original: maps_to.ident,
                    }
                }
            } else {
                // property does not map to another, re-export the property mod.
                if let Some(widget) = inheritance_map[&property.ident].setted_path() {
                    WidgetPropertyUse::Inherited {
                        widget: widget.clone(),
                        ident: property.ident,
                    }
                } else {
                    WidgetPropertyUse::Mod(property.ident)
                }
            });
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

            mod_whens.push(WhenCondition {
                index: when_index,
                properties: when_analysis.properties.iter().map(|p| &p.property).cloned().collect(),
                expr: when_analysis.expr,
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

            mod_defaults.when_defaults.push(WhenDefaults {
                index: when_index,
                defaults: when
                    .block
                    .properties
                    .into_iter()
                    .map(|p| WidgetDefault {
                        property: p.ident,
                        default: match p.value {
                            PropertyValue::Fields(fields) => FinalPropertyDefaultValue::Fields(fields),
                            PropertyValue::Args(args) => FinalPropertyDefaultValue::Args(args),
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
        if let Some(fn_) = new_fns.drain(..).next() {
            macro_new = BuiltNew {
                properties: fn_.inputs.iter().skip(1).cloned().collect(),
            };
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
        if let Some(fn_) = new_child_fns.drain(..).next() {
            macro_new_child = BuiltNew {
                properties: fn_.inputs.iter().cloned().collect(),
            };
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

pub mod output {
    use super::input::{keyword, BuiltPropertyKind, NewTarget, PropertyArgs, PropertyFields, WgtItemNew};
    use crate::util::{docs_with_first_line_js, uuid, zero_ui_crate_ident, Errors};
    use proc_macro2::{Ident, TokenStream};
    use quote::ToTokens;
    use std::{collections::HashSet, fmt};
    use syn::spanned::Spanned;
    use syn::{Attribute, Expr, Path, Token, Visibility};

    pub use super::input::{InheritedProperty as BuiltProperty, InheritedWhen as BuiltWhen};

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
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let crate_ = zero_ui_crate_ident();
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
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                BuiltPropertyKind::Required => keyword::required::default().to_tokens(tokens),
                BuiltPropertyKind::Local => keyword::local::default().to_tokens(tokens),
                BuiltPropertyKind::Default => <Token![default]>::default().to_tokens(tokens),
            }
        }
    }
    impl BuiltWhen {
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
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let props = &self.properties;
            tokens.extend(quote!( #(#props),* ))
        }
    }

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
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let attrs = self.attrs.iter();
            let docs = &self.docs;
            let vis = &self.vis;
            let widget_name = &self.widget_name;
            let crate_ = zero_ui_crate_ident();

            let some_mixin = if self.is_mixin { None } else { Some(()) };
            let use_implicit_mixin = some_mixin.map(|_| quote!( use #crate_::widgets::mixins::implicit_mixin; ));
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
                        #(pub use super::properties::#properties;)*
                    }
                })
            }
        }
    }
    impl ToTokens for WidgetDocs {
        // TODO generate when documentation.
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
                    r##"<h2 id="{0}" class="small-section-header">{1}<a href="#{0}" class="anchor"></a></h2>
                    <div class="methods" style="display: block;">"##,
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
                        r##"<h3 id="wgall" class="method"><code><a href="#wgall" class="fnname">*</a> -> 
                        <span title="applied to self">self</span>.<span class='wgprop'>"##
                    );
                    //generate link to properties module (needs to be separate and in between \n)
                    doc_extend!(tokens, "\n[<span class='mod'>*</span>](zero_ui::properties)\n");
                    doc_extend!(
                        tokens,
                        r##"<ul style='display:none;'></ul></span></code></h3>
                        <div class="docblock">Widgets are open-ended, all properties are accepted.</div>"##
                    );
                }
                close_section(tokens);
            }
        }
    }

    pub struct PropertyDocs {
        pub docs: Vec<Attribute>,
        pub target_child: bool,
        pub ident: Ident,
        pub property_source: PropertySource,
        pub is_required_provided: bool,
    }

    impl ToTokens for PropertyDocs {
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
                PropertySource::Widget(p) => {
                    is_inherited = true;
                    source_widget = p.to_token_stream().to_string().replace(" :: ", "::");
                    doc_extend!(
                        tokens,
                        "\n[<span class='mod' data-inherited>{0}</span>](module@{1}#wgproperty.{0})\n",
                        self.ident,
                        source_widget
                    );
                }
            }

            doc_extend!(tokens, "<ul style='display:none;'></ul></span></code></h3>");

            doc_extend!(tokens, "<div class='docblock'>\n");
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
        Widget(Path),
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
                    let crate_ = zero_ui_crate_ident();
                    let fn_doc = format!(
                        "Initializes a new [`{}`](self).\n\nThis calls the [`default_widget_new`]({}::core::default_widget_new) function.",
                        widget_name, crate_
                    );
                    quote!(
                        #[doc=#fn_doc]
                        #[inline]
                        pub fn new(child: impl #crate_::core::UiNode, id: impl properties::id::Args) -> impl #crate_::core::Widget {
                            #crate_::core::default_widget_new(child, id)
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
                let crate_ = zero_ui_crate_ident();

                r.extend(quote! {
                    #[doc(hidden)]
                    #[cfg(debug_assertions)]
                    pub fn decl_location() -> #crate_::core::debug::SourceLocation {
                        #crate_::core::debug::source_location!()
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
                    let crate_ = zero_ui_crate_ident();
                    let fn_doc = format!(
                        "Initializes a new [`{}`](self) content.\n\n[`default_widget_new_child`]({}::core::default_widget_new_child) function.",
                        widget_name, crate_
                    );
                    quote!(
                        #[doc=#fn_doc]
                        #[inline]
                        pub fn new_child() -> impl #crate_::core::UiNode {
                            #crate_::core::default_widget_new_child()
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
                    let mut crate_ = zero_ui_crate_ident();
                    crate_.set_span(child.span());
                    let child = quote_spanned! {child.span()=> #child: impl #crate_::core::UiNode};
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
            let crate_ = zero_ui_crate_ident();
            let p = &self.properties;
            let expr = &self.expr;
            let not_allowed_msg = p.iter().map(|p| format!("property `{}` is not allowed in when condition", p));

            tokens.extend(quote! {
                #(properties::#p::assert!(allowed_in_when, #not_allowed_msg);)*

                #[inline]
                pub fn #fn_ident(#(#p: &impl properties::#p::Args),*) -> impl #crate_::core::var::Var<bool> {
                    #expr
                }
            });

            #[cfg(debug_assertions)]
            {
                let fn_info_ident = ident!("{}_info", fn_ident);
                let info = if let Some(expr_str) = &self.expr_str {
                    let props_str = self.property_sets.iter().map(|p| p.to_string());
                    quote! {
                        #crate_::core::debug::WhenInfoV1 {
                            condition_expr: #expr_str,
                            condition_var: Some(condition_var),
                            properties:  vec![#(#props_str),*],
                            decl_location: #crate_::core::debug::source_location!(),
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
                        condition_var: #crate_::core::var::BoxedVar<bool>,
                        instance_location: #crate_::core::debug::SourceLocation)
                    -> #crate_::core::debug::WhenInfoV1 {
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
                    let crate_ = zero_ui_crate_ident();
                    tokens.extend(quote! {
                        #let_name
                        #crate_::core::var::Var::into_map(#name, |#name|{#expr})
                    })
                }
                WhenConditionExpr::Merge(let_names, expr) => {
                    let names: Vec<_> = let_names.iter().map(|n| n.name()).collect();
                    let crate_ = zero_ui_crate_ident();
                    let let_names = let_names.iter();
                    tokens.extend(quote! {
                        #(#let_names)*
                        #crate_::core::var::merge_var!(#(#names, )* |#(#names),*|{
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
            let crate_ = zero_ui_crate_ident();
            let property = &self.property;
            let arg = &self.arg;
            let name = self.name();
            tokens.extend(quote! {
                let #name = #crate_::core::var::IntoVar::into_var(std::clone::Clone::clone(properties::#property::#arg(#property)));
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
