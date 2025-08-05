use proc_macro2::{Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    Expr, Ident, Path, Token,
    parse::{Parse, ParseStream},
    parse_macro_input, parse2,
    spanned::Spanned,
    token,
};

use crate::util::{token_stream_eq, tokens_to_ident_str};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let VarExpr { mod_, vars, expr } = parse_macro_input!(input as VarExpr);

    let r = if vars.is_empty() {
        // no interpolation, just eval to var.

        if parse2::<Expr>(expr.clone()).is_ok() {
            quote_spanned! {expr.span()=>
                #mod_::expr_var_into(#expr)
            }
        } else {
            // support statement blocks using the macro braces, if we just add the braces for
            // all input it can cause the `unused_braces` lint, and we need the entire expression to have
            // the span so that type mismatch gets highlighted correctly, so we *try* parse as expr and only
            // add the braces if not.
            quote_spanned! {expr.span()=>
                #mod_::expr_var_into({#expr})
            }
        }
    } else if vars.len() == 1 {
        let (ident, eval) = &vars[0];

        if token_stream_eq(expr.clone(), quote!(#ident)) || token_stream_eq(expr.clone(), quote!(*#ident)) {
            // full expr is an interpolation, just return the var.
            quote_spanned! {expr.span()=>
                #mod_::expr_var_as(#eval)
            }
        } else {
            quote_spanned! {expr.span()=>
                // single var interpolation, use map.
                #mod_::expr_var_map(#eval, move |#[allow(non_snake_case)]#ident|{ #expr })
            }
        }
    } else {
        // multiple var interpolation, use merge.
        let idents = vars.iter().map(|(id, _)| id);
        let evals = vars.iter().map(|(_, ev)| ev);
        quote_spanned! {expr.span()=>
            #mod_::merge_var!{ #({#evals}),* , move |#(#[allow(non_snake_case)]#idents),*| { #expr } }
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
        // look for variable interpolation `#{<block>}` :
        if input.peek(Token![#]) && input.peek2(token::Brace) {
            input.parse::<Token![#]>().unwrap();
            let var = input.parse::<Group>().unwrap().stream();
            if let Some((var_ident, _)) = vars.iter().find(|(_, v)| token_stream_eq(v.clone(), var.clone())) {
                var_ident.to_tokens(&mut expr)
            } else {
                let var_ident = ident_spanned!(var.span()=> "__{}_{}", vars.len(), tokens_to_ident_str(&var));
                var_ident.to_tokens(&mut expr);
                vars.push((var_ident, var))
            }
        }
        // recursive parse groups:
        else if input.peek(token::Brace) {
            assert_group(|| {
                let inner;
                let group = syn::braced!(inner in input);
                let inner = parse_replace_expr(&inner, vars);
                group.surround(&mut expr, |e| e.extend(inner));
                Ok(())
            });
        } else if input.peek(token::Paren) {
            assert_group(|| {
                let inner;
                let group = syn::parenthesized!(inner in input);
                let inner = parse_replace_expr(&inner, vars);
                group.surround(&mut expr, |e| e.extend(inner));
                Ok(())
            });
        } else if input.peek(token::Bracket) {
            assert_group(|| {
                let inner;
                let group = syn::bracketed!(inner in input);
                let inner = parse_replace_expr(&inner, vars);
                group.surround(&mut expr, |e| e.extend(inner));
                Ok(())
            });
        }
        // keep other tokens the same:
        else {
            let tt = input.parse::<TokenTree>().unwrap();
            tt.to_tokens(&mut expr)
        }
    }

    expr
}
/// syn::braced! generates an error return.
fn assert_group(f: impl FnOnce() -> syn::parse::Result<()>) {
    f().unwrap()
}
