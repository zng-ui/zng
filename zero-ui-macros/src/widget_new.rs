use crate::widget::*;
use proc_macro2::{Span, TokenStream};
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{parse::*, *};

include!("util.rs");

/// `widget_new!` implementation
#[allow(clippy::cognitive_complexity)]
pub fn expand_widget_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as WidgetNewInput);

    let child = input.user_child_expr;
    let mut imports = input.imports;
    let mut crate_patch = IdentReplace {
        find: ident!("crate"),
        replace: input.crate_,
    };
    for import in imports.iter_mut() {
        crate_patch.visit_item_use_mut(import);
    }
    let imports = quote!(#(#imports)*);

    let mut user_sets: HashMap<_, _> = input.user_sets.into_iter().map(|pa| (pa.ident.clone(), pa)).collect();
    user_sets
        .entry(ident!("id"))
        .or_insert_with(|| parse_quote!(id: zero_ui::core::types::WidgetId::new_unique();));

    let mut new_child_args = Vec::with_capacity(input.new_child.len());
    for new_child_prop in &input.new_child {
        if let Some(pa) = user_sets.remove(new_child_prop) {
            new_child_args.push(pa.value);
        } else if let Some(p) = input.default_child.properties.iter().find(|p| &p.ident == new_child_prop) {
            match &p.default_value {
                Some(PropertyDefaultValue::Args(args)) => new_child_args.push(PropertyValue::Args(args.clone())),
                Some(PropertyDefaultValue::Fields(args)) => new_child_args.push(PropertyValue::Fields(args.clone())),
                Some(PropertyDefaultValue::Unset) | Some(PropertyDefaultValue::Required) | None => {
                    abort!(p.ident.span(), "property `{}` is required", p.ident)
                }
            }
        } else {
            abort!(Span::call_site(), "property `{}` is required", new_child_prop)
        }
    }

    let mut new_args = Vec::with_capacity(input.new.len());
    for new_prop in &input.new {
        if let Some(pa) = user_sets.remove(new_prop) {
            new_args.push(pa.value);
        } else if let Some(p) = input.default_child.properties.iter().find(|p| &p.ident == new_prop) {
            match &p.default_value {
                Some(PropertyDefaultValue::Args(args)) => new_args.push(PropertyValue::Args(args.clone())),
                Some(PropertyDefaultValue::Fields(args)) => new_args.push(PropertyValue::Fields(args.clone())),
                Some(PropertyDefaultValue::Unset) | Some(PropertyDefaultValue::Required) | None => {
                    abort!(p.ident.span(), "property `{}` is required", p.ident)
                }
            }
        } else {
            abort!(Span::call_site(), "property `{}` is required", new_prop)
        }
    }

    let new_child_args: Vec<_> = new_child_args.into_iter().map(|v| quote!(todo!())).collect();
    let new_args: Vec<_> = new_args.into_iter().map(|v| quote!(todo!())).collect();

    let ident = input.ident;
    let wgt_props = quote!(#ident::__props);

    let PropertyCalls {
        set_context: set_child_props_ctx,
        set_event: set_child_props_event,
        set_outer: set_child_props_outer,
        set_inner: set_child_props_inner,
    } = match make_property_calls(&wgt_props, &imports, input.default_child, &mut user_sets) {
        Ok(p) => p,
        Err(e) => abort_call_site!("{}", e),
    };

    let mut self_calls = match make_property_calls(&wgt_props, &imports, input.default_self, &mut user_sets) {
        Ok(p) => p,
        Err(e) => abort!(e.span(), "{}", e),
    };

    let let_id = if let Some(p) = user_sets.remove(&ident!("id")) {
        match p.value {
            PropertyValue::Args(a) => quote!(let __id = zero_ui::core::validate_widget_id_args(#a);),
            PropertyValue::Fields(a) => quote!(let __id = zero_ui::core::ValidateWidgetIdArgs{#a}.id;),
            PropertyValue::Todo(m) => quote! (let __id = #m;),
            PropertyValue::Unset => abort_call_site!("cannot unset required property `id`"),
        }
    } else {
        quote!(let __id = zero_ui::core::types::WidgetId::new_unique();)
    };

    for (ident, assign) in user_sets {
        make_property_call(&wgt_props, &ident, assign.value, &mut self_calls, &imports, false);
    }

    let PropertyCalls {
        set_context: set_self_props_ctx,
        set_event: set_self_props_event,
        set_outer: set_self_props_outer,
        set_inner: set_self_props_inner,
    } = self_calls;

    let r = quote! {{
        let __node = #child;

        let __node = {
            #(#set_child_props_ctx)*
            #(#set_child_props_event)*
            #(#set_child_props_outer)*
            #(#set_child_props_inner)*

            __node
        };

        let __node = #ident::new_child(__node, #(#new_child_args),*);

        #let_id

        let __node = {
            #(#set_self_props_ctx)*
            #(#set_self_props_event)*
            #(#set_self_props_outer)*
            #(#set_self_props_inner)*

           __node
        };

        #ident::new(__node, #(#new_args),*)
    }};

    r.into()
}

fn make_property_calls(
    wgt_props: &TokenStream,
    imports: &TokenStream,
    default: DefaultBlock,
    user_sets: &mut HashMap<Ident, PropertyAssign>,
) -> Result<PropertyCalls> {
    let mut r = PropertyCalls::default();

    for default in default.properties {
        let (value, default_value) = if let Some(p) = user_sets.remove(&default.ident) {
            if default.is_required() && p.value.is_unset() {
                return Err(Error::new(
                    Span::call_site(),
                    format!("cannot unset required property `{}`", default.ident),
                ));
            }

            (p.value, false)
        } else if let Some(d) = default.default_value {
            (
                match d {
                    PropertyDefaultValue::Args(a) => PropertyValue::Args(a),
                    PropertyDefaultValue::Fields(a) => PropertyValue::Fields(a),
                    PropertyDefaultValue::Unset => continue,
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

        make_property_call(wgt_props, &ident, value, &mut r, imports, default_value);
    }

    Ok(r)
}

fn make_property_call(
    wgt_props: &TokenStream,
    ident: &Ident,
    value: PropertyValue,
    r: &mut PropertyCalls,
    imports: &TokenStream,
    default_value: bool,
) {
    macro_rules! arg {
        ($n:expr) => {
            ident!("__{}_arg_{}", ident, $n)
        };
    }

    let mut args_init = vec![];
    let len;

    match value {
        PropertyValue::Args(a) => {
            len = a.len();
            for a in a.iter() {
                if default_value {
                    args_init.push(quote! {{
                        #imports
                        #a
                    },});
                } else {
                    args_init.push(quote!(#a,));
                }
            }
        }
        PropertyValue::Fields(f) => {
            len = f.len();

            args_init = (0..len)
                .map(|i| {
                    let arg = arg!(i);
                    quote!(#arg,)
                })
                .collect();

            if default_value {
                r.set_context.push(quote! {
                    let (#(#args_init)*) = {
                        #imports
                        #ident::Args {
                            __phantom: std::marker::PhantomData,
                            #f
                        }.pop()
                    };
                });
            } else {
                r.set_context.push(quote! {
                    let (#(#args_init)*) = #ident::Args {
                        __phantom: std::marker::PhantomData,
                        #f
                    }.pop();
                });
            }
        }
        PropertyValue::Todo(m) => {
            r.set_context.push(quote!(#m;));
            return;
        }
        PropertyValue::Unset => return,
    };

    let args = (0..len).map(|i| arg!(i));
    let args = quote!(__node, #(#args),*);

    let ident = if default_value {
        quote!(#wgt_props::#ident)
    } else {
        quote!(#ident)
    };

    r.set_context
        .push(quote!(let (#args) = #ident::set_context(__node, #(#args_init)*);));
    r.set_event.push(quote!(let (#args) = #ident::set_event(#args);));
    r.set_outer.push(quote!(let (#args) = #ident::set_outer(#args);));
    r.set_inner.push(quote!(let (#args) = #ident::set_inner(#args);));
}

#[derive(Default)]
struct PropertyCalls {
    set_context: Vec<TokenStream>,
    set_event: Vec<TokenStream>,
    set_outer: Vec<TokenStream>,
    set_inner: Vec<TokenStream>,
}

pub struct WidgetNewInput {
    crate_: Ident,
    ident: Ident,
    imports: Vec<ItemUse>,
    default_child: DefaultBlock,
    default_self: DefaultBlock,
    whens: Vec<WhenBlock>,
    new_child: Punctuated<Ident, Token![,]>,
    new: Punctuated<Ident, Token![,]>,
    user_sets: Vec<PropertyAssign>,
    user_whens: Vec<WhenBlock>,
    user_child_expr: Expr,
}
impl Parse for WidgetNewInput {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![crate]>()?;
        let crate_ = input.parse()?;
        input.parse::<Token![;]>()?;

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

        input.parse::<keyword::new_child>().expect(NON_USER_ERROR);

        let new_inner = non_user_parenthesized(input);
        let new_child = Punctuated::parse_terminated(&new_inner)?;

        input.parse::<keyword::new>().expect(NON_USER_ERROR);
        let new_inner = non_user_parenthesized(input);
        let new = Punctuated::parse_terminated(&new_inner)?;

        input.parse::<keyword::input>().expect(NON_USER_ERROR);

        let input = non_user_braced(input);

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
                    crate_,
                    ident,
                    imports,
                    default_child,
                    default_self,
                    whens,
                    new_child,
                    new,
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

struct IdentReplace {
    find: Ident,
    replace: Ident,
}

impl VisitMut for IdentReplace {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        if i == &self.find {
            *i = self.replace.clone();
        }
        visit_mut::visit_ident_mut(self, i);
    }
}
