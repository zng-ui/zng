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
        let macro_ident = ident!("{ident}_code_gen_{uuid:x}");
        let (impl_gens, ty_gens, where_gens) = generics.split_for_impl();

        let (default, macro_default, default_fn) = if let Some(dft) = args.default {
            let args = dft.args;
            (
                quote! {
                    pub fn __default__(info: #core::widget_builder::PropertyInstInfo) -> std::boxed::Box<dyn #core::widget_builder::PropertyArgs> {
                        Self::__new__(#args).__build__(info)
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
                },
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
        let mut get_state = quote!();
        let mut get_value = quote!();
        let mut get_ui_node = quote!();
        let mut get_ui_node_list = quote!();
        let mut get_widget_handler = quote!();

        let mut instantiate = quote!();
        let mut input_idents = vec![];
        let mut input_tys = vec![];
        let mut storage_tys = vec![];
        let mut input_to_storage = vec![];
        let mut macro_inputs = quote!();
        let mut macro_input_index = quote!();
        let mut macro_allowed_in_when = quote!();
        let mut macro_allowed_in_when_assign = quote!();
        let mut named_into_var = quote!();
        let mut get_when_input = quote!();
        let mut macro_get_when_input = quote!();

        for (i, input) in inputs[1..].iter().enumerate() {
            let ident = &input.ident;
            let input_ty = &input.ty;
            input_idents.push(ident);
            input_tys.push(input_ty);
            storage_tys.push(&input.storage_ty);

            let kind = input.kind;
            let info_ty = &input.info_ty;
            input_info.extend(quote! {
                #core::widget_builder::PropertyInput {
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
                (if input(#i) {
                    $($tt:tt)*
                }) => {
                    $($tt)*
                };
                (if !input(#i) {
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

            if matches!(kind, InputKind::Var | InputKind::StateVar | InputKind::Value) {
                macro_allowed_in_when.extend(quote! {
                    (if allowed_in_when(#ident) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if !allowed_in_when(#ident) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                    (if allowed_in_when(#i) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if !allowed_in_when(#i) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                });
            } else {
                macro_allowed_in_when.extend(quote! {
                    (if !allowed_in_when(#ident) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if allowed_in_when(#ident) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                    (if !allowed_in_when(#i) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if allowed_in_when(#i) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                });
            }

            if matches!(kind, InputKind::Var) {
                macro_allowed_in_when_assign.extend(quote! {
                    (if allowed_in_when_assign(#ident) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if !allowed_in_when_assign(#ident) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                    (if allowed_in_when_assign(#i) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if !allowed_in_when_assign(#i) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                });
            } else {
                macro_allowed_in_when_assign.extend(quote! {
                    (if !allowed_in_when_assign(#ident) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if allowed_in_when_assign(#ident) {
                        $($tt:tt)*
                    }) => {
                        // ignore
                    };
                    (if !allowed_in_when_assign(#i) {
                        $($tt:tt)*
                    }) => {
                        $($tt)*
                    };
                    (if allowed_in_when_assign(#i) {
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
                            #core::widget_builder::var_input_to_args(#ident)
                        }
                    });
                    let get_ident = ident!("__{ident}_when__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#info_ty>) {
                            #core::widget_builder::WhenInputVar::new::<#info_ty>()
                        }
                    });
                    macro_get_when_input.extend(quote! {
                        ({$($property:tt)*}::when_input(#ident)) => {
                            $($property)* :: #get_ident()
                        };
                        ({$($property:tt)*}::when_input(#i)) => {
                            $($property)* :: #get_ident()
                        };
                    })
                }
                InputKind::StateVar => {
                    input_to_storage.push(quote! {
                        #ident
                    });
                    get_var.extend(quote! {
                        #i => &self.#ident,
                    });
                    get_state.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    let get_ident = ident!("__{ident}_when__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<bool>) {
                            #core::widget_builder::WhenInputVar::new::<bool>()
                        }
                    });
                    macro_get_when_input.extend(quote! {
                        ({$($property:tt)*}::when_input(#ident)) => {
                            $($property)* :: #get_ident()
                        };
                        ({$($property:tt)*}::when_input(#i)) => {
                            $($property)* :: #get_ident()
                        };
                    })
                }
                InputKind::Value => {
                    input_to_storage.push(quote! {
                        #core::widget_builder::value_to_args(#ident)
                    });
                    get_value.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    let get_ident = ident!("__{ident}_when__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#info_ty>) {
                            #core::widget_builder::WhenInputVar::new::<#info_ty>()
                        }
                    })
                }
                InputKind::UiNode => {
                    input_to_storage.push(quote! {
                        #core::widget_builder::ui_node_to_args(#ident)
                    });
                    get_ui_node.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_on_init(),
                    });
                }
                InputKind::UiNodeList => {
                    input_to_storage.push(quote! {
                        #core::widget_builder::ui_node_list_to_args(#ident)
                    });
                    get_ui_node_list.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_on_init(),
                    });
                }
                InputKind::WidgetHandler => {
                    input_to_storage.push(quote! {
                        #core::widget_builder::widget_handler_to_args(#ident)
                    });
                    get_widget_handler.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                }
            }
        }

        if !get_var.is_empty() {
            get_var = quote! {
                fn var(&self, __index__: usize) -> &dyn #core::var::AnyVar {
                    match __index__ {
                        #get_var
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::Var),
                    }
                }
            }
        }
        if !get_state.is_empty() {
            get_state = quote! {
                fn state_var(&self, __index__: usize) -> &#core::var::StateVar {
                    match __index__ {
                        #get_state
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::StateVar),
                    }
                }
            }
        }
        if !get_value.is_empty() {
            get_value = quote! {
                fn value(&self, __index__: usize) -> &dyn #core::var::AnyVarValue {
                    match __index__ {
                        #get_value
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::Value),
                    }
                }
            }
        }
        if !get_ui_node.is_empty() {
            get_ui_node = quote! {
                fn ui_node(&self, __index__: usize) -> &#core::widget_instance::RcNode<#core::widget_instance::BoxedUiNode> {
                    match __index__ {
                        #get_ui_node
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::UiNode),
                    }
                }
            }
        }
        if !get_ui_node_list.is_empty() {
            get_ui_node_list = quote! {
                fn ui_node_list(&self, __index__: usize) -> &#core::widget_instance::RcNodeList<#core::widget_instance::BoxedUiNodeList> {
                    match __index__ {
                        #get_ui_node_list
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::UiNodeList),
                    }
                }
            }
        }
        if !get_widget_handler.is_empty() {
            get_widget_handler = quote! {
                fn widget_handler(&self, __index__: usize) -> &dyn std::any::Any {
                    match __index__ {
                        #get_widget_handler
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::WidgetHandler),
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

        let inputs_len = inputs[1..].len();

        let args_reexport_vis = match vis {
            Visibility::Inherited => quote!(pub(super)),
            vis => vis.to_token_stream(),
        };

        quote! {
            #cfg
            #[doc(hidden)]
            #[derive(std::clone::Clone)]
            #[allow(non_camel_case_types)]
            #vis struct #args_ident #impl_gens #where_gens {
                __instance__: #core::widget_builder::PropertyInstInfo,
                #(#input_idents: #storage_tys),*
            }
            #cfg
            impl #impl_gens #args_ident #ty_gens #where_gens {
                #[allow(clippy::too_many_arguments)]
                pub fn __new__(
                    #(#input_idents: #input_tys),*
                ) -> Self {
                    Self {
                        __instance__: #core::widget_builder::PropertyInstInfo::none(),
                        #(#input_idents: #input_to_storage),*
                    }
                }

                pub fn __build__(mut self, info: #core::widget_builder::PropertyInstInfo) -> std::boxed::Box<dyn #core::widget_builder::PropertyArgs> {
                    self.__instance__ = info;
                    Box::new(self)
                }

                pub fn __id__(name: &'static str) -> #core::widget_builder::PropertyId {
                    #core::widget_builder::PropertyId {
                        unique_id: std::any::TypeId::of::<Self>(),
                        name,
                    }
                }

                #default

                #named_into_var
                #get_when_input
            }
            #cfg
            impl #impl_gens #core::widget_builder::PropertyArgs for #args_ident #ty_gens #where_gens {
                fn clone_boxed(&self) -> std::boxed::Box<dyn #core::widget_builder::PropertyArgs> {
                    Box::new(std::clone::Clone::clone(self))
                }

                fn property(&self) -> #core::widget_builder::PropertyInfo {
                    #core::widget_builder::PropertyInfo {
                        priority: #priority,
                        unique_id: std::any::TypeId::of::<Self>(),
                        name: std::stringify!(#ident),
                        location: #core::widget_builder::source_location!(),
                        default: #default_fn,
                        inputs: std::boxed::Box::new([
                            #input_info
                        ]),
                    }
                }

                fn instance(&self) -> #core::widget_builder::PropertyInstInfo {
                    std::clone::Clone::clone(&self.__instance__)
                }

                fn instantiate(&self, __child__: #core::widget_instance::BoxedUiNode) -> #core::widget_instance::BoxedUiNode {
                    #core::widget_instance::UiNode::boxed(#ident(__child__, #instantiate))
                }

                #get_var
                #get_state
                #get_value
                #get_ui_node
                #get_ui_node_list
                #get_widget_handler
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
                    $($tt)*
                };

                #macro_input_index
                #macro_allowed_in_when
                #macro_allowed_in_when_assign

                (if !inputs_len(#inputs_len) {
                    $($tt:tt)*
                }) => {
                    // ignore
                };
                (if !inputs_len($other:tt) {
                    $($tt:tt)*
                }) => {
                    $($tt)*
                };

                ({$($property:tt)*}::__new__(#($#sorted_inputs:ident),*)) => {
                    $($property)*::__new__(#($#input_idents),*)
                };
                ({$($property:tt)*}::__new__($($sorted_inputs:ident),*)) => {
                    // ignore
                    ((), ())
                };

                #macro_get_when_input
            }

            #cfg
            #[doc(hidden)]
            #vis mod #ident {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                #args_reexport_vis use super::#args_ident as property;
                #args_reexport_vis use super::#ident as export;
                pub use #macro_ident as code_gen;
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
            #core::widget_builder::Priority::#pri
        })
    }
}

#[derive(Clone, Copy)]
enum InputKind {
    Var,
    StateVar,
    Value,
    UiNode,
    WidgetHandler,
    UiNodeList,
}
impl ToTokens for InputKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let kin = match self {
            InputKind::Var => ident!("Var"),
            InputKind::StateVar => ident!("StateVar"),
            InputKind::Value => ident!("Value"),
            InputKind::UiNode => ident!("UiNode"),
            InputKind::WidgetHandler => ident!("WidgetHandler"),
            InputKind::UiNodeList => ident!("UiNodeList"),
        };
        let core = crate_core();
        tokens.extend(quote! {
            #core::widget_builder::InputKind::#kin
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
                let core = crate_core();

                match *t.ty.clone() {
                    Type::ImplTrait(mut it) if it.bounds.len() == 1 => {
                        let bounds = it.bounds.pop().unwrap().into_value();
                        match bounds {
                            TypeParamBound::Trait(tra) if tra.lifetimes.is_none() && tra.paren_token.is_none() => {
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
                                            let t = &input.info_ty;
                                            input.storage_ty = quote!(#core::widget_builder::RcWidgetHandler<#t>);
                                        }
                                    }
                                    "UiNode" => {
                                        input.kind = InputKind::UiNode;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::widget_instance::BoxedUiNode);
                                        input.storage_ty = quote!(#core::widget_instance::RcNode<#core::widget_instance::BoxedUiNode>);
                                    }
                                    "UiNodeList" => {
                                        input.kind = InputKind::UiNodeList;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::widget_instance::BoxedUiNodeList);
                                        input.storage_ty =
                                            quote!(#core::widget_instance::RcNodeList<#core::widget_instance::BoxedUiNodeList>)
                                    }
                                    _ => {
                                        errors.push("property input can only have impl types for: IntoVar<T>, IntoValue<T>, UiNode, WidgetHandler<A>, UiNodeList", seg.span());
                                    }
                                }
                            }
                            t => {
                                errors.push("property input can only have `impl OneTrait`", t.span());
                            }
                        }
                    }
                    Type::Path(t) if t.path.segments.last().map(|s| s.ident == "StateVar").unwrap_or(false) => {
                        input.kind = InputKind::StateVar;
                        input.ty = quote_spanned!(t.span()=> #core::var::StateVar);
                        input.info_ty = quote_spanned!(t.span()=> bool);
                        input.storage_ty = input.ty.clone();
                    }
                    t => {
                        errors.push("property input can only have `impl OneTrait` or `StateVar` types", t.span());
                    }
                }
            }
        }
        input
    }
}
