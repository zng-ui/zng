use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, *};

use crate::util::{Attributes, Errors, crate_core};

fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut errors = Errors::default();

    let args = match parse::<Args>(args) {
        Ok(a) => a,
        Err(e) => {
            errors.push_syn(e);
            Args {
                priority: ident!("context"),
                default: None,
            }
        }
    };

    let args_valid = errors.is_empty();
    let priority = if args_valid {
        Priority::from_ident(&args.priority, &mut errors)
    } else {
        Priority::Context
    };
    let args_valid = errors.is_empty();

    let item = match parse::<ItemFn>(input.clone()) {
        Ok(i) => i,
        Err(e) => {
            errors.push_syn(e);
            let input = TokenStream::from(input);
            let r = quote! {
                #input
                #errors
            };
            return r.into();
        }
    };

    if let Some(async_) = &item.sig.asyncness {
        errors.push("property functions cannot be `async`", async_.span());
    }
    if let Some(unsafe_) = &item.sig.unsafety {
        errors.push("property functions cannot be `unsafe`", unsafe_.span());
    }
    if let Some(abi) = &item.sig.abi {
        errors.push("property functions cannot be `extern`", abi.span());
    }
    if let Some(lifetime) = item.sig.generics.lifetimes().next() {
        errors.push("property functions cannot declare lifetimes", lifetime.span());
    }
    if let Some(const_) = item.sig.generics.const_params().next() {
        errors.push("property functions do not support `const` generics", const_.span());
    }

    if args_valid {
        todo!("validate signature for priority");
    }

    let extra = if errors.is_empty() {
        // generate items if all is valid.

        let core = crate_core();
        let core = quote!(#core::property);
        let cfg = Attributes::new(item.attrs.clone()).cfg;
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let generics = &item.sig.generics;
        let args_ident = ident!("{ident}_Args");
        let macro_ident = ident!("{ident}_code_gen_hash");
        let (impl_gens, ty_gens, where_gens) = generics.split_for_impl();

        let default = quote!();
        let property_info = quote!();
        let instantiate = quote!();
        let get_var = quote!();
        let get_value = quote!();
        let get_takeout = quote!();

        quote! {
            #cfg
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #args_ident #generics {
                __instance__: #core::PropertyInstInfo,

            }
            #cfg
            impl #impl_gens #args_ident #ty_gens #where_gens {
                pub fn __new__(__instance__: #core::PropertyInstInfo, ) -> Box<dyn #core::PropertyArgs> {
                    Box::new(Self {
                        __instance__,

                    })
                }

                #default
            }
            #cfg
            impl #impl_gens #core::PropertyArgs for #args_ident #ty_gens #where_gens {
                fn property(&self) -> #core::PropertyInfo {
                    #property_info
                }

                fn instance(&self) -> #core::PropertyInstInfo {
                    self.__instance__.clone()
                }
        
                fn instantiate(&self, child: #core::BoxedUiNode) -> #core::BoxedUiNode {
                    #instantiate
                }

                #get_var
                #get_value
                #get_takeout
            }

            #cfg
            #[doc(hidden)]
            #vis mod #ident {
                pub use super::{#macro_ident as code_gen, #args_ident as Args};
            }
        }
    } else {
        quote!()
    };

    let r = quote! {
        #item
        #extra
        #errors
    };
    r.into()
}

struct Args {
    priority: Ident,
    default: Option<Default>,
}
impl Parse for Args {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Args {
            priority: input.parse()?,
            default: if input.peek(Token![,]) && input.peek2(Token![default]) {
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}

struct Default {
    comma: Token![,],
    default: Token![default],
    paren: token::Paren,
    args: Punctuated<Expr, Token![,]>,
}
impl Parse for Default {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let comma = input.parse()?;
        let default = input.parse()?;
        let inner;
        let paren = parenthesized!(inner in input);
        Ok(Default {
            comma,
            default,
            paren,
            args: Punctuated::parse_terminated(&inner)?,
        })
    }
}

enum Priority {
    Context,
    Event,
    Layout,
    Size,
    Border,
    Fill,
    ChildContext,
    ChildLayout,
}
impl Priority {
    fn from_ident(ident: &Ident, errors: &mut Errors) -> Priority {
        match ident.to_string().as_str() {
            "context" => Priority::Context,
            "event" => Priority::Event,
            "layout" => Priority::Layout,
            "size" => Priority::Size,
            "border" => Priority::Border,
            "fill" => Priority::Fill,
            "child_context" => Priority::ChildContext,
            "child_layout" => Priority::ChildLayout,
            err => {
                errors.push("expected property priority", ident.span());
                Priority::Context
            }
        }
    }
}
