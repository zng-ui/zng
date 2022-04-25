//! This macro can almost be implemented using macro_rules!, we only need a proc-macro because
//! of the ambiguity of parsing outer attributes before expressions.

use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Attribute, Expr, Path, Token,
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input {
        vars_mod,
        conditions,
        default: DefaultArm {
            attrs: default_attrs,
            value: default_value,
            ..
        },
        ..
    } = parse_macro_input!(input as Input);

    // start builder
    let mut out = if conditions.len() >= 8 {
        quote! {
            #(#default_attrs)*
            let __b = #vars_mod::types::WhenVarBuilderDyn::new(#default_value);
        }
    } else {
        quote! {
            #(#default_attrs)*
            let __b = #vars_mod::types::WhenVarBuilder::new(#default_value);
        }
    };

    // add conditions
    for ConditionArm {
        attrs, condition, value, ..
    } in conditions
    {
        out.extend(quote! {
            #(#attrs)*
            let __b = __b.push(#condition, #value);
        });
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
    condition: Expr,
    _fat_arrow_token: Token![=>],
    value: Expr,
}
impl Parse for ConditionArm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ConditionArm {
            attrs: Attribute::parse_outer(input)?,
            condition: input.parse()?,
            _fat_arrow_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

struct DefaultArm {
    attrs: Vec<Attribute>,
    _wild_token: Token![_],
    _fat_arrow_token: Token![=>],
    value: Expr,
}
impl Parse for DefaultArm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(DefaultArm {
            attrs: Attribute::parse_outer(input)?,
            _wild_token: input.parse()?,
            _fat_arrow_token: input.parse()?,
            value: input.parse()?,
        })
    }
}
