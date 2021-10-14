use proc_macro2::TokenStream;
use syn::{bracketed, parse::Parse, parse_macro_input, Expr, Token};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { init, attrs, exprs } = parse_macro_input!(input as Input);

    let r = quote! {
        {
            let l = #init;
            #(
            #attrs
            let l = l.push(#exprs);
            )*
            l
        }
    };

    r.into()
}

struct Input {
    init: Expr,
    attrs: Vec<TokenStream>,
    exprs: Vec<Expr>,
}
impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let init = input.parse::<Expr>()?;
        let _ = input.parse::<Token![;]>()?;

        let mut attrs = vec![];
        let mut exprs = vec![];

        while !input.is_empty() {
            let mut attr = TokenStream::new();
            while input.peek(Token![#]) {
                let _ = input.parse::<Token![#]>().unwrap();
                let a;
                let _ = bracketed!(a in input);
                let a = a.parse::<TokenStream>().unwrap();
                attr.extend(quote! { #[#a] });
            }
            attrs.push(attr);
            exprs.push(input.parse::<Expr>()?);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>().unwrap();
            }
        }

        Ok(Input { init, attrs, exprs })
    }
}
