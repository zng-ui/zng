//! Macro for `task::any!` or `task::all!` calls with more then 8 futures.

use syn::{
    Expr, Path, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { macro_path, futs, .. } = parse_macro_input!(input as Input);

    let fut_idents = (0..futs.len()).map(|i| ident!("__fut{i}"));
    let futs = futs.iter();

    let r = quote! {
        #macro_path! {
            #(#fut_idents: #futs;)*
        }
    };

    r.into()
}

struct Input {
    macro_path: Path,
    _path_semi: Token![;],
    futs: Punctuated<Expr, Token![,]>,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Input {
            macro_path: input.parse()?,
            _path_semi: input.parse()?,
            futs: Punctuated::parse_separated_nonempty(input)?,
        })
    }
}
