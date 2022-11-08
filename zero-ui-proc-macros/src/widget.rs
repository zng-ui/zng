use std::mem;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{parse::Parse, spanned::Spanned, *};

use crate::{
    util::{self, parse_outer_attrs, path_span, ErrorRecoverable, Errors},
    widget_util::{self, WgtProperty, WgtWhen},
};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream, mixin: bool) -> proc_macro::TokenStream {
    // the widget mod declaration.
    let mod_ = parse_macro_input!(input as ItemMod);

    if mod_.content.is_none() {
        let mut r = syn::Error::new(mod_.semi.span(), "only modules with inline content are supported")
            .to_compile_error()
            .to_token_stream();

        mod_.to_tokens(&mut r);

        return r.into();
    }

    let (mod_braces, items) = mod_.content.unwrap();

    // accumulate the most errors as possible before returning.
    let mut errors = Errors::default();

    let crate_core = util::crate_core();

    let vis = mod_.vis;
    let ident = mod_.ident;
    let mod_token = mod_.mod_token;
    let mut attrs = util::Attributes::new(mod_.attrs);
    attrs.tag_doc("W", "this module is also a widget macro");

    if mixin && !ident.to_string().ends_with("_mixin") {
        errors.push("mix-in names must end with suffix `_mixin`", ident.span());
    }

    // a `$crate` path to the widget module.
    let mod_path = match syn::parse::<ArgPath>(args) {
        Ok(a) => a.path,
        Err(e) => {
            errors.push_syn(e);
            quote! { $crate::missing_widget_path}
        }
    };
    let mod_path_str = mod_path.to_string().replace(' ', "");
    let mod_path_slug = mod_path_slug(&mod_path_str);

    let val_span = util::last_span(mod_path.clone());
    let validate_path_ident = ident_spanned!(val_span=> "__widget_path_{mod_path_slug}__");
    let validate_path = quote_spanned! {val_span=>
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub enum #validate_path_ident { }

        #[doc(hidden)]
        #[allow(unused)]
        mod __validate_path__ {
            macro_rules! #validate_path_ident {
                () => {
                    pub use #mod_path::#validate_path_ident;
                }
            }
            #validate_path_ident!{}
        }
    };

    let WidgetItems {
        uses,
        inherits,
        mut properties,
        mut include_fn,
        mut build_fn,
        others,
    } = WidgetItems::new(items, &mut errors);

    let mut include_item_imports = quote!();

    let mut has_parent = false;

    for inh in &inherits {
        let is_parent = !inh.has_mixin_suffix();
        if has_parent && is_parent {
            errors.push("can only inherit from one widget and multiple mix-ins", inh.path.span());
            continue;
        }

        has_parent |= is_parent;
        let attrs = &inh.attrs;
        let path = &inh.path;
        include_item_imports.extend(quote_spanned! {path_span(path)=>
            #(#attrs)*
            #path::include(__wgt__);
        });
    }

    let mut custom_include_docs = vec![];

    if let Some(include) = &mut include_fn {
        let attrs = util::Attributes::new(mem::take(&mut include.attrs));
        custom_include_docs = attrs.docs;
        include.attrs.extend(attrs.cfg);
        include.attrs.extend(attrs.lints);
        include.attrs.extend(attrs.inline);
        include.attrs.extend(attrs.others);
        include.vis = Visibility::Inherited;
        include.sig.ident = ident_spanned!(include.sig.ident.span()=> "__wgt_include__");
        include_item_imports.extend(quote_spanned! {include.span()=>
            self::__wgt_include__(__wgt__);
        });
    }

    let mut capture_decl = quote!();
    let mut pre_bind = quote!();

    for prop in properties.iter_mut().flat_map(|i| i.properties.iter_mut()) {
        capture_decl.extend(prop.declare_capture());
        pre_bind.extend(prop.pre_bind_args(false, None, ""));
    }
    for (i, when) in properties.iter_mut().flat_map(|i| i.whens.iter_mut()).enumerate() {
        pre_bind.extend(when.pre_bind(false, i));
    }

    let mut include_items = quote!();

    for prop in properties.iter().flat_map(|i| i.properties.iter()) {
        if prop.has_args() {
            let cfg = &prop.attrs.cfg;
            let lints = &prop.attrs.lints;
            let args = prop.args_new(quote!(#crate_core::widget_builder));
            include_items.extend(quote! {
                #cfg
                #(#lints)*
                __wgt__.push_property(#crate_core::widget_builder::Importance::WIDGET, #args);
            });
        } else if prop.is_unset() {
            let cfg = &prop.attrs.cfg;
            let id = prop.property_id();
            include_items.extend(quote! {
                #cfg
                __wgt__.push_unset(#crate_core::widget_builder::Importance::WIDGET, #id);
            });
        }
    }

    for when in properties.iter().flat_map(|i| i.whens.iter()) {
        let cfg = &when.attrs.cfg;
        let lints = &when.attrs.lints;
        let args = when.when_new(quote!(#crate_core::widget_builder));
        include_items.extend(quote! {
            #cfg
            #(#lints)*
            __wgt__.push_when(#crate_core::widget_builder::Importance::WIDGET, #args);
        });
    }

    let macro_if_mixin = if mixin {
        quote! {
            (>> if mixin { $($tt:tt)* }) => {
                $($tt)*
            };
            (>> if !mixin { $($tt:tt)* }) => {
                // ignore
            };
        }
    } else {
        quote! {
            (>> if !mixin { $($tt:tt)* }) => {
                $($tt)*
            };
            (>> if mixin { $($tt:tt)* }) => {
                // ignore
            };
        }
    };

    let build = if mixin {
        if let Some(build) = &build_fn {
            errors.push("mix-ins cannot have a build function", build.sig.ident.span());
        }

        let error = "mixin-in cannot inherit from full widget";
        let mut check = quote!();
        for inh in inherits.iter() {
            let path = &inh.path;
            check.extend(quote_spanned! {path_span(path)=>
                #path! {
                    >> if !mixin {
                        std::compile_error!{#error}
                    }
                }
            });
        }

        check
    } else if let Some(build) = &mut build_fn {
        let attrs = util::Attributes::new(mem::take(&mut build.attrs));
        let docs = attrs.docs;
        build.attrs.extend(attrs.cfg);
        build.attrs.extend(attrs.lints);
        build.attrs.extend(attrs.inline);
        build.attrs.extend(attrs.others);
        build.vis = Visibility::Inherited;
        build.sig.ident = ident_spanned!(build.sig.ident.span()=> "__build__");
        let out = &build.sig.output;
        quote_spanned! {build.span()=>
            /// Build the widget.
            /// 
            /// The widget macro calls this function to build the widget instance.
            /// 
            #(#docs)*
            pub fn build(builder: #crate_core::widget_builder::WidgetBuilder) #out {
                self::__build__(builder)
            }
        }
    } else if let Some(inh) = inherits.iter().find(|m| !m.has_mixin_suffix()) {
        let path = &inh.path;
        let id = path.segments.last().map(|s| &s.ident).unwrap();
        let error = format!("cannot inherit build from `{id}`, it is a mix-in\nmix-ins with suffix `_mixin` are ignored when inheriting build, but this one was renamed");
        quote_spanned! {path_span(path)=>
            #path! {
                >> if mixin {
                    std::compile_error!{ #error }
                }
            }
            #path! {
                >> if !mixin {
                    #[doc(inline)]
                    #[allow(unused_imports)]
                    pub use #path::build;
                }
            }
        }
    } else {
        errors.push(
            "missing `fn build(WidgetBuilder) -> T` function, must be provided or inherited",
            ident.span(),
        );
        quote! {
            /// placeholder
            pub fn build(_: #crate_core::widget_builder::WidgetBuilder) -> #crate_core::widget_instance::NilUiNode {
                #crate_core::widget_instance::NilUiNode
            }
        }
    };

    let mut inherit_export = quote!();

    for Inherit { attrs, path } in inherits {
        inherit_export.extend(quote_spanned! {path_span(&path)=>
            #(#attrs)*
            #[allow(unused_imports)]
            #[doc(no_inline)]
            pub use #path::*;
        });
    }
    for p in properties.iter().flat_map(|p| p.properties.iter()) {
        inherit_export.extend(p.reexport());
    }

    let macro_ident = ident!("__wgt_{}__", mod_path_slug);

    // !!: move auto-docs to `pub fn include`.
    let docs_span = properties.first().map(|p| p.properties_span).unwrap_or_else(Span::call_site);

    let mut doc_assigns = quote!();
    let mut doc_unsets = quote!();
    let mut doc_whens = quote!();
    for p in &properties {
        for p in &p.properties {
            if p.is_unset() {
                if doc_unsets.is_empty() {
                    doc_unsets.extend(quote_spanned! {docs_span=>
                        ///
                        /// # Default Unsets
                        ///
                        /// These properties are `unset!` by default:
                        ///
                    });
                }
                let doc = format!("* [`{0}`](fn@properties::{0})", p.ident());
                doc_unsets.extend(quote_spanned! {docs_span=>
                    #[doc=#doc]
                });
            } else if p.has_args() {
                if doc_assigns.is_empty() {
                    doc_assigns.extend(quote_spanned! {docs_span=>
                        ///
                        /// # Default Assigns
                        ///
                        /// These properties are set by default:
                        ///
                    });
                }

                let doc = if p.is_private() {
                    format!("* `{}`", p.ident())
                } else {
                    format!("* [`{0}`](fn@properties::{0})", p.ident())
                };
                doc_assigns.extend(quote_spanned! {docs_span=>
                    #[doc=#doc]
                });
            }
        }
        for w in &p.whens {
            if doc_whens.is_empty() {
                doc_unsets.extend(quote_spanned! {docs_span=>
                    ///
                    /// # Default Whens
                    ///
                    /// These `when` assigns are set by default:
                    ///
                });
            }
            let doc = format!("### `when {}`", w.condition_expr);
            doc_whens.extend(quote_spanned! {docs_span=>
                ///
                #[doc=#doc]
                ///
            });
            for p in &w.assigns {
                let doc = format!("* `{0}`", p.ident());
                doc_whens.extend(quote_spanned! {docs_span=>
                    #[doc=#doc]
                });
            }
        }
    }

    let docs_intro = if mixin {
        quote_spanned! {docs_span=>
            /// Include mix-in built-ins.
            ///
            /// This function is called by all widgets that inherit from this mix-in.
            /// 
            #(#custom_include_docs)*
        }
    } else {
        quote_spanned! {docs_span=>
            /// Include widget built-ins.
            ///
            /// The widget macro calls this function to start building the widget. This function is also called by
            /// inheritor widgets.
            /// 
            #(#custom_include_docs)*
        }
    };

    let mod_items = quote! {
        #validate_path

        // custom items
        #(#others)*

        // use items (after custom items in case of custom macro_rules re-export)
        #(#uses)*

        #inherit_export
        #capture_decl

        #include_fn

        #docs_intro
        #doc_assigns
        #doc_unsets
        #doc_whens
        pub fn include(builder: &mut #crate_core::widget_builder::WidgetBuilder) {
            let __wgt__ = builder;
            #include_item_imports
            #pre_bind
            #include_items
        }

        #build_fn
        #build

        #[doc(hidden)]
        #[allow(unused_imports)]
        pub mod __widget__ {
            pub use #crate_core::{widget_new, widget_builder};

            pub fn mod_info() -> widget_builder::WidgetMod {
                static impl_id: widget_builder::StaticWidgetImplId = widget_builder::StaticWidgetImplId::new_unique();

                widget_builder::WidgetMod {
                    impl_id: impl_id.get(),
                    path: #mod_path_str,
                    location: widget_builder::source_location!(),
                }
            }

            pub fn new() -> widget_builder::WidgetBuilder {
                let mut wgt = widget_builder::WidgetBuilder::new(mod_info());
                super::include(&mut wgt);
                wgt
            }
        }
    };

    let mut mod_block = quote!();
    mod_braces.surround(&mut mod_block, |t| t.extend(mod_items));

    // rust-analyzer does not find the macro if we don't set the call_site here.
    let mod_path = util::set_stream_span(mod_path, Span::call_site());

    let macro_new = if mixin {
        quote! {
            std::compile_error!{"cannot instantiate mix-in"}
        }
    } else {
        quote! {
            #mod_path::__widget__::widget_new! {
                widget { #mod_path }
                new { $($tt)* }
            }
        }
    };

    let r = quote! {
        #attrs
        #vis #mod_token #ident #mod_block

        #[doc(hidden)]
        #[macro_export]
        macro_rules! #macro_ident {
            #macro_if_mixin

            ($($tt:tt)*) => {
                #macro_new
            };
        }
        #[doc(hidden)]
        #[allow(unused_imports)]
        #vis use #macro_ident as #ident;

        #errors
    };
    r.into()
}

struct ArgPath {
    path: TokenStream,
}
impl Parse for ArgPath {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        let fork = input.fork();
        match (fork.parse::<Token![$]>(), fork.parse::<syn::Path>()) {
            (Ok(_), Ok(p)) => {
                if fork.is_empty() {
                    if p.segments[0].ident == "crate" {
                        Ok(ArgPath {
                            path: input.parse().unwrap(),
                        })
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

struct WidgetItems {
    uses: Vec<ItemUse>,
    inherits: Vec<Inherit>,
    properties: Vec<Properties>,
    include_fn: Option<ItemFn>,
    build_fn: Option<ItemFn>,
    others: Vec<Item>,
}
impl WidgetItems {
    fn new(items: Vec<Item>, errors: &mut Errors) -> Self {
        let mut uses = vec![];
        let mut inherits = vec![];
        let mut properties = vec![];
        let mut include_fn = None;
        let mut build_fn = None;
        let mut others = vec![];

        for item in items {
            match item {
                Item::Use(use_) => {
                    uses.push(use_);
                }
                // match properties!
                Item::Macro(ItemMacro { mac, ident: None, .. }) if mac.path.get_ident().map(|i| i == "properties").unwrap_or(false) => {
                    match syn::parse2::<Properties>(mac.tokens) {
                        Ok(mut p) => {
                            errors.extend(mem::take(&mut p.errors));
                            p.properties_span = path_span(&mac.path);
                            properties.push(p)
                        }
                        Err(e) => errors.push_syn(e),
                    }
                }
                // match inherit!
                Item::Macro(ItemMacro {
                    mac, attrs, ident: None, ..
                }) if mac.path.get_ident().map(|i| i == "inherit").unwrap_or(false) => match parse2::<Inherit>(mac.tokens) {
                    Ok(mut ps) => {
                        ps.attrs.extend(attrs);
                        inherits.push(ps)
                    }
                    Err(e) => errors.push_syn(e),
                },

                // match fn include(..)
                Item::Fn(fn_) if fn_.sig.ident == "include" => {
                    include_fn = Some(fn_);
                }
                // match fn build(..)
                Item::Fn(fn_) if fn_.sig.ident == "build" => {
                    build_fn = Some(fn_);
                }
                // other user items.
                item => others.push(item),
            }
        }

        WidgetItems {
            uses,
            inherits,
            properties,
            include_fn,
            build_fn,
            others,
        }
    }
}

struct Inherit {
    attrs: Vec<Attribute>,
    path: Path,
}
impl Inherit {
    fn has_mixin_suffix(&self) -> bool {
        self.path
            .segments
            .last()
            .map(|s| s.ident.to_string().ends_with("_mixin"))
            .unwrap_or(false)
    }
}
impl Parse for Inherit {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        Ok(Inherit {
            attrs: vec![],
            path: input.parse()?,
        })
    }
}

struct Properties {
    errors: Errors,
    properties_span: Span,
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

        Ok(Properties {
            errors,
            properties_span: Span::call_site(),
            properties,
            whens,
        })
    }
}

fn mod_path_slug(path: &str) -> String {
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
    let NewArgs { widget, properties: mut p } = parse_macro_input!(args as NewArgs);

    let mut pre_bind = quote!();
    for prop in &mut p.properties {
        pre_bind.extend(prop.pre_bind_args(true, None, ""));

        if !matches!(&prop.vis, Visibility::Inherited) {
            p.errors.push("cannot reexport property from instance", prop.vis.span());
        }
    }
    for (i, when) in p.whens.iter_mut().enumerate() {
        pre_bind.extend(when.pre_bind(true, i));
    }

    let mut init = quote!();
    for p in &p.properties {
        let cfg = &p.attrs.cfg;
        if p.is_unset() {
            let id = p.property_id();
            init.extend(quote! {
                #cfg
                __wgt__.push_unset(#widget::__widget__::widget_builder::Importance::INSTANCE, #id);
            });
        } else if p.has_args() {
            let args = p.args_new(quote!(#widget::__widget__::widget_builder));
            init.extend(quote! {
                #cfg
                __wgt__.push_property(#widget::__widget__::widget_builder::Importance::INSTANCE, #args);
            });
        }
    }

    for w in &p.whens {
        let cfg = &w.attrs.cfg;
        let args = w.when_new(quote!(#widget::__widget__::widget_builder));
        init.extend(quote! {
            #cfg
            __wgt__.push_when(#widget::__widget__::widget_builder::Importance::INSTANCE, #args);
        });
    }

    p.errors.to_tokens(&mut init);

    let r = quote! {
        {
            #pre_bind

            let mut __wgt__ = #widget::__widget__::new();
            {
                #[allow(unused_imports)]
                use #widget::*;
                #init
            }
            #widget::build(__wgt__)
        }
    };

    r.into()
}

struct NewArgs {
    widget: TokenStream,
    properties: Properties,
}
impl Parse for NewArgs {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Self {
            widget: non_user_braced!(input, "widget").parse().unwrap(),
            properties: non_user_braced!(input, "new").parse()?,
        })
    }
}
