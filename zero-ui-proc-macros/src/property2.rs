use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, *};

use crate::util::{crate_core, Attributes, Errors};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut errors = Errors::default();

    let uuid = crate::util::uuid(&input);

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

    let priority = if errors.is_empty() {
        Priority::from_ident(&args.priority, &mut errors)
    } else {
        Priority::Context
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

    if item.sig.inputs.len() < 2 {
        errors.push(
            "property functions must have at least 2 inputs: child: impl UiNode, arg0..",
            item.sig.inputs.span(),
        );
    }
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

    let inputs: Vec<_> = item.sig.inputs.iter().map(|arg| Input::from_arg(arg, &mut errors)).collect();
    if !inputs[0].ty.is_empty() {
        // first param passed Input::from_arg validation, check if is node.
        if !matches!(inputs[0].kind, InputKind::UiNode) {
            errors.push("first input must be `impl UiNode`", inputs[0].ty.span());
        }
    }

    let _output_span = match &item.sig.output {
        ReturnType::Default => {
            errors.push("output type must be `impl UiNode`", item.sig.ident.span());
            proc_macro2::Span::call_site()
        }
        ReturnType::Type(_, ty) => ty.span(),
    };

    // validate generics
    // validate input types, generate args

    let extra = if errors.is_empty() {
        // generate items if all is valid.

        let core = crate_core();
        let cfg = Attributes::new(item.attrs.clone()).cfg;
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let generics = &item.sig.generics;
        let args_ident = ident!("{ident}_Args");
        let macro_ident = ident!("{ident}_code_gen_{uuid}");
        let (impl_gens, ty_gens, where_gens) = generics.split_for_impl();
        
        let (default, macro_default, default_fn) = if let Some(dft) = args.default {
            let args = dft.args;
            (
                quote! {
                    pub fn __default__(__instance__: #core::property::PropertyInstInfo) -> std::boxed::Box<dyn #core::property::PropertyArgs> {
                        Self::__new__(__instance__, #args)
                    }
                },
                quote! {
                    (if default {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if !default {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                },
                quote! {
                    Some(Self::__default__)
                }
            )
        } else {
            (
                quote!(),
                quote! {
                    (if !default {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if default {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                },
                quote! {
                    None
                },
            )
        };
        let mut input_info = quote!();
        let mut get_var = quote!();
        let mut get_value = quote!();
        let mut get_takeout = quote!();

        let mut instantiate = quote!();
        let mut input_idents = vec![];
        let mut input_tys = vec![];
        let mut storage_tys = vec![];
        let mut input_to_storage = vec![];
        let mut macro_inputs = quote!();
        let mut macro_input_index = quote!();
        let mut macro_get_var = quote!();
        let mut macro_set_var = quote!();
        let mut named_into_var = quote!();
        let mut get_into_var = quote!();

        for (i, input) in inputs[1..].iter().enumerate() {
            let ident = &input.ident;
            let input_ty = &input.ty;
            input_idents.push(ident);
            input_tys.push(input_ty);
            storage_tys.push(&input.storage_ty);

            let kind = input.kind;
            let info_ty = &input.info_ty;
            input_info.extend(quote! {
                #core::property::PropertyInput {
                    name: stringify!(#ident),
                    kind: #kind,
                    ty: std::any::TypeId::of::<#info_ty>(),
                    ty_name: std::any::type_name::<#info_ty>(),
                },
            });

            macro_inputs.extend(quote! {
                (if input(#ident) {
                    $($tt:tt)*
                }) => {
                    $($tt)*
                };
                (if !input(#ident) {
                    $($tt:tt)*
                }) => {
                    // ignore
                };
            });

            macro_input_index.extend(quote! {
                (input_index(#ident)) => {
                    #i
                };
            });

            if matches!(kind, InputKind::Var | InputKind::Value) {
                macro_get_var.extend(quote! {
                    (if get_var(#ident) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if !get_var(#ident) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                });
            } else {
                macro_get_var.extend(quote! {
                    (if !get_var(#ident) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if get_var(#ident) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                });
            }

            if matches!(kind, InputKind::Var) {
                macro_set_var.extend(quote! {
                    (if set_var(boo) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if !set_var(boo) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                });
            } else {
                macro_set_var.extend(quote! {
                    (if !set_var(boo) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if set_var(boo) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                });
            }
            

            match kind {
                InputKind::Var => {
                    input_to_storage.push(quote! {
                        Self::#ident(#ident)
                    });
                    get_var.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    named_into_var.extend(quote! {
                        pub fn #ident(#ident: #input_ty) -> #core::var::BoxedVar<#info_ty> {
                            #core::var::Var::boxed(#core::var::IntoVar::into_var(#ident))
                        }
                    });
                    let get_ident = ident!("__{ident}_var__");
                    get_into_var.extend(quote! {
                        pub fn #get_ident(args: &dyn #core::property::PropertyArgs) -> #core::var::BoxedVar<#info_ty> {
                            #core::property::read_var(args, #i)
                        }
                    })
                },
                InputKind::Value => {
                    input_to_storage.push(quote! {
                        std::convert::Into::into(#ident)
                    });
                    get_value.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    let get_ident = ident!("__{ident}_var__");
                    get_into_var.extend(quote! {
                        pub fn #get_ident(args: &dyn #core::property::PropertyArgs) -> #core::var::BoxedVar<#info_ty> {
                            #core::property::read_value(args, #i)
                        }
                    })
                },
                InputKind::UiNode => {
                    input_to_storage.push(quote! {
                        #core::property::InputTakeout::new_ui_node(#ident)
                    });
                    get_takeout.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_ui_node(),
                    });
                },
                InputKind::Widget => {
                    input_to_storage.push(quote! {
                        #core::property::InputTakeout::new_widget(#ident)
                    });
                    get_takeout.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_widget(),
                    });
                },
                InputKind::WidgetHandler => {
                    input_to_storage.push(quote! {
                        #core::property::InputTakeout::new_widget_handler(#ident)
                    });
                    get_takeout.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_widget_handler(),
                    });
                },
                InputKind::UiNodeList => {
                    input_to_storage.push(quote! {
                        #core::property::InputTakeout::new_ui_node_list(#ident)
                    });
                    get_takeout.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_ui_node_list(),
                    });
                },
                InputKind::WidgetList => {
                    input_to_storage.push(quote! {
                        #core::property::InputTakeout::new_widget_list(#ident)
                    });
                    get_takeout.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_widget_list(),
                    });
                },                
            }
        }

        if !get_var.is_empty() {
            get_var = quote! {
                fn var(&self, __index__: usize) -> &dyn #core::var::AnyVar {
                    match __index__ {
                        #get_var
                        n => #core::property::panic_input(&self.property(), n, #core::property::InputKind::Var),
                    }
                }
            }
        }
        if !get_value.is_empty() {
            get_value = quote! {
                fn value(&self, __index__: usize) -> &dyn #core::var::AnyVarValue {
                    match __index__ {
                        #get_value
                        n => #core::property::panic_input(&self.property(), n, #core::property::InputKind::Value),
                    }
                }
            }
        }
        if !get_takeout.is_empty() {
            get_takeout = quote! {
                fn takeout(&self, __index__: usize) -> &#core::property::InputTakeout {
                    match __index__ {
                        #get_takeout
                        n => #core::property::panic_input(&self.property(), n, #core::property::InputKind::Takeout),
                    }
                }
            }
        }

        let mut sorted_inputs: Vec<_> = inputs[1..].iter().map(|i| &i.ident).collect();
        sorted_inputs.sort();

        let macro_generics = if generics.params.empty_or_trailing() {
            quote! {
                (if !generics {
                    $($tt:tt)*
                }) => {
                    $($tt)*
                };
                (if generics {
                    $($tt:tt)*
                }) => {
                    // ignore
                };
            }
        } else {
            quote! {
                (if generics {
                    $($tt:tt)*
                }) => {
                    $($tt)*
                };
                (if !generics {
                    $($tt:tt)*
                }) => {
                    // ignore
                };
            }
        };

        quote! {
            #cfg
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #args_ident #generics {
                __instance__: #core::property::PropertyInstInfo,
                #(#input_idents: #storage_tys),*
            }
            #cfg
            impl #impl_gens #args_ident #ty_gens #where_gens {
                pub fn __new__(
                    __instance__: #core::property::PropertyInstInfo, 
                    #(#input_idents: #input_tys),*
                ) -> std::boxed::Box<dyn #core::property::PropertyArgs> {
                    Box::new(Self {
                        __instance__,
                        #(#input_idents: #input_to_storage),*
                    })
                }

                pub fn __id__(name: &'static str) -> #core::property::PropertyId {
                    #core::property::PropertyId {
                        unique_id: TypeId::of::<Self>(),
                        name,
                    }
                }

                #default

                #named_into_var
                #get_into_var
            }
            #cfg
            impl #impl_gens #core::property::PropertyArgs for #args_ident #ty_gens #where_gens {
                fn property(&self) -> #core::property::PropertyInfo {
                    #core::property::PropertyInfo {
                        priority: #priority,
                        unique_id: std::any::TypeId::of::<Self>(),
                        name: std::stringify!(#ident),
                        location: #core::property::source_location!(),
                        default: #default_fn,
                        inputs: std::boxed::Box::new([
                            #input_info
                        ]),
                    }
                }

                fn instance(&self) -> #core::property::PropertyInstInfo {
                    std::clone::Clone::clone(&self.__instance__)
                }

                fn instantiate(&self, __child__: #core::BoxedUiNode) -> #core::BoxedUiNode {
                    #core::UiNode::boxed(#ident(__child__, #instantiate))
                }

                #get_var
                #get_value
                #get_takeout
            }

            #cfg
            #[doc(hidden)]
            #[macro_export]
            macro_rules! #macro_ident {
                #macro_default
                #macro_generics

                #macro_inputs
                (if input($other:ident) {
                    $($tt:tt)*
                }) => {
                    // ignore
                };
                (if !input($other:ident) {
                    $($tt:tt)*
                }) => {
                    $($tt:tt)*
                };

                #macro_input_index
                #macro_get_var
                #macro_set_var
                
                (<$Args:ty>::__new__($__instance__:expr, #($#sorted_inputs:ident),*)) => {
                    $Args::__new__(__instance__, #($#input_idents),*)
                };
            }
            #cfg
            #[doc(hidden)]
            pub use #macro_ident;

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
    args: Punctuated<Expr, Token![,]>,
}
impl Parse for Default {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let _: Token![,] = input.parse()?;
        let _: Token![default] = input.parse()?;
        let inner;
        parenthesized!(inner in input);
        Ok(Default {
            args: Punctuated::parse_terminated(&inner)?,
        })
    }
}

#[derive(Debug)]
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
            _ => {
                errors.push("expected property priority", ident.span());
                Priority::Context
            }
        }
    }
}
impl ToTokens for Priority {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let core = crate_core();
        let pri = ident!("{:?}", self);
        tokens.extend(quote! {
            #core::property::Priority::#pri
        })
    }
}

#[derive(Clone, Copy)]
enum InputKind {
    Var,
    Value,
    UiNode,
    Widget,
    WidgetHandler,
    UiNodeList,
    WidgetList,
}
impl ToTokens for InputKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let kin = match self {
            InputKind::Var => ident!("Var"),
            InputKind::Value => ident!("Value"),
            _ => ident!("Takeout"),
        };
        let core = crate_core();
        tokens.extend(quote! {
            #core::property::InputKind::#kin
        });
    }
}

struct Input {
    ident: Ident,
    kind: InputKind,
    ty: TokenStream,
    info_ty: TokenStream,
    storage_ty: TokenStream,
}
impl Input {
    fn from_arg(arg: &FnArg, errors: &mut Errors) -> Input {
        let mut input = Input {
            ident: ident!("__invalid__"),
            kind: InputKind::Value,
            ty: quote!(),
            storage_ty: quote!(),
            info_ty: quote!(),
        };
        match arg {
            FnArg::Receiver(rcv) => {
                errors.push("property functions cannot be methods", rcv.span());
            }
            FnArg::Typed(t) => {
                if !t.attrs.is_empty() {
                    errors.push("property input cannot have attributes", t.attrs[0].span());
                }

                match *t.pat.clone() {
                    Pat::Ident(id) => {
                        input.ident = id.ident;
                    }
                    p => {
                        errors.push("property input can only have a simple ident", p.span());
                    }
                }

                match *t.ty.clone() {
                    Type::ImplTrait(mut it) if it.bounds.len() == 1 => {
                        let bounds = it.bounds.pop().unwrap().into_value();
                        match bounds {
                            TypeParamBound::Trait(tra) if tra.lifetimes.is_none() && tra.paren_token.is_none() => {
                                let core = crate_core();
                                let path = tra.path;
                                let seg = path.segments.last().unwrap();

                                fn ty_from_generic(
                                    input: &mut Input,
                                    errors: &mut Errors,
                                    t: &Type,
                                    kind: InputKind,
                                    args: &PathArguments,
                                ) -> bool {
                                    if let PathArguments::AngleBracketed(it) = args {
                                        if it.args.len() == 1 {
                                            input.kind = kind;
                                            input.ty = t.to_token_stream();
                                            input.info_ty = it.args.last().unwrap().to_token_stream();
                                            return true;
                                        }
                                    }
                                    errors.push("expected single generic param", args.span());
                                    false
                                }

                                match seg.ident.to_string().as_str() {
                                    "IntoVar" if !seg.arguments.is_empty() => {
                                        if ty_from_generic(&mut input, errors, &t.ty, InputKind::Var, &seg.arguments) {
                                            let t = &input.info_ty;
                                            input.storage_ty = quote!(#core::var::BoxedVar<#t>);
                                        }
                                    }
                                    "IntoValue" if !seg.arguments.is_empty() => {
                                        if ty_from_generic(&mut input, errors, &t.ty, InputKind::Value, &seg.arguments) {
                                            input.storage_ty = input.info_ty.clone();
                                        }
                                    }
                                    "WidgetHandler" if !seg.arguments.is_empty() => {
                                        if ty_from_generic(&mut input, errors, &t.ty, InputKind::WidgetHandler, &seg.arguments) {
                                            input.storage_ty = quote!(#core::property::InputTakeout);
                                        }
                                    }
                                    "UiNode" => {
                                        input.kind = InputKind::UiNode;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::BoxedUiNode);
                                        input.storage_ty = quote!(#core::property::InputTakeout);
                                    }
                                    "Widget" => {
                                        input.kind = InputKind::Widget;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::BoxedWidget);
                                        input.storage_ty = quote!(#core::property::InputTakeout)
                                    }
                                    "UiNodeList" => {
                                        input.kind = InputKind::UiNodeList;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::BoxedUiNodeList);
                                        input.storage_ty = quote!(#core::property::InputTakeout)
                                    }
                                    "WidgetList" => {
                                        input.kind = InputKind::WidgetList;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::BoxedWidgetList);
                                        input.storage_ty = quote!(#core::property::InputTakeout)
                                    }
                                    _ => {
                                        errors.push("property input can only have impl types for: IntoVar<T>, IntoValue<T>, UiNode, Widget, WidgetHandler<A>, UiNodeList, WidgetList", seg.span());
                                    }
                                }
                            }
                            t => {
                                errors.push("property input can only have `impl OneTrait` types", t.span());
                            }
                        }
                    }
                    t => {
                        errors.push("property input can only have `impl OneTrait` types", t.span());
                    }
                }
            }
        }
        input
    }
}
