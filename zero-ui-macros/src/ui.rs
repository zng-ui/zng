use proc_macro2::TokenTree;
use proc_macro2::{Punct, Spacing, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use syn::spanned::Spanned;
use syn::{
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
        error!(
            Span::call_site(),
            "Function must take a child: impl Ui first and at least one other argument."
        );
    } else if let Some(FnArg::Receiver(_)) = fn_.sig.inputs.first() {
        error!(Span::call_site(), "Function must free-standing.");
    } else {
        for arg in fn_.sig.inputs.iter().skip(1) {
            if let FnArg::Typed(pat) = arg {
                if let Pat::Ident(pat) = &*pat.pat {
                    arg_names.push(pat.ident.clone());
                } else {
                    error!(arg.span(), "Widget arguments does not support patten deconstruction.");
                }
            } else {
                error!(arg.span(), "Unexpected `self`.");
            }
        }
    }

    let child_properties = input.child_properties;
    let self_properties = input.self_properties;

    let result = quote! {
        #(#docs_attrs)*
        #vis
        macro_rules! #ident {
            ($($tt:tt)*) => {
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

    crate::enum_hack::wrap(result).into()
}

/// `#[ui_property]` implementation.
pub(crate) fn expand_ui_property(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut fn_ = parse_macro_input!(input as ItemFn);
    let ident = fn_.sig.ident.clone();
    let vis = fn_.vis;

    fn_.sig.ident = Ident::new("build", ident.span());
    fn_.vis = Visibility::Public(VisPublic {
        pub_token: syn::token::Pub {
            span: Span::call_site(),
        },
    });

    if fn_.sig.output == ReturnType::Default {
        error!(Span::call_site(), "Function must return an Ui")
    }

    let mut arg_names = vec![];
    let mut arg_gen_types = vec![];

    if fn_.sig.inputs.len() < 2 {
        error!(
            Span::call_site(),
            "Function must take a child: impl Ui first and at least one other argument."
        );
    } else if let Some(FnArg::Receiver(_)) = fn_.sig.inputs.first() {
        error!(Span::call_site(), "Function must free-standing.");
    } else {
        for arg in fn_.sig.inputs.iter().skip(1) {
            if let FnArg::Typed(pat) = arg {
                if let Pat::Ident(pat) = &*pat.pat {
                    arg_names.push(pat.ident.clone());
                    arg_gen_types.push(Ident::new(&format!("T{}", arg_gen_types.len() + 1), Span::call_site()))
                } else {
                    error!(arg.span(), "Property arguments does not support patten deconstruction.");
                }
            } else {
                error!(arg.span(), "Unexpected `self`.");
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

    let doc_ident = Ident::new("doc", Span::call_site());
    let inline_ident = Ident::new("inline", Span::call_site());

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
            "Builds the `{0}` property.\n\nSee [the module level documentation]({0}) for more.",
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

    let id_name = Ident::new("id", Span::call_site());

    for p in properties {
        let name = p.name;

        match p.args {
            PropertyArgs::Exprs(args) => {
                let args = args.into_iter();

                if name == id_name {
                    if is_ui_part {
                        error!(name.span(), "cannot set `id` in `ui_part`");
                    }

                    id = quote!(#(#args),*);
                } else {
                    expanded_props.push(quote! {
                        let child = #name::build(child, #(#args),*);
                    });
                }
            }
            PropertyArgs::Fields(fields) => {
                let args: Vec<_> = (1..=fields.len()).map(|i| ident(&format!("arg_{}", i))).collect();

                expanded_props.push(quote! {
                    let child = {
                        let (#(#args),*) = #name::Args {
                            #fields
                        }.pop();

                        #name::build(child, #(#args),*)
                    };
                });
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
            $crate::primitive::ui_item(#id, child)
        }}
    };

    result.into()
}

/// `custom_ui!` implementation.
pub(crate) fn gen_custom_ui_init(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as CustomUiInput);
    // let ui_meta = parse;
    // let args = parse;
    // let fn = parse;

    /*
    #[derive_ui_macro {
            // optional, if not set does not wrap.
            padding => margin(child, $args);
            // or with default, if not set use value within ${}.
            padding => margin(child, ${(5.0, 4.0)});

            // can also any expression?
            padding => ui! {margin: $args};
            // or apply to function result?
            spacing => margin($self, $args);
        }]
    */
    unimplemented!()
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
    _separator: Token![:],
    args: PropertyArgs,
}

impl Parse for Property {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Property {
            name: input.parse()?,
            _separator: input.parse()?,
            args: input.parse()?,
        })
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
        // property-> actual_property: DEFAULT ;
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

struct UiWidgetInput {
    child_properties: CustomUiProperties,
    self_properties: CustomUiProperties,
    fn_: ItemFn,
}

impl Parse for UiWidgetInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let child_properties = input.parse()?;
        let self_properties = input.parse()?;
        let fn_ = input.parse()?;

        Ok(UiWidgetInput {
            child_properties,
            self_properties,
            fn_,
        })
    }
}

struct CustomUiInput {
    child_properties: CustomUiProperties,
    self_properties: CustomUiProperties,
    args: Punctuated<Property, Token![;]>,
    fn_name: Ident,
    fn_arg_names: Punctuated<Ident, Token![,]>,
}

impl Parse for CustomUiInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let child_properties = input.parse()?;
        let self_properties = input.parse()?;
        let args = parse_properties(input)?;
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

#[allow(unused)]
const T: &str = stringify! {
    // declared like:
    #[ui_widget {
        // optional, if not set does not wrap.
        padding => margin(child, $args);
        // or with default, if not set use value within ${}.
        padding => margin(child, ${(5.0, 4.0)});

        // can also any expression?
        padding => ui! {margin: $args};
        // or apply to function result?
        spacing => margin($self, $args);
    }]
    fn button(on_click: impl FnMut(&ClickArgs), child: impl Ui) -> impl Ui {
        ui! {
            background_color: rgb(100, 100, 100);
            on_click: on_click;
            => child
        }
    }

    ui_widget! {
         // optional, if not set does not wrap.
         padding => margin(child, $args);
         // or with default, if not set use value within ${}.
         padding => margin(child, ${(5.0, 4.0)});

         // can also any expression?
         padding => ui! {margin: $args};
         // or apply to function result?
         spacing => margin($self, $args);
        =>
        /// docs
        fn button(on_click: impl FnMut(&ClickArgs), child: impl Ui) -> impl Ui {
            ui! {
                background_color: rgb(100, 100, 100);
                on_click: on_click;
                => child
            }
        }
    }

    // expands to:

    /// function docs?
    #[macro_export]// export if function is pub
    macro_rules! button {
        ($($tt:tt)*) => {
            custom_ui! {
                // these two attributes are not real
                // they are just containers for custom_ui
                #[ui_meta {
                    // derive_ui_macro contents.
                    padding => margin(child, $args);
                    spacing => margin($self, $args);
                }]
                #[args($($tt)*)]
                // function to call, not an actual fn signature,
                // pattern is fn ident(list, of, parameter, idents);
                // child is the first parameter of the function and not
                // included in the pattern.
                fn button(on_click);
            }
        }
    }
    // same function
    fn button(..){..}

    // called like:
    button! {
        on_click: |_| {};// required, fn button arg.
        padding: (5.0, 2.0); // optional, maps to margin around child.
        text_color: rgb(255, 0, 0); // optional, same as ui!{}, around button.
        // parameters can be in any order but are expanded in declaration order.
        => text("content")
    }

    // expands to:
    {
        mod current_module {
            pub(crate) use super::*;
        }

        let child = text("content"); // => content
        let child = margin(child, (5.0, 2.0));// padding expression
        let child = current_module::text_color(child, rgb(255, 0, 0));// ui! like parameter.
        let child = button(_|{}, child);// button call with on_click args.
        // $self ui! like stuff here?
        {child}
    }

    #[ui_property]
    pub fn property(child: impl Ui, param: impl IntoValue<X>) -> impl Ui {
        ..
    }

    // expands to:
    mod property {
        use super::*;

        pub struct Args<T1> {
            param: T1
        }

        pub fn build(child: impl Ui, param: impl IntoValue<X>) -> impl Ui {
            ..
        }
    }

    // calls expands to (if using named args):
    let property_args = current_module::property::Args { #args };
    let child = current_module::property::build(child, property_args.param);
};
