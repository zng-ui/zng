use syn::{
    Attribute, Expr, Path, Token, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input {
        vars_mod,
        conditions,
        default:
            DefaultArm {
                attrs: default_attrs,
                use_macro: default_use_macro,
                value: default_value,
                ..
            },
        ..
    } = parse_macro_input!(input as Input);

    // start builder
    let mut out = quote! {
        #(#default_attrs)*
        let mut __b = #vars_mod::WhenVarBuilder::new(#default_value);
    };

    if let Some(m) = default_use_macro {
        let m = m.path;
        out = quote! {
            #m! {
                #out
            }
        };
    }

    // add conditions
    for ConditionArm {
        attrs,
        use_macro,
        condition,
        value,
        ..
    } in conditions
    {
        let mut arm = quote! {
            #(#attrs)*
            __b.push(#condition, #value);
        };

        if let Some(m) = use_macro {
            let m = m.path;
            arm = quote! {
                #m! {
                    #arm
                }
            };
        }

        out.extend(arm);
    }

    // build
    out = quote! {
        {
            #out
            __b.build()
        }
    };

    out.into()
}

struct Input {
    vars_mod: Path,
    conditions: Punctuated<ConditionArm, Token![,]>,
    default: DefaultArm,
    _trailing_comma: Option<Token![,]>,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vars_mod = input.parse()?;
        let mut conditions = Punctuated::new();
        while !input.peek(Token![_]) {
            conditions.push(input.parse()?);
            conditions.push_punct(input.parse()?);
        }
        Ok(Input {
            vars_mod,
            conditions,
            default: input.parse()?,
            _trailing_comma: input.parse()?,
        })
    }
}

struct ConditionArm {
    attrs: Vec<Attribute>,
    use_macro: Option<UseMacro>,
    condition: Expr,
    _fat_arrow_token: Token![=>],
    value: Expr,
}
impl Parse for ConditionArm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ConditionArm {
            attrs: Attribute::parse_outer(input)?,
            use_macro: if input.peek(Token![use]) { Some(input.parse()?) } else { None },
            condition: input.parse()?,
            _fat_arrow_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

struct DefaultArm {
    attrs: Vec<Attribute>,
    use_macro: Option<UseMacro>,
    _wild_token: Token![_],
    _fat_arrow_token: Token![=>],
    value: Expr,
}
impl Parse for DefaultArm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(DefaultArm {
            attrs: Attribute::parse_outer(input)?,
            use_macro: if input.peek(Token![use]) { Some(input.parse()?) } else { None },
            _wild_token: input.parse()?,
            _fat_arrow_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

struct UseMacro {
    path: Path,
}
impl Parse for UseMacro {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _: Token![use] = input.parse()?;
        let inner;
        let _ = parenthesized!(inner in input);
        Ok(UseMacro { path: inner.parse()? })
    }
}
