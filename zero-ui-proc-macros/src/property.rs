use std::mem;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, *};

use crate::util::{crate_core, Attributes, Errors};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut errors = Errors::default();

    let args = match parse::<Args>(args) {
        Ok(a) => a,
        Err(e) => {
            errors.push_syn(e);
            Args {
                nest_group: ident!("CONTEXT"),
                capture: false,
                default: None,
            }
        }
    };

    let nest_group = args.nest_group;
    let capture = args.capture;

    let mut item = match parse::<ItemFn>(input.clone()) {
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
    let mut attrs = Attributes::new(mem::take(&mut item.attrs));
    attrs.tag_doc("P", "This function is also a widget property");

    if item.sig.inputs.len() < 2 {
        errors.push(
            "property functions must have at least 2 inputs: child: impl UiNode, arg0..",
            item.sig.inputs.span(),
        );

        if item.sig.inputs.is_empty() {
            // patch to continue validation.
            let core = crate_core();
            item.sig.inputs.push(parse_quote!(__child__: impl #core::widget_instance::UiNode));
        }
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

    let mut item = item;
    if capture {
        let child = &inputs[0].ident;
        let inputs = inputs[1..].iter().map(|i| &i.ident);
        item.block.stmts.clear();
        item.block.stmts.push(parse_quote! {
            let _ = (#(#inputs,)*);
        });
        item.block.stmts.push(Stmt::Expr(parse_quote! {
            #child
        }));
    }
    let item = item;

    let output_span = match &item.sig.output {
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
        let has_generics = !generics.params.empty_or_trailing();
        let (impl_gens, ty_gens, where_gens) = generics.split_for_impl();

        let default;
        let default_fn;
        let args_default = match args.default {
            Some(d) => Some((d.default.span(), d.args.to_token_stream())),
            None => {
                let mut default = quote!();
                for input in &inputs[1..] {
                    match input.kind {
                        InputKind::StateVar => default.extend(quote! {
                            #core::var::state_var(),
                        }),
                        InputKind::UiNode => default.extend(quote! {
                            #core::widget_instance::NilUiNode,
                        }),
                        InputKind::UiNodeList => default.extend(quote! {
                            #core::widget_instance::ui_list![],
                        }),
                        InputKind::WidgetHandler if !has_generics => default.extend(quote! {
                            #core::handler::hn!(|_, _| {}),
                        }),
                        _ => {
                            default = quote!();
                            break;
                        }
                    }
                }

                if !default.is_empty() {
                    Some((Span::call_site(), default))
                } else {
                    None
                }
            }
        };

        if let Some((span, args)) = args_default {
            let new = quote_spanned!(span=> Self::__new__);
            default = quote! {
                pub fn __default__(info: #core::widget_builder::PropertyInstInfo) -> std::boxed::Box<dyn #core::widget_builder::PropertyArgs> {
                    #new(#args).__build__(info)
                }
            };
            default_fn = quote! {
                Some(Self::__default__)
            };
        } else {
            default = quote!();
            default_fn = quote! {
                None
            };
        }
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
        let mut named_into_var = quote!();
        let mut get_when_input = quote!();
        let mut input_new_dyn = vec![];

        let mut allowed_in_when_expr = true;
        let mut allowed_in_when_assign = true;

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
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget_builder::iter_input_build_actions(&mut __actions__, #i);
                        #core::widget_builder::new_dyn_var(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#info_ty>) {
                            #core::widget_builder::WhenInputVar::new::<#info_ty>()
                        }
                        pub fn #get_ident_i()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#info_ty>) {
                            #core::widget_builder::WhenInputVar::new::<#info_ty>()
                        }
                    });
                }
                InputKind::StateVar => {
                    allowed_in_when_assign = false;
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
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget_builder::iter_input_build_actions(&mut __actions__, #i);
                        #core::widget_builder::new_dyn_other(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<bool>) {
                            #core::widget_builder::WhenInputVar::new::<bool>()
                        }
                        pub fn #get_ident_i()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<bool>) {
                            #core::widget_builder::WhenInputVar::new::<bool>()
                        }
                    });
                }
                InputKind::Value => {
                    allowed_in_when_assign = false;
                    input_to_storage.push(quote! {
                        #core::widget_builder::value_to_args(#ident)
                    });
                    get_value.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget_builder::iter_input_build_actions(&mut __actions__, #i);
                        #core::widget_builder::new_dyn_other(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#info_ty>) {
                            #core::widget_builder::WhenInputVar::new::<#info_ty>()
                        }
                        pub fn #get_ident_i()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#info_ty>) {
                            #core::widget_builder::WhenInputVar::new::<#info_ty>()
                        }
                    });
                }
                InputKind::UiNode => {
                    allowed_in_when_expr = false;
                    input_to_storage.push(quote! {
                        #core::widget_builder::ui_node_to_args(#ident)
                    });
                    get_ui_node.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_on_init(),
                    });
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget_builder::iter_input_build_actions(&mut __actions__, #i);
                        #core::widget_builder::new_dyn_ui_node(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#core::widget_builder::UiNodeInWhenExprError>) {
                            #core::widget_builder::WhenInputVar::new::<#core::widget_builder::UiNodeInWhenExprError>()
                        }
                        pub fn #get_ident_i()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#core::widget_builder::UiNodeInWhenExprError>) {
                            #core::widget_builder::WhenInputVar::new::<#core::widget_builder::UiNodeInWhenExprError>()
                        }
                    });
                }
                InputKind::UiNodeList => {
                    allowed_in_when_expr = false;
                    input_to_storage.push(quote! {
                        #core::widget_builder::ui_node_list_to_args(#ident)
                    });
                    get_ui_node_list.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_on_init(),
                    });
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget_builder::iter_input_build_actions(&mut __actions__, #i);
                        #core::widget_builder::new_dyn_ui_node_list(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#core::widget_builder::UiNodeListInWhenExprError>) {
                            #core::widget_builder::WhenInputVar::new::<#core::widget_builder::UiNodeListInWhenExprError>()
                        }
                        pub fn #get_ident_i()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#core::widget_builder::UiNodeListInWhenExprError>) {
                            #core::widget_builder::WhenInputVar::new::<#core::widget_builder::UiNodeListInWhenExprError>()
                        }
                    });
                }
                InputKind::WidgetHandler => {
                    allowed_in_when_expr = false;
                    input_to_storage.push(quote! {
                        #core::widget_builder::widget_handler_to_args(#ident)
                    });
                    get_widget_handler.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget_builder::iter_input_build_actions(&mut __actions__, #i);
                        #core::widget_builder::new_dyn_widget_handler(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#core::widget_builder::WidgetHandlerInWhenExprError>) {
                            #core::widget_builder::WhenInputVar::new::<#core::widget_builder::WidgetHandlerInWhenExprError>()
                        }
                        pub fn #get_ident_i()
                        -> (#core::widget_builder::WhenInputVar, impl #core::var::Var<#core::widget_builder::WidgetHandlerInWhenExprError>) {
                            #core::widget_builder::WhenInputVar::new::<#core::widget_builder::WidgetHandlerInWhenExprError>()
                        }
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
                fn ui_node(&self, __index__: usize) -> &#core::widget_instance::ArcNode<#core::widget_instance::BoxedUiNode> {
                    match __index__ {
                        #get_ui_node
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::UiNode),
                    }
                }
            }
        }
        if !get_ui_node_list.is_empty() {
            get_ui_node_list = quote! {
                fn ui_node_list(&self, __index__: usize) -> &#core::widget_instance::ArcNodeList<#core::widget_instance::BoxedUiNodeList> {
                    match __index__ {
                        #get_ui_node_list
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::UiNodeList),
                    }
                }
            }
        }
        if !get_widget_handler.is_empty() {
            get_widget_handler = quote! {
                fn widget_handler(&self, __index__: usize) -> &dyn #core::widget_builder::AnyArcWidgetHandler {
                    match __index__ {
                        #get_widget_handler
                        n => #core::widget_builder::panic_input(&self.property(), n, #core::widget_builder::InputKind::WidgetHandler),
                    }
                }
            }
        }

        let mut sorted_inputs: Vec<_> = inputs[1..].iter().collect();
        sorted_inputs.sort_by_key(|i| &i.ident);
        let sorted_idents = sorted_inputs.iter().map(|i| &i.ident);
        let sorted_tys = sorted_inputs.iter().map(|i| &i.ty);

        let node_instance = ident_spanned!(output_span=> "__node__");

        quote! {
            #cfg
            #[doc(hidden)]
            #[derive(std::clone::Clone)]
            #[allow(non_camel_case_types)]
            #vis struct #ident #impl_gens #where_gens {
                __instance__: #core::widget_builder::PropertyInstInfo,
                #(#input_idents: #storage_tys),*
            }
            #cfg
            impl #impl_gens #ident #ty_gens #where_gens {
                pub const ALLOWED_IN_WHEN_EXPR: bool = #allowed_in_when_expr;
                pub const ALLOWED_IN_WHEN_ASSIGN: bool = #allowed_in_when_assign;

                #[doc(hidden)]
                pub fn __id__(name: &'static str) -> #core::widget_builder::PropertyId {
                    static impl_id: #core::widget_builder::StaticPropertyImplId = #core::widget_builder::StaticPropertyImplId::new_unique();

                    #core::widget_builder::PropertyId {
                        impl_id: impl_id.get(),
                        name,
                    }
                }

                #[doc(hidden)]
                pub fn __property__() -> #core::widget_builder::PropertyInfo {
                    #core::widget_builder::PropertyInfo {
                        group: #core::widget_builder::NestGroup::#nest_group,
                        capture: #capture,
                        impl_id: Self::__id__("").impl_id,
                        name: std::stringify!(#ident),
                        location: #core::widget_builder::source_location!(),
                        default: #default_fn,
                        new: Self::__new_dyn__,
                        inputs: std::boxed::Box::new([
                            #input_info
                        ]),
                    }
                }

                #[doc(hidden)]
                pub const fn __input_types__() -> #core::widget_builder::PropertyInputTypes<(#(#storage_tys,)*)> {
                    #core::widget_builder::PropertyInputTypes::unit()
                }

                #[allow(clippy::too_many_arguments)]
                pub fn __new__(
                    #(#input_idents: #input_tys),*
                ) -> Self {
                    Self {
                        __instance__: #core::widget_builder::PropertyInstInfo::none(),
                        #(#input_idents: #input_to_storage),*
                    }
                }

                #[allow(clippy::too_many_arguments)]
                pub fn __new_sorted__(#(#sorted_idents: #sorted_tys),*) -> Self {
                    Self::__new__(#(#input_idents),*)
                }

                pub fn __new_dyn__(
                    __args__: #core::widget_builder::PropertyNewArgs,
                ) -> std::boxed::Box<dyn #core::widget_builder::PropertyArgs> {
                    let mut __inputs__ = __args__.args.into_iter();
                    let mut __actions__ = __args__.build_actions;

                    Box::new(Self {
                        __instance__: __args__.inst_info,
                        #(#input_idents: { #input_new_dyn },)*
                    })
                }

                pub fn __build__(mut self, info: #core::widget_builder::PropertyInstInfo) -> std::boxed::Box<dyn #core::widget_builder::PropertyArgs> {
                    self.__instance__ = info;
                    Box::new(self)
                }

                #default

                pub fn __default_fn__() -> std::option::Option<fn (info: #core::widget_builder::PropertyInstInfo) -> std::boxed::Box<dyn #core::widget_builder::PropertyArgs>> {
                    #default_fn
                }

                #named_into_var
                #get_when_input
            }
            #cfg
            impl #impl_gens #core::widget_builder::PropertyArgs for #ident #ty_gens #where_gens {
                fn clone_boxed(&self) -> std::boxed::Box<dyn #core::widget_builder::PropertyArgs> {
                    Box::new(std::clone::Clone::clone(self))
                }

                fn property(&self) -> #core::widget_builder::PropertyInfo {
                    Self::__property__()
                }

                fn instance(&self) -> #core::widget_builder::PropertyInstInfo {
                    std::clone::Clone::clone(&self.__instance__)
                }

                fn instantiate(&self, __child__: #core::widget_instance::BoxedUiNode) -> #core::widget_instance::BoxedUiNode {
                    let #node_instance = #ident(__child__, #instantiate);
                    #core::widget_instance::UiNode::boxed(#node_instance)
                }

                #get_var
                #get_state
                #get_value
                #get_ui_node
                #get_ui_node_list
                #get_widget_handler
            }
        }
    } else {
        quote!()
    };

    let r = quote! {
        #attrs
        #item
        #extra
        #errors
    };
    r.into()
}

struct Args {
    nest_group: Ident,
    capture: bool,
    default: Option<Default>,
}
impl Parse for Args {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Args {
            nest_group: input.parse()?,
            capture: if input.peek(Token![,]) && input.peek2(keyword::capture) {
                let _: Token![,] = input.parse()?;
                let _: keyword::capture = input.parse()?;
                true
            } else {
                false
            },
            default: if input.peek(Token![,]) && input.peek2(Token![default]) {
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}

struct Default {
    default: Token![default],
    args: Punctuated<Expr, Token![,]>,
}
impl Parse for Default {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let _: Token![,] = input.parse()?;
        let default = input.parse()?;
        let inner;
        parenthesized!(inner in input);
        Ok(Default {
            default,
            args: Punctuated::parse_terminated(&inner)?,
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
                errors.push("methods cannot be properties", rcv.span());
            }
            FnArg::Typed(t) => {
                if !t.attrs.is_empty() {
                    errors.push("property input cannot have attributes", t.attrs[0].span());
                }

                match *t.pat.clone() {
                    Pat::Ident(id) => {
                        if id.ident == "self" {
                            errors.push("methods cannot be properties", id.ident.span());
                        }
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
                                            input.storage_ty = quote!(#core::widget_builder::ArcWidgetHandler<#t>);
                                        }
                                    }
                                    "UiNode" => {
                                        input.kind = InputKind::UiNode;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::widget_instance::BoxedUiNode);
                                        input.storage_ty = quote!(#core::widget_instance::ArcNode<#core::widget_instance::BoxedUiNode>);
                                    }
                                    "UiNodeList" => {
                                        input.kind = InputKind::UiNodeList;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::widget_instance::BoxedUiNodeList);
                                        input.storage_ty =
                                            quote!(#core::widget_instance::ArcNodeList<#core::widget_instance::BoxedUiNodeList>)
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

pub mod keyword {
    syn::custom_keyword!(capture);
}
