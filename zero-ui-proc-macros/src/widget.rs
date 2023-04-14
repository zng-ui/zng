use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{ext::IdentExt, parse::Parse, spanned::Spanned, *};

use crate::{
    util::{self, parse_outer_attrs, ErrorRecoverable, Errors},
    widget_util::{self, WgtProperty, WgtWhen},
};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream, mixin: bool) -> proc_macro::TokenStream {
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
    let mut parent;

    if let Fields::Unnamed(f) = &struct_.fields {
        if f.unnamed.len() != 1 {
            let mut r = syn::Error::new(struct_.fields.span(), "expected `struct Name(Parent);`")
                .to_compile_error()
                .to_token_stream();
            struct_.to_tokens(&mut r);
            return r.into();
        }
        parent = f.unnamed[0].to_token_stream();
    } else {
        let mut r = syn::Error::new(struct_.fields.span(), "expected `struct Name(Parent);`")
            .to_compile_error()
            .to_token_stream();
        struct_.to_tokens(&mut r);
        return r.into();
    }

    let crate_core = util::crate_core();

    let (mixin_p, mixin_p_bounded) = if mixin {
        parent = quote! {
            #crate_core::widget_base::WidgetMix<#parent>
        };

        if struct_.generics.params.is_empty() {
            let mut r = syn::Error::new(struct_.ident.span(), "mix-ins must have one generic `P`")
                .to_compile_error()
                .to_token_stream();
            struct_.to_tokens(&mut r);
            return r.into();
        } else if struct_.generics.params.len() > 1 {
            let mut r = syn::Error::new(struct_.generics.span(), "mix-ins must have one generic `P` only")
                .to_compile_error()
                .to_token_stream();
            struct_.to_tokens(&mut r);
            return r.into();
        }
        match struct_.generics.params.first().unwrap() {
            GenericParam::Lifetime(l) => {
                let mut r = syn::Error::new(l.span(), "mix-ins must have one generic `P` only")
                    .to_compile_error()
                    .to_token_stream();
                struct_.to_tokens(&mut r);
                return r.into();
            }
            GenericParam::Const(c) => {
                let mut r = syn::Error::new(c.span(), "mix-ins must have one generic `P` only")
                    .to_compile_error()
                    .to_token_stream();
                struct_.to_tokens(&mut r);
                return r.into();
            }

            GenericParam::Type(t) => {
                if !t.bounds.is_empty() || t.default.is_some() {
                    let mut r = syn::Error::new(t.span(), "mix-ins must have one unbounded generic `P`")
                        .to_compile_error()
                        .to_token_stream();
                    struct_.to_tokens(&mut r);
                    return r.into();
                }
                if let Some(where_) = &struct_.generics.where_clause {
                    let mut r = syn::Error::new(where_.span(), "mix-ins must have one unbounded generic `P`")
                        .to_compile_error()
                        .to_token_stream();
                    struct_.to_tokens(&mut r);
                    return r.into();
                }

                let id = &t.ident;
                (quote!(<#id>), quote!(<#id : #crate_core::widget_base::WidgetImpl>))
            }
        }
    } else {
        if !struct_.generics.params.is_empty() {
            let mut r = syn::Error::new(struct_.generics.span(), "widgets cannot be generic")
                .to_compile_error()
                .to_token_stream();
            struct_.to_tokens(&mut r);
            return r.into();
        }
        (quote!(), quote!())
    };

    // a `$crate` path to the widget struct.
    let (struct_path, custom_rules) = if mixin {
        (quote!(), vec![])
    } else {
        match syn::parse::<Args>(args) {
            Ok(a) => (a.path.to_token_stream(), a.custom_rules),
            Err(e) => {
                let mut r = e.to_compile_error().to_token_stream();
                struct_.to_tokens(&mut r);
                return r.into();
            }
        }
    };

    let vis = struct_.vis;
    let ident = struct_.ident;
    let mut attrs = util::Attributes::new(struct_.attrs);
    if !mixin {
        attrs.tag_doc("W", "This struct is also a widget macro");
    }

    let struct_token = struct_.struct_token;

    let (macro_r, start_r) = if mixin {
        (quote!(), quote!())
    } else {
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
            #crate_core::widget_new! {
                start {
                    let mut wgt__ = #struct_path::start();
                    let wgt__ = &mut wgt__;
                }
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

        let r = quote! {
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
            #validate_path
        };

        let start_r = quote! {
            impl #mixin_p_bounded #ident #mixin_p {
                /// Start building a new instance.
                pub fn start() -> Self {
                    <Self as #crate_core::widget_base::WidgetImpl>::inherit(Self::widget_type())
                }

                /// Gets the widget type info.
                pub fn widget_type() -> #crate_core::widget_builder::WidgetType {
                    #crate_core::widget_builder::WidgetType {
                        type_id: std::any::TypeId::of::<Self>(),
                        path: #struct_path_str,
                        location: #crate_core::widget_builder::source_location!(),
                    }
                }
            }
        };

        (r, start_r)
    };

    let r = quote! {
        #attrs
        #vis #struct_token #ident #mixin_p {
            base: #parent,
            started: bool,
        }
        impl #mixin_p_bounded std::ops::Deref for #ident #mixin_p {
            type Target = #parent;

            fn deref(&self) -> &Self::Target {
                &self.base
            }
        }
        impl #mixin_p_bounded std::ops::DerefMut for #ident #mixin_p {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.base
            }
        }
        #start_r

        #[doc(hidden)]
        impl #mixin_p_bounded #crate_core::widget_base::WidgetImpl for #ident #mixin_p {
            fn inherit(widget: #crate_core::widget_builder::WidgetType) -> Self {
                let mut wgt = Self {
                    base: <#parent as #crate_core::widget_base::WidgetImpl>::inherit(widget),
                    started: false,
                };
                wgt.on_start__();
                wgt
            }

            fn base(&mut self) -> &mut #crate_core::widget_base::WidgetBase {
                #crate_core::widget_base::WidgetImpl::base(&mut self.base)
            }

            fn base_ref(&self) -> &#crate_core::widget_base::WidgetBase {
                #crate_core::widget_base::WidgetImpl::base_ref(&self.base)
            }

            fn info_instance__() -> Self {
                Self {
                    base: <#parent as #crate_core::widget_base::WidgetImpl>::info_instance__(),
                    started: false,
                }
            }
        }

        #macro_r
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
                || input.peek(Ident::peek_any)
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
                                || input.peek(Ident::peek_any)
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
    let NewArgs {
        start,
        end,
        properties: mut p,
    } = parse_macro_input!(args as NewArgs);

    let core = util::crate_core();

    let mut set_props = quote!();
    for prop in &p.properties {
        set_props.extend(prop_assign(prop, &mut p.errors, false));
    }

    let mut set_whens = quote!();
    for when in &p.whens {
        let when_expr = match syn::parse2::<widget_util::WhenExpr>(when.condition_expr.clone()) {
            Ok(w) => w,
            Err(e) => {
                p.errors.push_syn(e);
                continue;
            }
        };

        let mut when_expr_vars = quote!();
        let mut inputs = quote!();
        for ((property, member), var) in when_expr.inputs {
            let (property, generics) = widget_util::split_path_generics(property).unwrap();
            let p_ident = &property.segments.last().unwrap().ident;
            let p_meta = ident_spanned!(p_ident.span()=> "{p_ident}_meta__");
            let var_input = ident!("{var}_in__");
            let member_ident = ident_spanned!(property.span()=> "__w_{member}__");

            let member = match member {
                widget_util::WhenInputMember::Named(ident) => {
                    let ident_str = ident.to_string();
                    quote! {
                        Named(#ident_str)
                    }
                }
                widget_util::WhenInputMember::Index(i) => quote! {
                    Index(#i)
                },
            };

            macro_rules! quote_call {
                (#$mtd:ident ( $($args:tt)* )) => {
                    if property.get_ident().is_some() {
                        quote! {
                            wgt__.#$mtd #generics($($args)*);
                        }
                    } else {
                        quote! {
                            #property::#$mtd #generics(#core::widget_base::WidgetImpl::base(&mut *wgt__), $($args)*);
                        }
                    }
                }
            }

            let get_meta = quote_call!(#p_meta());

            when_expr_vars.extend(quote! {
                let (#var_input, #var) = {
                    let meta__ =  #get_meta
                    meta__.allowed_in_when_expr();
                    meta__.inputs #generics().#member_ident()
                };
            });

            inputs.extend(quote! {
                {
                    let meta__ = #get_meta
                    #core::widget_builder::WhenInput {
                        property: meta__.id(),
                        member: #core::widget_builder::WhenInputMember::#member,
                        var: #var_input,
                        property_default: meta__ .default_fn #generics(),
                    }
                },
            });
        }

        let mut assigns = quote!();
        for prop in &when.assigns {
            assigns.extend(prop_assign(prop, &mut p.errors, true));
        }

        let attrs = when.attrs.cfg_and_lints();
        let expr = when_expr.expr;
        let expr_str = &when.condition_expr_str;

        set_whens.extend(quote! {
            #attrs {
                #when_expr_vars
                let inputs__ = std::boxed::Box::new([
                    #inputs
                ]);
                #core::widget_base::WidgetImpl::base(&mut *wgt__).start_when_block(
                    inputs__,
                    #core::widget_builder::when_condition_expr_var! { #expr },
                    #expr_str,
                    #core::widget_builder::source_location!(),
                );

                #assigns

                #core::widget_base::WidgetImpl::base(&mut *wgt__).end_when_block();
            }
        });
    }

    let errors = p.errors;

    let r = quote! {
        #errors
        {
            #start
            #set_props
            #set_whens
            #end
        }
    };

    r.into()
}

fn prop_assign(prop: &WgtProperty, errors: &mut Errors, is_when: bool) -> TokenStream {
    let core = util::crate_core();

    let custom_expand = if prop.has_custom_attrs() {
        prop.custom_attrs_expand(ident!("wgt__"), is_when)
    } else {
        quote!()
    };
    let attrs = prop.attrs.cfg_and_lints();

    let ident = prop.ident();
    let path = &prop.path;

    let generics = &prop.generics;

    macro_rules! quote_call {
        (#$mtd:ident ( $($args:tt)* )) => {
            if path.get_ident().is_some() {
                quote! {
                    wgt__.#$mtd #generics($($args)*);
                }
            } else {
                quote! {
                    #path::#$mtd #generics(#core::widget_base::WidgetImpl::base(&mut *wgt__), $($args)*);
                }
            }
        }
    }

    let prop_init;

    match &prop.value {
        Some((_, val)) => match val {
            widget_util::PropertyValue::Special(special, _) => {
                if prop.is_unset() {
                    let unset_ident = ident_spanned!(ident.span()=> "unset_{}", ident);
                    prop_init = quote_call! {
                        #unset_ident()
                    };
                } else {
                    errors.push("unknown value, expected `unset!`", special.span());
                    return quote!();
                }
            }
            widget_util::PropertyValue::Unnamed(val) => {
                prop_init = quote_call! {
                    #ident(#val)
                };
            }
            widget_util::PropertyValue::Named(_, fields) => {
                let mut idents_sorted: Vec<_> = fields.iter().map(|f| &f.ident).collect();
                idents_sorted.sort();
                let idents = fields.iter().map(|f| &f.ident);
                let values = fields.iter().map(|f| &f.expr);
                let ident_sorted = ident_spanned!(ident.span()=> "{}_sorted__", ident);
                let ident_meta = ident_spanned!(ident.span()=> "{}_meta__", ident);

                let call = quote_call! {
                    #ident_sorted(#(#idents_sorted),*)
                };
                let meta = if path.get_ident().is_some() {
                    quote! {
                        wgt__.#ident_meta()
                    }
                } else {
                    quote! {
                        <#core::widget_builder::WgtInfo as #path>::#ident_meta(&#core::widget_builder::WgtInfo)
                    }
                };
                prop_init = quote! {
                    {
                        let meta__ = #meta;
                        #(
                            let #idents = meta__.inputs().#idents(#values);
                        )*
                        #call
                    }
                };
            }
        },
        None => match prop.path.get_ident() {
            Some(_) => {
                prop_init = quote_call! {
                    #ident(#ident)
                };
            }
            None => {
                errors.push("missing value", util::path_span(&prop.path));
                return quote!();
            }
        },
    }

    quote! {
        #attrs {
            #custom_expand
            #prop_init
        }
    }
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
