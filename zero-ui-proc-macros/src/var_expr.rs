use proc_macro2::{Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, token, Ident, Path, Token,
};

use crate::util::{non_user_braced, non_user_bracketed, non_user_parenthesized, token_stream_eq, tokens_to_ident_str};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let VarExpr { mod_, vars, expr } = parse_macro_input!(input as VarExpr);

    let r = if vars.is_empty() {
        // no interpolation, just eval to var.
        quote! {
            #mod_::IntoVar::into_var({ #expr })
        }
    } else if vars.len() == 1 {
        let (ident, eval) = &vars[0];
        if token_stream_eq(expr.clone(), ident.to_token_stream()) {
            // full expr is an interpolation, just return the  var.
            quote! {
                {#eval}
            }
        } else {
            quote! {
                // single var interpolation, use map.
                #mod_::Var::into_map({#eval}, |#ident|{ #expr })
            }
        }
    } else {
        // multiple var interpolation, use merge.
        let idents = vars.iter().map(|(id, _)| id);
        let evals = vars.iter().map(|(_, ev)| ev);
        quote! {
            #mod_::merge_var!{ #({#evals}),* , |#(#idents),*| { #expr } }
        }
    };

    r.into()
}

struct VarExpr {
    mod_: Path,
    vars: Vec<(Ident, TokenStream)>,
    expr: TokenStream,
}
impl Parse for VarExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mod_ = input.parse().unwrap_or_else(|e| non_user_error!(e));
        input.parse::<Token![,]>().unwrap_or_else(|e| non_user_error!(e));
        let mut vars = vec![];
        let expr = parse_replace_expr(input, &mut vars);

        Ok(VarExpr { mod_, vars, expr })
    }
}

fn parse_replace_expr(input: ParseStream, vars: &mut Vec<(Ident, TokenStream)>) -> TokenStream {
    let mut expr = TokenStream::default();

    while !input.is_empty() {
        // look for variable interpolation `v{<block>}` :
        if input.peek(keyword::v) && input.peek2(token::Brace) {
            input.parse::<keyword::v>().unwrap();
            let var = input.parse::<Group>().unwrap().stream();
            if let Some((var_ident, _)) = vars.iter().find(|(_, v)| token_stream_eq(v.clone(), var.clone())) {
                var_ident.to_tokens(&mut expr)
            } else {
                let var_ident = ident!("__{}_{}", vars.len(), tokens_to_ident_str(&var));
                var_ident.to_tokens(&mut expr);
                vars.push((var_ident, var))
            }
        }
        // recursive parse groups:
        else if input.peek(token::Brace) {
            let inner = parse_replace_expr(&non_user_braced(input), vars);
            expr.extend(quote! { { #inner } });
        } else if input.peek(token::Paren) {
            let inner = parse_replace_expr(&non_user_parenthesized(input), vars);
            expr.extend(quote! { ( #inner ) });
        } else if input.peek(token::Bracket) {
            let inner = parse_replace_expr(&non_user_bracketed(input), vars);
            expr.extend(quote! { [ #inner ] });
        }
        // keep other tokens the same:
        else {
            let tt = input.parse::<TokenTree>().unwrap();
            tt.to_tokens(&mut expr)
        }
    }

    expr
}

mod keyword {
    syn::custom_keyword!(v);
}
