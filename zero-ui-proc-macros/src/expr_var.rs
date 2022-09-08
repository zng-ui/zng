use proc_macro2::{Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse2, parse_macro_input,
    spanned::Spanned,
    token, Expr, Ident, Path, Token,
};

use crate::util::{token_stream_eq, tokens_to_ident_str};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let VarExpr { mod_, vars, expr } = parse_macro_input!(input as VarExpr);

    let r = if vars.is_empty() {
        // no interpolation, just eval to var.

        if parse2::<Expr>(expr.clone()).is_ok() {
            quote_spanned! {expr.span()=>
                #mod_::IntoVar::<bool>::into_var(#expr)
            }
        } else {
            // support statement blocks using the macro braces, if we just add the braces for
            // all input it can cause the `unused_braces` lint, and we need the entire expression to have
            // the span so that type mismatch gets highlighted correctly, so we *try* parse as expr and only
            // add the braces if not.
            quote_spanned! {expr.span()=>
                #mod_::IntoVar::<bool>::into_var({#expr})
            }
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
                #mod_::Var::map(&{#eval}, move |#[allow(non_snake_case)]#ident|{ #expr })
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
        if input.peek(Token![match]) || input.peek(Token![while]) {
            // keyword
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            // expr
            r.extend(parse_without_eager_brace(input));
            // block
            if input.peek(token::Brace) {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            }
        } else if input.peek(Token![if]) {
            // keyword
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            // expr
            r.extend(parse_without_eager_brace(input));
            // block
            if input.peek(token::Brace) {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);

                if input.peek(Token![else]) {
                    input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                    if input.peek(token::Brace) {
                        // else { }
                        input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                    } else {
                        // maybe another if
                        continue;
                    }
                }
            }
        } else if input.peek(Token![loop]) {
            // keyword
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            // block
            if input.peek(token::Brace) {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            }
        } else if input.peek(Token![for]) {
            // keyword (for)
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            while !input.is_empty() && !input.peek(Token![in]) {
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            }
            if !input.is_empty() {
                // keyword (in)
                input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                //expr
                r.extend(parse_without_eager_brace(input));
                // block
                if input.peek(token::Brace) {
                    input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
                }
            }
        } else if input.peek2(token::Brace) {
            if let Some(p) = input.cursor().punct() {
                if p.0.as_char() != '.' {
                    let tt = input.parse::<TokenTree>().unwrap();
                    tt.to_tokens(&mut r);
                    let tt = input.parse::<TokenTree>().unwrap();
                    tt.to_tokens(&mut r);
                    continue; // found #{ }
                }
            }
            input.parse::<TokenTree>().unwrap().to_tokens(&mut r);
            break; // found { } after expr or Struct { }
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
                "\n\nexpected: `{expected}`\n  actual: `{actual}`\n\n",
            );
        }};
    }

    #[test]
    fn parse_expr_without_interpolation_simple1() {
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
    fn parse_expr_without_interpolation_simple2() {
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
    fn parse_expr_without_interpolation_escaped1() {
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
    fn parse_expr_without_interpolation_invalid1() {
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
    fn parse_expr_without_interpolation_invalid2() {
        let input = quote! {
            // if
            A { } == a {
                println!("is true");
            }
        };
        let expected = quote! {
            A
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
    fn parse_expr_without_interpolation_j() {
        let input = quote! {
            // if
            a == !{ true } {
                println!("is false");
            }
        };
        let expected = quote! {
            a == !{ true }
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

    #[test]
    fn parse_expr_end_punct() {
        let input = quote! {
            // if
            a == 0. {
                println!("is zero")
            }
        };
        let expected = quote! {
            a == 0.
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_end_match() {
        let input = quote! {
            // if
           match a { 5 => false, 8 => true } {
                println!("is 8")
            }
        };
        let expected = quote! {
            match a { 5 => false, 8 => true }
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_end_if() {
        let input = quote! {
            // if
           if a == 8 {  false } else { true } {
                println!("is 8")
            }
        };
        let expected = quote! {
            if a == 8 {  false } else { true }
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_end_if2() {
        let input = quote! {
            // if
            if a == 8 {  false } else if a > 10 { true } else { true } {
                println!("is 8")
            }
        };
        let expected = quote! {
            if a == 8 {  false } else if a > 10 { true } else { true }
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_end_loop() {
        let input = quote! {
            // if
            loop { break true; } {
                println!("is 8")
            }
        };
        let expected = quote! {
            loop { break true; }
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }

    #[test]
    fn parse_expr_end_while() {
        let input = quote! {
            // if
            () == while a { } {
                println!("is 8")
            }
        };
        let expected = quote! {
            () == while a { }
        };
        let actual = test_parse(input);

        assert_tt_eq!(expected, actual);
    }
    #[test]
    fn parse_expr_end_for() {
        let input = quote! {
            // if
            () == for i in name { }  {
                println!("is 8")
            }
        };
        let expected = quote! {
            () == for i in name { }
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
