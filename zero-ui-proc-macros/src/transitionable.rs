use syn::{spanned::Spanned, *};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let mut errors = crate::util::Errors::default();
    let mut lerp_block = quote!();

    let crate_ = crate::util::crate_core();
    let path = quote!(#crate_::var::animation::Transitionable);

    // params: self, to, step
    match input.data {
        Data::Struct(s) => match s.fields {
            Fields::Named(f) => {
                let f = f.named.iter().map(|f| {
                    let ident = &f.ident;
                    quote_spanned! {f.span()=>
                        #ident: #path::lerp(self.#ident, &to.#ident, step)
                    }
                });
                lerp_block = quote! {
                    {
                        Self {
                            #(#f,)*
                        }
                    }
                }
            }
            Fields::Unnamed(f) => {
                let f = f.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! {f.span()=>
                        #path::lerp(self.#index, &to.#index, step)
                    }
                });
                lerp_block = quote! {
                    {
                        Self(#(#f),*)
                    }
                }
            }
            Fields::Unit => {
                lerp_block = quote! {
                    {
                        let _ = (to, step);
                        self
                    }
                }
            }
        },
        Data::Enum(e) => {
            errors.push("cannot derive `Transitionable` for enums", e.enum_token.span);
        }
        Data::Union(u) => {
            errors.push("cannot derive `Transitionable` for unions", u.union_token.span);
        }
    }

    let out = if errors.is_empty() {
        let ident = input.ident;

        let mut generics = input.generics;
        for param in &mut generics.params {
            if let GenericParam::Type(ref mut type_param) = *param {
                type_param.bounds.push(parse_quote!(#path));
            }
        }
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        quote! {
            impl #impl_generics #path for #ident #ty_generics #where_clause {
                fn lerp(self, to: &Self, step: #crate_::var::animation::easing::EasingStep) -> Self #lerp_block
            }
        }
    } else {
        quote! {
            #errors
        }
    };

    out.into()
}
