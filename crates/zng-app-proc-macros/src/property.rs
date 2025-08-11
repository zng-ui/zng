use std::mem;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, *};

use crate::util::{Attributes, Errors, crate_core, set_stream_span};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut errors = Errors::default();

    let args = match parse::<Args>(args) {
        Ok(a) => a,
        Err(e) => {
            errors.push_syn(e);
            Args {
                nest_group: parse_quote!(CONTEXT),
                capture: false,
                default: None,
                impl_for: None,
            }
        }
    };

    let nest_group = args.nest_group;
    let capture = args.capture;
    let impl_for = args.impl_for;

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
    let mut mtd_attrs = Attributes::new(vec![]);
    mtd_attrs.docs.clone_from(&attrs.docs);
    let mut extra_docs = quote!();

    // note that the tags "c" and "P" are used by the `widget.js` to find properties.
    if capture {
        attrs.tag_doc("c", "Capture-only property function");
        mtd_attrs.tag_doc("c", "Capture-only property method");
        if !attrs.docs.is_empty() {
            extra_docs = quote! {
                ///
                /// # Capture-Only
                ///
                /// This property is capture-only, it only defines a property signature, it does not implement any behavior by itself.
                /// Widgets can capture and implement this property as part of their intrinsics, otherwise it will have no
                /// effect if set on a widget that does not implement it.
            };
        }

        if item.sig.inputs.is_empty() {
            errors.push(
                "capture property functions must have at least 1 input: arg0, ..",
                item.sig.inputs.span(),
            );
        }
    } else {
        attrs.tag_doc("P", "Property function");
        mtd_attrs.tag_doc("P", "Property method");

        if item.sig.inputs.len() < 2 {
            errors.push(
                "property functions must have at least 2 inputs: child: impl IntoUiNode, arg0, ..",
                item.sig.inputs.span(),
            );
        }
    }

    if item.sig.inputs.is_empty() {
        // patch to continue validation.
        let core = crate_core();
        item.sig.inputs.push(parse_quote!(__child__: impl #core::widget::node::IntoUiNode));
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
    if !inputs[0].ty.is_empty() && !capture {
        // first param passed Input::from_arg validation, check if is node.
        if !matches!(inputs[0].kind, InputKind::UiNode) {
            errors.push("first input must be `impl IntoUiNode`", inputs[0].ty.span());
        }
    }

    if capture {
        let inputs = inputs.iter().map(|i| &i.ident);
        if !item.block.stmts.is_empty() {
            errors.push("capture property must have an empty body", item.block.span());
        }
        item.block.stmts.clear();
        item.block.stmts.push(parse_quote! {
            let _ = (#(#inputs,)*);
        });
    }
    let item = item;
    let first_input = if capture { 0 } else { 1 };

    match &item.sig.output {
        ReturnType::Default => {
            if !capture {
                errors.push("output type must be `UiNode`", item.sig.ident.span());
            }
        }
        ReturnType::Type(_, ty) => {
            if capture {
                errors.push("capture must not have output", ty.span());
            }
        }
    };

    // validate generics
    // validate input types, generate args

    let extra = if errors.is_empty() {
        // generate items if all is valid.

        let core = crate_core();
        let cfg = &attrs.cfg;
        let deprecated = &attrs.deprecated;
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let ident_str = ident.to_string();
        let generics = &item.sig.generics;
        let has_generics = !generics.params.empty_or_trailing();
        let (impl_gens, ty_gens, where_gens) = generics.split_for_impl();
        let path_gens = if has_generics { quote!(::#ty_gens) } else { quote!() };

        let ident_unset = ident!("unset_{}", ident);
        let ident_args = ident!("{}_args__", ident);
        let ident_inputs = ident!("{}_inputs__", ident);
        let ident_meta = ident!("{}_", ident);
        let ident_sorted = ident!("{}__", ident);

        let default;
        let default_fn;
        let args_default = match args.default {
            Some(d) => Some((d.default.span(), d.args.to_token_stream())),
            None => {
                let mut default = quote!();
                let first_input = if capture { 0 } else { 1 };
                for input in &inputs[first_input..] {
                    match input.kind {
                        InputKind::Var => {
                            if ident_str.starts_with("is_") || ident_str.starts_with("has_") {
                                let core = set_stream_span(core.clone(), input.ty.span());
                                default.extend(quote_spanned! {input.ty.span()=>
                                    #core::widget::builder::var_state(),
                                })
                            } else if ident_str.starts_with("get_") || ident_str.starts_with("actual_") {
                                let core = set_stream_span(core.clone(), input.ty.span());
                                default.extend(quote_spanned! {input.ty.span()=>
                                    #core::widget::builder::var_getter(),
                                })
                            }
                        }
                        InputKind::UiNode => default.extend(quote! {
                            #core::widget::node::UiNode::nil(),
                        }),
                        InputKind::WidgetHandler if !has_generics => default.extend(quote! {
                            #core::handler::hn!(|_| {}),
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
            default = quote_spanned! {span=>
                fn __default__ #impl_gens() -> std::boxed::Box<dyn #core::widget::builder::PropertyArgs> #where_gens {
                    #ident_meta {}.args(#args)
                }
            };
            default_fn = quote! {
                Some(__default__ #path_gens)
            };
        } else {
            default = quote!();
            default_fn = quote! {
                None
            };
        }
        let mut input_info = quote!();
        let mut get_var = quote!();
        let mut get_value = quote!();
        let mut get_ui_node = quote!();
        let mut get_widget_handler = quote!();

        let mut instantiate = quote!();
        let mut input_idents = vec![];
        let mut input_tys = vec![];
        let mut storage_tys = vec![];
        let mut input_to_storage = vec![];
        let mut named_into = quote!();
        let mut get_when_input = quote!();
        let mut input_new_dyn = vec![];

        let mut allowed_in_when_expr = true;
        let mut allowed_in_when_assign = true;

        for (i, input) in inputs[first_input..].iter().enumerate() {
            let ident = &input.ident;
            let input_ty = &input.ty;
            input_idents.push(ident);
            input_tys.push(input_ty);
            storage_tys.push(&input.storage_ty);

            let kind = input.kind;
            let info_ty = &input.info_ty;
            let storage_ty = &input.storage_ty;
            input_info.extend(quote! {
                #core::widget::builder::PropertyInput {
                    name: stringify!(#ident),
                    kind: #kind,
                    ty: std::any::TypeId::of::<#info_ty>(),
                    ty_name: std::any::type_name::<#info_ty>(),
                    _non_exhaustive: (),
                },
            });

            match kind {
                InputKind::Var => {
                    input_to_storage.push(quote! {
                        self.inputs().#ident(#ident)
                    });
                    get_var.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    named_into.extend(quote! {
                        pub fn #ident #impl_gens(&self, #ident: #input_ty) -> #storage_ty #where_gens {
                            #core::widget::builder::var_to_args(#ident)
                        }
                    });
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget::builder::iter_input_build_actions(&__args__.build_actions, &__args__.build_actions_when_data, #i);
                        #core::widget::builder::new_dyn_var(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident #impl_gens(&self)
                        -> (#core::widget::builder::WhenInputVar, #core::var::Var<#info_ty>) #where_gens {
                            #core::widget::builder::WhenInputVar::new::<#info_ty>()
                        }
                        pub fn #get_ident_i #impl_gens(&self)
                        -> (#core::widget::builder::WhenInputVar, #core::var::Var<#info_ty>) #where_gens {
                            #core::widget::builder::WhenInputVar::new::<#info_ty>()
                        }
                    });
                }
                InputKind::Value => {
                    allowed_in_when_assign = false;
                    input_to_storage.push(quote! {
                        self.inputs().#ident(#ident)
                    });
                    named_into.extend(quote! {
                        pub fn #ident #impl_gens(&self, #ident: #input_ty) -> #storage_ty #where_gens {
                            #core::widget::builder::value_to_args(#ident)
                        }
                    });
                    get_value.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget::builder::iter_input_build_actions(&__args__.build_actions, &__args__.build_actions_when_data, #i);
                        #core::widget::builder::new_dyn_other(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident #impl_gens(&self)
                        -> (#core::widget::builder::WhenInputVar, #core::var::Var<#info_ty>) #where_gens {
                            #core::widget::builder::WhenInputVar::new::<#info_ty>()
                        }
                        pub fn #get_ident_i #impl_gens(&self)
                        -> (#core::widget::builder::WhenInputVar, #core::var::Var<#info_ty>) #where_gens {
                            #core::widget::builder::WhenInputVar::new::<#info_ty>()
                        }
                    });
                }
                InputKind::UiNode => {
                    allowed_in_when_expr = false;
                    input_to_storage.push(quote! {
                        #core::widget::builder::ui_node_to_args(#ident)
                    });
                    named_into.extend(quote! {
                        pub fn #ident(&self, #ident: #input_ty) -> #core::widget::node::UiNode {
                            #core::widget::node::IntoUiNode::into_node(#ident)
                        }
                    });
                    get_ui_node.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        self.#ident.take_on_init(),
                    });
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget::builder::iter_input_build_actions(&__args__.build_actions, &__args__.build_actions_when_data, #i);
                        #core::widget::builder::new_dyn_ui_node(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident(&self)
                        -> (#core::widget::builder::WhenInputVar, #core::var::Var<#core::widget::builder::UiNodeInWhenExprError>) {
                            #core::widget::builder::WhenInputVar::new::<#core::widget::builder::UiNodeInWhenExprError>()
                        }
                        pub fn #get_ident_i(&self)
                        -> (#core::widget::builder::WhenInputVar, #core::var::Var<#core::widget::builder::UiNodeInWhenExprError>) {
                            #core::widget::builder::WhenInputVar::new::<#core::widget::builder::UiNodeInWhenExprError>()
                        }
                    });
                }
                InputKind::WidgetHandler => {
                    allowed_in_when_expr = false;
                    input_to_storage.push(quote! {
                        #core::widget::builder::widget_handler_to_args(#ident)
                    });
                    named_into.extend(quote! {
                        pub fn #ident #impl_gens(&self, #ident: #input_ty) -> #input_ty #where_gens {
                            #ident
                        }
                    });
                    get_widget_handler.extend(quote! {
                        #i => &self.#ident,
                    });
                    instantiate.extend(quote! {
                        std::clone::Clone::clone(&self.#ident),
                    });
                    input_new_dyn.push(quote! {
                        let __actions__ = #core::widget::builder::iter_input_build_actions(&__args__.build_actions, &__args__.build_actions_when_data, #i);
                        #core::widget::builder::new_dyn_widget_handler(&mut __inputs__, __actions__)
                    });
                    let get_ident = ident!("__w_{ident}__");
                    let get_ident_i = ident!("__w_{i}__");
                    get_when_input.extend(quote! {
                        pub fn #get_ident #impl_gens(&self)
                        -> (#core::widget::builder::WhenInputVar, #core::var::Var<#core::widget::builder::WidgetHandlerInWhenExprError>) #where_gens {
                            #core::widget::builder::WhenInputVar::new::<#core::widget::builder::WidgetHandlerInWhenExprError>()
                        }
                        pub fn #get_ident_i #impl_gens(&self)
                        -> (#core::widget::builder::WhenInputVar, #core::var::Var<#core::widget::builder::WidgetHandlerInWhenExprError>) #where_gens {
                            #core::widget::builder::WhenInputVar::new::<#core::widget::builder::WidgetHandlerInWhenExprError>()
                        }
                    });
                }
            }
        }

        if !get_var.is_empty() {
            get_var = quote! {
                fn var(&self, __index__: usize) -> &#core::var::AnyVar {
                    match __index__ {
                        #get_var
                        n => #core::widget::builder::panic_input(&self.property(), n, #core::widget::builder::InputKind::Var),
                    }
                }
            }
        }
        if !get_value.is_empty() {
            get_value = quote! {
                fn value(&self, __index__: usize) -> &dyn #core::var::AnyVarValue {
                    match __index__ {
                        #get_value
                        n => #core::widget::builder::panic_input(&self.property(), n, #core::widget::builder::InputKind::Value),
                    }
                }
            }
        }
        if !get_ui_node.is_empty() {
            get_ui_node = quote! {
                fn ui_node(&self, __index__: usize) -> &#core::widget::node::ArcNode {
                    match __index__ {
                        #get_ui_node
                        n => #core::widget::builder::panic_input(&self.property(), n, #core::widget::builder::InputKind::UiNode),
                    }
                }
            }
        }
        if !get_widget_handler.is_empty() {
            get_widget_handler = quote! {
                fn widget_handler(&self, __index__: usize) -> &dyn #core::widget::builder::AnyArcWidgetHandler {
                    match __index__ {
                        #get_widget_handler
                        n => #core::widget::builder::panic_input(&self.property(), n, #core::widget::builder::InputKind::WidgetHandler),
                    }
                }
            }
        }

        let mut sorted_inputs: Vec<_> = inputs[first_input..].iter().collect();
        sorted_inputs.sort_by_key(|i| &i.ident);
        let sorted_idents: Vec<_> = sorted_inputs.iter().map(|i| &i.ident).collect();
        let sorted_tys: Vec<_> = sorted_inputs.iter().map(|i| &i.ty).collect();

        let docs = &attrs.docs;

        let allowed_in_when_expr = if allowed_in_when_expr {
            quote! {
                pub const fn allowed_in_when_expr(&self) {}
            }
        } else {
            quote!()
        };
        let allowed_in_when_assign = if allowed_in_when_assign {
            quote! {
                pub const fn allowed_in_when_assign(&self) {}
            }
        } else {
            quote!()
        };

        let source_location = crate::widget_util::source_location(&core, ident.span());

        let meta = quote! {
            #cfg
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #[non_exhaustive]
            #vis struct #ident_meta { }
            #cfg
            #[doc(hidden)]
            #[allow(dead_code)]
            impl #ident_meta {
                pub fn id(&self) -> #core::widget::builder::PropertyId {
                    #core::static_id! {
                        static ref ID: #core::widget::builder::PropertyId;
                    }
                    *ID
                }

                #allowed_in_when_expr
                #allowed_in_when_assign

                pub fn info #impl_gens(&self) -> #core::widget::builder::PropertyInfo #where_gens {
                    #core::widget::builder::PropertyInfo {
                        group: {
                            use #core::widget::builder::nest_group_items::*;
                            #nest_group
                        },
                        capture: #capture,
                        id: self.id(),
                        name: std::stringify!(#ident),
                        location: #source_location,
                        default: self.default_fn #path_gens(),
                        new: Self::args_dyn #path_gens,
                        inputs: std::boxed::Box::new([
                            #input_info
                        ]),
                        _non_exhaustive: (),
                    }
                }

                #vis const fn input_types #impl_gens(&self) -> #core::widget::builder::PropertyInputTypes<(#(#storage_tys,)*)> #where_gens {
                    #core::widget::builder::PropertyInputTypes::unit()
                }

                pub fn default_fn #impl_gens(&self) -> std::option::Option<fn () -> std::boxed::Box<dyn #core::widget::builder::PropertyArgs>> #where_gens {
                    #default
                    #default_fn
                }

                #vis fn args #impl_gens(
                    &self,
                    #(#input_idents: #input_tys),*
                ) -> std::boxed::Box<dyn #core::widget::builder::PropertyArgs> #where_gens {
                    std::boxed::Box::new(#ident_args {
                        #(#input_idents: #input_to_storage),*
                    })
                }

                #vis fn args_sorted #impl_gens(
                    &self,
                    #(#sorted_idents: #sorted_tys),*
                ) -> std::boxed::Box<dyn #core::widget::builder::PropertyArgs> #where_gens {
                    self.args(#(#input_idents),*)
                }

                fn args_dyn #impl_gens(
                    __args__: #core::widget::builder::PropertyNewArgs,
                ) -> std::boxed::Box<dyn #core::widget::builder::PropertyArgs> #where_gens {
                    let mut __inputs__ = __args__.args.into_iter();
                    Box::new(#ident_args #path_gens {
                        #(#input_idents: { #input_new_dyn },)*
                    })
                }

                pub fn inputs(&self) -> #ident_inputs {
                    #ident_inputs { }
                }
            }
        };
        let instantiate = if capture {
            quote! {
                #[allow(unused)]
                use self::#ident;
                __child__
            }
        } else {
            quote! {
                #ident(__child__, #instantiate)
            }
        };

        let allow_deprecated = deprecated.as_ref().map(|_| {
            quote! {
                #[allow(deprecated)]
            }
        });

        let args = quote! {
            #cfg
            #[doc(hidden)]
            #[derive(std::clone::Clone)]
            #[allow(non_camel_case_types)]
            #[non_exhaustive]
            #vis struct #ident_args #impl_gens #where_gens {
                #(#input_idents: #storage_tys),*
            }
            #cfg
            #[doc(hidden)]
            impl #impl_gens #core::widget::builder::PropertyArgs for #ident_args #ty_gens #where_gens {
                fn clone_boxed(&self) -> std::boxed::Box<dyn #core::widget::builder::PropertyArgs> {
                    Box::new(std::clone::Clone::clone(self))
                }

                fn property(&self) -> #core::widget::builder::PropertyInfo {
                    #ident_meta { }.info #path_gens()
                }

                #allow_deprecated
                fn instantiate(&self, __child__: #core::widget::node::UiNode) -> #core::widget::node::UiNode {
                    #instantiate
                }

                #get_var
                #get_value
                #get_ui_node
                #get_widget_handler
            }
        };
        let inputs = quote! {
            #cfg
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #[non_exhaustive]
            #vis struct #ident_inputs { }
            #cfg
            #[doc(hidden)]
            impl #ident_inputs {
                #named_into
                #get_when_input
            }
        };

        let direct_impl = if let Some(impl_for) = impl_for {
            let mut target = impl_for.target;
            let (generics_impl, generics) = match &target.segments.last().unwrap().arguments {
                PathArguments::None => (quote!(), quote!()),
                PathArguments::AngleBracketed(b) => {
                    if b.args.is_empty() {
                        (quote!(), quote!())
                    } else if b.args.len() > 1 {
                        errors.push("only `<P>` generics is allowed", b.span());
                        (quote!(), quote!())
                    } else {
                        target.segments.last_mut().unwrap().arguments = PathArguments::None;
                        (quote!(<P: #core::widget::base::WidgetImpl>), quote!(<P>))
                    }
                }
                PathArguments::Parenthesized(p) => {
                    errors.push("only `<P>` generics is allowed", p.span());
                    (quote!(), quote!())
                }
            };
            let docs = &mtd_attrs.docs;
            quote! {
                #cfg
                impl #generics_impl #target #generics {
                    #(#docs)*
                    #deprecated
                    #vis fn #ident #impl_gens(&self, #(#input_idents: #input_tys),*) #where_gens {
                        let args = #ident_meta { }.args(#(#input_idents),*);
                        #core::widget::base::WidgetImpl::base_ref(self).mtd_property__(args)
                    }

                    #[doc(hidden)]
                    #[allow(dead_code)]
                    #vis fn #ident_unset(&self) {
                        #core::widget::base::WidgetImpl::base_ref(self).mtd_property_unset__(#ident_meta { }.id())
                    }

                    #[doc(hidden)]
                    #[allow(dead_code)]
                    #vis fn #ident_sorted #impl_gens(&mut self, #(#sorted_idents: #sorted_tys),*) #where_gens {
                        let args = #ident_meta { }.args_sorted(#(#sorted_idents),*);
                        #core::widget::base::WidgetImpl::base_ref(self).mtd_property__(args)
                    }

                    #[doc(hidden)]
                    #[allow(dead_code)]
                    #vis fn #ident_meta(&self) -> #ident_meta {
                        #ident_meta { }
                    }
                }
            }
        } else {
            quote!()
        };

        quote! {
            #direct_impl

            #cfg
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis trait #ident: #core::widget::base::WidgetExt {
                type MetaType;

                #(#docs)*
                #deprecated
                #[allow(clippy::too_many_arguments)]
                fn #ident #impl_gens(&mut self, #(#input_idents: #input_tys),*) #where_gens {
                    let args = #ident_meta { }.args(#(#input_idents),*);
                    self.ext_property__(args)
                }

                /// Unset the property.
                fn #ident_unset(&mut self) {
                    self.ext_property_unset__(#ident_meta {}.id())
                }

                #[doc(hidden)]
                fn #ident_sorted #impl_gens(&mut self, #(#sorted_idents: #sorted_tys),*) #where_gens {
                    let args = #ident_meta { }.args_sorted(#(#sorted_idents),*);
                    self.ext_property__(args)
                }

                #[doc(hidden)]
                fn #ident_meta(&self) -> #ident_meta {
                    #ident_meta { }
                }
            }
            #cfg
            #[doc(hidden)]
            impl self::#ident for #core::widget::base::WidgetBase {
                type MetaType = ();
            }
            #cfg
            #[doc(hidden)]
            impl self::#ident for #core::widget::base::NonWidgetBase {
                type MetaType = ();
            }
            #cfg
            #[doc(hidden)]
            impl self::#ident for #core::widget::builder::WgtInfo {
                type MetaType = #ident_meta;
            }

            #meta
            #args
            #inputs
        }
    } else {
        quote!()
    };

    let r = quote! {
        #attrs
        #extra_docs
        #item
        #extra
        #errors
    };
    r.into()
}

struct Args {
    nest_group: Expr,
    capture: bool,
    default: Option<Default>,
    impl_for: Option<ImplFor>,
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
            impl_for: if input.peek(Token![,]) && (input.peek2(keyword::widget_impl) || input.peek2(Token![for])) {
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

struct ImplFor {
    target: Path,
}
impl Parse for ImplFor {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let _: Token![,] = input.parse()?;
        let _: keyword::widget_impl = input.parse()?;
        let inner;
        parenthesized!(inner in input);

        Ok(ImplFor { target: inner.parse()? })
    }
}

#[derive(Clone, Copy)]
enum InputKind {
    Var,
    Value,
    UiNode,
    WidgetHandler,
}
impl ToTokens for InputKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let kin = match self {
            InputKind::Var => ident!("Var"),
            InputKind::Value => ident!("Value"),
            InputKind::UiNode => ident!("UiNode"),
            InputKind::WidgetHandler => ident!("WidgetHandler"),
        };
        let core = crate_core();
        tokens.extend(quote! {
            #core::widget::builder::InputKind::#kin
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
                                    if let PathArguments::AngleBracketed(it) = args
                                        && it.args.len() == 1
                                    {
                                        input.kind = kind;
                                        input.ty = t.to_token_stream();
                                        input.info_ty = it.args.last().unwrap().to_token_stream();
                                        return true;
                                    }
                                    errors.push("expected single generic param", args.span());
                                    false
                                }

                                match seg.ident.to_string().as_str() {
                                    "IntoVar" if !seg.arguments.is_empty() => {
                                        if ty_from_generic(&mut input, errors, &t.ty, InputKind::Var, &seg.arguments) {
                                            let t = &input.info_ty;
                                            input.storage_ty = quote!(#core::var::Var<#t>);
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
                                            input.storage_ty = quote!(#core::widget::builder::ArcWidgetHandler<#t>);
                                        }
                                    }
                                    "IntoUiNode" => {
                                        input.kind = InputKind::UiNode;
                                        input.ty = t.ty.to_token_stream();
                                        input.info_ty = quote_spanned!(t.ty.span()=> #core::widget::node::UiNode);
                                        input.storage_ty = quote!(#core::widget::node::ArcNode);
                                    }
                                    _ => {
                                        errors.push("property input can only have impl types for: IntoVar<T>, IntoValue<T>, IntoUiNode, WidgetHandler<A>", seg.span());
                                    }
                                }
                            }
                            t => {
                                errors.push("property input can only have `impl OneTrait`", t.span());
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

pub mod keyword {
    syn::custom_keyword!(capture);
    syn::custom_keyword!(widget_impl);
}

pub fn expand_meta(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as MetaArgs);
    let core = crate_core();

    let r = match args {
        MetaArgs::Method { self_ty, sep, property } => {
            let meta_ident = ident!("{}_", property);
            quote! {
                <#self_ty as #core::widget::base::WidgetImpl> #sep info_instance__() . #meta_ident()
            }
        }
        MetaArgs::Function { path } => {
            let ident = &path.segments.last().unwrap().ident;
            let meta_ident = ident!("{}_", ident);

            quote! {
                <#core::widget::builder::WgtInfo as #path>::#meta_ident(&#core::widget::builder::WgtInfo)
            }
        }
    };
    r.into()
}
enum MetaArgs {
    Method {
        self_ty: Token![Self],
        sep: Token![::],
        property: Ident,
    },
    Function {
        path: Path,
    },
}
impl Parse for MetaArgs {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        if input.peek(Token![Self]) {
            Ok(Self::Method {
                self_ty: input.parse()?,
                sep: input.parse()?,
                property: input.parse()?,
            })
        } else {
            Ok(Self::Function { path: input.parse()? })
        }
    }
}

pub fn expand_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let p = parse_macro_input!(input as PropertyImplArgs);
    if p.args.empty_or_trailing() {
        return quote_spanned! {p.path.span()=>
            compile_error!("missing args")
        }
        .into();
    }

    let mut attrs = Attributes::new(p.attrs);
    attrs.tag_doc("P", "This method is a widget property");

    let cfg = &attrs.cfg;
    let vis = p.vis;
    let path = p.path;
    let ident = &path.segments.last().unwrap().ident;
    let ident_unset = ident!("unset_{ident}");
    let ident_meta = ident!("{}_", ident);
    let ident_sorted = ident!("{}__", ident);
    let args = p.args;
    let mut sorted_args: Vec<_> = args.iter().collect();
    sorted_args.sort_by_key(|a| &a.ident);

    let arg_idents = args.iter().map(|a| &a.ident);
    let sorted_idents: Vec<_> = sorted_args.iter().map(|a| &a.ident).collect();
    let sorted_tys = sorted_args.iter().map(|a| &a.ty);

    let core = crate_core();

    let r = quote! {
        #attrs
        #vis fn #ident(&self, #args) {
            #core::widget::base::WidgetImpl::base_ref(self).reexport__(|base__| {
                #path::#ident(base__, #(#arg_idents),*);
            });
        }

        /// Unset the property.
        #cfg
        #[doc(hidden)]
        #[allow(dead_code)]
        #vis fn #ident_unset(&self) {
            #core::widget::base::WidgetImpl::base_ref(self).reexport__(|base__| {
                #path::#ident_unset(base__);
            });
        }

        #cfg
        #[doc(hidden)]
        #[allow(dead_code)]
        #vis fn #ident_sorted(&self, #(#sorted_idents: #sorted_tys),*) {
            #core::widget::base::WidgetImpl::base_ref(self).reexport__(|base__| {
                #path::#ident_sorted(base__, #(#sorted_idents),*);
            });
        }

        #cfg
        #[doc(hidden)]
        #[allow(dead_code)]
        #vis fn #ident_meta(&self) -> <#core::widget::builder::WgtInfo as #path>::MetaType {
            <#core::widget::builder::WgtInfo as #path>::#ident_meta(&#core::widget::builder::WgtInfo)
        }
    };
    r.into()
}

struct PropertyImplArgs {
    attrs: Vec<Attribute>,
    vis: syn::Visibility,
    path: Path,
    args: Punctuated<SimpleFnArg, Token![,]>,
}
impl Parse for PropertyImplArgs {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Self {
            attrs: Attribute::parse_outer(&non_user_braced!(input, "attrs"))?,
            vis: non_user_braced!(input, "vis").parse().unwrap(),
            path: non_user_braced!(input, "path").parse()?,
            args: Punctuated::parse_terminated(&non_user_braced!(input, "args"))?,
        })
    }
}

#[derive(Clone)]
struct SimpleFnArg {
    ident: Ident,
    _s: Token![:],
    ty: Type,
}
impl ToTokens for SimpleFnArg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        self._s.to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}
impl Parse for SimpleFnArg {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            _s: input.parse()?,
            ty: input.parse()?,
        })
    }
}
