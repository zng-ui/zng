use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{Ident, Token, parse::Parse, punctuated::Punctuated, spanned::Spanned};

use crate::util::{Attributes, Errors};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(args as Args);
    let input_parse = input.clone();
    let item = syn::parse_macro_input!(input_parse as syn::Item);

    let mut errors = Errors::default();
    let (mut out, self_attrs, self_ident) = match item {
        syn::Item::Enum(enum_decl) => expand_enum(&args, &mut errors, enum_decl),
        syn::Item::Impl(impl_block) => expand_impl(&args, &mut errors, impl_block),
        _ => {
            errors.push("only `enum` declaration and `impl` blocks are supported", Span::call_site());
            (quote!(), Attributes::new(vec![]), None)
        }
    };

    if let Some(self_ident) = self_ident
        && args.generate_impl()
    {
        let crate_core = crate::util::crate_core();
        let proxy_ident = args.proxy_ident(&self_ident);
        out.extend(quote! {
            #self_attrs
            impl #crate_core::PropertyValue for #self_ident {
                type AssocItems = #proxy_ident;
                fn assoc_items(&self) -> Self::AssocItems {
                    #proxy_ident
                }
            }
        })
    }

    out.extend(errors.to_token_stream());
    let mut input = input;
    input.extend(proc_macro::TokenStream::from(out));
    input
}

fn expand_enum(args: &Args, errors: &mut Errors, enum_decl: syn::ItemEnum) -> (TokenStream, Attributes, Option<Ident>) {
    if !enum_decl.generics.params.is_empty() {
        errors.push("generics are not supported", enum_decl.generics.params.span());
        return (quote!(), Attributes::new(vec![]), None);
    }
    if !matches!(enum_decl.vis, syn::Visibility::Public(_)) {
        errors.push("only public items are supported", enum_decl.vis.span());
        return (quote!(), Attributes::new(vec![]), None);
    }

    let self_ident = enum_decl.ident;
    let proxy_ident = args.proxy_ident(&self_ident);

    let mut variants = quote!();
    for variant in enum_decl.variants {
        let var_ident = variant.ident;
        let call = match variant.fields {
            syn::Fields::Named(_) => continue,
            syn::Fields::Unnamed(f) => {
                let arg_idents: Vec<_> = (0..f.unnamed.len()).map(|i| ident!("f{i}")).collect();
                let arg_tys = f.unnamed.iter().map(|f| &f.ty);
                quote! {
                    (&self, #(#arg_idents: #arg_tys),*) -> #self_ident {
                        #self_ident::#var_ident(#(#arg_idents),*)
                    }
                }
            }
            syn::Fields::Unit => quote! {
                (&self) -> #self_ident { #self_ident::#var_ident }
            },
        };

        let mut attrs = Attributes::new(variant.attrs);
        let is_default = attrs.others.iter().any(|a| {
            if let syn::Meta::Path(p) = &a.meta
                && let Some(ident) = p.get_ident()
            {
                ident == "default"
            } else {
                false
            }
        });
        attrs.others.clear();

        variants.extend(quote! {
            #attrs
            #[inline(always)]
            pub fn #var_ident #call
        });

        if is_default {
            variants.extend(quote! {
                #attrs
                #[inline(always)]
                pub fn default #call
            })
        }
    }

    let mut item_attrs = Attributes::new(enum_decl.attrs);
    item_attrs.others.clear();
    item_attrs.docs.clear();

    let deref_impl = if let Some(deref_ident) = args.deref_ident(&self_ident) {
        quote! {
            #item_attrs
            impl std::ops::Deref for #proxy_ident {
                type Target = #deref_ident;

                fn deref(&self) -> &Self::Target {
                    &#deref_ident
                }
            }
        }
    } else {
        quote!()
    };

    let r = quote! {
        #[doc(hidden)]
        #item_attrs
        #[allow(warnings, unused)]
        pub struct #proxy_ident;
        #item_attrs
        #[allow(warnings, unused)]
        impl #proxy_ident {
            #variants
        }
        #deref_impl
    };

    (r, item_attrs, Some(self_ident))
}

fn expand_impl(args: &Args, errors: &mut Errors, impl_block: syn::ItemImpl) -> (TokenStream, Attributes, Option<Ident>) {
    if !impl_block.generics.params.is_empty() {
        errors.push("generics are not supported", impl_block.generics.params.span());
        return (quote!(), Attributes::new(vec![]), None);
    }

    let self_ident = if let syn::Type::Path(p) = &*impl_block.self_ty
        && let Some(id) = p.path.get_ident()
    {
        id.clone()
    } else {
        errors.push("only simple type idents are supported", impl_block.self_ty.span());
        return (quote!(), Attributes::new(vec![]), None);
    };
    let ty_is_self = |t: &syn::Type| {
        if let syn::Type::Path(p) = t
            && let Some(id) = p.path.get_ident()
        {
            &self_ident == id || id == "Self"
        } else {
            false
        }
    };

    let mut is_trait = false;
    let call_path = if let Some((not, trait_, _)) = impl_block.trait_ {
        if let Some(not) = not {
            errors.push("negative impls are not supported", not.span());
            return (quote!(), Attributes::new(vec![]), None);
        }
        is_trait = true;

        quote! {
            <#self_ident as #trait_>
        }
    } else {
        quote! {
            #self_ident
        }
    };

    let proxy_ident = args.proxy_ident(&self_ident);

    let mut assoc_items = quote!();
    for item in impl_block.items {
        match item {
            syn::ImplItem::Const(c) if !is_trait => {
                if !matches!(&c.vis, syn::Visibility::Public(_)) || !ty_is_self(&c.ty) {
                    continue;
                }

                let const_ident = c.ident;
                let mut attrs = Attributes::new(c.attrs);
                attrs.others.clear();

                assoc_items.extend(quote! {
                    #attrs
                    #[inline(always)]
                    pub fn #const_ident(&self) -> #self_ident {
                        #call_path::#const_ident
                    }
                });
            }
            syn::ImplItem::Fn(f) => {
                if !if is_trait {
                    matches!(&f.vis, syn::Visibility::Inherited)
                } else {
                    matches!(&f.vis, syn::Visibility::Public(_))
                } || f.sig.asyncness.is_some()
                    || f.sig.unsafety.is_some()
                    || f.sig.variadic.is_some()
                    || f.sig.inputs.first().map(|a| matches!(a, syn::FnArg::Receiver(_))).unwrap_or(false)
                    || match &f.sig.output {
                        syn::ReturnType::Default => false,
                        syn::ReturnType::Type(_, t) => !ty_is_self(t),
                    }
                {
                    continue;
                }

                let mut attrs = Attributes::new(f.attrs);
                attrs.others.clear();
                let fn_ident = f.sig.ident;
                let fn_gen_lt = f.sig.generics.lt_token;
                let fn_gen_params = f.sig.generics.params;
                let fn_gen_gt = f.sig.generics.gt_token;
                let fn_gen_where = f.sig.generics.where_clause;

                let mut arg_attrs = Vec::with_capacity(f.sig.inputs.len());
                let mut arg_idents = Vec::with_capacity(f.sig.inputs.len());
                let mut arg_tys = Vec::with_capacity(f.sig.inputs.len());
                for (i, a) in f.sig.inputs.iter().enumerate() {
                    if let syn::FnArg::Typed(p) = a {
                        arg_tys.push(&p.ty);
                        if let syn::Pat::Ident(id) = &*p.pat {
                            arg_idents.push(id.ident.clone());
                        } else {
                            arg_idents.push(ident!("__arg{i}"));
                        }
                        let mut attrs = Attributes::new(p.attrs.clone());
                        attrs.others.clear();
                        attrs.docs.clear();
                        arg_attrs.push(attrs);
                    } else {
                        unreachable!()
                    }
                }

                assoc_items.extend(quote! {
                    #attrs
                    #[inline(always)]
                    pub fn #fn_ident #fn_gen_lt #fn_gen_params #fn_gen_gt (&self, #(#arg_attrs #arg_idents: #arg_tys),*) #fn_gen_where -> #self_ident {
                        #call_path::#fn_ident(#(#arg_idents),*)
                    }
                })
            }
            _ => {}
        }
    }

    let mut item_attrs = Attributes::new(impl_block.attrs);
    item_attrs.others.clear();
    item_attrs.docs.clear();

    let deref_impl = if let Some(deref_ident) = args.deref_ident(&self_ident) {
        quote! {
            #item_attrs
            impl std::ops::Deref for #proxy_ident {
                type Target = #deref_ident;

                fn deref(&self) -> &Self::Target {
                    &#deref_ident
                }
            }
        }
    } else {
        quote!()
    };

    let r = quote! {
        #[doc(hidden)]
        #item_attrs
        #[allow(warnings, unused)]
        pub struct #proxy_ident;
        #item_attrs
        #[allow(warnings, unused)]
        impl #proxy_ident {
            #assoc_items
        }
        #deref_impl
    };

    (r, item_attrs, Some(self_ident))
}

struct Args {
    prefix: Option<Token![:]>,
    proxies: Punctuated<Ident, Token![:]>,
}
impl Args {
    pub fn generate_impl(&self) -> bool {
        self.prefix.is_some() || self.proxies.is_empty()
    }

    pub fn proxy_ident(&self, type_ident: &Ident) -> Ident {
        if let Some(p) = self.proxies.first()
            && self.prefix.is_none()
        {
            ident_spanned!(p.span()=> "{type_ident}_AssocItemsProxy_{p}")
        } else {
            ident!("{type_ident}_AssocItemsProxy")
        }
    }

    pub fn deref_ident(&self, type_ident: &Ident) -> Option<Ident> {
        self.proxies
            .iter()
            .nth(if self.prefix.is_some() { 0 } else { 1 })
            .map(|p| ident!("{type_ident}_AssocItemsProxy_{p}"))
    }
}
impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            prefix: input.parse()?,
            proxies: Punctuated::parse_terminated(input)?,
        })
    }
}
