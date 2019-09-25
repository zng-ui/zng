extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::{TokenStream, TokenTree};
use quote::__rt::Span;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Ident, ImplItem, ItemImpl};

macro_rules! error {
    ($span: expr, $msg: expr) => {{
        let error = quote_spanned! {
            $span=>
            compile_error!(concat!("#[impl_ui] ", $msg));
        };

        return TokenStream::from(error);
    }};
}

fn ui_leaf_defaults(crate_: quote::__rt::TokenStream) -> Vec<ImplItem> {
    let token_stream = TokenStream::from(quote! {
        impl Dummy {
            fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
                let mut size = available_size;

                if size.width.is_infinite() {
                    size.width = 0.0;
                }

                if size.height.is_infinite() {
                    size.height = 0.0;
                }

                size
            }

            fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {}

            fn arrange(&mut self, final_size: LayoutSize) {}

            fn render(&self, f: &mut NextFrame);

            fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {}

            fn focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {}

            fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {}

            fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {}

            fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate) {}

            fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) {}

            fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate) {}

            fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
                None
            }

            fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {}

            fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {}
        }
    });
    let parsed_impl = parse_macro_input!(token_stream as ItemImpl);
    parsed_impl.items
}

fn impl_ui_impl(_args: TokenStream, input: TokenStream, crate_: quote::__rt::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemImpl);

    if let Some((_, trait_, _)) = input.trait_ {
        error!(trait_.span(), "expected type impl found trait")
    }

    let ui_marker = ref_ident("Ui");

    let mut ui_items = vec![];
    let mut other_items = vec![];

    for mut item in input.items {
        let mut is_ui = false;

        if let ImplItem::Method(m) = &mut item {
            if let Some(index) = m.attrs.iter().position(|a| a.path.get_ident() == Some(&ui_marker)) {
                m.attrs.remove(index);
                is_ui = true;
            }
        }

        if is_ui {
            ui_items.push(item);
        } else {
            other_items.push(item);
        }
    }

    let mut ui_default_items = ui_leaf_defaults(crate_);
    ui_default_items.retain(|d| !ui_items.iter().any(|m| m.sig.ident == d.sig.ident));

    let impl_ui = ref_ident("impl_ui");
    let mut impl_attrs = input.attrs;
    impl_attrs.retain(|a| a.path.get_ident() != Some(&impl_ui));

    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;

    let result = quote! {
        #(#impl_attrs)*
        impl #impl_generics #self_ty #ty_generics #where_clause {
            #(#other_items)*
        }

        impl #impl_generics #crate_::ui::Ui for #self_ty #ty_generics #where_clause {
            #(#ui_items)*
            #(#ui_default_items)*
        }
    };

    TokenStream::from(result)
}

#[proc_macro_attribute]
pub fn impl_ui_crate(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_impl(args, input, quote! {crate})
}

#[proc_macro_attribute]
pub fn impl_ui(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_impl(args, input, quote! {zero_ui})
}

fn ref_ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}
