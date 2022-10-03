use proc_macro2::{TokenStream, TokenTree};
use syn::{ext::IdentExt, parse::*, punctuated::Punctuated, spanned::Spanned, *};

use crate::util::*;

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input {
        mut errors,
        self_inits,
        items,
    } = parse_macro_input!(input as Input);
    let crate_ = crate_core();

    let mut delegate = ident!("none");
    let mut impl_generics = TokenStream::new();
    let mut node_generics = TokenStream::new();
    let mut node_fields = TokenStream::new();
    let mut node_inst = TokenStream::new();

    let mut var_generics = TokenStream::new();
    let mut var_fields = TokenStream::new();
    let mut var_inst = TokenStream::new();

    let mut event_generics = TokenStream::new();
    let mut event_fields = TokenStream::new();
    let mut event_inst = TokenStream::new();

    for SelfInit {
        attrs,
        path,
        ty,
        instantiate,
        ..
    } in self_inits
    {
        let attrs = Attributes::new(attrs);
        let cfg = attrs.cfg;
        let docs = attrs.docs;
        let mut inst_attrs = attrs.lints;
        inst_attrs.extend(attrs.others);

        match path.len() {
            3 => {
                let kind = &path[1];
                let name = &path[2];

                let (kind_generics, kind_fields, kind_inst, kind_t) = if kind == &ident!("var") {
                    (&mut var_generics, &mut var_fields, &mut var_inst, "V")
                } else if kind == &ident!("event") {
                    (&mut event_generics, &mut event_fields, &mut event_inst, "E")
                } else {
                    errors.push("expected `var` or `event`", kind.span());
                    continue;
                };

                match ty {
                    SelfInitType::Generic(t) => {
                        let t_ident = ident!("{kind_t}_{name}");

                        kind_fields.extend(quote! {
                            #cfg
                            #name: #t_ident,
                        });

                        let bounds = t.bounds;
                        impl_generics.extend(quote! {
                            #cfg
                            #t_ident: #bounds,
                        });
                        node_generics.extend(quote! {
                            #cfg
                            #t_ident,
                        });
                        kind_generics.extend(quote! {
                            #cfg
                            #t_ident,
                        });
                    }
                    SelfInitType::Path(t) => {
                        kind_fields.extend(quote! {
                            #cfg
                            #(#docs)*
                            #name: #t,
                        });
                    }
                }

                kind_inst.extend(quote! {
                    #cfg
                    #(#inst_attrs)*
                    #name: #instantiate,
                });
            }
            2 => {
                let custom = &path[1];

                if custom == &ident!("child") || custom == &ident!("children") {
                    delegate = custom.clone();
                }

                match ty {
                    SelfInitType::Generic(t) => {
                        let t_ident = ident!("T_{custom}");
                        node_fields.extend(quote! {
                            #cfg
                            #custom: #t_ident,
                        });

                        let bounds = t.bounds;
                        impl_generics.extend(quote! {
                            #cfg
                            #t_ident: #bounds,
                        });

                        node_generics.extend(quote! {
                            #cfg
                            #t_ident,
                        });
                    }
                    SelfInitType::Path(t) => {
                        node_fields.extend(quote! {
                            #cfg
                            #(#docs)*
                            #custom: #t,
                        });
                    }
                }

                node_inst.extend(quote! {
                    #cfg
                    #(#inst_attrs)*
                    #custom: #instantiate,
                });
            }
            _ => {
                errors.push("expected `self.var.ident`, `self.event.ident` or `self.ident: ty`", path.span());
            }
        }
    }

    let (var_struct, var_field, var_inst) = if !var_fields.is_empty() {
        (
            quote! {
                struct NodeVars<#var_generics> {
                    #var_fields
                }
            },
            quote! {
                var: NodeVars<#var_generics>,
            },
            quote! {
                var: NodeVars {
                    #var_inst
                },
            },
        )
    } else {
        (TokenStream::new(), TokenStream::new(), TokenStream::new())
    };

    let (event_struct, event_field, event_inst) = if !event_fields.is_empty() {
        (
            quote! {
                struct NodeEvents<#event_generics> {
                    #event_fields
                }
            },
            quote! {
                event: NodeEvents<#event_generics>,
            },
            quote! {
                event: NodeEvents {
                    #event_inst
                },
            },
        )
    } else {
        (TokenStream::new(), TokenStream::new(), TokenStream::new())
    };

    let r = quote! {
        #errors

        #var_struct

        #event_struct

        struct Node<#node_generics> {
            #node_fields
            #var_field
            #event_field
        }


        #[#crate_::impl_ui_node(#delegate)]
        impl<#impl_generics> Node<#node_generics> {
            #(#items)*
        }

        Node {
            #node_inst
            #var_inst
            #event_inst
        }
    };

    r.into()
}

struct Input {
    pub errors: Errors,
    pub self_inits: Vec<SelfInit>,
    pub items: Vec<ItemFn>,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut self_inits = vec![];
        let mut items = vec![];
        let mut errors = Errors::default();

        let skip_error = || {
            while !input.is_empty() && !input.peek(Token![self]) && !input.peek(Token![#]) && !input.peek(Token![fn]) {
                let _ = input.parse::<TokenTree>().unwrap();
            }
        };

        while !input.is_empty() {
            let attrs = match Attribute::parse_outer(input) {
                Ok(a) => a,
                Err(e) => {
                    errors.push_syn(e);
                    skip_error();
                    continue;
                }
            };

            if input.peek(Token![self]) {
                let mut init = match SelfInit::parse(input) {
                    Ok(s) => s,
                    Err(e) => {
                        errors.push_syn(e);
                        skip_error();
                        continue;
                    }
                };
                init.attrs = attrs;
                self_inits.push(init);
            } else {
                let stmt = match Stmt::parse(input) {
                    Ok(s) => s,
                    Err(e) => {
                        errors.push_syn(e);
                        skip_error();
                        continue;
                    }
                };

                match stmt {
                    Stmt::Item(Item::Fn(mut it)) => {
                        it.attrs = attrs;
                        items.push(it);
                    }
                    e => {
                        errors.push("unexpected, only methods and `self.ident: ty = expr;` stmts are allowed", e.span());
                        continue;
                    }
                }
            }
        }

        Ok(Input { errors, self_inits, items })
    }
}

struct SelfInit {
    attrs: Vec<Attribute>,
    path: Punctuated<Ident, Token![.]>,
    _colon: Token![:],
    ty: SelfInitType,
    _eq: Token![=],
    instantiate: Expr,
    _semi: Token![;],
}
impl Parse for SelfInit {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(SelfInit {
            attrs: vec![],
            path: Punctuated::parse_separated_nonempty_with(input, Ident::parse_any)?,
            _colon: input.parse()?,
            ty: input.parse()?,
            _eq: input.parse()?,
            instantiate: input.parse()?,
            _semi: input.parse()?,
        })
    }
}

enum SelfInitType {
    Generic(TypeImplTrait),
    Path(TypePath),
}
impl Parse for SelfInitType {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![impl]) {
            input.parse().map(SelfInitType::Generic)
        } else {
            input.parse().map(SelfInitType::Path)
        }
    }
}
