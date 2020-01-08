use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned;
use syn::{parse::*, *};

include!("util.rs");

#[allow(clippy::cognitive_complexity)]
pub(crate) fn expand_property(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let priority = parse_macro_input!(args as Priority);

    let mut fn_ = parse_macro_input!(input as ItemFn);
    let prop_ident = fn_.sig.ident.clone();
    let vis = fn_.vis;

    fn_.sig.ident = Ident::new("set", prop_ident.span());
    fn_.vis = pub_vis();

    if fn_.sig.output == ReturnType::Default {
        abort_call_site!("function must return an `UiNode`")
    }

    let mut arg_names = vec![];
    let mut arg_gen_types = vec![];
    let mut arg_types = vec![];

    if fn_.sig.inputs.len() < 2 {
        abort_call_site!("function must take a `child: impl UiNode` first and at least one other argument");
    } else if let Some(FnArg::Receiver(_)) = fn_.sig.inputs.first() {
        abort_call_site!("function must free-standing");
    } else {
        for arg in fn_.sig.inputs.iter().skip(1) {
            if let FnArg::Typed(pat) = arg {
                arg_types.push(pat.ty.clone());
                if let Pat::Ident(pat) = &*pat.pat {
                    arg_names.push(pat.ident.clone());
                    arg_gen_types.push(ident(&format!("T{}", arg_gen_types.len() + 1)))
                } else {
                    abort!(arg.span(), "property arguments does not support pattern deconstruction");
                }
            } else {
                abort!(arg.span(), "unexpected `self`");
            }
        }
    }

    let mut sorted_sets = vec![];
    let arg_nils: Vec<_> = arg_names.iter().map(|_| quote! {()}).collect();
    let gen_tys = if let Some(gen_lt) = fn_.sig.generics.lt_token {
        let gen_tys = fn_.sig.generics.type_params();
        let gen_gt = fn_.sig.generics.gt_token;

        quote! {
            #gen_lt
            #(#gen_tys),*
            #gen_gt
        }
    } else {
        TokenStream::new()
    };
    let where_ = fn_.sig.generics.where_clause.clone();

    let mut set_pre: ItemFn = parse_quote! {
        #[doc(hidden)]
        #[inline]
        pub fn set_pre #gen_tys (child: impl UiNode, #(#arg_names: #arg_types),*) -> (impl UiNode, #(#arg_types),*) #where_ {
            (child, #(#arg_names),*)
        }
    };
    let mut set_priority: ItemFn = parse_quote! {
        #[doc(hidden)]
        #[inline]
        pub fn set_priority #gen_tys (child: impl UiNode, #(#arg_names: #arg_types),*) -> (impl UiNode, #(#arg_nils),*) #where_ {
            (set(child, #(#arg_names),*), #(#arg_nils),*)
        }
    };

    let mut set_post: ItemFn = parse_quote! {
        #[doc(hidden)]
        #[inline]
        pub fn set_pos(child: impl UiNode, #(_: #arg_nils),*) -> (impl UiNode, #(#arg_nils),*) {
            (child, #(#arg_nils),*)
        }
    };

    match priority {
        Priority::ContextVar => {
            set_priority.sig.ident = ident("set_context_var");
            sorted_sets.push(set_priority);
            set_post.sig.ident = ident("set_event");
            sorted_sets.push(set_post.clone());
            set_post.sig.ident = ident("set_outer");
            sorted_sets.push(set_post.clone());
            set_post.sig.ident = ident("set_inner");
            sorted_sets.push(set_post);
        }
        Priority::Event => {
            set_pre.sig.ident = ident("set_context_var");
            sorted_sets.push(set_pre);
            set_priority.sig.ident = ident("set_event");
            sorted_sets.push(set_priority);
            set_post.sig.ident = ident("set_outer");
            sorted_sets.push(set_post.clone());
            set_post.sig.ident = ident("set_inner");
            sorted_sets.push(set_post);
        }
        Priority::Outer => {
            set_pre.sig.ident = ident("set_context_var");
            sorted_sets.push(set_pre.clone());
            set_pre.sig.ident = ident("set_event");
            sorted_sets.push(set_pre);
            set_priority.sig.ident = ident("set_outer");
            sorted_sets.push(set_priority);
            set_post.sig.ident = ident("set_inner");
            sorted_sets.push(set_post);
        }
        Priority::Inner => {
            set_pre.sig.ident = ident("set_context_var");
            sorted_sets.push(set_pre.clone());
            set_pre.sig.ident = ident("set_event");
            sorted_sets.push(set_pre.clone());
            set_pre.sig.ident = ident("set_outer");
            sorted_sets.push(set_pre);
            set_priority.sig.ident = ident("set_inner");
            sorted_sets.push(set_priority);
        }
    }

    let (docs_attrs, other_attrs) = extract_attributes(&mut fn_.attrs);

    let build_doc = LitStr::new(
        &format!(
            "Sets the `{0}` property.\n\nSee [the module level documentation]({0}) for more.",
            prop_ident
        ),
        Span::call_site(),
    );

    let output = quote! {
        #(#docs_attrs)*
        #vis mod #prop_ident {
            use super::*;

            #[doc(hidden)]
            pub struct Args<#(#arg_gen_types),*> {
                #(pub #arg_names: #arg_gen_types),*
            }
            impl<#(#arg_gen_types),*>  Args<#(#arg_gen_types),*> {
                pub fn pop(self) -> (#(#arg_gen_types),*) {
                    (#(self.#arg_names),*)
                }
            }

            #(#other_attrs)*
            #[doc=#build_doc]
            #[inline]
            #fn_
            #(#sorted_sets)*
        }
    };

    output.into()
}

#[derive(PartialEq)]
enum Priority {
    ContextVar,
    Event,
    Outer,
    Inner,
}

impl Parse for Priority {
    fn parse(input: ParseStream) -> Result<Self> {
        let parsed: Ident = input.parse()?;

        if parsed == ident("context_var") {
            Ok(Priority::ContextVar)
        } else if parsed == ident("event") {
            Ok(Priority::Event)
        } else if parsed == ident("outer") {
            Ok(Priority::Outer)
        } else if parsed == ident("inner") {
            Ok(Priority::Inner)
        } else {
            Err(Error::new(
                parsed.span(),
                format!(
                    "expected `context_var`, `event`, `outer` or `inner` found `{}`",
                    quote!(#parsed)
                ),
            ))
        }
    }
}
