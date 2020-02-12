use proc_macro2::{Punct, Spacing, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{
    parse::*,
    punctuated::Punctuated,
    token::{Brace, Token},
    *,
};

include!("util.rs");

/// `widget!` implementation
pub(crate) fn expand_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as WidgetInput);

    let mut fn_ = input.fn_;
    let uses = input.uses;

    if fn_.sig.output == ReturnType::Default {
        abort!(fn_.sig.span(), "function must return an `UiNode`")
    }

    let (doc_attrs, other_attrs) = split_doc_other(&mut input.attrs);
    let (_, fn_other_attrs) = split_doc_other(&mut fn_.attrs);

    let macro_vis = match fn_.vis {
        Visibility::Public(_) => {
            quote! {
                #[macro_export]
            }
        }
        _ => TokenStream::new(),
    };
    let ident = fn_.sig.ident.clone();
    fn_.sig.ident = Ident::new("new", ident.span());
    let vis = fn_.vis;
    fn_.vis = pub_vis();

    let mut arg_names = vec![];

    if fn_.sig.inputs.is_empty() {
        abort!(
            fn_.sig.span(),
            "function must take a `child: impl UiNode` first and at least one other argument"
        );
    } else if let Some(FnArg::Receiver(_)) = fn_.sig.inputs.first() {
        abort!(fn_.span(), "function must free-standing");
    } else {
        for arg in fn_.sig.inputs.iter().skip(1) {
            if let FnArg::Typed(pat) = arg {
                if let Pat::Ident(pat) = &*pat.pat {
                    arg_names.push(pat.ident.clone());
                } else {
                    abort!(arg.span(), "widget arguments does not support pattern deconstruction");
                }
            } else {
                abort!(arg.span(), "unexpected `self`");
            }
        }
    }

    let child_properties = input.child_properties;
    let self_properties = input.self_properties;

    let macro_args = quote_spanned! {ident.span()=>
        ($($tt:tt)*)
    };

    let result = quote! {
        #[doc(hidden)]
        #macro_vis
        macro_rules! #ident {
            #macro_args => {
                widget_new!{
                    $crate
                    #(#uses)*
                    #child_properties
                    #self_properties
                    args {$($tt)*}
                    fn #ident(#(#arg_names),*)
                }
            };
        }

        #(#doc_attrs)*
        #(#other_attrs)*
        #vis mod #ident {
            #(#uses)*
            use super::*;

            #[inline]
            #(#fn_other_attrs)*
            #fn_
        }
    };

    result.into()
}

/// `widget_new!` implementation
#[allow(clippy::cognitive_complexity)]
pub(crate) fn expand_widget_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let WidgetNewInput {
        crate_ident,
        child,
        fn_name,
        child_properties,
        self_properties,
        args,
        fn_arg_names,
        mut uses,
        ..
    } = parse_macro_input!(input as WidgetNewInput);

    if !uses.is_empty() && crate_ident != ident("crate") {
        let mut crate_visitor = CrateToCrateName { crate_ident };

        for use_ in uses.iter_mut() {
            crate_visitor.visit_use_tree_mut(&mut use_.tree);
        }
    }

    let mut args: HashMap<_, _> = args.into_iter().map(|p| (p.name.clone(), p)).collect();

    // takes required arguments.
    let mut expanded_fn_args = Vec::with_capacity(fn_arg_names.len());

    for a in fn_arg_names {
        if let Some(p) = args.remove(&a) {
            if let PropertyArgs::Exprs(exprs) = p.args {
                if exprs.len() > 1 {
                    // arg: 10, 20;
                    abort!(p.name.span(), "expected single unamed argument for `{}`", p.name);
                } else {
                    // arg: 10;
                    expanded_fn_args.push(exprs.into_iter().next().unwrap())
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
    }

    // takes or generates properties.
    let child_properties = take_properties(&mut args, child_properties.properties);
    let self_properties = take_properties(&mut args, self_properties.properties);

    let (child_args, expanded_child_props, _) = expand_properties(child_properties);
    let (self_args, expanded_self_props, id) = expand_properties(
        self_properties
            .into_iter()
            // args here is properties the used added that where not in the widget declaration
            .chain(args.into_iter().map(|(_, v)| v))
            .collect(),
    );

    let widget_call = if let Some(id) = id {
        quote!($crate::core::widget(#id, child))
    } else {
        quote!()
    };

    let result = quote! {{
        #(#uses);*

        let child = #child;
        #(#child_args)*
        #(#expanded_child_props)*
        let child = #fn_name::new(child, #(#expanded_fn_args),*);
        #(#self_args)*
        #(#expanded_self_props)*
        #widget_call
    }};

    result.into()
}

// -> (let args, set_priorities, id(None==unset))
fn expand_properties(properties: Vec<Property>) -> (Vec<TokenStream>, Vec<TokenStream>, Option<TokenStream>) {
    let mut unset_id = false;
    let mut custom_id = None;
    let id_name = ident("id");

    let properties: Vec<_> = properties
        .into_iter()
        .filter(|p| {
            let name = &p.name;
            match &p.args {
                PropertyArgs::Exprs(args) => {
                    if name == &id_name {
                        unset_id = false;
                        custom_id = Some(quote!(#(#args)+));
                        false // id is not an UiNode property
                    } else {
                        true
                    }
                }
                PropertyArgs::Unset => {
                    if name == &id_name {
                        unset_id = true;
                        custom_id = None;
                    }
                    false
                }
                PropertyArgs::Fields(_) => true,
            }
        })
        .collect();

    let mut let_args = Vec::with_capacity(properties.len());
    let mut all_arg_names = Vec::with_capacity(properties.len());

    let mut expanded_props = Vec::with_capacity(properties.len());

    for p in properties.iter() {
        let name = &p.name;
        match &p.args {
            PropertyArgs::Exprs(args) => {
                let arg_names: Vec<_> = (0..args.len()).map(|i| ident(&format!("__{}{}", quote! {#name}, i))).collect();
                let args = args.into_iter();

                let_args.push(quote! {
                    #(let #arg_names = #args;)*
                });

                all_arg_names.push(arg_names);
            }
            PropertyArgs::Fields(fields) => {
                let arg_names: Vec<_> = (0..fields.len()).map(|i| ident(&format!("__{}{}", quote! {#name}, i))).collect();
                let_args.push(quote! {
                    let (#(#arg_names),*) = #name::Args {
                        #fields
                    }.pop();
                });

                all_arg_names.push(arg_names);
            }
            PropertyArgs::Unset => unreachable!(),
        }
    }

    for priority in &[ident("context_var"), ident("event"), ident("outer"), ident("inner")] {
        for (p, arg_names) in properties.iter().zip(all_arg_names.iter()) {
            let name = &p.name;

            expanded_props.push(quote! {
                let (child, #(#arg_names),*) = #name::#priority(child, #(#arg_names),*);
            });
        }
    }

    let id = if unset_id {
        None
    } else if custom_id.is_some() {
        custom_id
    } else {
        Some(quote! ($crate::core::UiItemId::new_unique()))
    };

    (let_args, expanded_props, id)
}

fn take_properties(args: &mut HashMap<Ident, Property>, properties: Punctuated<PropertyDeclaration, Token![;]>) -> Vec<Property> {
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

struct WidgetInput {
    child_properties: PropertiesDeclaration,
    self_properties: PropertiesDeclaration,
    fn_: ItemFn,
    uses: Vec<ItemUse>,
    attrs: Vec<Attribute>,
}

impl Parse for WidgetInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut child_props = None;
        let mut self_props = None;
        let mut fn_ = None;
        let mut attrs = vec![];
        let mut uses = vec![];

        while !input.is_empty() {
            attrs.extend(Attribute::parse_inner(input)?.into_iter().map(|mut a| {
                a.style = AttrStyle::Outer;
                a
            }));

            if input.peek(keyword::child_properties) {
                if child_props.is_some() {
                    let span = input.parse::<Ident>()?.span();
                    return Err(Error::new(span, "`child_properties` is defined multiple times"));
                }
                child_props = Some(input.parse()?)
            } else if input.peek(keyword::self_properties) {
                if self_props.is_some() {
                    let span = input.parse::<Ident>()?.span();
                    return Err(Error::new(span, "`self_properties` is defined multiple times"));
                }
                self_props = Some(input.parse()?)
            } else {
                let item = input.parse::<Item>()?;
                match item {
                    Item::Fn(f) => {
                        if fn_.is_some() {
                            return Err(Error::new(f.span(), "ui_widget! only supports one function"));
                        }
                        fn_ = Some(f)
                    }
                    Item::Use(u) => uses.push(u),
                    item => return Err(Error::new(item.span(), "unexpected token")),
                }
            }
        }

        if let Some(fn_) = fn_ {
            Ok(WidgetInput {
                child_properties: child_props.unwrap_or_else(|| PropertiesDeclaration::empty(ident("child_properties"))),
                self_properties: self_props.unwrap_or_else(|| PropertiesDeclaration::empty(ident("self_properties"))),
                fn_,
                attrs,
                uses,
            })
        } else {
            Err(Error::new(Span::call_site(), "expected a function declaration"))
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

struct PropertiesDeclaration {
    ident: Ident,
    properties: Punctuated<PropertyDeclaration, Token![;]>,
}

impl PropertiesDeclaration {
    fn empty(ident: Ident) -> Self {
        PropertiesDeclaration {
            ident,
            properties: Punctuated::new(),
        }
    }
}

impl Parse for PropertiesDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        // child_properties { PropertyDeclaration }

        let ident = input.parse()?;

        let inner;
        braced!(inner in input);

        let properties = Punctuated::parse_terminated(&inner)?;

        Ok(PropertiesDeclaration { ident, properties })
    }
}

impl ToTokens for PropertiesDeclaration {
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
    syn::custom_keyword!(unset);
}

struct PropertyDeclaration {
    ident: Ident,
    maps_to: Option<Ident>,
    default_value: Option<PropertyArgs>,
}

impl Parse for PropertyDeclaration {
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

        Ok(PropertyDeclaration {
            ident,
            default_value,
            maps_to,
        })
    }
}
impl ToTokens for PropertyDeclaration {
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

enum PropertyArgs {
    Fields(Punctuated<FieldValue, Token![,]>),
    Exprs(Punctuated<Expr, Token![,]>),
    Unset,
}

impl Parse for PropertyArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let r = if input.peek(token::Brace) {
            let inner;
            braced!(inner in input);
            PropertyArgs::Fields(Punctuated::parse_separated_nonempty(&inner)?)
        } else if input.peek(keyword::unset) && input.peek2(Token![!]) {
            input.parse::<keyword::unset>()?;
            input.parse::<Token![!]>()?;

            PropertyArgs::Unset
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
            PropertyArgs::Unset => tokens.extend(quote!(unset!)),
        }
    }
}

struct WidgetNewInput {
    crate_ident: Ident,
    uses: Vec<ItemUse>,
    child_properties: PropertiesDeclaration,
    self_properties: PropertiesDeclaration,
    args: Punctuated<Property, Token![;]>,
    fn_name: Ident,
    fn_arg_names: Punctuated<Ident, Token![,]>,
    child: Expr,
}

impl Parse for WidgetNewInput {
    fn parse(input: ParseStream) -> Result<Self> {
        // $crate
        //
        // use x;
        //
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

        let crate_ident = input.parse()?;

        let mut uses = vec![];
        while input.peek(Token![use]) {
            uses.push(input.parse()?);
        }

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

        Ok(WidgetNewInput {
            crate_ident,
            uses,
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

    while !input.is_empty() && !<Token![=>]>::peek(input.cursor()) {
        let value = input.parse()?;
        punctuated.push_value(value);
        let punct = input.parse()?;
        punctuated.push_punct(punct);
    }

    Ok(punctuated)
}

struct CrateToCrateName {
    crate_ident: Ident,
}

impl VisitMut for CrateToCrateName {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        if i == &ident("crate") {
            *i = self.crate_ident.clone();
        }
    }
}
