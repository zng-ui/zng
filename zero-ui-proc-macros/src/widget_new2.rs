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

use crate::util::{crate_core, display_path, expr_to_ident_str, non_user_braced, non_user_braced_id, Errors};

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
            .filter(|id| user_assigns.contains(&parse_quote! { #id }))
            .collect();
        if w_assigns.is_empty() {
            // skip, when does not assign any property.
            continue;
        }

        let when_ident = when.ident;
        let inputs = when.inputs.into_iter().map(|id| ident!("__{}", id));

        whens_init.extend(quote! {
            let #when_ident = #widget_mod::#when_ident(#(std::clone::Clone::clone(&#inputs)),*);
        });

        for w_assign in w_assigns {
            let d_ident = ident!("{}_d_{}", when_ident, w_assign);
            let value = quote! { #widget_mod::#d_ident() };

            let w_assign: syn::Path = parse_quote! { #w_assign };
            push_when_assign(w_assign, when_ident.clone(), value);
        }
    }
    for (i, when) in user_input.whens.into_iter().enumerate() {
        let w_assigns: Vec<_> = when.assigns.iter().filter(|id| user_assigns.contains(&id.path)).collect();
        if w_assigns.is_empty() {
            // skip, when does not assign any property.
            continue;
        }
        let (input_properties, init) = when.make_init(&widget_mod, &widget_properties);
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
    let new_child_inputs = widget_data.new_child_caps.into_iter().map(|id| ident!("__{}", id));
    let new_child = quote! {
        let node__ = #widget_mod::__new_child(#(#new_child_inputs),*);
    };

    // child assigns.
    // TODO

    // normal assigns.
    // TODO

    // new call.
    let new_inputs = widget_data.new_caps.into_iter().map(|id| ident!("__{}", id));
    let new = quote! {
        #widget_mod::__new(node__, #(#new_inputs),*)
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
    pub attrs: Vec<Attribute>,
    pub when: keyword::when,
    pub condition_expr: Expr,
    pub brace_token: syn::token::Brace,
    pub assigns: Vec<PropertyAssign>,
}
impl When {
    /// Call only if peeked `when`. Parse outer attribute before calling.
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
        ident!("__w{}_{}", i, expr_to_ident_str(&self.condition_expr))
    }

    /// Returns a set of properties used in the condition and the condition transformed to new var.
    pub fn make_init(&self, widget_path: &Path, widget_properties: &HashSet<Ident>) -> (HashSet<syn::Path>, TokenStream) {
        let mut visitor = WhenConditionVisitor::default();
        let mut expr = self.condition_expr.clone();
        visitor.visit_expr_mut(&mut expr);

        let crate_core = crate_core();
        let init = if visitor.properties.is_empty() {
            // does not reference any property, just eval into_var.
            quote! {
                #crate_core::var::IntoVar::into_var({ #expr })
            }
        } else if visitor.properties.len() == 1 {
            let p_0 = visitor.properties.drain().next().unwrap();

            let var = p_0.into_var_tokens(widget_path, widget_properties);

            if visitor.found_mult_exprs {
                // references a single property but does something with the value.
                let ident_in_expr = p_0.ident_in_expr();
                quote! {
                    #crate_core::var::Var::into_map(#var, |#ident_in_expr|#expr)
                }
            } else {
                // references a single property.
                var
            }
        } else {
            // references multiple properties.
            let idents = visitor.properties.iter().map(|p| p.ident_in_expr());
            let vars = visitor.properties.iter().map(|p| p.into_var_tokens(widget_path, widget_properties));
            quote! {
                #crate_core::var::merge_var! { #(#vars),* , |#(#idents),*|#expr }
            }
        };

        let properties = visitor.properties.into_iter().map(|p| p.property).collect();

        (properties, init)
    }
}

#[derive(Default)]
struct WhenConditionVisitor {
    properties: HashSet<WhenPropertyRef>,
    found_mult_exprs: bool,
}
impl VisitMut for WhenConditionVisitor {
    //visit expressions like:
    // self.is_hovered
    // self.is_hovered.0
    // self.is_hovered.state
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        //get self or child
        fn is_self(expr_path: &syn::ExprPath) -> bool {
            if let Some(ident) = expr_path.path.get_ident() {
                return ident == &ident!("self");
            }
            false
        }

        let mut found = None;

        if let Expr::Field(expr_field) = expr {
            match &mut *expr_field.base {
                // self.is_hovered
                Expr::Path(expr_path) => {
                    if let (true, syn::Member::Named(property)) = (is_self(expr_path), expr_field.member.clone()) {
                        found = Some(WhenPropertyRef {
                            property: parse_quote! { #property },
                            arg: WhenPropertyRefArg::Index(0),
                        })
                    }
                }
                // self.is_hovered.0
                // self.is_hovered.state
                Expr::Field(i_expr_field) => {
                    if let Expr::Path(expr_path) = &mut *i_expr_field.base {
                        if let (true, syn::Member::Named(property)) = (is_self(expr_path), i_expr_field.member.clone()) {
                            found = Some(WhenPropertyRef {
                                property: parse_quote! { #property },
                                arg: expr_field.member.clone().into(),
                            })
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(p) = found {
            let replacement = p.ident_in_expr();
            *expr = parse_quote!((*#replacement));
            self.properties.insert(p);
        } else {
            self.found_mult_exprs = true;
            visit_mut::visit_expr_mut(self, expr);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WhenPropertyRef {
    pub property: syn::Path,
    pub arg: WhenPropertyRefArg,
}
impl WhenPropertyRef {
    fn ident_in_expr(&self) -> Ident {
        ident!("__self_{}_{}", display_path(&self.property), &self.arg)
    }
    fn into_var_tokens(&self, widget_path: &Path, widget_properties: &HashSet<Ident>) -> TokenStream {
        let ident = ident!("__{}", display_path(&self.property));
        let mtd_ident = match &self.arg {
            WhenPropertyRefArg::Index(i) => ident!("__{}", i),
            WhenPropertyRefArg::Named(name) => ident!("__{}", name),
        };
        let args_path = if let Some(id) = self.property.get_ident().and_then(|id| widget_properties.get(id)) {
            let p_ident = ident!("__p_{}", id);
            quote! {
                #widget_path::#p_ident::Args
            }
        } else {
            self.property.to_token_stream()
        };
        let crate_core = crate_core();
        quote! {
            #crate_core::var::IntoVar::into_var(
                std::clone::Clone::clone(
                    #args_path.#mtd_ident(&#ident)
                )
            )
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum WhenPropertyRefArg {
    Index(u32),
    Named(Ident),
}
impl From<syn::Member> for WhenPropertyRefArg {
    fn from(member: syn::Member) -> Self {
        match member {
            syn::Member::Named(ident) => WhenPropertyRefArg::Named(ident),
            syn::Member::Unnamed(i) => WhenPropertyRefArg::Index(i.index),
        }
    }
}
impl fmt::Display for WhenPropertyRefArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhenPropertyRefArg::Index(i) => fmt::Display::fmt(&i, f),
            WhenPropertyRefArg::Named(n) => fmt::Display::fmt(&n, f),
        }
    }
}

pub mod keyword {
    syn::custom_keyword!(when);
}
