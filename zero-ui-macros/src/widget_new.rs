use crate::widget::*;
use proc_macro2::{Span, TokenStream};
use std::collections::HashMap;
use syn::{parse::*, *};

include!("util.rs");

/// `widget_new!` implementation
pub fn expand_widget_new(input: proc_macro::TokenStream, crate_: Ident) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as WidgetNewInput);

    let child = input.user_child_expr;
    let imports = input.imports;

    let user_sets: HashMap<_, _> = input.user_sets.into_iter().map(|pa| (pa.ident.clone(), pa)).collect();

    let mut let_child_args = vec![];
    let mut set_child_props = vec![];
    for default in input.default_child.properties {
        let value = if let Some(p) = user_sets.remove(&default.ident) {
            p.value
        } else if let Some(d) = default.default_value {
            match d {
                PropertyDefaultValue::Args(a) => PropertyValue::Args(a),
                PropertyDefaultValue::Fields(a) => PropertyValue::Fields(a),
                PropertyDefaultValue::Unset => PropertyValue::Unset,
                PropertyDefaultValue::Required => abort_call_site!("property `{}` is required", default.ident)
            }
        } else {
            // no default value and user did not set
            continue;
        };

        match value {
            PropertyValue::Args(a) => {

            },
            PropertyValue::Fields(f) => {

            },
            PropertyValue::Unset => continue,
        }
    }

    let r = quote! {{
        let __child = #child;
        #(#let_child_args)*

        let __child = {
            #(#imports)*

            #(#set_child_props)*

            #ident::child(__child)
        };

        #(#let_self_args)*
        #let_id

        {
            #(#imports)*

            #(#set_self_props)*

            #crate_::core::widget(__id, __self)
        }
    }};

    r.into()
}

/// Input error not caused by the user.
const NON_USER_ERROR: &str = "invalid non-user input";

struct WidgetNewInput {
    ident: Ident,
    imports: Vec<ItemUse>,
    default_child: DefaultBlock,
    default_self: DefaultBlock,
    whens: Vec<WhenBlock>,
    user_sets: Vec<PropertyAssign>,
    user_whens: Vec<WhenBlock>,
    user_child_expr: Expr,
}
impl Parse for WidgetNewInput {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![mod]>().expect(NON_USER_ERROR);
        let ident = input.parse().expect(NON_USER_ERROR);

        let mut imports = vec![];
        while input.peek(Token![use]) {
            imports.push(input.parse().expect(NON_USER_ERROR));
        }

        let default_child: DefaultBlock = input.parse().expect(NON_USER_ERROR);
        default_child.assert(DefaultBlockTarget::Child);

        let default_self: DefaultBlock = input.parse().expect(NON_USER_ERROR);
        default_self.assert(DefaultBlockTarget::Self_);

        let mut whens = vec![];
        while input.peek(keyword::when) {
            whens.push(input.parse().expect(NON_USER_ERROR));
        }

        input.parse::<keyword::input>().expect(NON_USER_ERROR);
        input.parse::<Token![:]>().expect(NON_USER_ERROR);

        fn input_stream(input: ParseStream) -> Result<ParseBuffer> {
            let inner;
            // this macro inserts a return Err(..) but we want to panic
            braced!(inner in input);
            Ok(inner)
        }
        let input = input_stream(input).expect(NON_USER_ERROR);

        let mut user_sets = vec![];
        let mut user_whens = vec![];
        while !input.is_empty() {
            let lookahead = input.lookahead1();

            // expect `when` at start or after `property:`
            if lookahead.peek(keyword::when) {
                user_whens.push(input.parse()?);
            }
            // expect `property:` only before `when` blocks.
            else if user_whens.is_empty() && lookahead.peek(Ident) {
                user_sets.push(input.parse()?);
            }
            // expect `=>` to be the last item.
            else if lookahead.peek(Token![=>]) {
                return Ok(WidgetNewInput {
                    ident,
                    imports,
                    default_child,
                    default_self,
                    whens,
                    user_sets,
                    user_whens,
                    user_child_expr: input.parse()?,
                });
            } else {
                return Err(lookahead.error());
            }
        }

        // if user input is empty, use a lookahead to make an error message.
        let lookahead = input.lookahead1();
        lookahead.peek(Ident);
        lookahead.peek(keyword::when);
        lookahead.peek(Token![=>]);
        Err(lookahead.error())
    }
}

impl DefaultBlock {
    pub fn assert(&self, expected: DefaultBlockTarget) {
        if self.target != expected {
            panic!("{}, expected default({})", NON_USER_ERROR, quote!(#expected))
        }

        for p in &self.properties {
            if !p.attrs.is_empty() {
                panic!("{}, unexpected attributes", NON_USER_ERROR)
            }
        }
    }
}

macro_rules! demo {
    ($($tt:tt)*) => {};
}

// Input:
demo! {
    /// Docs generated by all the docs attributes and property names.
    #[other_name_attrs]
    #[macro_export]// if pub
    macro_rules! button {
        ($($tt::tt)+) => {
            widget_new! {
                mod button;

                // uses with `crate` converted to `$crate`
                use $crate::something;

                default(child) {
                    // all the default(child) blocks grouped or an empty block
                }
                default(self) {
                    // all the default(self) blocks grouped or an empty block
                }

                // all the when blocks
                when(expr) {}
                when(expr) {}

                // user args
                input: {
                    // zero or more property assigns; required! not allowed.
                    // => child
                    $($tt)+
                }
            }
        };
    }

    #[doc(hidden)]
    pub mod button {
        use super::*;

        // => { child }
        pub fn child(child: impl Ui) -> impl Ui {
            child
        }

        // compile test of the property declarations
        #[allow(unused)]
        fn test(child: impl Ui) -> impl Ui {
            button! {
                => child
            }
        }
    }
}

// Output:
demo! {
    {
        // eval child and args without using the widget imports.
        let __child = child_expr;

        // eval all child properties.
        let __prop1_0 = "eval";
        let __prop1_1 = "eval";
        let (__prop2_0, __prop2_1) = prop2::Args {
            a: 1,
            b: 1,
        }.pop();

        let __child = {
            // build child in a block with the widget imports.
            use crate_name::something;

            // do the static sorting dance for child properties
            let (__child, __prop1_0, __prop1_1) = prop1::set_context_var(__child, __prop1_0, __prop1_1);
            let (__child, __prop2_0, __prop2_1) = prop2::set_context_var(__child, __prop2_0, __prop2_1);
            ..
            let (__child, __prop1_0, __prop1_1) = prop1::set_inner(__child, __prop1_0, __prop1_1);
            let (__child, __prop2_0, __prop2_1) = prop2::set_inner(__child, __prop2_0, __prop2_1);

            // do widget custom child processing.
            button::child(__child)
        };

        // eval all self properties.
        let __prop1_0 = "eval";
        let __id = id_expr;

        {
            // build self in a block with the widget imports.
            use crate_name::something;

            // do the static sorting dance for self properties
            let (__self, __prop1_0) = prop1::set_context_var(__child, __prop1_0);
            ..
            let (__self, __prop1_0) = prop1::set_inner(__child, __prop1_0);

            zero_ui::core::widget(__id, __child)
        }
    }
}
