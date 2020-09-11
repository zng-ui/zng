use crate::util::*;
use proc_macro2::{Span, TokenStream};
use syn::LitInt;

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_str: String = input.to_string();
    let hex_only = input_str
        .trim()
        .trim_start_matches('#')
        .trim_start_matches("0x")
        .trim_start()
        .replace('_', "");
    let crate_ = zero_ui_crate_ident();

    match hex_only.len() {
        // RRGGBB
        6 => {
            let rgb = pair_to_f32(&hex_only);
            quote! {
                #crate_::core::color::Rgba::new(#rgb 1.0)
            }
        }
        // RRGGBBAA
        8 => {
            let rgba = pair_to_f32(&hex_only);
            quote! {
                #crate_::core::color::Rgba::new(#rgba)
            }
        }
        // RGB
        3 => {
            let rgb = single_to_f32(&hex_only);
            quote! {
                #crate_::core::color::Rgba::new(#rgb 1.0)
            }
        }
        // RGBA
        4 => {
            let rgba = single_to_f32(&hex_only);
            quote! {
                #crate_::core::color::Rgba::new(#rgba)
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
            let lit = LitInt::new(&format!("0x{0}{0}", c), Span::call_site());
            quote! { #lit as f32 / 255.0, }
        })
        .collect()
}
