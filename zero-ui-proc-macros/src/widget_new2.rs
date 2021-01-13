use std::collections::HashSet;

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    parse::{discouraged::Speculative, Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, FieldValue, Ident, LitBool, Path, Token,
};

use crate::util::{display_path, non_user_braced, non_user_braced_id, Errors};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { widget_data, user_input } = match syn::parse::<Input>(input) {
        Ok(i) => i,
        Err(e) => non_user_error!(e),
    };

    let widget_mod = widget_data.mod_path;

    let mut errors = Errors::default();

    let mut child_ps_init = inherited_inits(&widget_mod, widget_data.child_properties);
    let mut wgt_ps_init = inherited_inits(&widget_mod, widget_data.properties);
    let mut user_ps_init = vec![];
    let mut user_assigns = HashSet::new();

    for p in user_input.properties {
        if !user_assigns.insert(p.path.clone()) {
            errors.push(&format!("property `{}` already assigned", display_path(&p.path)), p.path.span());
            continue;
        }

        // does `unset!`
        if let PropertyValue::Special(sp, _) = p.value {
            if sp == "unset" {
                if let Some(p_ident) = p.path.get_ident() {
                    let mut required = false;
                    if let Some(i) = child_ps_init.iter().position(|(id, _, _)| id == p_ident) {
                        required = child_ps_init[i].2;
                        if !required {
                            child_ps_init.remove(i);
                            user_assigns.remove(&p.path); // so the user can set a custom property with the same name after?
                            continue; // done
                        }
                    }
                    if let Some(i) = wgt_ps_init.iter().position(|(id, _, _)| id == p_ident) {
                        required = child_ps_init[i].2;
                        if !required {
                            wgt_ps_init.remove(i);
                            user_assigns.remove(&p.path);
                            continue; // done
                        }
                    }

                    if required {
                        errors.push(
                            &format!("cannot unset property `{}` because it is required", p_ident),
                            p_ident.span(),
                        );
                        continue; // skip invalid
                    }
                }
                // else property not previously set:
                errors.push(
                    &format!("cannot unset property `{}` because it is not set", display_path(&p.path)),
                    p.path.span(),
                );
                continue; // skip invalid
            } else {
                errors.push(&format!("value `{}` is not valid in this context", sp), sp.span());
                continue; // skip invalid
            }
        }

        if let Some(ident) = p.path.get_ident() {
            let target = child_ps_init
                .iter_mut()
                .find(|(id, _, _)| id == ident)
                .or_else(|| wgt_ps_init.iter_mut().find(|(id, _, _)| id == ident));
            if let Some((_, existing, _)) = target {
                let var_ident = ident!("__{}", ident);
                let p_ident = ident!("__p_{}", ident);

                // replace default value.
                *existing = match p.value {
                    PropertyValue::Unnamed(args) => quote! {
                        let #var_ident = #widget_mod::#p_ident::ArgsImpl::new(#args);
                    },
                    PropertyValue::Named(_, fields) => {
                        let property_path = quote! { #widget_mod::#p_ident };
                        quote! {
                            let #var_ident = #property_path::code_gen! { named_new #property_path {
                                #fields
                            }};
                        }
                    }
                    PropertyValue::Special(_, _) => unreachable!(),
                };
                continue; // replaced existing.
            }
        }

        // else is custom property.
        let var_ident = ident!("__{}", display_path(&p.path).replace("::", "__"));
        let property_path = p.path;
        user_ps_init.push(match p.value {
            PropertyValue::Unnamed(args) => quote! {
                let #var_ident = #property_path::ArgsImpl::new(#args);
            },
            PropertyValue::Named(_, fields) => quote! {
                let #var_ident = #property_path::code_gen!{ named_new #property_path { #fields } };
            },
            PropertyValue::Special(_, _) => unreachable!(),
        });
    }

    // add errors for missing required.
    for (ident, value, required) in child_ps_init.iter().chain(wgt_ps_init.iter()) {
        if *required && value.is_empty() {
            errors.push(&format!("required property `{}` not set", ident), ident.span());
        }
    }

    // property initializers in order they must be called.
    let properties_init = child_ps_init
        .into_iter()
        .map(|(_, init, _)| init)
        .chain(wgt_ps_init.into_iter().map(|(_, init, _)| init)) // widget properties, child target first.
        .filter(|tt| !tt.is_empty()) // - without default value.
        .chain(user_ps_init); // + custom user properties.

    let r = quote! {
        #errors
        #(#properties_init)*
        //#whens
        //#new_child
        //#child_assigns
        //#wgt_assigns
        //#new
    };

    r.into()
}

/// Returns (property_ident, default_value, is_required)
fn inherited_inits(widget_mod: &Path, properties: Vec<BuiltProperty>) -> Vec<(Ident, TokenStream, bool)> {
    let mut inits = Vec::with_capacity(properties.len());
    for p in properties {
        let init = if p.has_default {
            let var_ident = ident!("__{}", p.ident);
            let default_ident = ident!("__d_{}", p.ident);
            quote! {
                let #var_ident = #widget_mod::#default_ident();
            }
        } else {
            TokenStream::default()
        };

        inits.push((p.ident, init, p.is_required));
    }
    inits
}

struct Input {
    widget_data: WidgetData,
    user_input: UserInput,
}
impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Input {
            widget_data: input.parse().unwrap_or_else(|e| non_user_error!(e)),
            // user errors go into UserInput::errors field.
            user_input: input.parse().unwrap_or_else(|e| non_user_error!(e)),
        })
    }
}

struct WidgetData {
    mod_path: Path,
    child_properties: Vec<BuiltProperty>,
    properties: Vec<BuiltProperty>,
    whens: Vec<BuiltWhen>,
    new_child_caps: Vec<Ident>,
    new_caps: Vec<Ident>,
}
impl Parse for WidgetData {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = non_user_braced_id(input, "widget");

        let mod_path_tks = non_user_braced_id(&input, "module");
        let mod_path = mod_path_tks.parse().unwrap_or_else(|e| non_user_error!(e));

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
            mod_path,
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

pub struct BuiltWhen {
    pub ident: Ident,
    pub inputs: Vec<Ident>,
    pub assigns: Vec<Ident>,
}
impl Parse for BuiltWhen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced(input);

        let mut expr_properties = vec![];
        let expr = non_user_braced_id(&input, "inputs");
        while !expr.is_empty() {
            expr_properties.push(expr.parse().unwrap_or_else(|e| non_user_error!(e)));
            expr.parse::<Token![,]>().ok();
        }

        let mut set_properties = vec![];
        let set = non_user_braced_id(&input, "assigns");
        while !set.is_empty() {
            set_properties.push(set.parse().unwrap_or_else(|e| non_user_error!(e)));
            set.parse::<Token![,]>().ok();
        }

        Ok(BuiltWhen {
            ident,
            inputs: expr_properties,
            assigns: set_properties,
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
        let input = non_user_braced_id(input, "user");

        let mut errors = Errors::default();
        let mut properties = vec![];
        let mut whens = vec![];

        while !input.is_empty() {
            if input.peek(keyword::when) {
                if let Some(when) = When::parse(&input, &mut errors) {
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
    pub fn is_special_eq(&self, keyword: &str) -> bool {
        matches!(self, PropertyValue::Special(sp, _) if sp == keyword)
    }

    pub fn incorrect_special(&self, expected: &str) -> Option<&Ident> {
        match self {
            PropertyValue::Special(sp, _) if sp != expected => Some(sp),
            _ => None,
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
    pub fn parse(input: ParseStream, errors: &mut Errors) -> Option<When> {
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
