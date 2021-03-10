use proc_macro2::{Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    token, Ident, Path, Token,
};

use crate::util::{token_stream_eq, tokens_to_ident_str};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let VarExpr { mod_, vars, expr } = parse_macro_input!(input as VarExpr);

    let r = if vars.is_empty() {
        // no interpolation, just eval to var.
        quote_spanned! {expr.span()=>
            #mod_::IntoVar::into_var({ #expr })
        }
    } else if vars.len() == 1 {
        let (ident, eval) = &vars[0];
        if token_stream_eq(expr.clone(), ident.to_token_stream()) {
            // full expr is an interpolation, just return the  var.
            quote_spanned! {expr.span()=>
                {#eval}
            }
        } else {
            quote_spanned! {expr.span()=>
                // single var interpolation, use map.
                #mod_::Var::into_map({#eval}, move |#[allow(non_snake_case)]#ident|{ #expr })
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
            let inner = parse_replace_expr(&non_user_braced!(input), vars);
            expr.extend(quote! { { #inner } });
        } else if input.peek(token::Paren) {
            let inner = parse_replace_expr(&non_user_parenthesized!(input), vars);
            expr.extend(quote! { ( #inner ) });
        } else if input.peek(token::Bracket) {
            let inner = parse_replace_expr(&non_user_bracketed!(input), vars);
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

/// Like [`syn::Expr::parse_without_eager_brace`] but does not actually parse anything and includes
/// the braces of interpolation.
pub fn parse_without_eager_brace(input: ParseStream) -> TokenStream {
    let mut r = TokenStream::default();
    let mut is_start = true;
    while !input.is_empty() {
        if input.peek2(token::Brace) {
            if input.cursor().punct().is_some() {
                // #{} or ={}
                let tt = input.parse::<TokenTree>().unwrap();
                tt.to_tokens(&mut r);
                let tt = input.parse::<TokenTree>().unwrap();
                tt.to_tokens(&mut r);
            } else {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                break; // found { } after expr or Struct { }
            }
        } else if !is_start && input.peek(token::Brace) {
            break; // found { } after expr
        } else {
            let tt = input.parse::<TokenTree>().unwrap();
            tt.to_tokens(&mut r);
        }
        is_start = false;
    }
    r
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use syn::parse::Parse;

    macro_rules! assert_tt_eq {
        ($expected:expr, $actual:expr) => {{
            let expected = $expected;
            let actual = $actual;
            assert!(
                $crate::util::token_stream_eq(expected.clone(), actual.clone()),
                "\n\nexpected: `{}`\n  actual: `{}`\n\n",
                expected,
                actual
            );
        }};
    }

    #[test]
    fn parse_expr_without_interpolation_a() {
        let input = quote! {
            // if
            a == self.b {
                println!("is true");
            }
        };
        let expected = quote! {
            a == self.b
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_without_interpolation_b() {
        let input = quote! {
            // if
            (a == self.b) {
                println!("is true");
            }
        };
        let expected = quote! {
            (a == self.b)
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_without_interpolation_c() {
        let input = quote! {
            // if
            a == ( A { } ) {
                println!("is true");
            }
        };
        let expected = quote! {
            a == ( A { } )
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_without_interpolation_d() {
        let input = quote! {
            // if
            a == A { } {
                println!("is true");
            }
        };
        let expected = quote! {
            a == A
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_without_interpolation_e() {
        let input = quote! {
            // if
            a == { true } {
                println!("is true");
            }
        };
        let expected = quote! {
            a == { true }
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_with_interpolation() {
        let input = quote! {
            // if
            a == #{b} {
                println!("is true");
            }
        };
        let expected = quote! {
            a == #{b}
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    struct ParseWithoutEagerBrace(TokenStream);
    impl Parse for ParseWithoutEagerBrace {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let l_expr = super::parse_without_eager_brace(input);
            let _: TokenStream = input.parse()?;
            Ok(ParseWithoutEagerBrace(l_expr))
        }
    }

    fn test_parse(input: TokenStream) -> TokenStream {
        syn::parse2::<ParseWithoutEagerBrace>(input).unwrap().0
    }
}
