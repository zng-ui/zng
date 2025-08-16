use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{ext::IdentExt, parse::Parse, spanned::Spanned, *};

use crate::{
    util::{self, ErrorRecoverable, Errors, parse_outer_attrs},
    widget_util::{self, WgtItem, WgtProperty, WgtWhen},
};

lazy_static! {
    static ref DOCS_JS: String = {
        let js = include_str!("../js/widget.js");
        let js = minifier::js::minify(js);
        format!("<script>{js}</script>")
    };
}

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream, mixin: bool) -> proc_macro::TokenStream {
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
                (quote!(<#id>), quote!(<#id : #crate_core::widget::base::WidgetImpl>))
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
        if !args.is_empty() {
            let span = match syn::parse::<Args>(args) {
                Ok(a) => a.path.span(),
                Err(e) => e.span(),
            };
            let mut r = syn::Error::new(span, "mix-ins do not need a `$crate` path")
                .to_compile_error()
                .to_token_stream();
            struct_.to_tokens(&mut r);
            return r.into();
        }

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
    if mixin {
        attrs.tag_doc("m", "Widget mix-in struct");
    } else {
        attrs.tag_doc("W", "Widget struct and macro");
    }
    let allow_deprecated = attrs.deprecated.as_ref().map(|_| {
        quote! {
            #[allow(deprecated)]
        }
    });

    let struct_token = struct_.struct_token;

    let (macro_r, start_r) = if mixin {
        (quote!(), quote!())
    } else {
        let struct_path_str = struct_path.to_string().replace(' ', "");
        let struct_path_slug = path_slug(&struct_path_str);

        let val_span = util::last_span(struct_path.clone());
        let validate_path_ident = ident_spanned!(val_span=> "zzz_widget_path_{struct_path_slug}");
        let validate_path = quote_spanned! {val_span=>
            #[doc(hidden)]
            #[allow(unused)]
            #[allow(non_snake_case)]
            mod #validate_path_ident {
                macro_rules! #validate_path_ident {
                    () => {
                        #allow_deprecated
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

        let macro_ident = ident!("{}__", struct_path_slug);

        // rust-analyzer does not find the macro if we don't set the call_site here.
        let struct_path = util::set_stream_span(struct_path, Span::call_site());

        let macro_new = quote! {
            zng::__proc_macro_util::widget::widget_new! {
                new {
                    let mut wgt__ = #struct_path::widget_new();
                    let wgt__ = &mut wgt__;
                }
                build {
                    let wgt__ = wgt__.widget_build();
                    wgt__
                }
                set { $($tt)* }
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
                (zng_widget: $($tt:tt)*) => {
                    #macro_new
                };

                // enforce normal syntax, property = <expr> ..
                ($(#[$attr:meta])* $($property_path:ident)::+ = $($rest:tt)*) => {
                    #struct_path! {
                        zng_widget: $(#[$attr])* $($property_path)::+ = $($rest)*
                    }
                };
                // enforce normal syntax, when <expr> { .. } ..
                ($(#[$attr:meta])* when $($rest:tt)*) => {
                    #struct_path! {
                        zng_widget: $(#[$attr])* when $($rest)*
                    }
                };

                // custom rules, can be (<expr>), why we need enforce some rules
                #custom_rules

                // fallback, single property shorthand or error.
                ($($tt:tt)*) => {
                    #struct_path! {
                        zng_widget: $($tt)*
                    }
                };
            }
            #[doc(hidden)]
            #[allow(unused_imports)]
            #vis use #macro_ident as #ident;
            #validate_path
        };

        let source_location = widget_util::source_location(&crate_core, ident.span());
        let start_r = quote! {
            #allow_deprecated
            impl #mixin_p_bounded #ident #mixin_p {
                /// Start building a new instance.
                pub fn widget_new() -> Self {
                    <Self as #crate_core::widget::base::WidgetImpl>::inherit(Self::widget_type())
                }

                /// Gets the widget type info.
                pub fn widget_type() -> #crate_core::widget::builder::WidgetType {
                    #crate_core::widget::builder::WidgetType::new(
                        std::any::TypeId::of::<Self>(), #struct_path_str, #source_location,
                    )
                }
            }
        };

        (r, start_r)
    };

    let docs_js = if attrs.docs.is_empty() {
        // cause docs missing warning
        quote!()
    } else {
        let docs_js = DOCS_JS.as_str();
        quote!(#[doc=#docs_js])
    };

    let r = quote! {
        #attrs
        #docs_js
        #vis #struct_token #ident #mixin_p(#parent);
        #allow_deprecated
        impl #mixin_p std::ops::Deref for #ident #mixin_p {
            type Target = #parent;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        #allow_deprecated
        impl #mixin_p std::ops::DerefMut for #ident #mixin_p {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
        #start_r

        #[doc(hidden)]
        #allow_deprecated
        impl #mixin_p_bounded #crate_core::widget::base::WidgetImpl for #ident #mixin_p {
            fn inherit(widget: #crate_core::widget::builder::WidgetType) -> Self {
                let mut wgt = Self(<#parent as #crate_core::widget::base::WidgetImpl>::inherit(widget));
                *#crate_core::widget::base::WidgetImpl::base(&mut wgt).widget_importance() = #crate_core::widget::builder::Importance::WIDGET;
                {
                    use #crate_core::widget::base::WidgetImpl;
                    wgt.widget_intrinsic();
                }
                *#crate_core::widget::base::WidgetImpl::base(&mut wgt).widget_importance() = #crate_core::widget::builder::Importance::INSTANCE;
                wgt
            }

            fn base(&mut self) -> &mut #crate_core::widget::base::WidgetBase {
                #crate_core::widget::base::WidgetImpl::base(&mut self.0)
            }

            fn base_ref(&self) -> &#crate_core::widget::base::WidgetBase {
                #crate_core::widget::base::WidgetImpl::base_ref(&self.0)
            }

            fn info_instance__() -> Self {
                Self(<#parent as #crate_core::widget::base::WidgetImpl>::info_instance__())
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
    items: Vec<WgtItem>,
}
impl Parse for Properties {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut errors = Errors::default();
        let mut items = vec![];

        while !input.is_empty() {
            let attrs = parse_outer_attrs(input, &mut errors);

            if input.peek(widget_util::keyword::when) {
                if let Some(mut when) = WgtWhen::parse(input, &mut errors) {
                    when.attrs = util::Attributes::new(attrs);
                    items.push(WgtItem::When(when));
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
                        items.push(WgtItem::Property(p));
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

        Ok(Properties { errors, items })
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

    let mut items = quote!();

    for item in &p.items {
        match item {
            WgtItem::Property(prop) => {
                items.extend(prop_assign(prop, &mut p.errors, false));
            }
            WgtItem::When(when) => {
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
                    let p_meta = ident_spanned!(p_ident.span()=> "{p_ident}_");
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
                                    #property::#$mtd #generics(#core::widget::base::WidgetImpl::base(&mut *wgt__), $($args)*);
                                }
                            }
                        }
                    }

                    let get_meta = quote_call!(#p_meta());

                    when_expr_vars.extend(quote! {
                        let (#var_input, #var) = {
                            let meta__ = #get_meta
                            meta__.allowed_in_when_expr();
                            meta__.inputs #generics().#member_ident()
                        };
                    });

                    inputs.extend(quote! {
                        {
                            let meta__ = #get_meta
                            #core::widget::builder::WhenInput {
                                property: meta__.id(),
                                member: #core::widget::builder::WhenInputMember::#member,
                                var: #var_input,
                                property_default: meta__ .default_fn #generics(),
                                _non_exhaustive: ()
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

                let box_expr = quote_spanned! {expr.span()=>
                    #core::var::expr_var!{#expr}
                };

                let source_location = widget_util::source_location(&core, Span::call_site());
                items.extend(quote! {
                    #attrs {
                        #when_expr_vars
                        let inputs__ = std::boxed::Box::new([
                            #inputs
                        ]);
                        #core::widget::base::WidgetImpl::base(&mut *wgt__).start_when_block(
                            inputs__,
                            #box_expr,
                            #expr_str,
                            #source_location,
                        );

                        #assigns

                        #core::widget::base::WidgetImpl::base(&mut *wgt__).end_when_block();
                    }
                });
            }
        }
    }

    let errors = p.errors;

    let r = quote! {
        {
            #errors

            #start
            #items
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
                    #path::#$mtd #generics(#core::widget::base::WidgetImpl::base(&mut *wgt__), $($args)*);
                }
            }
        }
    }

    let ident_meta = ident_spanned!(ident.span()=> "{}_", ident);

    let when_check = if is_when {
        let meta = quote_call!(#ident_meta());
        quote! {
            {
                let meta__ = #meta
                meta__.allowed_in_when_assign();
            }
        }
    } else {
        quote!()
    };

    let prop_init;

    match &prop.value {
        Some((_, val)) => match val {
            widget_util::PropertyValue::Special(special, _) => {
                if prop.is_unset() {
                    let unset_ident = ident_spanned!(ident.span()=> "unset_{}", ident);
                    prop_init = quote_call!(#unset_ident());
                } else {
                    errors.push("unknown value, expected `unset!`", special.span());
                    return quote!();
                }
            }
            widget_util::PropertyValue::Unnamed(val) => {
                prop_init = quote_call!( #ident(#val));
            }
            widget_util::PropertyValue::Named(_, fields) => {
                let mut idents_sorted: Vec<_> = fields.iter().map(|f| &f.ident).collect();
                idents_sorted.sort();
                let idents = fields.iter().map(|f| &f.ident);
                let values = fields.iter().map(|f| &f.expr);
                let ident_sorted = ident_spanned!(ident.span()=> "{}__", ident);

                let call = quote_call!(#ident_sorted(#(#idents_sorted),*));
                let meta = if path.get_ident().is_some() {
                    quote! {
                        wgt__.#ident_meta()
                    }
                } else {
                    quote! {
                        <#core::widget::builder::WgtInfo as #path>::#ident_meta(&#core::widget::builder::WgtInfo)
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
        None => {
            let ident = &prop.path.segments.last().unwrap().ident;
            prop_init = quote_call!(#ident(#ident));
        }
    }

    quote! {
        #attrs {
            #when_check
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
            start: non_user_braced!(input, "new").parse().unwrap(),
            end: non_user_braced!(input, "build").parse().unwrap(),
            properties: non_user_braced!(input, "set").parse()?,
        })
    }
}
