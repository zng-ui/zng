use crate::util;
use crate::widget::*;
use proc_macro2::{Span, TokenStream};
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{parse::*, *};

/// `widget_new!` implementation
#[allow(clippy::cognitive_complexity)]
pub fn expand_widget_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as WidgetNewInput);

    // widget child expression `=> {this}`
    let child = input.user_child_expr;

    // use items inside the widget.
    let mut imports = input.imports;
    // replace the `crate` keyword with the name of the crate.
    let mut crate_patch = IdentReplace {
        find: ident!("crate"),
        replace: input.crate_,
    };
    for import in imports.iter_mut() {
        crate_patch.visit_item_use_mut(import);
    }

    // property aliases.
    let mut aliases: HashMap<Ident, Ident> = HashMap::default();

    // dictionary of property assigns.
    // 1 - start with user assigns.
    let mut assigns: HashMap<_, _> = input
        .user_sets
        .into_iter()
        // (PropertyValue, Target, IsDefault)
        .map(|pa| (pa.ident, (pa.value, DefaultBlockTarget::Self_, false)))
        .collect();

    // 2 - add defaults, and validate required.
    for (t, p) in input
        .default_child
        .properties
        .into_iter()
        .map(|p| (DefaultBlockTarget::Child, p))
        .chain(input.default_self.properties.into_iter().map(|p| (DefaultBlockTarget::Self_, p)))
    {
        if let Some(user_assign) = assigns.get_mut(&p.ident) {
            // correct property
            user_assign.1 = t;
        } else {
            // if the user did not assign the property:

            match p.default_value {
                // but it is required, return an error
                Some(PropertyDefaultValue::Required) => {
                    abort_call_site!("property `{}` is required", p.ident);
                }
                // but it has a default value, use the default value
                Some(PropertyDefaultValue::Fields(f)) => {
                    assigns.insert(p.ident.clone(), (PropertyValue::Fields(f), t, true));
                }
                Some(PropertyDefaultValue::Args(a)) => {
                    assigns.insert(p.ident.clone(), (PropertyValue::Args(a), t, true));
                }
                Some(PropertyDefaultValue::Unset) | None => continue,
            }
        }

        // also collect aliases
        if let Some(actual) = p.maps_to {
            aliases.insert(p.ident, actual);
        }
    }

    // remove assigns that are used in the `new_child` and `new` functions.
    let mut new_child_args = Vec::with_capacity(input.new_child.len());
    let mut new_args = Vec::with_capacity(input.new.len());
    for p in input.new_child.into_iter() {
        // if a a user assign matches, capture its value.
        if let Some(v) = assigns.remove(&p) {
            assert_eq!(v.1, DefaultBlockTarget::Child, "{}", util::NON_USER_ERROR);
            let mut pname = p;
            if let Some(actual) = aliases.get(&pname) {
                pname = actual.clone();
            }
            new_child_args.push((v.0, pname, v.2)); // (PropertyValue, Ident, is_default)
        } else {
            panic!("{}", util::NON_USER_ERROR);
        }
    }
    for p in input.new.into_iter() {
        // if a a user assign matches, capture its value.
        if let Some(v) = assigns.remove(&p) {
            assert_eq!(v.1, DefaultBlockTarget::Self_, "{}", util::NON_USER_ERROR);
            new_args.push((v.0, p, v.2));
        } else {
            panic!("{}", util::NON_USER_ERROR);
        }
    }

    // widget mod
    let ident = input.ident;
    let props = quote!(#ident::__props);

    let new_arg = |(v, p, d)| {
        let args = property_value_to_args(v, &p, &props, d);
        quote!({#args})
    };

    // generate `new_child` and `new` arguments code.
    let new_child_args: Vec<_> = new_child_args.into_iter().map(new_arg).collect();
    let new_args: Vec<_> = new_args.into_iter().map(new_arg).collect();

    // generate property::set calls.
    let mut let_child_args = vec![];
    let mut set_child_props_ctx = vec![];
    let mut set_child_props_event = vec![];
    let mut set_child_props_outer = vec![];
    let mut set_child_props_inner = vec![];
    let mut let_self_args = vec![];
    let mut set_self_props_ctx = vec![];
    let mut set_self_props_event = vec![];
    let mut set_self_props_outer = vec![];
    let mut set_self_props_inner = vec![];
    for (mut prop, (val, tgt, dft)) in assigns {
        let aliased = prop.clone();
        if let Some(actual) = aliases.get(&prop) {
            prop = actual.clone();
        }

        let len;
        match &val {
            PropertyValue::Args(a) => {
                len = a.len();
            }
            PropertyValue::Fields(f) => {
                len = f.len();
            }
            PropertyValue::Unset => continue,
        }

        let args = property_value_to_args(val, &prop, &props, dft);
        let arg_names: Vec<_> = (0..len).map(|i| ident!("__{}_{}", aliased, i)).collect();
        let args = quote!(let (#(#arg_names,)*) = #args.pop(););

        let props = if dft { quote!(#props::) } else { quote!() };
        let set_ctx = quote!(let (__node, #(#arg_names,)*) = #props #prop::set_context(__node, #(#arg_names),*););
        let set_event = quote!(let (__node, #(#arg_names,)*) = #props #prop::set_event(__node, #(#arg_names),*););
        let set_outer = quote!(let (__node, #(#arg_names,)*) = #props #prop::set_outer(__node, #(#arg_names),*););
        let set_inner = quote!(let (__node, #(#arg_names,)*) = #props #prop::set_inner(__node, #(#arg_names),*););

        match tgt {
            DefaultBlockTarget::Self_ => {
                let_self_args.push(args);
                set_self_props_ctx.push(set_ctx);
                set_self_props_event.push(set_event);
                set_self_props_outer.push(set_outer);
                set_self_props_inner.push(set_inner);
            }
            DefaultBlockTarget::Child => {
                let_child_args.push(args);
                set_child_props_ctx.push(set_ctx);
                set_child_props_event.push(set_event);
                set_child_props_outer.push(set_outer);
                set_child_props_inner.push(set_inner);
            }
        }
    }

    let r = quote! {{
        #(#let_child_args)*
        let __node = #child;

        let __node = {
            #(#set_child_props_ctx)*
            #(#set_child_props_event)*
            #(#set_child_props_outer)*
            #(#set_child_props_inner)*

            #ident::new_child(__node, #(#new_child_args),*)
        };

        #(#let_self_args)*

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

fn property_value_to_args(v: PropertyValue, property_name: &Ident, props: &TokenStream, is_default: bool) -> TokenStream {
    match v {
        PropertyValue::Fields(f) => {
            if is_default {
                quote! {{
                    use #props::*;
                    #property_name::NamedArgs {
                        __phantom: std::marker::PhantomData,
                        #f
                    }
                }}
            } else {
                quote! {
                    #property_name::NamedArgs {
                        __phantom: std::marker::PhantomData,
                        #f
                    }
                }
            }
        }
        PropertyValue::Args(a) => {
            if is_default {
                quote! {{
                    use #props::*;
                    #property_name::args(#a)
                }}
            } else {
                quote!(#property_name::args(#a))
            }
        }
        _ => panic!("{}", util::NON_USER_ERROR),
    }
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

        input.parse::<Token![mod]>().expect(util::NON_USER_ERROR);
        let ident = input.parse().expect(util::NON_USER_ERROR);
        input.parse::<Token![;]>()?;

        let mut imports = vec![];
        while input.peek(Token![use]) {
            imports.push(input.parse().expect(util::NON_USER_ERROR));
        }

        let default_child: DefaultBlock = input.parse().expect(util::NON_USER_ERROR);
        default_child.assert(DefaultBlockTarget::Child);

        let default_self: DefaultBlock = input.parse().expect(util::NON_USER_ERROR);
        default_self.assert(DefaultBlockTarget::Self_);

        let mut whens = vec![];
        while input.peek(keyword::when) {
            whens.push(input.parse().expect(util::NON_USER_ERROR));
        }

        input.parse::<keyword::new_child>().expect(util::NON_USER_ERROR);

        let new_inner = util::non_user_parenthesized(input);
        let new_child = Punctuated::parse_terminated(&new_inner)?;

        input.parse::<keyword::new>().expect(util::NON_USER_ERROR);
        let new_inner = util::non_user_parenthesized(input);
        let new = Punctuated::parse_terminated(&new_inner)?;

        input.parse::<keyword::input>().expect(util::NON_USER_ERROR);

        let input = util::non_user_braced(input);

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
            panic!("{}: expected default({})", util::NON_USER_ERROR, quote!(#expected))
        }

        for p in &self.properties {
            if !p.attrs.is_empty() {
                panic!("{}: unexpected attributes", util::NON_USER_ERROR)
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
