use input::WidgetDeclaration;
use syn::parse_macro_input;

/// `widget!` actual expansion, in stage3 we have all the inherited tokens to work with.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as WidgetDeclaration);

    todo!()
}

mod input {
    #![allow(unused)]

    use crate::util::{non_user_braced, non_user_parenthesized, NON_USER_ERROR};
    use crate::widget_stage1::WidgetHeader;
    use proc_macro2::{TokenTree, TokenStream};
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

    struct PropertyBlock<P> {
        brace_token: token::Brace,
        properties: Vec<P>,
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
        inherits: Vec<InheritItem>,
        header: WidgetHeader,
        items: Vec<WgtItem>,
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

    struct InheritItem {
        ident: Ident,
        inherit_path: Path,
        default: Punctuated<InheritedProperty, Token![,]>,
        default_child: Punctuated<InheritedProperty, Token![,]>,
        whens: Punctuated<InheritedWhen, Token![,]>,
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

    struct InheritedProperty {
        docs: Vec<Attribute>,
        kind: BuiltPropertyKind,
        ident: Ident,
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
    enum BuiltPropertyKind {
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

    struct InheritedWhen {
        docs: Vec<Attribute>,
        args: Punctuated<Ident, Token![,]>,
        sets: Punctuated<Ident, Token![,]>,
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

    enum WgtItem {
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

    struct WgtItemDefault {
        target: DefaultTarget,
        block: DefaultBlock,
    }

    impl Parse for WgtItemDefault {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(WgtItemDefault {
                target: input.parse()?,
                block: input.parse()?,
            })
        }
    }

    enum DefaultTarget {
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

    type DefaultBlock = PropertyBlock<PropertyDeclaration>;

    struct PropertyDeclaration {
        attrs: Vec<Attribute>,
        ident: Ident,
        maps_to: Option<MappedProperty>,
        default_value: Option<(Token![:], PropertyDefaultValue)>,
        semi_token: Token![;],
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

    struct MappedProperty {
        r_arrow_token: Token![->],
        ident: Ident,
    }
    impl Parse for MappedProperty {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(MappedProperty {
                r_arrow_token: input.parse()?,
                ident: input.parse()?,
            })
        }
    }

    enum PropertyDefaultValue {
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

    struct PropertyFields {
        brace_token: token::Brace,
        fields: Punctuated<FieldValue, Token![,]>,
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

    struct PropertyArgs(Punctuated<Expr, Token![,]>);

    impl Parse for PropertyArgs {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyArgs(Punctuated::parse_terminated(input)?))
        }
    }

    struct PropertyUnset {
        unset_token: keyword::unset,
        bang_token: Token![!],
    }

    impl Parse for PropertyUnset {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyUnset {
                unset_token: input.parse()?,
                bang_token: input.parse()?,
            })
        }
    }

    struct PropertyRequired {
        required_token: keyword::required,
        bang_token: Token![!],
    }

    impl Parse for PropertyRequired {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyRequired {
                required_token: input.parse()?,
                bang_token: input.parse()?,
            })
        }
    }

    struct WgtItemNew {
        attrs: Vec<Attribute>,
        fn_token: Token![fn],
        target: NewTarget,
        paren_token: token::Paren,
        inputs: Punctuated<Ident, Token![,]>,
        r_arrow_token: Token![->],
        return_type: Box<Type>,
        block: Block,
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

    enum NewTarget {
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

    struct WgtItemWhen {
        attrs: Vec<Attribute>,
        when_token: keyword::when,
        condition: Box<Expr>,
        block: WhenBlock,
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

    type WhenBlock = PropertyBlock<PropertyAssign>;

    struct PropertyAssign {
        ident: Ident,
        colon_token: Token![:],
        value: PropertyValue,
        semi_token: Token![;],
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

    enum PropertyValue {
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

mod output {
    
}