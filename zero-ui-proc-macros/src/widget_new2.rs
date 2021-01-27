use std::{collections::HashSet, fmt};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    parse::{discouraged::Speculative, Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Attribute, Expr, FieldValue, Ident, LitBool, Path, Token,
};

use crate::util::{crate_core, display_path, parse_all, tokens_to_ident_str, Errors};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { widget_data, user_input } = match syn::parse::<Input>(input) {
        Ok(i) => i,
        Err(e) => non_user_error!(e),
    };

    let module = widget_data.module;

    let mut errors = Errors::default();

    let mut child_ps_init = inherited_inits(&module, widget_data.properties_child);
    let mut wgt_ps_init = inherited_inits(&module, widget_data.properties);
    let mut user_ps_init = vec![];

    let widget_properties: HashSet<Ident> = child_ps_init.iter().chain(wgt_ps_init.iter()).map(|(p, ..)| p.clone()).collect();

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
                        let #var_ident = #module::#p_ident::ArgsImpl::new(#args);
                    },
                    PropertyValue::Named(_, fields) => {
                        let property_path = quote! { #module::#p_ident };
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

    let mut assigns = user_assigns.clone();
    let user_assigns = user_assigns;

    for (ident, value, required) in child_ps_init.iter().chain(wgt_ps_init.iter()) {
        if value.is_empty() {
            if *required {
                // add errors for missing required.
                errors.push(&format!("required property `{}` not set", ident), ident.span());
            }
        } else {
            // add widget assigns.
            assigns.insert(parse_quote! { #ident });
        }
    }
    let assigns = assigns;

    // property initializers in order they must be called.
    let properties_init = child_ps_init
        .into_iter()
        .map(|(_, init, _)| init)
        .chain(wgt_ps_init.into_iter().map(|(_, init, _)| init)) // widget properties, child target first.
        .filter(|tt| !tt.is_empty()) // - without default value.
        .chain(user_ps_init); // + custom user properties.

    // when condition initializers and properties used in when reassign.
    let mut whens_init = TokenStream::default();
    // linear map of [(property, [(assign_ident, assign_property_value)])]
    let mut when_props_assigns = Vec::<(syn::Path, Vec<(Ident, TokenStream)>)>::new();
    let mut push_when_assign = |property: syn::Path, when_ident: Ident, when_prop_value: TokenStream| {
        if let Some((_, entry)) = when_props_assigns.iter_mut().find(|(id, _)| id == &property) {
            entry.push((when_ident.clone(), when_prop_value));
        } else {
            when_props_assigns.push((property, vec![(when_ident.clone(), when_prop_value)]));
        }
    };
    for when in widget_data.whens {
        if !when.inputs.iter().all(|id| user_assigns.contains(&parse_quote! { #id })) {
            // skip, not all properties used in the when condition are assigned.
            continue;
        }

        let w_assigns: Vec<_> = when
            .assigns
            .into_iter()
            // when can only assign properties that have an initial value.
            .filter(|a| {
                let id = &a.property;
                user_assigns.contains(&parse_quote! { #id })
            })
            .collect();
        if w_assigns.is_empty() {
            // skip, when does not assign any property.
            continue;
        }

        let when_ident = when.ident;
        let inputs = when.inputs.into_iter().map(|id| ident!("__{}", id));

        whens_init.extend(quote! {
            let #when_ident = #module::#when_ident(#(std::clone::Clone::clone(&#inputs)),*);
        });

        for w_assign in w_assigns {
            let p_ident = &w_assign.property;
            let d_ident = ident!("{}_d_{}", when_ident, p_ident);
            let value = quote! { #module::#d_ident() };

            let w_assign: syn::Path = parse_quote! { #p_ident };
            push_when_assign(w_assign, when_ident.clone(), value);
        }
    }
    for (i, when) in user_input.whens.into_iter().enumerate() {
        let w_assigns: Vec<_> = when.assigns.iter().filter(|id| user_assigns.contains(&id.path)).collect();
        if w_assigns.is_empty() {
            // skip, when does not assign any property.
            continue;
        }
        let (input_properties, init) = when.make_init(&module, &widget_properties);
        if !user_assigns.is_superset(&input_properties) {
            // skip, not all properties used in the when condition are assigned.
            continue;
        }

        let when_ident = when.make_ident(i);
        whens_init.extend(quote! {
            let #when_ident = #init;
        });

        for w_assign in w_assigns {
            // TODO assign to value.
            let value = quote! {};
            push_when_assign(w_assign.path.clone(), when_ident.clone(), value);
        }
    }

    // generate when switches for when affected properties.
    let mut when_props_init = TokenStream::default();
    //TODO

    // new_child call.
    let new_child_inputs = widget_data.new_child.into_iter().map(|id| ident!("__{}", id));
    let new_child = quote! {
        let node__ = #module::__new_child(#(#new_child_inputs),*);
    };

    // child assigns.
    // TODO

    // normal assigns.
    // TODO

    // new call.
    let new_inputs = widget_data.new.into_iter().map(|id| ident!("__{}", id));
    let new = quote! {
        #module::__new(node__, #(#new_inputs),*)
    };

    let r = quote! {
        {
        #errors
        #(#properties_init)*
        #whens_init
        #when_props_init
        #new_child
        //#child_assigns
        //#wgt_assigns
        #new
        }
    };

    r.into()
}

/// Returns (property_ident, default_value, is_required)
fn inherited_inits(module: &TokenStream, properties: Vec<BuiltProperty>) -> Vec<(Ident, TokenStream, bool)> {
    let mut inits = Vec::with_capacity(properties.len());
    for p in properties {
        let init = if p.default {
            let var_ident = ident!("__{}", p.ident);
            let default_ident = ident!("__d_{}", p.ident);
            quote! {
                let #var_ident = #module::#default_ident();
            }
        } else {
            TokenStream::default()
        };

        inits.push((p.ident, init, p.required));
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
    module: TokenStream,
    properties_child: Vec<BuiltProperty>,
    properties: Vec<BuiltProperty>,
    whens: Vec<BuiltWhen>,
    new_child: Vec<Ident>,
    new: Vec<Ident>,
}
impl Parse for WidgetData {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = non_user_braced!(input, "widget");
        let r = Ok(Self {
            module: non_user_braced!(&input, "module").parse().unwrap(),
            properties_child: parse_all(&non_user_braced!(&input, "properties_child")).unwrap_or_else(|e| non_user_error!(e)),
            properties: parse_all(&non_user_braced!(&input, "properties")).unwrap_or_else(|e| non_user_error!(e)),
            whens: parse_all(&non_user_braced!(&input, "whens")).unwrap_or_else(|e| non_user_error!(e)),
            new_child: parse_all(&non_user_braced!(&input, "new_child")).unwrap_or_else(|e| non_user_error!(e)),
            new: parse_all(&non_user_braced!(&input, "new")).unwrap_or_else(|e| non_user_error!(e)),
        });

        r
    }
}

pub struct BuiltProperty {
    pub ident: Ident,
    pub docs: TokenStream,
    pub cfg: TokenStream,
    pub default: bool,
    pub required: bool,
}
impl Parse for BuiltProperty {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);

        let r = Ok(BuiltProperty {
            ident,
            docs: non_user_braced!(&input, "docs").parse().unwrap(),
            cfg: non_user_braced!(&input, "cfg").parse().unwrap(),
            default: non_user_braced!(&input, "default")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
            required: non_user_braced!(&input, "required")
                .parse::<LitBool>()
                .unwrap_or_else(|e| non_user_error!(e))
                .value,
        });
        r
    }
}

pub struct BuiltWhen {
    pub ident: Ident,
    pub docs: TokenStream,
    pub cfg: TokenStream,
    pub inputs: Vec<Ident>,
    pub assigns: Vec<BuiltWhenAssign>,
}
impl Parse for BuiltWhen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);

        let r = Ok(BuiltWhen {
            ident,
            docs: non_user_braced!(&input, "docs").parse().unwrap(),
            cfg: non_user_braced!(&input, "cfg").parse().unwrap(),
            inputs: parse_all(&non_user_braced!(&input, "inputs")).unwrap_or_else(|e| non_user_error!(e)),
            assigns: parse_all(&non_user_braced!(&input, "assigns")).unwrap_or_else(|e| non_user_error!(e)),
        });
        r
    }
}

pub struct BuiltWhenAssign {
    pub property: Ident,
    pub cfg: TokenStream,
}
impl Parse for BuiltWhenAssign {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let property = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);
        let r = Ok(BuiltWhenAssign {
            property,
            cfg: non_user_braced!(&input, "cfg").parse().unwrap(),
        });
        r
    }
}

struct UserInput {
    errors: Errors,
    properties: Vec<PropertyAssign>,
    whens: Vec<When>,
}
impl Parse for UserInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input = non_user_braced!(input, "user");

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
    pub attrs: Vec<Attribute>,
    pub path: Path,
    pub eq: Token![=],
    pub value: PropertyValue,
    pub semi: Option<Token![;]>,
}
impl Parse for PropertyAssign {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
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
            attrs,
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

    /// Convert this value to an expr. Panics if `self` is [`Special`].
    pub fn expr_tokens(&self, property_path: &TokenStream) -> TokenStream {
        match self {
            PropertyValue::Unnamed(args) => {
                quote! {
                    #property_path::ArgsImpl::new(#args)
                }
            }
            PropertyValue::Named(_, fields) => {
                quote! {
                    #property_path::code_gen! { named_new #property_path {
                        #fields
                    }}
                }
            }
            PropertyValue::Special(_, _) => panic!("cannot expand special"),
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
    pub attrs: Vec<Attribute>,
    pub when: keyword::when,
    pub condition_expr: TokenStream,
    pub brace_token: syn::token::Brace,
    pub assigns: Vec<PropertyAssign>,
}
impl When {
    /// Call only if peeked `when`. Parse outer attribute before calling.
    pub fn parse(input: ParseStream, errors: &mut Errors) -> Option<When> {
        let when = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let condition_expr = crate::expr_var::parse_without_eager_brace(input);

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

        if assigns.is_empty() {
            None
        } else {
            Some(When {
                attrs: vec![], // must be parsed before.
                when,
                condition_expr,
                brace_token,
                assigns,
            })
        }
    }

    /// Returns an ident `__w{i}_{expr_to_str}`
    pub fn make_ident(&self, i: usize) -> Ident {
        ident!("__w{}_{}", i, tokens_to_ident_str(&self.condition_expr.to_token_stream()))
    }

    /// Returns a set of properties used in the condition and the condition transformed to new var.
    pub fn make_init(&self, widget_path: &TokenStream, widget_properties: &HashSet<Ident>) -> (HashSet<syn::Path>, TokenStream) {
        todo!()
    }
}

pub mod keyword {
    syn::custom_keyword!(when);
}
