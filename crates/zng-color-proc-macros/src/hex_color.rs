use proc_macro2::{Span, TokenStream};
use syn::{LitInt, Path, Token, parse::Parse, parse_macro_input};

struct Input {
    crate_: Path,
    #[expect(unused)]
    comma: Token![,],
    hex: TokenStream,
}
impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Input {
            crate_: input.parse()?,
            comma: input.parse()?,
            hex: input.parse()?,
        })
    }
}

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { crate_, hex, .. } = parse_macro_input!(input as Input);

    let input_str: String = hex.to_string();

    let hex_only = input_str
        .trim()
        .trim_start_matches('#')
        .trim_start_matches("0x")
        .trim_start()
        .replace('_', "");

    match hex_only.len() {
        // RRGGBB
        6 => {
            let rgb = pair_to_f32(&hex_only);
            quote! {
                #crate_::Rgba::new(#rgb 1.0)
            }
        }
        // RRGGBBAA
        8 => {
            let rgba = pair_to_f32(&hex_only);
            quote! {
                #crate_::Rgba::new(#rgba)
            }
        }
        // RGB
        3 => {
            let rgb = single_to_f32(&hex_only);
            quote! {
                #crate_::Rgba::new(#rgb 1.0)
            }
        }
        // RGBA
        4 => {
            let rgba = single_to_f32(&hex_only);
            quote! {
                #crate_::Rgba::new(#rgba)
            }
        }
        // error
        _ => {
            quote! {
                compile_error!("expected [#|0x]RRGGBB[AA] or [#|0x]RGB[A] color hexadecimal");
            }
        }
    }
    .into()
}

fn pair_to_f32(s: &str) -> TokenStream {
    let mut r = TokenStream::new();
    for i in 0..s.len() / 2 {
        let i = i * 2;
        let lit = LitInt::new(&format!("0x{}", &s[i..i + 2]), Span::call_site());
        r.extend(quote! { #lit as f32 / 255.0, })
    }
    r
}

fn single_to_f32(s: &str) -> TokenStream {
    s.chars()
        .map(|c| {
            let lit = LitInt::new(&format!("0x{c}{c}"), Span::call_site());
            quote! { #lit as f32 / 255.0, }
        })
        .collect()
}
