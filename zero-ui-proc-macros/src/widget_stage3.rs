use quote::ToTokens;
use syn::parse_macro_input;

/// `widget!` actual expansion, in stage3 we have all the inherited tokens to work with.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as input::WidgetDeclaration);
    let output = analysis::generate(input);
    let output_stream = output.to_token_stream();
    output_stream.into()
}

mod input {
    #![allow(unused)]

    use crate::util::{non_user_braced, non_user_parenthesized, NON_USER_ERROR};
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
        syn::custom_keyword!(new);
        syn::custom_keyword!(new_child);
        syn::custom_keyword!(inherit);
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
        pub header: WidgetHeader,
        pub items: Vec<WgtItem>,
    }

    impl Parse for WidgetDeclaration {
        fn parse(input: ParseStream) -> Result<Self> {
            let mut inherits = Vec::new();
            while input.peek(Token![=>]) && input.peek2(keyword::inherit) {
                inherits.push(input.parse().expect(NON_USER_ERROR));
            }

            let header = input.parse().expect(NON_USER_ERROR);
            let mut items = Vec::new();
            while !input.is_empty() {
                items.push(input.parse()?);
            }
            Ok(WidgetDeclaration { inherits, header, items })
        }
    }

    pub struct InheritItem {
        pub ident: Ident,
        pub inherit_path: Path,
        pub default: Punctuated<InheritedProperty, Token![,]>,
        pub default_child: Punctuated<InheritedProperty, Token![,]>,
        pub whens: Punctuated<InheritedWhen, Token![,]>,
    }

    impl Parse for InheritItem {
        fn parse(input: ParseStream) -> Result<Self> {
            fn parse_block<T: Parse, R: Parse>(input: ParseStream) -> Punctuated<R, Token![,]> {
                input.parse::<T>().expect(NON_USER_ERROR);
                let inner = non_user_braced(input);
                Punctuated::parse_terminated(&inner).expect(NON_USER_ERROR)
            }

            input.parse::<Token![=>]>().expect(NON_USER_ERROR);
            input.parse::<keyword::inherit>().expect(NON_USER_ERROR);
            Ok(InheritItem {
                ident: input.parse().expect(NON_USER_ERROR),
                inherit_path: input.parse().expect(NON_USER_ERROR),
                default: parse_block::<Token![default], InheritedProperty>(input),
                default_child: parse_block::<keyword::default_child, InheritedProperty>(input),
                whens: parse_block::<keyword::whens, InheritedWhen>(input),
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
                docs: Attribute::parse_outer(input).expect(NON_USER_ERROR),
                kind: input.parse().expect(NON_USER_ERROR),
                ident: input.parse().expect(NON_USER_ERROR),
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
                input.parse::<Token![default]>().expect(NON_USER_ERROR);
                Ok(BuiltPropertyKind::Default)
            } else if input.peek(keyword::local) {
                input.parse::<keyword::local>().expect(NON_USER_ERROR);
                Ok(BuiltPropertyKind::Local)
            } else if input.peek(keyword::required) {
                input.parse::<keyword::required>().expect(NON_USER_ERROR);
                Ok(BuiltPropertyKind::Required)
            } else {
                panic!("{} {}", NON_USER_ERROR, "expected one of: required, default, local")
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
                docs: Attribute::parse_outer(input).expect(NON_USER_ERROR),
                args: Punctuated::parse_terminated(&non_user_parenthesized(input)).expect(NON_USER_ERROR),
                sets: Punctuated::parse_terminated(&non_user_braced(input)).expect(NON_USER_ERROR),
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
            let lookahead = input.lookahead1();

            if lookahead.peek(Token![default]) || lookahead.peek(keyword::default_child) {
                input.parse().map(WgtItem::Default)
            } else {
                let attrs = Attribute::parse_outer(input)?;
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
        while !ahead.is_empty() {
            let tt: TokenTree = ahead.parse().unwrap();
            if let TokenTree::Punct(p) = tt {
                if p.as_char() == ';' {
                    break;
                } else {
                    TokenTree::Punct(p).to_tokens(&mut buffer);
                }
            } else {
                tt.to_tokens(&mut buffer);
            }
        }

        if let Ok(args) = syn::parse2(buffer.clone()) {
            Ok(PropertyDefaultValue::Args(args))
        } else if let Ok(fields) = syn::parse2(buffer.clone()) {
            Ok(PropertyDefaultValue::Fields(fields))
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
            let attrs = Attribute::parse_outer(input)?;
            let when_token: keyword::when = input.parse()?;

            let ahead = input.fork();

            let mut cond_buffer = TokenStream::new();

            while !ahead.is_empty() {
                let ttree_ahead = ahead.fork();
                let next: TokenTree = ttree_ahead.parse().unwrap();
                if let g @ TokenTree::Group { .. } = next {
                    // if found group in the root stream it can be a WhenBlock,
                    // lookahead for WhenBlock.
                    let block_ahead = ahead.fork();
                    if let Ok(block) = block_ahead.parse::<WhenBlock>() {
                        // it was WhenBlock, validate missing condition expression.
                        if cond_buffer.is_empty() {
                            return Err(Error::new(when_token.span(), "missing condition for `when` item"));
                        }

                        ahead.advance_to(&block_ahead);
                        input.advance_to(&ahead);

                        // it was WhenBlock and we have the condition expression, success!
                        return Ok(WgtItemWhen {
                            attrs,
                            when_token,
                            condition: syn::parse2(cond_buffer)?,
                            block,
                        });
                    } else {
                        // found group, but was not WhenBlock, buffer condition expression.
                        g.to_tokens(&mut cond_buffer);
                        ahead.advance_to(&ttree_ahead);
                    }
                } else {
                    // did not find group, buffer condition expression.
                    next.to_tokens(&mut cond_buffer);
                    ahead.advance_to(&ttree_ahead);
                }
            }

            Err(Error::new(when_token.span(), "expected property assign block"))
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

mod analysis {
    use super::input::{BuiltPropertyKind, DefaultTarget, NewTarget, PropertyDefaultValue, WgtItem, WidgetDeclaration};
    use super::output::{when_fn_name, InheritedWhen, WhenCondition, WhenConditionExpr, WidgetOutput};
    use crate::util::Errors;
    use proc_macro2::{Span, TokenStream};
    use std::collections::{HashMap, HashSet};
    use std::fmt;
    use syn::{spanned::Spanned, Visibility};

    pub fn generate(input: WidgetDeclaration) -> WidgetOutput {
        // check if included all inherits in the recursive call.
        debug_assert!(input
            .header
            .inherits
            .iter()
            .rev()
            .zip(input.inherits.iter().map(|i| &i.inherit_path))
            .all(|(header, included)| header == included));

        // #[macro_export] if `pub` or `pub(crate)`
        let macro_export = match &input.header.vis {
            Visibility::Public(_) => true,
            Visibility::Restricted(r) => r.path.get_ident().map(|i| i == &ident!("crate")).unwrap_or_default(),
            Visibility::Crate(_) | Visibility::Inherited => false,
        };

        // unwrap items
        enum PropertyTarget {
            Default,
            DefaultChild,
        }
        let mut properties = vec![];
        let mut new = vec![];
        let mut new_child = vec![];
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
                    NewTarget::New(_) => new.push(n),
                    NewTarget::NewChild(_) => new_child.push(n),
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
        for extra_new in new.iter().skip(1).chain(new_child.iter().skip(1)) {
            errors.push(format!("function `{}` already declared", extra_new.target), extra_new.target.span())
        }
        for when in &mut whens {
            let mut properties = HashSet::new();
            when.block.properties.retain(|property| {
                let inserted = properties.insert(property.ident.clone());
                if !inserted {
                    errors.push(format!("property `{}` already set", property.ident), property.ident.span());
                }
                inserted
            })
        }

        // map that defines each property origin.
        // widgets override properties with the same name when inheriting,
        // the map is (property: Ident, widget: Path), the widget is `Self` for
        // properties declared locally.
        let mut inheritance_map = HashMap::new();
        for inherit in &input.inherits {
            for property in inherit.default_child.iter().chain(inherit.default.iter()) {
                inheritance_map.insert(property.ident.clone(), Some(inherit.inherit_path.clone()));
            }
        }
        for property in properties.iter() {
            inheritance_map.insert(property.1.ident.clone(), None);
        }

        // all `when` for the macro
        let mut macro_whens = vec![];
        // all inherited `when` for the mod.
        let mut mod_whens = vec![];
        // next available index for when function names.
        let mut when_index = 0;
        //all properties that have a initial value
        let mut inited_properties = HashSet::new();

        for inherit in input.inherits {
            for child_property in inherit.default_child {
                if inheritance_map[&child_property.ident].as_ref() == Some(&inherit.inherit_path) {
                    if child_property.kind == BuiltPropertyKind::Local {
                        assert!(inited_properties.insert(child_property.ident));
                    }
                    todo!()
                }
            }

            for property in inherit.default {
                if inheritance_map[&property.ident].as_ref() == Some(&inherit.inherit_path) {
                    if property.kind == BuiltPropertyKind::Local {
                        assert!(inited_properties.insert(property.ident));
                    }
                    todo!()
                }
            }

            for when in inherit.whens {
                mod_whens.push(WhenCondition {
                    index: when_index,
                    properties: when.args.iter().cloned().collect(),
                    expr: WhenConditionExpr::Inherited(InheritedWhen {
                        widget: inherit.inherit_path.clone(),
                        when_name: when_fn_name(when_index),
                        properties: when.args.iter().cloned().collect(),
                    }),
                });

                macro_whens.push(when);

                when_index += 1;
            }
        }
        debug_assert_eq!(when_index, macro_whens.len());
        debug_assert_eq!(when_index, mod_whens.len());

        for (target, property) in properties {
            let mut has_value = true;
            match property.default_value {
                Some((_, value)) => match value {
                    PropertyDefaultValue::Fields(fields) => todo!(),

                    PropertyDefaultValue::Args(args) => todo!(),

                    PropertyDefaultValue::Unset(_) => has_value = false,

                    PropertyDefaultValue::Required(_) => {}
                },
                None => has_value = false,
            }
            if has_value {
                assert!(inited_properties.insert(property.ident));
            }
        }

        //validation after widget properties found
        for when in &mut whens {
            when.block.properties.retain(|property| {
                let used = inited_properties.contains(&property.ident);
                if !used {
                    errors.push(
                        format!("property `{}` is not used in this widget", property.ident),
                        property.ident.span(),
                    );
                }
                used
            })
        }
        todo!()
    }

    impl fmt::Display for NewTarget {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", quote!(#self))
        }
    }
}

mod output {
    use super::input::{keyword, BuiltPropertyKind, NewTarget, PropertyArgs, PropertyFields, WgtItemNew};
    use crate::util::{uuid, zero_ui_crate_ident};
    use proc_macro2::{Ident, TokenStream};
    use quote::ToTokens;
    use std::fmt;
    use syn::spanned::Spanned;
    use syn::{Attribute, Expr, Path, Token, Visibility};

    pub use super::input::{InheritedProperty as BuiltProperty, InheritedWhen as BuiltWhen};

    pub struct WidgetOutput {
        macro_: WidgetMacro,
        mod_: WidgetMod,
    }
    impl ToTokens for WidgetOutput {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.macro_.to_tokens(tokens);
            self.mod_.to_tokens(tokens);
        }
    }

    struct WidgetMacro {
        widget_name: Ident,
        vis: Visibility,
        export: bool,
        is_mixin: bool,
        default_child: Vec<BuiltProperty>,
        default: Vec<BuiltProperty>,
        whens: Vec<BuiltWhen>,
        new: BuiltNew,
        new_child: BuiltNew,
    }
    impl ToTokens for WidgetMacro {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.extend(quote!( #[doc(hidden)] ));
            if self.export {
                tokens.extend(quote!( #[macro_export] ));
            }

            let crate_ = zero_ui_crate_ident();
            let name = &self.widget_name;

            let default = &self.default;
            let default_child = &self.default_child;
            let whens = &self.whens;
            let inherit_info = quote! {
                mod #name
                default { #(#default),* }
                default_child { #(#default_child),* }
                whens { #(#whens),* }
            };
            let inherit_args = quote! {
                $($inherit_next)*

                inherit {
                    $named_as;
                    #inherit_info
                }

                $($rest)*
            };
            let inherit_arm = quote! {
                (-> inherit { $named_as:path; $($inherit_next:tt)* } $($rest:tt)*) => {
                    #crate_::widget_inherit! {
                        #inherit_args
                    }
                };
            };
            let inherit_mixin_arm = quote! {
                (-> inherit_mixin { $named_as:path; $($inherit_next:tt)* } $($rest:tt)*) => {
                    #crate_::widget_mixin_inherit! {
                        #inherit_args
                    }
                };
            };

            let new_arm = if self.is_mixin {
                None
            } else {
                let default = self.default.iter().map(|p| p.tokens(false));
                let default_child = self.default_child.iter().map(|p| p.tokens(false));
                let whens = self.whens.iter().map(|p| p.tokens(false));
                let new = &self.new;
                let new_child = &self.new_child;
                let widget_new_info = quote! {
                    mod #name
                    default { #(#default)* }
                    default_child { #(#default_child)* }
                    whens { #(#whens)* }
                    new(#new)
                    new_child(#new_child)
                };
                Some(quote! {
                    ($($input:tt)*) => {
                        #crate_::widget_new! {
                            #widget_new_info
                            user_input { $($input)* }
                        }
                    };
                })
            };

            let unique_name = ident!("{}_{}", self.widget_name, uuid());
            let vis = &self.vis;

            tokens.extend(quote! {
                macro_rules! #unique_name {
                    #inherit_arm
                    #inherit_mixin_arm
                    #new_arm
                }

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

    struct BuiltNew {
        properties: Vec<Ident>,
    }

    impl ToTokens for BuiltNew {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let props = &self.properties;
            tokens.extend(quote!( #(#props),* ))
        }
    }

    struct WidgetMod {
        docs: WidgetDocs,
        attrs: Vec<Attribute>,
        vis: Visibility,
        widget_name: Ident,
        is_mixin: bool,
        new: Option<WgtItemNew>,
        new_child: Option<WgtItemNew>,
        properties: WidgetProperties,
        defaults: WidgetDefaults,
        whens: WidgetWhens,
    }

    impl ToTokens for WidgetMod {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let attrs = self.attrs.iter();
            let docs = &self.docs;
            let vis = &self.vis;
            let widget_name = &self.widget_name;
            let crate_ = zero_ui_crate_ident();

            let some_mixin = if self.is_mixin { Some(()) } else { None };
            let use_implicit_mixin = some_mixin.map(|_| quote!( use #crate_::widgets::implicit_mixin; ));
            let new = some_mixin.map(|_| {
                if let Some(new) = &self.new {
                    new.to_token_stream()
                } else {
                    let fn_doc = format!("Manually initializes a new [`{0}`](self).", widget_name);
                    quote!(
                        #fn_doc
                        #[inline]
                        pub fn new(child: impl #crate_::core::UiNode, id: impl properties::id::Args) -> impl #crate_::core::UiNode {
                            #crate_::core::default_widget_new(child, id)
                        }
                    )
                }
            });
            let new_child = some_mixin.map(|_| {
                if let Some(new_child) = &self.new_child {
                    new_child.to_token_stream()
                } else {
                    let fn_doc = format!("Manually initializes a new [`{}`](self) content.", widget_name);
                    quote!(
                        #[doc=#fn_doc]
                        #[inline]
                        pub fn new_child<C: #crate_::core::UiNode>(child: C) -> C {
                            #crate_::core::default_widget_new_child(child)
                        }
                    )
                }
            });

            let properties = &self.properties;
            let defaults = &self.defaults;
            let whens = &self.whens;

            tokens.extend(quote! {
                #(#attrs)*
                #docs
                #vis mod #widget_name {
                    #[doc(hidden)]
                    pub use super::*;
                    #use_implicit_mixin;

                    // new functions.
                    #new
                    #new_child

                    // properties re-export mod.
                    #properties

                    // property default values mod.
                    #defaults

                    // when condition var init fns mod.
                    #whens
                }
            })
        }
    }

    struct WidgetDocs {
        docs: Vec<Attribute>,
        is_mixin: bool,
        ///required properties
        required: Vec<PropertyDocs>,
        ///properties with provided default value
        provided: Vec<PropertyDocs>,
        ///properties that are defined in the widget, but have no default value and are not required
        other: Vec<PropertyDocs>,
    }

    impl ToTokens for WidgetDocs {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            for doc in &self.docs {
                doc.to_tokens(tokens)
            }

            doc_extend!(
                tokens,
                "\n</div><style>span.wgprop p {{ display: inline; margin-left:-1ch; }}</style><script>{}</script>",
                include_str!("widget_docs_ext.js")
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

    struct PropertyDocs {
        docs: Vec<Attribute>,
        target_child: bool,
        ident: Ident,
        property_source: PropertySource,
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
                    source_widget = format!("{}", quote!(#p)).replace(" :: ", "::");
                    doc_extend!(
                        tokens,
                        "\n[<span class='mod' data-inherited>{}</span>]({})\n",
                        self.ident,
                        source_widget
                    );
                }
            }

            doc_extend!(tokens, "<ul style='display:none;'></ul></span></code></h3>");

            if is_inherited || !self.docs.is_empty() {
                doc_extend!(tokens, "<div class='docblock'>\n");
                for doc in &self.docs {
                    doc.to_tokens(tokens)
                }
                if is_inherited {
                    let name_start = source_widget.rfind(':').map(|i| i + 1).unwrap_or_default();
                    doc_extend!(
                        tokens,
                        "\n*Inherited from [`{}`]({}).*",
                        &source_widget[name_start..],
                        source_widget
                    );
                }
                doc_extend!(tokens, "\n</div>");
            }
        }
    }

    enum PropertySource {
        Property(Ident),
        Widget(Path),
    }

    impl ToTokens for WgtItemNew {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            for attr in &self.attrs {
                attr.to_tokens(tokens)
            }
            tokens.extend(quote_spanned!(self.fn_token.span()=> pub));
            self.fn_token.to_tokens(tokens);
            let child = self.inputs.first().unwrap();
            let mut crate_ = zero_ui_crate_ident();
            crate_.set_span(child.span());
            let child = quote_spanned! {child.span()=> #child: impl #crate_::core::UiNode};
            let args = self
                .inputs
                .iter()
                .skip(1)
                .map(|a| quote_spanned! {a.span()=> #a: impl properties::#a::Args});
            tokens.extend(quote_spanned! {self.paren_token.span=> (#child, #(#args)*) });
            self.r_arrow_token.to_tokens(tokens);
            self.return_type.to_tokens(tokens);
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
    struct WidgetProperties {
        props: Vec<WidgetPropertyUse>,
    }

    impl ToTokens for WidgetProperties {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let props = &self.props;
            tokens.extend(quote! {
                #[doc(hidden)]
                pub mod properties {
                    pub use super::*;
                    #(#props)*
                }
            })
        }
    }

    enum WidgetPropertyUse {
        Mod(Ident),
        Alias { ident: Ident, original: Ident },
        Inherited { widget: Path, ident: Ident },
    }

    impl ToTokens for WidgetPropertyUse {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let tt = match self {
                WidgetPropertyUse::Mod(ident) => quote!(pub use #ident;),
                WidgetPropertyUse::Alias { ident, original } => quote!(pub use #original as #ident;),
                WidgetPropertyUse::Inherited { widget, ident } => quote!(pub use #widget::properties::#ident;),
            };
            tokens.extend(tt);
        }
    }

    struct WidgetDefaults {
        defaults: Vec<WidgetDefault>,
    }

    impl ToTokens for WidgetDefaults {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let defaults = &self.defaults;
            let tt = quote! {
                #[doc(hidden)]
                pub mod defaults {
                    use super::*;
                    #(#defaults)*
                }
            };
            tokens.extend(tt);
        }
    }

    struct WidgetDefault {
        property: Ident,
        default: PropertyDefaultValue,
    }

    impl ToTokens for WidgetDefault {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let property = &self.property;
            let tt = match &self.default {
                PropertyDefaultValue::Fields(f) => {
                    let fields = &f.fields;
                    quote! {
                        property::#property::NamedArgs {
                            _phantom: std::marker::PhantomData,
                            #fields
                        }
                    }
                }
                PropertyDefaultValue::Args(a) => {
                    let args = &a.0;
                    quote!(properties::#property::args(#args))
                }
                PropertyDefaultValue::Inherited(widget) => quote!(#widget::defaults::#property()),
            };
            tokens.extend(quote! {
                #[inline]
                pub fn #property() -> impl properties::#property::Args {
                    #tt
                }
            });
        }
    }

    enum PropertyDefaultValue {
        Fields(PropertyFields),
        Args(PropertyArgs),
        Inherited(Path),
    }

    struct WidgetWhens {
        conditions: Vec<WhenCondition>,
    }

    impl ToTokens for WidgetWhens {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let conditions = &self.conditions;
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

    pub struct WhenCondition {
        pub index: usize,
        pub properties: Vec<Ident>,
        pub expr: WhenConditionExpr,
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
            let tt = quote! {
                #[inline]
                pub fn #fn_ident(#(#p: impl properties::#p::Args),*) -> #crate_:::core::var::Var<bool> {
                    #(
                       {#[allow(unused)]
                        use properties::#p::is_allowed_in_when;}
                    )*
                    #expr
                }
            };
            tokens.extend(tt);
        }
    }

    pub enum WhenConditionExpr {
        Ref(WhenPropertyRef),
        Map(WhenPropertyRef, Box<Expr>),
        Merge(Vec<WhenPropertyRef>, Box<Expr>),
        Inherited(InheritedWhen),
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

    impl ToTokens for InheritedWhen {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let widget = &self.widget;
            let properties = &self.properties;
            let fn_ = &self.when_name;
            tokens.extend(quote! { #widget::whens::#fn_(#(#properties),*) });
        }
    }

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

    pub enum WhenPropertyRefArg {
        Index(usize),
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
