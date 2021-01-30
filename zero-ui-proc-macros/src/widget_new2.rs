use std::collections::{HashMap, HashSet};

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    braced,
    parse::{discouraged::Speculative, Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Expr, FieldValue, Ident, LitBool, Path, Token,
};

use crate::util::{self, parse_all, tokens_to_ident_str, Attributes, Errors};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Input { widget_data, user_input } = match syn::parse::<Input>(input) {
        Ok(i) => i,
        Err(e) => non_user_error!(e),
    };

    let module = widget_data.module;

    let mut errors = user_input.errors;

    let inherited_properties: HashSet<_> = widget_data
        .properties_child
        .iter()
        .chain(widget_data.properties.iter())
        .map(|p| &p.ident)
        .collect();
    // inherited properties that are assigned by the user.
    let overriden_properties: HashSet<_> = user_input
        .properties
        .iter()
        .filter_map(|p| p.path.get_ident())
        .filter(|p_id| inherited_properties.contains(p_id))
        .collect();
    // properties that must be assigned by the user.
    let required_properties: HashSet<_> = widget_data
        .properties_child
        .iter()
        .chain(widget_data.properties.iter())
        .filter(|p| p.required)
        .map(|p| &p.ident)
        .collect();
    // properties that have a default value.
    let default_properties: HashSet<_> = widget_data
        .properties_child
        .iter()
        .chain(widget_data.properties.iter())
        .filter(|p| p.default)
        .map(|p| &p.ident)
        .collect();
    let captured_properties: HashSet<_> = widget_data.new_child.iter().chain(widget_data.new.iter()).collect();
    // all widget properties that will be set (property_path, property_var).
    let mut wgt_properties = HashMap::<syn::Path, Ident>::new();

    let mut property_inits = TokenStream::default();
    let mut child_prop_set_calls = vec![];
    let mut prop_set_calls = vec![];

    // for each inherited property that has a default value and is not overridden by the user:
    for (ip, is_child) in widget_data
        .properties_child
        .iter()
        .map(|ip| (ip, true))
        .chain(widget_data.properties.iter().map(|ip| (ip, true)))
        .filter(|(ip, _)| ip.default && !overriden_properties.contains(&ip.ident))
    {
        let ident = &ip.ident;
        let p_default_fn_ident = ident!("__d_{}", ident);
        let p_var_ident = ident!("__{}", ident);
        let cfg = &ip.cfg;

        wgt_properties.insert(parse_quote! { #ident }, p_var_ident.clone());

        // generate call to default args.
        property_inits.extend(quote! {
            #cfg
            let #p_var_ident = #module::#p_default_fn_ident();
        });

        if captured_properties.contains(ident) {
            continue; // we don't set captured properties.
        }

        let p_mod_ident = ident!("__p_{}", ident);
        // register data for the set call generation.
        let property_set_calls = if is_child { &mut child_prop_set_calls } else { &mut prop_set_calls };
        #[cfg(debug_assertions)]
        property_set_calls.push((
            quote! { #module::#p_mod_ident },
            p_var_ident,
            ip.ident.to_string(),
            {
                let p_source_loc_ident = ident!("__loc_{}", ip.ident);
                quote! { #module::#p_source_loc_ident() }
            },
            cfg.clone(),
            /*user_assigned: */ false,
        ));
        #[cfg(not(debug_assertions))]
        delayed_assigns.push((quote! { #module::#p_mod_ident }, p_var_ident, cfg.clone()));
    }

    let mut user_prop_set_calls = vec![];
    let mut unset_properties = HashSet::new();
    let mut user_properties = HashSet::new();

    // for each property assigned in the widget instantiation call (excluding when blocks).
    for up in &user_input.properties {
        let p_name = util::display_path(&up.path);

        // validates and skips `unset!`.
        if let PropertyValue::Special(sp, _) = &up.value {
            if sp == "unset" {
                if let Some(maybe_inherited) = up.path.get_ident() {
                    if required_properties.contains(maybe_inherited) || captured_properties.contains(maybe_inherited) {
                        errors.push(
                            format_args!("cannot unset required property `{}`", maybe_inherited),
                            maybe_inherited.span(),
                        );
                    } else if !default_properties.contains(maybe_inherited) {
                        errors.push(
                            format_args!("cannot unset `{}` because it is not set by the widget", maybe_inherited),
                            maybe_inherited.span(),
                        );
                    } else {
                        unset_properties.insert(maybe_inherited);
                        continue;
                    }
                } else {
                    errors.push(
                        format_args!(
                            "cannot unset `{}` because it is not set by the widget",
                            util::display_path(&up.path)
                        ),
                        up.path.span(),
                    );
                }
            } else {
                errors.push(format_args!("unknown value `{}!`", sp), sp.span());
            }
        }

        if !user_properties.insert(&up.path) {
            errors.push(format_args!("property `{}` already set", p_name), up.path.span());
            continue;
        }

        let p_mod = match up.path.get_ident() {
            Some(maybe_inherited) if inherited_properties.contains(maybe_inherited) => {
                let p_ident = ident!("__p_{}", maybe_inherited);
                quote! { #module::#p_ident }
            }
            _ => up.path.to_token_stream(),
        };
        let p_var_ident = ident!("__u_{}", p_name.replace("::", "_"));
        let attrs = Attributes::new(up.attrs.clone());
        let cfg = attrs.cfg;
        let lints = attrs.lints;

        wgt_properties.insert(up.path.clone(), p_var_ident.clone());

        let init_expr = up.value.expr_tokens(&p_mod);
        property_inits.extend(quote! {
            #cfg
            #(#lints)*
            let #p_var_ident = #init_expr;
        });

        if let Some(maybe_inherited) = up.path.get_ident() {
            if captured_properties.contains(maybe_inherited) {
                continue;
            }
        }

        // register data for the set call generation.
        #[cfg(debug_assertions)]
        user_prop_set_calls.push((
            p_mod.to_token_stream(),
            p_var_ident,
            p_name,
            quote_spanned! {up.path.span()=>
                #module::__core::source_location!()
            },
            cfg.to_token_stream(),
            /*user_assigned: */ true,
        ));
        #[cfg(not(debug_assertions))]
        delayed_assigns_user.push((p_mod.to_token_stream(), p_var_ident, p_name, cfg.to_token_stream()));
    }
    let unset_properties = unset_properties;
    let wgt_properties = wgt_properties;

    // generate property assigns.
    let mut property_set_calls = TokenStream::default();
    for set_calls in vec![child_prop_set_calls, prop_set_calls, user_prop_set_calls] {
        for priority in &crate::property::Priority::all_settable() {
            #[cfg(debug_assertions)]
            for (p_mod, p_var_ident, p_name, source_loc, cfg, user_assigned) in &set_calls {
                property_set_calls.extend(quote! {
                    #cfg
                    #p_mod::code_gen! {
                        set #priority, node__, #p_mod, #p_var_ident, #p_name, #source_loc, #user_assigned
                    }
                });
            }
            #[cfg(not(debug_assertions))]
            for (p_mod, p_var_ident, cfg) in delayed_assigns {
                property_assigns.extend(quote! {
                    #cfg
                    #p_mod::code_gen! {
                        set #priority, node__, #p_mod, #p_var_ident
                    }
                });
            }
        }
    }
    let property_set_calls = property_set_calls;

    // validate required properties.
    for required in required_properties.into_iter().chain(captured_properties) {
        if !wgt_properties.contains_key(&parse_quote! { #required }) {
            errors.push(format!("missing required property `{}`", required), Span::call_site());
        }
    }

    // generate whens.
    let mut when_inits = TokenStream::default();
    for iw in widget_data.whens {
        if iw.inputs.iter().any(|p| unset_properties.contains(p)) {
            // deactivate when block because user unset one of the inputs.
            continue;
        }

        let assigns: Vec<_> = iw.assigns.into_iter().filter(|a| !unset_properties.contains(&a.property)).collect();
        if assigns.is_empty() {
            // deactivate when block because user unset all of the properties assigned.
            continue;
        }

        when_inits.extend(quote! {});
    }
    for w in user_input.whens {}

    // generate new function calls.
    let new_child_caps = widget_data.new_child.iter().map(|p| {
        wgt_properties
            .get(&parse_quote! {#p})
            .unwrap_or_else(|| non_user_error!("captured property is unknown"))
    });
    let new_caps = widget_data.new.iter().map(|p| {
        wgt_properties
            .get(&parse_quote! {#p})
            .unwrap_or_else(|| non_user_error!("captured property is unknown"))
    });
    let new_child_call = quote! {
        let node__ = #module::__new_child(#(#new_child_caps),*);
    };
    let new_call = quote! {
        #module::__new(node__, #(#new_caps),*)
    };

    let r = quote! {
        {
            #errors
            #property_inits
            #when_inits
            #new_child_call
            #property_set_calls
            #new_call
        }
    };
    r.into()
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
    pub value_fn: Ident,
}
impl Parse for BuiltWhenAssign {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let property = input.parse().unwrap_or_else(|e| non_user_error!(e));
        let input = non_user_braced!(input);
        let r = Ok(BuiltWhenAssign {
            property,
            cfg: non_user_braced!(&input, "cfg").parse().unwrap(),
            value_fn: non_user_braced!(&input, "value_fn").parse().unwrap_or_else(|e| non_user_error!(e)),
        });
        r
    }
}

/// The content of the widget macro call.
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

/// Property assign in a widget instantiation or when block.
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

/// Value [assigned](PropertyAssign) to a property.
pub enum PropertyValue {
    /// `unset!` or `required!`.
    Special(Ident, Token![!]),
    /// `arg0, arg1,`
    Unnamed(Punctuated<Expr, Token![,]>),
    /// `{ field0: true, field1: false, }`
    Named(syn::token::Brace, Punctuated<FieldValue, Token![,]>),
}
impl PropertyValue {
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

/// When block in a widget instantiation or declaration.
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
}

pub mod keyword {
    syn::custom_keyword!(when);
}
