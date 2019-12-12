use proc_macro2::{Span, TokenStream};
use syn::{parse::*, punctuated::Punctuated, token::Token, *};

pub(crate) fn gen_ui_init(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { properties, child, .. } = parse_macro_input!(input as Input);

    let mut expanded_props = Vec::with_capacity(properties.len());
    let mut id = quote! {$crate::core::UiItemId::new_unique()};

    let id_name = Ident::new("id", Span::call_site());

    for p in properties {
        let name = p.name;
        let args = p.args.into_iter();

        if name == id_name {
            id = quote!(#(#args),*);
        } else {
            expanded_props.push(quote! {
                let child = current_module::#name(child, #(#args),*);
            });
        }
    }

    let result = quote! {{
        mod current_module {
            pub(crate) use super::*;
        }
        let child = #child;
        #(#expanded_props)*

        $crate::primitive::ui_item(#id, child)
    }};

    result.into()
}

pub(crate) fn gen_derive_hack(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = TokenStream::from(args);
    let input = TokenStream::from(input);
    // check if input is function
    // check if is pub
    // check if takes last param child: impl Ui -> impl Ui
    //

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

    let result = quote! {
        #[doc(..)]
        #[macro_export]// export if function is pub
        macro_rules! button {
            ($($tt:tt)*) => {
                custom_ui! {
                    // these two attributes are not real
                    // they are just containers for custom_ui
                    #[ui_meta {
                       #args
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

        #[doc(hidden)]
        #input
    };

    crate::enum_hack::wrap(result).into()
}

pub(crate) fn gen_custom_ui_init(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // let ui_meta = parse;
    // let args = parse;
    // let fn = parse;
    unimplemented!()
}

struct Property {
    name: Ident,
    _separator: Token![:],
    args: Punctuated<Expr, Token![,]>,
}

impl Parse for Property {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Property {
            name: input.parse()?,
            _separator: input.parse()?,
            args: Punctuated::parse_separated_nonempty(input)?,
        })
    }
}

struct Input {
    properties: Punctuated<Property, Token![;]>,
    _separator: Token![=>],
    child: Expr,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Input {
            properties: parse_properties(input)?,
            _separator: input.parse()?,
            child: input.parse()?,
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
    fn button(on_click: impl FnMut(&ClickArgs), child: impl Ui) -> impl Ui {
        ui! {
            background_color: rgb(100, 100, 100);
            on_click: on_click;
            => child
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
};
