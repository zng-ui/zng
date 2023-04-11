use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{parse::Parse, spanned::Spanned, *};

use crate::{
    util::{self, parse_outer_attrs, ErrorRecoverable, Errors},
    widget_util::{self, WgtProperty, WgtWhen},
};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    if let Ok(special) = syn::parse::<Ident>(args.clone()) {
        if special == "on_start" {
            let on_start = parse_macro_input!(input as ItemFn);
            let ident = &on_start.sig.ident;

            return quote_spanned! {special.span()=>
                #[doc(hidden)]
                pub fn on_start__(&mut self) {
                    if !self.started {
                        self.started = true;
                        self.#ident();
                    }
                }

                #on_start
            }
            .into();
        }
    }

    // the widget struct declaration.
    let struct_ = parse_macro_input!(input as ItemStruct);
    let parent;

    if let Fields::Unnamed(f) = &struct_.fields {
        if f.unnamed.len() != 1 {
            let mut r = syn::Error::new(struct_.fields.span(), "expected `struct Name(Parent);`")
                .to_compile_error()
                .to_token_stream();
            struct_.to_tokens(&mut r);
            return r.into();
        }
        parent = &f.unnamed[0];
    } else {
        let mut r = syn::Error::new(struct_.fields.span(), "expected `struct Name(Parent);`")
            .to_compile_error()
            .to_token_stream();
        struct_.to_tokens(&mut r);
        return r.into();
    }

    // accumulate the most errors as possible before returning.
    let mut errors = Errors::default();

    let crate_core = util::crate_core();

    let vis = struct_.vis;
    let ident = struct_.ident;
    let mut attrs = util::Attributes::new(struct_.attrs);
    attrs.tag_doc("W", "This struct is also a widget macro");

    // a `$crate` path to the widget struct.
    let (struct_path, custom_rules) = match syn::parse::<Args>(args) {
        Ok(a) => (a.path, a.custom_rules),
        Err(e) => {
            errors.push_syn(e);
            (quote! { $crate::missing_widget_path}, vec![])
        }
    };
    let struct_path_str = struct_path.to_string().replace(' ', "");
    let struct_path_slug = path_slug(&struct_path_str);

    let val_span = util::last_span(struct_path.clone());
    let validate_path_ident = ident_spanned!(val_span=> "__widget_path_{struct_path_slug}__");
    let validate_path = quote_spanned! {val_span=>
        #[doc(hidden)]
        #[allow(unused)]
        #[allow(non_snake_case)]
        mod #validate_path_ident {
            macro_rules! #validate_path_ident {
                () => {
                    use #struct_path;
                }
            }
            #validate_path_ident!{}
        }
    };

    let custom_rules = {
        let mut tt = quote!();
        for widget_util::WidgetCustomRule { rule, init } in custom_rules {
            tt.extend(quote! {
                (#rule) => {
                    #struct_path! {
                        #init
                    }
                };
            })
        }
        tt
    };

    let macro_ident = ident!("__wgt_{}__", struct_path_slug);

    // rust-analyzer does not find the macro if we don't set the call_site here.
    let struct_path = util::set_stream_span(struct_path, Span::call_site());

    let macro_new = quote! {
        $crate::widget_new! {
            start { #struct_path::start() }
            end { wgt__.build() }
            new { $($tt)* }
        }
    };

    let macro_docs = if util::is_rust_analyzer() {
        let docs = &attrs.docs;
        quote! {
            #(#docs)*
        }
    } else {
        quote! {
            #[doc(hidden)]
        }
    };

    let struct_token = struct_.struct_token;

    let r = quote! {
        #attrs
        #vis #struct_token #ident {
            base: #parent,
            started: bool,
        }
        impl std::ops::Deref for #ident {
            type Target = #parent;

            fn deref(&self) -> &Self::Target {
                &self.base
            }
        }
        impl std::ops::DerefMut for #ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.base
            }
        }
        impl #ident {
            /// Start building a new instance.
            pub fn start() -> Self {
                Self::inherit(#crate_core::widget_builder::WidgetType {
                    type_id: std::any::TypeId::of::<Self>(),
                    path: #struct_path_str,
                    location: #crate_core::widget_builder::source_location!(),
                })
            }

            /// Start building a widget derived from this one.
            pub fn inherit(widget: #crate_core::widget_builder::WidgetType) -> Self {
                let mut wgt = Self {
                    base: #parent::inherit(widget),
                    started: false,
                };
                wgt.on_start__();
                wgt
            }
        }

        #macro_docs
        #[macro_export]
        macro_rules! #macro_ident {
            // actual new
            (zero_ui_widget: $($tt:tt)*) => {
                #macro_new
            };

            // enforce normal syntax, property = <expr> ..
            ($(#[$attr:meta])* $property:ident = $($rest:tt)*) => {
                #struct_path! {
                    zero_ui_widget: $(#[$attr])* $property = $($rest)*
                }
            };
            // enforce normal syntax, when <expr> { .. } ..
            ($(#[$attr:meta])* when $($rest:tt)*) => {
                #struct_path! {
                    zero_ui_widget: $(#[$attr])* when $($rest)*
                }
            };

            // custom rules, can be (<expr>), why we need enforce some rules
            #custom_rules

            // fallback, single property shorthand or error.
            ($($tt:tt)*) => {
                #struct_path! {
                    zero_ui_widget: $($tt)*
                }
            };
        }
        #[doc(hidden)]
        #[allow(unused_imports)]
        #vis use #macro_ident as #ident;

        #errors
        #validate_path
    };
    r.into()
}

struct Args {
    path: TokenStream,
    custom_rules: Vec<widget_util::WidgetCustomRule>,
}
impl Parse for Args {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        let fork = input.fork();
        match (fork.parse::<Token![$]>(), fork.parse::<syn::Path>()) {
            (Ok(_), Ok(p)) => {
                let has_custom_rules = fork.peek(token::Brace);
                if has_custom_rules {
                    let _ = fork.parse::<TokenTree>();
                }
                if fork.is_empty() {
                    if p.segments[0].ident == "crate" {
                        let mut raw_parts = vec![];
                        while !input.is_empty() {
                            raw_parts.push(input.parse::<TokenTree>().unwrap());
                        }

                        let mut path = quote!();
                        let mut custom_rules = vec![];

                        if has_custom_rules {
                            let rules = raw_parts.pop().unwrap().to_token_stream();
                            custom_rules = syn::parse2::<widget_util::WidgetCustomRules>(rules)?.rules;
                        }
                        for part in &raw_parts {
                            part.to_tokens(&mut path);
                        }

                        Ok(Args { path, custom_rules })
                    } else {
                        Err(syn::Error::new(p.segments[0].ident.span(), "expected `crate`"))
                    }
                } else {
                    Err(syn::Error::new(fork.span(), "unexpected token"))
                }
            }
            (Ok(_), Err(e)) => {
                if !util::span_is_call_site(e.span()) {
                    Err(e)
                } else {
                    Err(syn::Error::new(util::last_span(input.parse().unwrap()), e.to_string()))
                }
            }
            _ => Err(syn::Error::new(
                input.span(),
                "expected a macro_rules `$crate` path to this widget mod",
            )),
        }
    }
}

struct Properties {
    errors: Errors,
    properties: Vec<WgtProperty>,
    whens: Vec<WgtWhen>,
}
impl Parse for Properties {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut errors = Errors::default();
        let mut properties = vec![];
        let mut whens = vec![];

        while !input.is_empty() {
            let attrs = parse_outer_attrs(input, &mut errors);

            if input.peek(widget_util::keyword::when) {
                if let Some(mut when) = WgtWhen::parse(input, &mut errors) {
                    when.attrs = util::Attributes::new(attrs);
                    whens.push(when);
                }
            } else if input.peek(Token![pub])
                || input.peek(Ident)
                || input.peek(Token![crate])
                || input.peek(Token![super])
                || input.peek(Token![self])
            {
                // peek ident or path (including keywords because of super:: and self::). {
                match input.parse::<WgtProperty>() {
                    Ok(mut p) => {
                        p.attrs = util::Attributes::new(attrs);
                        if !input.is_empty() && p.semi.is_none() {
                            errors.push("expected `;`", input.span());
                            while !(input.is_empty()
                                || input.peek(Ident)
                                || input.peek(Token![crate])
                                || input.peek(Token![super])
                                || input.peek(Token![self])
                                || input.peek(Token![#]) && input.peek(token::Bracket))
                            {
                                // skip to next value item.
                                let _ = input.parse::<TokenTree>();
                            }
                        }
                        properties.push(p);
                    }
                    Err(e) => {
                        let (recoverable, e) = e.recoverable();
                        if recoverable {
                            errors.push_syn(e);
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else {
                errors.push("expected property or when", input.span());

                // suppress the "unexpected token" error from syn parse.
                let _ = input.parse::<TokenStream>();
            }
        }

        Ok(Properties { errors, properties, whens })
    }
}

fn path_slug(path: &str) -> String {
    path.replace("crate", "")
        .replace("::", "_")
        .replace('$', "")
        .trim()
        .replace(' ', "")
}

/*
    NEW
*/

pub fn expand_new(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let NewArgs { start, end, properties: mut p } = parse_macro_input!(args as NewArgs);

    let mut set_props = quote!();
    for prop in &p.properties {
        prop.attrs.to_tokens(&mut set_props);

        let ident = prop.ident();

        let prop_init;

        match &prop.value {
            Some((_, val)) => match val {
                widget_util::PropertyValue::Special(special, _) => {
                    if prop.is_unset() {
                        let unset_ident = ident_spanned!(ident.span()=> "unset_{}", ident);
                        prop_init = quote! {
                            wgt__.#unset_ident();
                        };
                    } else {
                        p.errors.push("unknown value, expected `unset!`", special.span());
                        continue;
                    }
                }
                widget_util::PropertyValue::Unnamed(val) => {
                    prop_init = quote! {
                        wgt__.#ident(#val);
                    };
                }
                widget_util::PropertyValue::Named(_, fields) => {
                    let mut idents_sorted: Vec<_> = fields.iter().map(|f| &f.ident).collect();
                    idents_sorted.sort();
                    let idents = fields.iter().map(|f| &f.ident);
                    let values = fields.iter().map(|f| &f.expr);
                    let ident_sorted = ident_spanned!(ident.span()=> "{}_sorted__", ident);
                    let ident_meta = ident_spanned!(ident.span()=> "{}_meta__", ident);

                    prop_init = quote! {
                        {
                            #(
                                let #idents = wgt_.#ident_meta().inputs().#idents(#values);
                            )*
                            wgt__.#ident_sorted(#(#idents_sorted),*);
                        }
                    };
                }
            },
            None => match prop.path.get_ident() {
                Some(_) => {
                    prop_init = quote! {
                        wgt__.#ident(#ident);
                    };
                }
                None => {
                    p.errors.push("missing value", util::path_span(&prop.path));
                    continue;
                }
            },
        }

        if prop.path.get_ident().is_some() {
            set_props.extend(prop_init);
        } else {
            // trait path
            let path = &prop.path;
            set_props.extend(quote! {
                {
                    use #path;
                    #prop_init
                }
            });
        }
    }

    let r = quote! {
        {
            #[allow(unused_mut)]
            let mut wgt__ = #start;

            #set_props

            #end
        }
    };

    r.into()
}

struct NewArgs {
    start: TokenStream,
    end: TokenStream,
    properties: Properties,
}
impl Parse for NewArgs {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Self {
            start: non_user_braced!(input, "start").parse().unwrap(),
            end: non_user_braced!(input, "end").parse().unwrap(),
            properties: non_user_braced!(input, "new").parse()?,
        })
    }
}
