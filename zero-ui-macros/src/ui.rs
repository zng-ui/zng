use proc_macro::TokenStream;
use syn::{parse::*, punctuated::Punctuated, token::Token, *};

pub(crate) fn implementation(input: TokenStream) -> TokenStream {
    let Input { properties, child, .. } = parse_macro_input!(input as Input);

    let properties = properties.into_iter().map(|Property { name, args, .. }| {
        let args = args.into_iter();
        quote! {
            let child = #name(child, #(#args),*);
        }
    });

    let result = quote! {{
        let child = #child;
        #(#properties)*

        child
    }};

    TokenStream::from(result)
}

struct Property {
    name: Ident,
    _separator: Token![:],
    args: Punctuated<Expr, Token![,]>,
}

impl Parse for Property {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Property {
            name: input.parse()?,
            _separator: input.parse()?,
            args: Punctuated::parse_separated_nonempty(input)?,
        })
    }
}

struct Input {
    properties: Punctuated<Property, Token![;]>,
    _separator: Token![=>],
    child: Expr,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Input {
            properties: parse_properties(input)?,
            _separator: input.parse()?,
            child: input.parse()?,
        })
    }
}

fn parse_properties(input: ParseStream) -> Result<Punctuated<Property, Token![;]>> {
    let mut punctuated = Punctuated::new();

    loop {
        let value = input.parse()?;
        punctuated.push_value(value);
        let punct = input.parse()?;
        punctuated.push_punct(punct);
        if <Token![=>]>::peek(input.cursor()) {
            break;
        }
    }

    Ok(punctuated)
}
