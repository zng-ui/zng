use proc_macro2::*;
use quote::*;
use syn::{
    parse::{Parse, Result},
    spanned::Spanned as _,
    *,
};

use crate::util::Errors;

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut errors = Errors::default();

    let args = match parse::<Args>(args) {
        Ok(a) => a,
        Err(e) => {
            errors.push_syn(e);
            Args {
                name: LitStr::new("", Span::call_site()),
            }
        }
    };

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
        errors.push("hot node functions cannot be `async`", async_.span());
    }
    if let Some(unsafe_) = &item.sig.unsafety {
        errors.push("hot node functions cannot be `unsafe`", unsafe_.span());
    }
    if let Some(abi) = &item.sig.abi {
        errors.push("hot node functions cannot be `extern`", abi.span());
    }
    if let Some(lifetime) = item.sig.generics.lifetimes().next() {
        errors.push("hot node functions cannot declare lifetimes", lifetime.span());
    }
    if let Some(const_) = item.sig.generics.const_params().next() {
        errors.push("hot node functions do not support `const` generics", const_.span());
    }
    if let Some(ty_) = item.sig.generics.type_params().next() {
        errors.push("hot node functions do not support named generics", ty_.span());
    }

    let ident = &item.sig.ident;
    let slice_ident = ident!("__ZNG_HOT_{}", ident);
    let builder_ident = ident!("__zng_hot_{}", ident);

    let mut name = args.name;
    if name.value().is_empty() {
        name = LitStr::new(&ident.to_string(), ident.span());
    }

    let inputs: Vec<_> = item.sig.inputs.iter().map(|arg| Input::from_arg(arg, &mut errors)).collect();

    let mut unpack_args = quote!();
    for input in &inputs {
        let t = &input.gen_ty;
        match input.kind {
            InputKind::Var => unpack_args.extend(quote! {
                __args__.pop_var::<#t>(),
            }),
            InputKind::Value => unpack_args.extend(quote! {
                __args__.pop_value::<#t>(),
            }),
            InputKind::UiNode => unpack_args.extend(quote! {
                __args__.pop_ui_node(),
            }),
            InputKind::WidgetHandler => unpack_args.extend(quote! {
                __args__.pop_widget_handler::<#t>(),
            }),
            InputKind::UiNodeList => unpack_args.extend(quote! {
                __args__.pop_ui_node_list(),
            }),
            InputKind::TryClone => unpack_args.extend(quote! {
                __args__.pop_clone::<#t>(),
            }),
        }
    }

    let hot_side = quote! {
        #[allow(unexpected_cfgs)]
        #[cfg(zng_hot_build)]
        #[doc(hidden)]
        #[crate::zng_hot_entry::distributed_slice(crate::zng_hot_entry::HOT_NODES)]
        static #slice_ident: crate::zng_hot_entry::HotNodeEntry = (env!("CARGO_MANIFEST_DIR"), #name, #builder_ident);

        #[allow(unexpected_cfgs)]
        #[cfg(zng_hot_build)]
        #[doc(hidden)]
        fn #builder_ident(__args__: crate::zng_hot_entry::HotNodeArgs) -> crate::zng_hot_entry::HotNode {
            #ident(
                #unpack_args
            )
        }

        #[allow(unexpected_cfgs)]
        #[cfg(zng_hot_build)]
        #item
    };

    let mut item = item;
    let mut proxy_item = item.clone();

    let proxy_ident = builder_ident.clone();
    item.vis = Visibility::Inherited;
    item.sig.ident = proxy_ident;
    let input_len = inputs.len();

    let mut pack_args = quote!();
    for input in inputs.iter().rev() {
        let ident = &input.ident;
        let t = &input.gen_ty;
        match input.kind {
            InputKind::Var => {
                pack_args.extend(quote_spanned! {ident.span()=>
                    __args__.push_var::<#t>(#ident);
                });
            }
            InputKind::Value => {
                pack_args.extend(quote_spanned! {ident.span()=>
                    __args__.push_value::<#t>(#ident);
                });
            }
            InputKind::UiNode => {
                pack_args.extend(quote_spanned! {ident.span()=>
                    __args__.push_ui_node(#ident);
                });
            }
            InputKind::WidgetHandler => {
                pack_args.extend(quote_spanned! {ident.span()=>
                    __args__.push_widget_handler::<#t>(#ident);
                });
            }
            InputKind::UiNodeList => {
                pack_args.extend(quote_spanned! {ident.span()=>
                    __args__.push_ui_node_list(#ident);
                });
            }
            InputKind::TryClone => {
                pack_args.extend(quote_spanned! {ident.span()=>
                    __args__.push_clone::<#t>(#ident);
                });
            }
        }
    }

    proxy_item.block = parse_quote! {
        {
            let mut __args__ = crate::zng_hot_entry::HotNodeArgs::with_capacity(#input_len);
            #pack_args

            crate::zng_hot_entry::HotNodeHost::new(env!("CARGO_MANIFEST_DIR"), #name, __args__)
        }
    };

    let host_side = quote! {
        #[allow(unexpected_cfgs)]
        #[cfg(not(zng_hot_build))]
        #item

        #[allow(unexpected_cfgs)]
        #[cfg(not(zng_hot_build))]
        #proxy_item
    };

    let r = quote! {
        #hot_side
        #host_side
        #errors
    };

    r.into()
}

struct Args {
    name: LitStr,
}
impl Parse for Args {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(Args {
                name: LitStr::new("", input.span()),
            });
        }

        Ok(Args { name: input.parse()? })
    }
}

#[derive(Clone, Copy)]
enum InputKind {
    Var,
    Value,
    UiNode,
    WidgetHandler,
    UiNodeList,
    TryClone,
}
struct Input {
    ident: Ident,
    kind: InputKind,
    gen_ty: TokenStream,
}
impl Input {
    fn from_arg(arg: &FnArg, errors: &mut Errors) -> Input {
        let mut input = Input {
            ident: ident!("__invalid__"),
            kind: InputKind::Value,
            gen_ty: quote!(),
        };
        match arg {
            FnArg::Receiver(rcv) => {
                errors.push("methods cannot be hot nodes", rcv.span());
            }
            FnArg::Typed(t) => {
                if !t.attrs.is_empty() {
                    errors.push("hot node input cannot have attributes", t.attrs[0].span());
                }

                match *t.pat.clone() {
                    Pat::Ident(id) => {
                        if id.ident == "self" {
                            errors.push("methods cannot be hot nodes", id.ident.span());
                        }
                        input.ident = id.ident;
                    }
                    p => {
                        errors.push("hot node input can only have a simple ident", p.span());
                    }
                }

                match *t.ty.clone() {
                    Type::ImplTrait(mut it) if it.bounds.len() == 1 => {
                        let bounds = it.bounds.pop().unwrap().into_value();
                        match bounds {
                            TypeParamBound::Trait(tra) if tra.lifetimes.is_none() && tra.paren_token.is_none() => {
                                let path = tra.path;
                                let seg = path.segments.last().unwrap();

                                fn ty_from_generic(input: &mut Input, errors: &mut Errors, kind: InputKind, args: &PathArguments) -> bool {
                                    if let PathArguments::AngleBracketed(it) = args {
                                        if it.args.len() == 1 {
                                            input.kind = kind;
                                            input.gen_ty = it.args.last().unwrap().to_token_stream();
                                            return true;
                                        }
                                    }
                                    errors.push("expected single generic param", args.span());
                                    false
                                }

                                match seg.ident.to_string().as_str() {
                                    "IntoVar" if !seg.arguments.is_empty() => {
                                        ty_from_generic(&mut input, errors, InputKind::Var, &seg.arguments);
                                    }
                                    "IntoValue" if !seg.arguments.is_empty() => {
                                        ty_from_generic(&mut input, errors, InputKind::Value, &seg.arguments);
                                    }
                                    "WidgetHandler" if !seg.arguments.is_empty() => {
                                        ty_from_generic(&mut input, errors, InputKind::WidgetHandler, &seg.arguments);
                                    }
                                    "UiNode" => {
                                        input.kind = InputKind::UiNode;
                                    }
                                    "UiNodeList" => {
                                        input.kind = InputKind::UiNodeList;
                                    }
                                    _ => {
                                        errors.push("hot node input can only have impl types for: IntoVar<T>, IntoValue<T>, UiNode, WidgetHandler<A>, UiNodeList", seg.span());
                                    }
                                }
                            }
                            t => {
                                errors.push("hot node input can only have `impl OneTrait`", t.span());
                            }
                        }
                    }
                    Type::Array(a) => {
                        input.kind = InputKind::TryClone;
                        input.gen_ty = a.to_token_stream();
                    }
                    Type::BareFn(f) => {
                        input.kind = InputKind::TryClone;
                        input.gen_ty = f.to_token_stream();
                    }
                    Type::Path(p) => {
                        input.kind = InputKind::TryClone;
                        input.gen_ty = p.to_token_stream();
                    }
                    Type::Tuple(t) => {
                        input.kind = InputKind::TryClone;
                        input.gen_ty = t.to_token_stream();
                    }
                    t => {
                        errors.push(
                            "hot node input can only have `Clone+Send+Any` types or `impl OneTrait` property types",
                            t.span(),
                        );
                    }
                }
            }
        }
        input
    }
}
