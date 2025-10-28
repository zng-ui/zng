use quote::ToTokens as _;
use syn::{parse::Parse, *};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input {
        shorthand_struct, ident, ..
    } = parse_macro_input!(input as Input);

    if ident == "unset" {
        return quote! {
            compile_error!("shorthand unit unset! is reserved");
        }
        .into();
    }

    let chars: Vec<_> = ident.to_string().chars().take(33).collect();
    if chars.len() > 32 {
        return quote! {
            compile_error!("shorthand unit exceeds maximum 32 characters");
        }
        .into();
    }

    let mut out = shorthand_struct.to_token_stream();
    out.extend(quote!(::<));
    for c in chars {
        out.extend(quote!(#c,));
    }
    out.extend(quote!(>));

    out.into()
}

struct Input {
    shorthand_struct: Path,
    _comma: Token![,],
    ident: Ident,
}
impl Parse for Input {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Self {
            shorthand_struct: input.parse()?,
            _comma: input.parse()?,
            ident: input.parse()?,
        })
    }
}
