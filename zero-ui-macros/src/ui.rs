use proc_macro2::{Punct, Spacing, Span, TokenStream};
use proc_macro_error::*;
use quote::{ToTokens, TokenStreamExt};
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{
    ext::IdentExt,
    parse::*,
    punctuated::Punctuated,
    token::{Brace, Token},
    *,
};

include!("util.rs");

/// `ui_widget!` implementation.
pub(crate) fn expand_ui_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as UiWidgetInput);

    let mut fn_ = input.fn_;

    if fn_.sig.output == ReturnType::Default {
        abort!(fn_.sig.span(), "Function must return an Ui")
    }

    let (docs_attrs, other_attrs) = extract_attributes(&mut fn_.attrs);

    let vis = match fn_.vis {
        Visibility::Public(_) => {
            quote! {
                #[macro_export]
            }
        }
        _ => TokenStream::new(),
    };
    let ident = fn_.sig.ident.clone();
    let mut arg_names = vec![];

    if fn_.sig.inputs.is_empty() {
        abort!(
            fn_.sig.span(),
            "Function must take a child: impl Ui first and at least one other argument."
        );
    } else if let Some(FnArg::Receiver(_)) = fn_.sig.inputs.first() {
        abort!(fn_.span(), "Function must free-standing.");
    } else {
        for arg in fn_.sig.inputs.iter().skip(1) {
            if let FnArg::Typed(pat) = arg {
                if let Pat::Ident(pat) = &*pat.pat {
                    arg_names.push(pat.ident.clone());
                } else {
                    abort!(arg.span(), "Widget arguments does not support patten deconstruction.");
                }
            } else {
                abort!(arg.span(), "Unexpected `self`.");
            }
        }
    }

    let child_properties = input.child_properties;
    let self_properties = input.self_properties;

    let macro_args = quote_spanned! {ident.span()=>
        ($($tt:tt)*)
    };

    let result = quote! {
        #(#docs_attrs)*
        #vis
        macro_rules! #ident {
            #macro_args => {
                custom_ui!{
                    #child_properties
                    #self_properties
                    args {$($tt)*}
                    fn #ident(#(#arg_names),*)
                }
            };
        }

        #[doc(hidden)]
        #[inline]
         #(#other_attrs)*
        #fn_
    };

    result.into()
}

/// `#[ui_property]` implementation.
pub(crate) fn expand_ui_property(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut fn_ = parse_macro_input!(input as ItemFn);
    let ident = fn_.sig.ident.clone();
    let vis = fn_.vis;

    fn_.sig.ident = Ident::new("set", ident.span());
    fn_.vis = Visibility::Public(VisPublic {
        pub_token: syn::token::Pub {
            span: Span::call_site(),
        },
    });

    if fn_.sig.output == ReturnType::Default {
        abort_call_site!("Function must return an Ui")
    }

    let mut arg_names = vec![];
    let mut arg_gen_types = vec![];

    if fn_.sig.inputs.len() < 2 {
        abort_call_site!("Function must take a child: impl Ui first and at least one other argument.");
    } else if let Some(FnArg::Receiver(_)) = fn_.sig.inputs.first() {
        abort_call_site!("Function must free-standing.");
    } else {
        for arg in fn_.sig.inputs.iter().skip(1) {
            if let FnArg::Typed(pat) = arg {
                if let Pat::Ident(pat) = &*pat.pat {
                    arg_names.push(pat.ident.clone());
                    arg_gen_types.push(self::ident(&format!("T{}", arg_gen_types.len() + 1)))
                } else {
                    abort!(arg.span(), "Property arguments does not support patten deconstruction.");
                }
            } else {
                abort!(arg.span(), "Unexpected `self`.");
            }
        }
    }

    let (docs_attrs, other_attrs) = extract_attributes(&mut fn_.attrs);

    expand_ui_property_output(docs_attrs, vis, ident, arg_gen_types, arg_names, other_attrs, fn_)
}

///-> (docs, other_attrs)
fn extract_attributes(attrs: &mut Vec<Attribute>) -> (Vec<Attribute>, Vec<Attribute>) {
    let mut docs = vec![];
    let mut other_attrs = vec![];

    let doc_ident = ident("doc");
    let inline_ident = ident("inline");

    for attr in attrs.drain(..) {
        if let Some(ident) = attr.path.get_ident() {
            if ident == &doc_ident {
                docs.push(attr);
                continue;
            } else if ident == &inline_ident {
                continue;
            }
        }
        other_attrs.push(attr);
    }

    (docs, other_attrs)
}

fn expand_ui_property_output(
    item_docs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    arg_gen_types: Vec<Ident>,
    arg_names: Vec<Ident>,
    other_attrs: Vec<Attribute>,
    fn_: ItemFn,
) -> proc_macro::TokenStream {
    let build_doc = LitStr::new(
        &format!(
            "Sets the `{0}` property.\n\nSee [the module level documentation]({0}) for more.",
            ident
        ),
        Span::call_site(),
    );

    let output = quote! {
        #(#item_docs)*
        #vis mod #ident {
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
        }
    };

    output.into()
}

/// `ui! {}` implementation.
pub(crate) fn gen_ui_init(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gen_ui_impl(input, false)
}

/// `ui_part!{}` implementation.
pub(crate) fn gen_ui_part_init(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gen_ui_impl(input, true)
}

fn gen_ui_impl(input: proc_macro::TokenStream, is_ui_part: bool) -> proc_macro::TokenStream {
    let UiInput { properties, child, .. } = parse_macro_input!(input as UiInput);

    let mut expanded_props = Vec::with_capacity(properties.len());
    let mut id = quote! {$crate::core::UiItemId::new_unique()};

    let id_name = ident("id");

    for p in properties {
        let name = p.name;

        match p.args {
            PropertyArgs::Exprs(args) => {
                let args = args.into_iter();

                if name == id_name {
                    if is_ui_part {
                        abort!(name.span(), "cannot set `id` in `ui_part`");
                    }

                    id = quote!(#(#args),*);
                } else {
                    expanded_props.push(quote! {
                        let child = #name::set(child, #(#args),*);
                    });
                }
            }
            PropertyArgs::Fields(fields) => {
                expanded_props.push(expand_property_args_fields(name, fields));
            }
        }
    }

    let result = if is_ui_part {
        quote! {{
            let child = #child;
            #(#expanded_props)*
            child
        }}
    } else {
        quote! {{
            let child = #child;
            #(#expanded_props)*
            $crate::primitive::ui_item(child, #id)
        }}
    };

    result.into()
}

/// `custom_ui!` implementation.
pub(crate) fn gen_custom_ui_init(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let CustomUiInput {
        child,
        fn_name,
        child_properties,
        self_properties,
        args,
        fn_arg_names,
        ..
    } = parse_macro_input!(input as CustomUiInput);

    let mut args: HashMap<_, _> = args.into_iter().map(|p| (p.name.clone(), p)).collect();

    // takes required arguments.
    let expanded_fn_args: Vec<_> = fn_arg_names
        .into_iter()
        .map(|a| {
            if let Some(p) = args.remove(&a) {
                if let PropertyArgs::Exprs(exprs) = p.args {
                    if exprs.len() > 1 {
                        // arg: 10, 20;
                        abort!(p.name.span(), "expected single unamed argument for `{}`", p.name);
                    } else {
                        // arg: 10;
                        exprs.into_iter().next().unwrap()
                    }
                } else {
                    // arg: {
                    //     sub_arg: 10;
                    // };
                    abort!(p.name.span(), "expected single unamed argument for `{}`", p.name);
                }
            } else {
                abort_call_site!("missing required parameter `{}`", a)
            }
        })
        .collect();

    // takes or generates properties.
    let child_properties = take_properties(&mut args, child_properties.properties);
    let self_properties = take_properties(&mut args, self_properties.properties);

    let mut expanded_child_props = Vec::with_capacity(child_properties.len());
    let mut expanded_props = Vec::with_capacity(self_properties.len() + args.len());

    let mut id = quote! {$crate::core::UiItemId::new_unique()};
    let id_name = ident("id");

    for p in child_properties {
        let name = p.name;

        match p.args {
            PropertyArgs::Exprs(args) => {
                let args = args.into_iter();

                expanded_child_props.push(quote! {
                    let child = #name::set(child, #(#args),*);
                });
            }
            PropertyArgs::Fields(fields) => {
                expanded_child_props.push(expand_property_args_fields(name, fields));
            }
        }
    }

    for p in self_properties.into_iter().chain(args.into_iter().map(|(_, v)| v)) {
        let name = p.name;

        match p.args {
            PropertyArgs::Exprs(args) => {
                let args = args.into_iter();

                if name == id_name {
                    id = quote!(#(#args),*);
                } else {
                    expanded_props.push(quote! {
                        let child = #name::set(child, #(#args),*);
                    });
                }
            }
            PropertyArgs::Fields(fields) => {
                expanded_props.push(expand_property_args_fields(name, fields));
            }
        }
    }

    // let child = text("Hello");
    // let child = margin::set(child, 4.0);
    // let child = prop::set(child, 4.0);

    let result = quote! {
        let child = #child;
        #(#expanded_child_props)*
        let child = #fn_name(child, #(#expanded_fn_args),*);
        #(#expanded_props)*
        $crate::primitive::ui_item(child, #id)
    };

    result.into()
}

fn expand_property_args_fields(property_name: Ident, fields: Punctuated<FieldValue, Token![,]>) -> TokenStream {
    let args: Vec<_> = (1..=fields.len()).map(|i| ident(&format!("arg_{}", i))).collect();

    quote! {
        let child = {
            let (#(#args),*) = #property_name::Args {
                #fields
            }.pop();

            #property_name::set(child, #(#args),*)
        };
    }
}

fn take_properties(
    args: &mut HashMap<Ident, Property>,
    properties: Punctuated<CustomUiProperty, Token![;]>,
) -> Vec<Property> {
    properties
        .into_iter()
        .filter_map(|pd| {
            if let Some(p) = args.remove(&pd.ident) {
                Some(Property {
                    name: pd.maps_to.unwrap_or(pd.ident),
                    args: p.args,
                })
            } else if let Some(default_value) = pd.default_value {
                Some(Property {
                    name: pd.maps_to.unwrap_or(pd.ident),
                    args: default_value,
                })
            } else {
                None
            }
        })
        .collect()
}

enum PropertyArgs {
    Fields(Punctuated<FieldValue, Token![,]>),
    Exprs(Punctuated<Expr, Token![,]>),
}

impl Parse for PropertyArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let r = if input.peek(token::Brace) {
            let inner;
            braced!(inner in input);
            PropertyArgs::Fields(Punctuated::parse_separated_nonempty(&inner)?)
        } else {
            PropertyArgs::Exprs(Punctuated::parse_separated_nonempty(input)?)
        };

        Ok(r)
    }
}

impl ToTokens for PropertyArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            PropertyArgs::Fields(fields) => {
                Brace { span: fields.span() }.surround(tokens, |t| fields.to_tokens(t));
            }
            PropertyArgs::Exprs(args) => args.to_tokens(tokens),
        }
    }
}

struct Property {
    name: Ident,
    args: PropertyArgs,
}

impl Parse for Property {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let args = input.parse()?;
        Ok(Property { name, args })
    }
}

struct UiInput {
    properties: Punctuated<Property, Token![;]>,
    child: Expr,
}

impl Parse for UiInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let properties = parse_properties(input)?;
        input.parse::<Token![=>]>()?;
        let child = input.parse()?;

        Ok(UiInput { properties, child })
    }
}

struct CustomUiProperty {
    ident: Ident,
    maps_to: Option<Ident>,
    default_value: Option<PropertyArgs>,
}

impl Parse for CustomUiProperty {
    fn parse(input: ParseStream) -> Result<Self> {
        // property -> actual_property: DEFAULT;
        // OR
        // property -> actual_property;
        // OR
        // property: DEFAULT;

        let ident = input.parse()?;

        let maps_to = if input.peek(Token![->]) {
            input.parse::<Token![->]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        let default_value = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(CustomUiProperty {
            ident,
            default_value,
            maps_to,
        })
    }
}
impl ToTokens for CustomUiProperty {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);

        if let Some(maps_to) = &self.maps_to {
            tokens.append(Punct::new('-', Spacing::Joint));
            tokens.append(Punct::new('>', Spacing::Alone));

            maps_to.to_tokens(tokens);
        }

        if let Some(default_value) = &self.default_value {
            tokens.append(Punct::new(':', Spacing::Alone));
            default_value.to_tokens(tokens);
        }
    }
}

struct CustomUiProperties {
    ident: Ident,
    properties: Punctuated<CustomUiProperty, Token![;]>,
}

impl CustomUiProperties {
    fn empty(ident: Ident) -> Self {
        CustomUiProperties {
            ident,
            properties: Punctuated::new(),
        }
    }
}

impl Parse for CustomUiProperties {
    fn parse(input: ParseStream) -> Result<Self> {
        // child_properties { CustomUiProperty }

        let ident = input.parse()?;

        let inner;
        braced!(inner in input);

        let properties = Punctuated::parse_terminated(&inner)?;

        Ok(CustomUiProperties { ident, properties })
    }
}

impl ToTokens for CustomUiProperties {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        Brace {
            span: self.properties.span(),
        }
        .surround(tokens, |t| self.properties.to_tokens(t));
    }
}
mod keyword {
    syn::custom_keyword!(child_properties);
    syn::custom_keyword!(self_properties);
}

struct UiWidgetInput {
    child_properties: CustomUiProperties,
    self_properties: CustomUiProperties,
    fn_: ItemFn,
}

impl Parse for UiWidgetInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut child_properties = None;
        let mut self_properties = None;
        let mut fn_ = None;

        while !input.is_empty() {
            if input.peek(keyword::child_properties) {
                if child_properties.is_some() {
                    let span = input.parse::<Ident>()?.span();
                    return Err(Error::new(span, "`child_properties` is defined multiple times"));
                }
                child_properties = Some(input.parse()?)
            } else if input.peek(keyword::self_properties) {
                if self_properties.is_some() {
                    let span = input.parse::<Ident>()?.span();
                    return Err(Error::new(span, "`self_properties` is defined multiple times"));
                }
                self_properties = Some(input.parse()?)
            } else {
                let item = input.parse::<Item>()?;
                match item {
                    Item::Fn(f) => {
                        if fn_.is_some() {
                            return Err(Error::new(f.span(), "ui_widget! only supports one function"));
                        }
                        fn_ = Some(f)
                    },
                    Item::Use(u) => { todo!()}
                    item => return Err(Error::new(item.span(), "unexpected token")),
                }
            }
        }

        if let Some(fn_) = fn_ {
            Ok(UiWidgetInput {
                child_properties: child_properties
                    .unwrap_or_else(|| CustomUiProperties::empty(ident("child_properties"))),
                self_properties: self_properties.unwrap_or_else(|| CustomUiProperties::empty(ident("self_properties"))),
                fn_,
            })
        } else {
            Err(Error::new(Span::call_site(), "expected a function declaration"))
        }
    }
}

struct CustomUiInput {
    child_properties: CustomUiProperties,
    self_properties: CustomUiProperties,
    args: Punctuated<Property, Token![;]>,
    fn_name: Ident,
    fn_arg_names: Punctuated<Ident, Token![,]>,
    child: Expr,
}

impl Parse for CustomUiInput {
    fn parse(input: ParseStream) -> Result<Self> {
        // child_properties {
        //     padding -> margin;
        //     content_align -> align: CENTER;
        //     background_color: rgb(0, 0, 0);
        // }
        //
        // self_properties {
        //     border: border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
        // }
        //
        // args {
        //     // same as ui!{..}
        //     ident: expr;
        //     => expr
        // }
        //
        // fn button(on_click);

        let child_properties = input.parse()?;
        let self_properties = input.parse()?;

        input.parse::<Ident>()?; // args { #same_as_ui! }
        let inner;
        braced!(inner in input);
        let args = parse_properties(&inner)?;
        inner.parse::<Token![=>]>()?;
        let child = inner.parse()?;

        input.parse::<Token![fn]>()?;
        let fn_name = input.parse()?;

        let inner;
        parenthesized!(inner in input);

        let fn_arg_names = Punctuated::parse_separated_nonempty(&inner)?;

        Ok(CustomUiInput {
            child_properties,
            self_properties,
            args,
            fn_name,
            fn_arg_names,
            child,
        })
    }
}

fn parse_properties(input: ParseStream) -> Result<Punctuated<Property, Token![;]>> {
    let mut punctuated = Punctuated::new();

    loop {
        let value = input.parse()?;
        punctuated.push_value(value);
        let punct = input.parse()?;
        punctuated.push_punct(punct);
        if <Token![=>]>::peek(input.cursor()) {
            break;
        }
    }

    Ok(punctuated)
}
