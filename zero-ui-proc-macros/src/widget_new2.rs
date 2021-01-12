use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    parse::{discouraged::Speculative, Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    Expr, FieldValue, Ident, LitBool, Path, Token,
};

use crate::util::{non_user_braced, non_user_braced_id, non_user_parenthesized, Errors};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = match syn::parse::<Input>(input) {
        Ok(i) => i,
        Err(e) => non_user_error!(e),
    };

    todo!()
}

struct Input {
    mod_path: Path,
    widget_data: WidgetData,
    user_input: UserInput,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Input {
            mod_path: input.parse().unwrap_or_else(|e| non_user_error!(e)),
            widget_data: input.parse().unwrap_or_else(|e| non_user_error!(e)),
            // user errors go into UserInput::errors field.
            user_input: input.parse().unwrap_or_else(|e| non_user_error!(e)),
        })
    }
}

struct WidgetData {
    child_properties: Vec<BuiltProperty>,
    properties: Vec<BuiltProperty>,
    whens: Vec<BuiltWhen>,
    new_child_caps: Vec<Ident>,
    new_caps: Vec<Ident>,
}
impl Parse for WidgetData {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = non_user_braced_id(input, "widget");

        let mut child_properties = vec![];
        let child_props = non_user_braced_id(&input, "properties_child");
        while !child_props.is_empty() {
            child_properties.push(child_props.parse().unwrap_or_else(|e| non_user_error!(e)));
        }

        let mut properties = vec![];
        let props = non_user_braced_id(&input, "properties");
        while !props.is_empty() {
            properties.push(props.parse().unwrap_or_else(|e| non_user_error!(e)));
        }

        let mut whens = vec![];
        let ws = non_user_braced_id(&input, "whens");
        while !ws.is_empty() {
            whens.push(ws.parse().unwrap_or_else(|e| non_user_error!(e)));
        }

        let mut new_child_caps = vec![];
        let new_child_cs = non_user_braced_id(&input, "new_child");
        while !new_child_cs.is_empty() {
            new_child_caps.push(new_child_cs.parse().unwrap_or_else(|e| non_user_error!(e)));
            new_child_cs.parse::<Token![,]>().ok();
        }

        let mut new_caps = vec![];
        let new_cs = non_user_braced_id(&input, "new");
        while !new_cs.is_empty() {
            new_caps.push(new_cs.parse().unwrap_or_else(|e| non_user_error!(e)));
            new_cs.parse::<Token![,]>().unwrap_or_else(|e| non_user_error!(e));
        }

        Ok(WidgetData {
            child_properties,
            properties,
            whens,
            new_child_caps,
            new_caps,
        })
    }
}

struct BuiltProperty {
    ident: Ident,
    has_default: bool,
    is_required: bool,
}
impl Parse for BuiltProperty {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced(input);

        let flag = |ident| {
            let id: Ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
            if id != ident {
                non_user_error!(format!("expected `{}`", ident));
            }
            let flag: LitBool = input.parse().unwrap_or_else(|e| non_user_error!(e));
            input.parse::<Token![,]>().ok();
            flag.value
        };

        Ok(BuiltProperty {
            ident,
            has_default: flag("default"),
            is_required: flag("required"),
        })
    }
}

struct BuiltWhen {
    ident: Ident,
    expr_properties: Vec<Ident>,
    set_properties: Vec<Ident>,
}
impl Parse for BuiltWhen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));

        let mut expr_properties = vec![];
        let expr = non_user_parenthesized(input);
        while !expr.is_empty() {
            expr_properties.push(expr.parse().unwrap_or_else(|e| non_user_error!(e)));
            expr.parse::<Token![,]>().ok();
        }

        let mut set_properties = vec![];
        let set = non_user_braced(input);
        while !set.is_empty() {
            set_properties.push(set.parse().unwrap_or_else(|e| non_user_error!(e)));
            set.parse::<Token![,]>().ok();
        }

        Ok(BuiltWhen {
            ident,
            expr_properties,
            set_properties,
        })
    }
}

struct UserInput {
    errors: Errors,
    properties: Vec<PropertyAssign>,
    whens: Vec<When>,
}
impl Parse for UserInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut errors = Errors::default();
        let mut properties = vec![];
        let mut whens = vec![];

        while !input.is_empty() {
            if input.peek(keyword::when) {
                if let Some(when) = When::parse(input, &mut errors) {
                    whens.push(when);
                }
            } else if input.peek(Ident) {
                // peek ident or path.
                match input.parse() {
                    Ok(p) => properties.push(p),
                    Err(e) => errors.push_syn(e),
                }
            } else {
                errors.push("expected `when` or a property path", input.span());
                break;
            }
        }

        Ok(UserInput { errors, properties, whens })
    }
}

pub struct PropertyAssign {
    pub path: Path,
    pub eq: Token![=],
    pub value: PropertyValue,
    pub semi: Option<Token![;]>,
}
impl Parse for PropertyAssign {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;
        let eq = input.parse()?;

        // the value is terminated by the end of `input` or by a `;` token.
        let mut value_stream = TokenStream::new();
        let mut semi = None;
        while !input.is_empty() {
            if input.peek(Token![;]) {
                semi = input.parse().unwrap();
                break;
            } else {
                let tt: TokenTree = input.parse().unwrap();
                tt.to_tokens(&mut value_stream);
            }
        }

        Ok(PropertyAssign {
            path,
            eq,
            value: syn::parse2(value_stream)?,
            semi,
        })
    }
}

pub enum PropertyValue {
    /// `unset!` or `required!`.
    Special(Ident, Token![!]),
    /// `arg0, arg1,`
    Unnamed(Punctuated<Expr, Token![,]>),
    /// `{ field0: true, field1: false, }`
    Named(syn::token::Brace, Punctuated<FieldValue, Token![,]>),
}
impl PropertyValue {
    pub fn is_unset(&self) -> bool {
        match self {
            PropertyValue::Special(sp, _) => sp == "unset",
            _ => false,
        }
    }
}
impl Parse for PropertyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) && input.peek2(Token![!]) {
            // input stream can be `unset!` with no third token.
            let unset = input.fork();
            let r = PropertyValue::Special(unset.parse().unwrap(), unset.parse().unwrap());
            if unset.is_empty() {
                input.advance_to(&unset);
                return Ok(r);
            }
        }

        if input.peek(syn::token::Brace) {
            // Differentiating between a fields declaration and a single unnamed arg declaration gets tricky.
            //
            // This is a normal fields decl.: `{ field0: "value" }`
            // This is a block single argument decl.: `{ foo(); bar() }`
            //
            // Fields can use the shorthand field name only `{ field0 }`
            // witch is also a single arg block expression. In this case
            // we parse as Unnamed, if it was a field it will still work because
            // we only have one field.

            let maybe_fields = input.fork();
            let fields_input;
            let fields_brace = braced!(fields_input in maybe_fields);

            if maybe_fields.is_empty() {
                // is only block in assign, still can be a block expression.
                if fields_input.peek(Ident) && (fields_input.peek2(Token![:]) || fields_input.peek2(Token![,])) {
                    // is named fields, { field: .. } or { field, .. }.
                    input.advance_to(&maybe_fields);
                    Ok(PropertyValue::Named(fields_brace, Punctuated::parse_terminated(&fields_input)?))
                } else {
                    // is an unnamed block expression or { field } that works as an expression.
                    Ok(PropertyValue::Unnamed(Punctuated::parse_terminated(input)?))
                }
            } else {
                // first arg is a block expression but has other arg expression e.g: `{ <expr> }, ..`
                Ok(PropertyValue::Unnamed(Punctuated::parse_terminated(input)?))
            }
        } else {
            Ok(PropertyValue::Unnamed(Punctuated::parse_terminated(input)?))
        }
    }
}

pub struct When {
    pub when: keyword::when,
    pub condition_expr: Expr,
    pub brace_token: syn::token::Brace,
    pub assigns: Vec<PropertyAssign>,
}
impl When {
    /// Call only if peeked `when`.
    fn parse(input: ParseStream, errors: &mut Errors) -> Option<When> {
        let mut any_error = false;
        let mut push_error = |e| {
            errors.push_syn(e);
            any_error = true;
        };

        let when = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let condition_expr = match Expr::parse_without_eager_brace(input) {
            Ok(x) => x,
            Err(e) => {
                push_error(e);
                parse_quote! { false }
            }
        };

        let (brace_token, assigns) = if input.peek(syn::token::Brace) {
            let brace = syn::group::parse_braces(input).unwrap();
            let mut assigns = vec![];
            while !brace.content.is_empty() {
                match brace.content.parse() {
                    Ok(p) => assigns.push(p),
                    Err(e) => errors.push_syn(e),
                }
            }
            (brace.token, assigns)
        } else {
            errors.push("expected `{ <property-assigns> }`", input.span());
            return None;
        };

        if any_error {
            None
        } else {
            Some(When {
                when,
                condition_expr,
                brace_token,
                assigns,
            })
        }
    }
}

pub mod keyword {
    syn::custom_keyword!(when);
}
