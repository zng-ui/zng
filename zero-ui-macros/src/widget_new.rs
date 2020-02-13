use crate::widget::*;
use proc_macro2::{Span, TokenStream};
use std::collections::HashMap;
use syn::{parse::*, *};

include!("util.rs");

/// `widget_new!` implementation
#[allow(clippy::cognitive_complexity)]
pub fn expand_widget_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as WidgetNewInput);

    let child = input.user_child_expr;
    let imports = input.imports;
    let imports = quote!(#(#imports)*);

    let mut user_sets: HashMap<_, _> = input.user_sets.into_iter().map(|pa| (pa.ident.clone(), pa)).collect();

    let PropertyCalls {
        let_args: let_child_args,
        set_ctx: set_child_props_ctx,
        set_event: set_child_props_event,
        set_outer: set_child_props_outer,
        set_inner: set_child_props_inner,
    } = match make_property_calls(&imports, input.default_child, &mut user_sets) {
        Ok(p) => p,
        Err(e) => abort_call_site!("{}", e),
    };

    let mut self_calls = match make_property_calls(&imports, input.default_self, &mut user_sets) {
        Ok(p) => p,
        Err(e) => abort!(e.span(), "{}", e),
    };

    let let_id = if let Some(p) = user_sets.remove(&self::ident("id")) {
        match p.value {
            PropertyValue::Args(a) => quote!(let __id = zero_ui::core::validate_widget_id_args(#a)),
            PropertyValue::Fields(a) => quote!(let __id = zero_ui::core::ValidateWidgetIdArgs{#a}.id),
            PropertyValue::Unset => abort_call_site!("cannot unset id"),
        }
    } else {
        quote!(let __id = zero_ui::core::types::WidgetId::new_unique();)
    };

    for (ident, assign) in user_sets {
        make_property_call(ident, assign.value, &mut self_calls, &imports, false);
    }

    let PropertyCalls {
        let_args: let_self_args,
        set_ctx: mut set_self_props_ctx,
        set_event: mut set_self_props_event,
        set_outer: mut set_self_props_outer,
        set_inner: mut set_self_props_inner,
    } = self_calls;

    let ident = input.ident;

    let r = quote! {{
        let __node = #child;
        #(#let_child_args)*

        let __node = {
            #imports

            #(#set_child_props_ctx)*
            #(#set_child_props_event)*
            #(#set_child_props_outer)*
            #(#set_child_props_inner)*

            #ident::child(__node)
        };

        #(#let_self_args)*
        #let_id

        {
            #imports

            #(#set_self_props_ctx)*
            #(#set_self_props_event)*
            #(#set_self_props_outer)*
            #(#set_self_props_inner)*

            zero_ui::core::widget(__id, __node)
        }
    }};

    r.into()
}

fn make_property_calls(
    imports: &TokenStream,
    default: DefaultBlock,
    user_sets: &mut HashMap<Ident, PropertyAssign>,
) -> Result<PropertyCalls> {
    let mut r = PropertyCalls::default();

    for default in default.properties {
        let (value, default_value) = if let Some(p) = user_sets.remove(&default.ident) {
            (p.value, false)
        } else if let Some(d) = default.default_value {
            (
                match d {
                    PropertyDefaultValue::Args(a) => PropertyValue::Args(a),
                    PropertyDefaultValue::Fields(a) => PropertyValue::Fields(a),
                    PropertyDefaultValue::Unset => PropertyValue::Unset,
                    PropertyDefaultValue::Required => {
                        return Err(Error::new(Span::call_site(), format!("property `{}` is required", default.ident)))
                    }
                },
                true,
            )
        } else {
            // no default value and user did not set
            continue;
        };

        let ident = default.maps_to.unwrap_or(default.ident);

        if make_property_call(ident, value, &mut r, imports, default_value) {
            continue; // unset
        }
    }

    Ok(r)
}

type Unset = bool;

fn make_property_call(ident: Ident, value: PropertyValue, r: &mut PropertyCalls, imports: &TokenStream, default_value: bool) -> Unset {
    macro_rules! arg {
        ($n:expr) => {
            self::ident(&format!("__{}_arg_{}", ident, $n))
        };
    }

    let len = match value {
        PropertyValue::Args(a) => {
            for (i, a) in a.iter().enumerate() {
                let arg = arg!(i);

                if default_value {
                    r.let_args.push(quote! {
                        let #arg = {
                            #imports
                            #a
                        };
                    });
                } else {
                    r.let_args.push(quote!(let #arg = #a;));
                }
            }
            a.len()
        }
        PropertyValue::Fields(f) => {
            let len = f.len();

            let args = (0..len).map(|i| arg!(i));
            if default_value {
                r.let_args.push(quote! {
                    let (#(#args),*) = {
                        #imports
                        #ident::Args {
                            #f
                        }.pop()
                    };
                });
            } else {
                r.let_args.push(quote! {
                    let (#(#args),*) = #ident::Args {
                        #f
                    }.pop();
                });
            }

            len
        }
        PropertyValue::Unset => return true,
    };

    let args = (0..len).map(|i| arg!(i));
    let args = quote!(__node, #(#args),*);

    r.set_ctx.push(quote!(let (#args) = #ident::set_context_var(#args);));
    r.set_event.push(quote!(let (#args) = #ident::set_event(#args);));
    r.set_outer.push(quote!(let (#args) = #ident::set_outer(#args);));
    r.set_inner.push(quote!(let (#args) = #ident::set_inner(#args);));

    false
}

#[derive(Default)]
struct PropertyCalls {
    let_args: Vec<TokenStream>,
    set_ctx: Vec<TokenStream>,
    set_event: Vec<TokenStream>,
    set_outer: Vec<TokenStream>,
    set_inner: Vec<TokenStream>,
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
        input.parse::<Token![;]>()?;

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
                input.parse::<Token![=>]>()?;

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
            panic!("{}: expected default({})", NON_USER_ERROR, quote!(#expected))
        }

        for p in &self.properties {
            if !p.attrs.is_empty() {
                panic!("{}: unexpected attributes", NON_USER_ERROR)
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
