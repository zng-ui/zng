use std::env::VarError;

use proc_macro2::{Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{parse::Parse, parse_macro_input, token, Ident, Path, Token};

use crate::util::{token_stream_eq, tokens_to_ident_str};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as VarExpr);
    todo!()
}

struct VarExpr {
    mod_: Path,
    vars: Vec<(Ident, TokenStream)>,
    expr: TokenStream,
}
impl Parse for VarExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mod_ = input.parse().unwrap_or_else(|e| non_user_error!(e));
        input.parse::<Token![,]>().unwrap_or_else(|e| non_user_error!(e));
        let mut expr = TokenStream::default();
        let mut vars: Vec<(Ident, TokenStream)> = vec![];

        while !input.is_empty() {
            if input.peek(keyword::v) && input.peek2(token::Brace) {
                input.parse::<keyword::v>().unwrap();
                let var = input.parse::<Group>().unwrap().stream();
                if let Some((var_ident, _)) = vars.iter().find(|(_, v)| token_stream_eq(v, &var)) {
                    var_ident.to_tokens(&mut expr)
                } else {
                    let var_ident = ident!("__{}_{}", vars.len(), tokens_to_ident_str(&var));
                    var_ident.to_tokens(&mut expr);
                    vars.push((var_ident, var))
                }
            //} else if input.peek(??) {
            //  // TODO: search for v{} inside Groups
            } else {
                let tt = input.parse::<TokenTree>().unwrap();
                tt.to_tokens(&mut expr)
            }
        }

        Ok(VarExpr { mod_, vars, expr })
    }
}

mod keyword {
    syn::custom_keyword!(v);
}
