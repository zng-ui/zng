use quote::ToTokens;
use syn::parse_macro_input;

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as input::MacroArgs);
    let fn_ = parse_macro_input!(input as input::PropertyFn);

    let output = analysis::generate(args, fn_);

    output.to_token_stream().into()
}

pub mod input {
    use crate::util::parse_terminated2;
    use proc_macro2::TokenStream;
    use punctuated::Punctuated;
    use syn::{parse::*, *};

    pub mod keyword {
        syn::custom_keyword!(context);
        syn::custom_keyword!(event);
        syn::custom_keyword!(outer);
        syn::custom_keyword!(size);
        syn::custom_keyword!(inner);
        syn::custom_keyword!(capture_only);
        syn::custom_keyword!(allowed_in_when);
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Prefix {
        State,
        Event,
        None,
    }
    impl Prefix {
        pub fn new(fn_ident: &Ident) -> Self {
            let ident_str = fn_ident.to_string();

            if ident_str.starts_with("is_") {
                Prefix::State
            } else if ident_str.starts_with("on_") {
                Prefix::Event
            } else {
                Prefix::None
            }
        }

        pub fn is_state(fn_ident: &Ident) -> bool {
            let ident_str = fn_ident.to_string();
            ident_str.starts_with("is_")
        }
    }

    pub struct MacroArgs {
        pub priority: Priority,
        //", allowed_in_when: true"
        pub allowed_in_when: Option<(Token![,], keyword::allowed_in_when, Token![:], LitBool)>,
        // trailing comma
        pub comma_token: Option<Token![,]>,
    }
    impl Parse for MacroArgs {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(MacroArgs {
                priority: input.parse()?,
                allowed_in_when: {
                    if input.peek(Token![,]) {
                        Some((input.parse()?, input.parse()?, input.parse()?, input.parse()?))
                    } else {
                        None
                    }
                },
                comma_token: input.parse()?,
            })
        }
    }

    #[derive(Clone, Copy)]
    pub enum Priority {
        Context(keyword::context),
        Event(keyword::event),
        Outer(keyword::outer),
        Size(keyword::size),
        Inner(keyword::inner),
        CaptureOnly(keyword::capture_only),
    }
    impl Priority {
        pub fn is_event(self) -> bool {
            match self {
                Priority::Event(_) => true,
                _ => false,
            }
        }
        pub fn is_capture_only(self) -> bool {
            match self {
                Priority::CaptureOnly(_) => true,
                _ => false,
            }
        }
    }
    impl Parse for Priority {
        fn parse(input: ParseStream) -> Result<Self> {
            let lookahead = input.lookahead1();

            if lookahead.peek(keyword::context) {
                input.parse().map(Priority::Context)
            } else if lookahead.peek(keyword::event) {
                input.parse().map(Priority::Event)
            } else if lookahead.peek(keyword::outer) {
                input.parse().map(Priority::Outer)
            } else if lookahead.peek(keyword::size) {
                input.parse().map(Priority::Size)
            } else if lookahead.peek(keyword::inner) {
                input.parse().map(Priority::Inner)
            } else if lookahead.peek(keyword::capture_only) {
                input.parse().map(Priority::CaptureOnly)
            } else {
                Err(lookahead.error())
            }
        }
    }

    pub struct PropertyFn {
        pub attrs: Vec<Attribute>,
        pub vis: Visibility,
        pub fn_token: Token![fn],
        pub ident: Ident,
        pub generics: Option<PropertyGenerics>,
        pub paren_token: token::Paren,
        pub args: Punctuated<PropertyArg, Token![,]>,
        pub output: (Token![->], Box<Type>),
        pub where_clause: Option<PropertyWhereClause>,
        pub block: Box<Block>,
    }
    impl Parse for PropertyFn {
        fn parse(input: ParseStream) -> Result<Self> {
            let args_stream;
            Ok(PropertyFn {
                attrs: Attribute::parse_outer(input)?,
                vis: input.parse()?,
                fn_token: input.parse()?,
                ident: input.parse()?,
                generics: {
                    if input.peek(Token![<]) {
                        Some(input.parse()?)
                    } else {
                        None
                    }
                },
                paren_token: parenthesized!(args_stream in input),
                args: Punctuated::parse_terminated(&args_stream)?,
                output: (input.parse()?, input.parse()?),
                where_clause: {
                    if input.peek(Token![where]) {
                        Some(input.parse()?)
                    } else {
                        None
                    }
                },
                block: input.parse()?,
            })
        }
    }

    #[derive(Clone)]
    pub struct PropertyArg {
        pub ident: Ident,
        pub colon_token: Token![:],
        pub ty: Box<Type>,
    }
    impl Parse for PropertyArg {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyArg {
                ident: input.parse()?,
                colon_token: input.parse()?,
                ty: input.parse()?,
            })
        }
    }

    pub struct PropertyGenerics {
        pub lt_token: Token![<],
        pub params: Punctuated<PropertyGenericParam, Token![,]>,
        pub gt_token: Token![>],
    }
    impl Parse for PropertyGenerics {
        fn parse(input: ParseStream) -> Result<Self> {
            let lt_token = input.parse()?;
            let mut depth = 1;
            let mut params_stream = TokenStream::new();
            while !input.is_empty() {
                if input.peek(Token![<]) {
                    depth += 1;
                } else if input.peek(Token![>]) {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }

                params_stream.extend(input.parse::<proc_macro2::TokenTree>());
            }

            Ok(PropertyGenerics {
                lt_token,
                params: parse_terminated2(params_stream)?,
                gt_token: input.parse()?,
            })
        }
    }

    pub struct PropertyGenericParam {
        pub ident: Ident,
        pub bounds: Option<(Token![:], Punctuated<TypeParamBound, Token![+]>)>,
    }
    impl Parse for PropertyGenericParam {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyGenericParam {
                ident: input.parse()?,
                bounds: {
                    if input.peek(Token![:]) {
                        Some((input.parse()?, Punctuated::parse_separated_nonempty(input)?))
                    } else {
                        None
                    }
                },
            })
        }
    }

    pub struct PropertyWhereClause {
        pub where_token: Token![where],
        pub predicates: Punctuated<PropertyWherePredicate, Token![,]>,
    }
    impl Parse for PropertyWhereClause {
        fn parse(input: ParseStream) -> Result<Self> {
            let where_token = input.parse()?;
            let mut predicates_stream = TokenStream::new();
            while !input.peek(token::Brace) {
                predicates_stream.extend(input.parse::<proc_macro2::TokenTree>());
            }
            Ok(PropertyWhereClause {
                where_token,
                predicates: parse_terminated2(predicates_stream)?,
            })
        }
    }

    pub struct PropertyWherePredicate {
        pub ident: Ident,
        pub colon_token: Token![:],
        pub bounds: Punctuated<TypeParamBound, Token![+]>,
    }
    impl Parse for PropertyWherePredicate {
        fn parse(input: ParseStream) -> Result<Self> {
            let ident = input.parse()?;
            let colon_token = input.parse()?;

            let mut bounds_stream = TokenStream::new();
            while !input.peek(token::Brace) && !input.peek(Token![,]) {
                bounds_stream.extend(input.parse::<proc_macro2::TokenTree>());
            }

            Ok(PropertyWherePredicate {
                ident,
                colon_token,
                bounds: parse_terminated2(bounds_stream)?,
            })
        }
    }
}

mod analysis {
    use super::input::{MacroArgs, Prefix, PropertyFn};
    use super::output::*;
    use crate::util::{zero_ui_crate_ident, Attributes, Errors};
    use heck::CamelCase;
    use proc_macro2::Ident;
    use std::{
        collections::{HashMap, HashSet},
        mem,
    };
    use syn::{
        parse_quote,
        punctuated::Punctuated,
        spanned::Spanned,
        visit::{self, Visit},
        visit_mut::{self, VisitMut},
        Token, Type, TypeParamBound,
    };

    pub fn generate(args: MacroArgs, fn_: PropertyFn) -> PropertyMod {
        let mut errors = Errors::default();

        let prefix = Prefix::new(&fn_.ident);

        // validate prefix
        if args.priority.is_capture_only() {
            match prefix {
                Prefix::State => {
                    if fn_.args.len() != 1 {
                        errors.push(
                            "is_* capture_only properties must have 1 parameter, `IsStateVar`",
                            fn_.paren_token.span,
                        );
                    }
                }
                Prefix::Event => {
                    if fn_.args.len() != 1 {
                        errors.push("on_* capture_only properties must have 1 parameter, `FnMut`", fn_.paren_token.span);
                    }
                }
                Prefix::None => {
                    if fn_.args.is_empty() {
                        errors.push("capture_only properties must have at least 1 parameter", fn_.paren_token.span);
                    }
                }
            }
        } else {
            match prefix {
                Prefix::State => {
                    if fn_.args.len() != 2 {
                        errors.push(
                            "is_* properties functions must have 2 parameters, `UiNode` and `IsStateVar`",
                            fn_.paren_token.span,
                        );
                    }
                }
                Prefix::Event => {
                    if fn_.args.len() != 2 {
                        errors.push("on_* properties must have 2 parameters, `UiNode` and `FnMut`", fn_.paren_token.span);
                    }
                    if !args.priority.is_event() {
                        errors.push(
                            "only `event` or `capture_only` priority properties can have the prefix `on_`",
                            fn_.ident.span(),
                        )
                    }
                }
                Prefix::None => {
                    if fn_.args.len() < 2 {
                        errors.push(
                            "properties must have at least 2 parameters, `UiNode` and one or more values",
                            fn_.paren_token.span,
                        );
                    }
                }
            }
        }

        let priority = args.priority;

        // explicit allowed_in_when or default.
        let allowed_in_when = args.allowed_in_when.map(|(_, _, _, b)| b.value).unwrap_or_else(|| match prefix {
            Prefix::State | Prefix::None => true,
            Prefix::Event => false,
        });

        let mut args: Vec<_> = fn_.args.into_iter().collect();
        let crate_ = zero_ui_crate_ident();

        // fix args to continue validation, this errors where already added during prefix validation.
        if args.is_empty() {
            args.push(parse_quote!(_missing_child: impl #crate_::core::UiNode));
        }
        if args.len() == 1 {
            if priority.is_capture_only() {
                args.insert(0, parse_quote!(__: impl #crate_::core::UiNode));
            } else {
                args.push(parse_quote!(_missing_value: ()));
            }
        }

        // convert generics to a single format [(TIdent, [TypeParamBound])]
        // 1 - generics in the function <..> declaration are re-mapped to a tuple,
        //     generics without bounds get an empty bounds collection.
        let mut generics: Vec<_> = fn_
            .generics
            .map(|g| g.params)
            .unwrap_or_default()
            .into_iter()
            .map(|p| (p.ident, p.bounds.map(|(_, b)| b).unwrap_or_default()))
            .collect();
        // generic idents lookup.
        let generic_idents: HashSet<_> = generics.iter().map(|(id, _)| id.clone()).collect();

        // 2 - consume where clause, associating the bounds with their declaration.
        for c in fn_.where_clause.map(|c| c.predicates).unwrap_or_default() {
            let i = generics.iter().position(|(p, _)| p == &c.ident).unwrap();
            generics[i].1.extend(c.bounds);
        }
        // 3 - impl Trait in args are replaced with a new generic type.
        for a in &mut args {
            let mut new_ty = None;
            if let Type::ImplTrait(b) = &mut *a.ty {
                let mut t_ident = ident!("T{}", a.ident.to_string().to_camel_case());
                let mut tn = 2;
                while generic_idents.contains(&t_ident) {
                    t_ident = ident!("T{}{}", a.ident.to_string().to_camel_case(), tn);
                    tn += 1;
                }
                let ty: Type = parse_quote!(#t_ident);
                generics.push((t_ident, mem::take(&mut b.bounds)));
                new_ty = Some(ty);
            }
            if let Some(ty) = new_ty {
                let addr = &mut *a.ty;
                *addr = ty;
            }
        }

        // referencing generic idents lookup.
        let generic_idents: HashSet<_> = generics.iter().map(|(id, _)| id).collect();

        let property_docs = args
            .iter()
            .skip(1)
            .enumerate()
            .map(|(i, a)| PropertyDocArg {
                index: i,
                ident: a.ident.clone(),
                ty: {
                    let mut ty = a.ty.clone();
                    let mut gen_to_impl = GenericsToImpl::new(&generics);
                    gen_to_impl.visit_type_mut(&mut ty);
                    PropertyDocType { ty }
                },
            })
            .collect();

        let fn_generics = generics
            .iter()
            .map(|(id, b)| {
                let mut search = TypeSearch::new(&generic_idents);
                for arg in args.iter().skip(1) {
                    search.visit_type(&arg.ty);
                }
                for (gen_id, bounds) in generics.iter() {
                    if search.found_types.contains(gen_id) {
                        for bound in bounds.iter() {
                            search.visit_type_param_bound(bound);
                        }
                    }
                }

                PropertyGenParam {
                    ident: id.clone(),
                    bounds: b.clone(),
                    used_by_args: search.found_types.contains(id),
                }
            })
            .collect();

        let property_args: Vec<_> = args.iter().skip(1).cloned().collect();

        let property_arg_idents: Vec<_> = property_args.iter().map(|a| a.ident.clone()).collect();

        // property arg generic types are transformed to be used in trait Args associated types.
        let property_arg_tys: Vec<_> = property_args.iter().map(|a| (*a.ty).clone()).collect();
        let args_tys_trait_return = {
            let mut t = property_arg_tys;
            let mut transform = GenericsToTraitStyle::new(&generic_idents);
            t.iter_mut().for_each(|t| transform.visit_type_mut(t));
            t
        };

        // generics used by property argument types.
        let ty_generics: Vec<_> = {
            let mut search = TypeSearch::new(&generic_idents);
            property_args.iter().for_each(|a| search.visit_type(&a.ty));
            for (id, bounds) in &generics {
                if search.found_types.contains(id) {
                    for bound in bounds.iter() {
                        search.visit_type_param_bound(bound);
                    }
                }
            }
            generics.iter().filter(|g| search.found_types.contains(&g.0)).cloned().collect()
        };

        let generic_idents = ty_generics.iter().map(|g| &g.0).collect();
        let args_tys_trait_decl: Vec<_> = {
            let mut transform = GenericsToTraitStyle::new(&generic_idents);
            ty_generics
                .iter()
                .map(|(_, bounds)| {
                    let mut bounds = bounds.clone();
                    bounds.iter_mut().for_each(|b| transform.visit_type_param_bound_mut(b));
                    bounds
                })
                .collect()
        };

        // generics that need to used in a PhantomData field in the args bundle struct.
        let phantom_generics: Vec<_> = {
            let mut search = TypeSearch::new(&generic_idents);
            for arg in &property_args {
                search.visit_type(&arg.ty);
            }
            for bounds in &args_tys_trait_decl {
                for bound in bounds.iter() {
                    search.visit_type_param_bound(bound);
                }
            }
            generic_idents.difference(&search.found_types).map(|&i| i.clone()).collect()
        };

        // separate attributes
        let attrs = Attributes::new(fn_.attrs);
        let mut mod_attrs = attrs.others;
        if let Some(cfg) = attrs.cfg {
            mod_attrs.push(cfg)
        }
        let set_attrs = attrs.inline.into_iter().collect();

        let can_export = match fn_.vis {
            syn::Visibility::Public(_) => true,
            _ => false,
        };

        let fn_block;
        let is_capture_only = priority.is_capture_only();
        if is_capture_only {
            match &*fn_.output.1 {
                Type::Never(_) => {}
                t => {
                    errors.push("capture_only property never returns (-> !)", t.span());
                }
            }
            if !fn_.block.stmts.is_empty() {
                errors.push("capture_only property must have an empty body", fn_.block.span());
            }
            let msg = format!("cannot set capture_only property `{}`", fn_.ident);
            fn_block = parse_quote! {{
                #(let _ = #property_arg_idents;)*
                panic!(#msg);
            }};
        } else {
            fn_block = fn_.block;
        };

        PropertyMod {
            errors,
            docs: PropertyDocs {
                user_docs: attrs.docs,
                priority,
                allowed_in_when,
                args: property_docs,
                generics,
            },
            attrs: mod_attrs,
            vis: fn_.vis,
            can_export,
            ident: fn_.ident.clone(),
            fns: PropertyFns {
                set_attrs,
                generics: fn_generics,
                args,
                output: fn_.output.1,
                block: fn_block,
                is_capture_only,
                priority,
                allowed_in_when,
                property_ident: fn_.ident.clone(),
            },
            tys: PropertyTypes {
                generics: ty_generics,
                phantom_generics,
                args: property_args,
                args_tys_trait_decl,
                args_tys_trait_return,
            },
            macros: PropertyMacros {
                priority,
                can_export,
                allowed_in_when,
                arg_idents: property_arg_idents,
                ident: fn_.ident,
            },
            asserts: PropertyAsserts { prefix },
        }
    }

    struct TypeSearch<'a> {
        types: &'a HashSet<&'a Ident>,
        found_types: HashSet<&'a Ident>,
    }
    impl<'a> TypeSearch<'a> {
        fn new(types: &'a HashSet<&'a Ident>) -> Self {
            TypeSearch {
                types,
                found_types: HashSet::new(),
            }
        }
    }
    impl<'a> Visit<'a> for TypeSearch<'a> {
        fn visit_type(&mut self, i: &'a Type) {
            if let Type::Path(p) = i {
                if let Some(id) = p.path.get_ident() {
                    if let Some(&id) = self.types.get(id) {
                        self.found_types.insert(id);
                    }
                }
            }
            visit::visit_type(self, i);
        }
    }

    struct GenericsToImpl<'a> {
        generics: HashMap<&'a Ident, &'a Punctuated<TypeParamBound, Token![+]>>,
    }
    impl<'a> GenericsToImpl<'a> {
        fn new(generics: &'a [(Ident, Punctuated<TypeParamBound, Token![+]>)]) -> Self {
            GenericsToImpl {
                generics: generics.iter().map(|(i, b)| (i, b)).collect(),
            }
        }
    }
    impl<'a> VisitMut for GenericsToImpl<'a> {
        fn visit_type_mut(&mut self, i: &mut Type) {
            visit_mut::visit_type_mut(self, i);
            if let Type::Path(p) = i {
                if let Some(id) = p.path.get_ident() {
                    if let Some(bounds) = self.generics.get(id) {
                        if !bounds.is_empty() {
                            *i = Type::ImplTrait(parse_quote!(impl #bounds));
                        }
                    }
                }
            }
        }
    }

    struct GenericsToTraitStyle<'a> {
        generics: &'a HashSet<&'a Ident>,
    }
    impl<'a> GenericsToTraitStyle<'a> {
        fn new(generics: &'a HashSet<&'a Ident>) -> Self {
            GenericsToTraitStyle { generics }
        }
    }
    impl<'a> VisitMut for GenericsToTraitStyle<'a> {
        fn visit_type_mut(&mut self, i: &mut Type) {
            visit_mut::visit_type_mut(self, i);

            if let Type::Path(p) = i {
                if let Some(id) = p.path.get_ident() {
                    if self.generics.contains(id) {
                        p.path = parse_quote!(Self::#p);
                    }
                }
            }
        }
    }
}

mod output {
    use super::input::{Prefix, Priority, PropertyArg};
    use crate::util::{uuid, zero_ui_crate_ident, Errors};
    use proc_macro2::{Ident, TokenStream};
    use quote::ToTokens;
    use std::fmt;
    use syn::{punctuated::Punctuated, spanned::Spanned, Attribute, Block, Index, Token, Type, TypeParamBound, Visibility};

    pub struct PropertyMod {
        pub errors: Errors,
        pub docs: PropertyDocs,
        pub attrs: Vec<Attribute>,
        pub vis: Visibility,
        pub can_export: bool,
        pub ident: Ident,
        pub fns: PropertyFns,
        pub tys: PropertyTypes,
        pub macros: PropertyMacros,
        pub asserts: PropertyAsserts,
    }
    impl ToTokens for PropertyMod {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            self.errors.to_tokens(tokens);

            let docs = &self.docs;
            let attrs = &self.attrs;
            let vis = &self.vis;
            let ident = &self.ident;
            let fns = &self.fns;
            let tys = &self.tys;
            let macros = &self.macros;
            let asserts = &self.asserts;

            let docs_inner = docs.inner_tokens();

            let export = if self.can_export {
                Some(quote! {
                    #[doc(hidden)]
                    #vis use super::#ident as export;
                })
            } else {
                None
            };

            tokens.extend(quote! {
                #docs
                #(#attrs)*
                #vis mod #ident {
                    use super::*;

                    #export

                    #fns
                    #tys
                    #macros

                    #docs_inner

                    #asserts
                }
            });
        }
    }

    /// Property mod outer docs. Must also insert `PropertyDocs::inner_tokens` inside the mod.
    pub struct PropertyDocs {
        pub user_docs: Vec<Attribute>,
        pub priority: Priority,
        pub allowed_in_when: bool,
        pub args: Vec<PropertyDocArg>,
        pub generics: Vec<(Ident, Punctuated<TypeParamBound, Token![+]>)>,
    }
    impl PropertyDocs {
        /// Generate dummy function for argument type links.
        fn inner_tokens(&self) -> TokenStream {
            let mut t = TokenStream::new();
            doc_extend!(
                t,
                "<span></span>\n\n<script>{}</script>",
                include_str!("js/property_doc_helper_ext.js")
            );
            let args = &self.args;

            let generics = if self.generics.is_empty() {
                None
            } else {
                let phantom = self.generics.iter().map(|(id, _)| id);
                let phantom_bounds = self.generics.iter().map(|(_, b)| {
                    if b.is_empty() {
                        quote!()
                    } else {
                        quote! {: #b}
                    }
                });
                Some(quote!(<#(#phantom #phantom_bounds),*>))
            };

            t.extend(quote! {
                // this function is hidden using CSS inserted by `PropertyFns`
                #[allow(unused)]
                pub fn doc_helper#generics(#(#args),*) { }
            });
            t
        }
    }
    impl ToTokens for PropertyDocs {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            for attr in &self.user_docs {
                attr.to_tokens(tokens);
            }
            doc_extend!(tokens, "\n# Property\n");
            doc_extend!(
                tokens,
                "This module is a widget [`{0}`](zero_ui::core::property#{0}) property. It {1} be used in widget `when` condition expressions.",
                self.priority,
                if self.allowed_in_when { "can also" } else { "cannot" }
            );

            doc_extend!(tokens, "\n## Arguments\n");
            doc_extend!(tokens, "</div><div id='args_example' class='content'>\n");
            doc_extend!(tokens, "<pre class='rust'>");
            for arg in &self.args {
                doc_extend!(tokens, "{}", arg);
            }
            doc_extend!(tokens, "</pre>\n");
            doc_extend!(tokens, "</div>");
            doc_extend!(tokens, "<script>{}</script>", include_str!("js/property_args_ext.js"));
            doc_extend!(
                tokens,
                "<style>a[href='fn.doc_helper.html']{ display: none; } #args_example { margin: 0 0 0 24px; padding: 0; }</style>"
            );
            doc_extend!(
                tokens,
                "<iframe id='args_example_load' style='display:none;' src='fn.doc_helper.html'></iframe>"
            );
        }
    }
    impl ToTokens for Priority {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                Priority::Context(kw) => kw.to_tokens(tokens),
                Priority::Event(kw) => kw.to_tokens(tokens),
                Priority::Outer(kw) => kw.to_tokens(tokens),
                Priority::Size(kw) => kw.to_tokens(tokens),
                Priority::Inner(kw) => kw.to_tokens(tokens),
                Priority::CaptureOnly(kw) => kw.to_tokens(tokens),
            }
        }
    }
    impl fmt::Display for Priority {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.to_token_stream())
        }
    }

    pub struct PropertyDocArg {
        pub index: usize,
        pub ident: Ident,
        pub ty: PropertyDocType,
    }
    impl ToTokens for PropertyDocArg {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let ident = &self.ident;
            let ty = &self.ty;
            tokens.extend(quote!(#ident: #ty))
        }
    }
    impl fmt::Display for PropertyDocArg {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "<span class='ident'>{}</span>: {}, <span class='comment'>// .{}</span>",
                self.ident, self.ty, self.index
            )
        }
    }

    pub struct PropertyDocType {
        pub ty: Box<Type>,
    }
    impl ToTokens for PropertyDocType {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.ty.to_tokens(tokens)
        }
    }
    impl fmt::Display for PropertyDocType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let ty_str = self.ty.to_token_stream().to_string();

            return write!(f, "{}", cleanup_arg_ty(ty_str));

            fn cleanup_arg_ty(ty: String) -> String {
                let mut r = String::with_capacity(ty.len());
                let mut lifetime = false;
                let mut word = String::with_capacity(3);
                for c in ty.chars() {
                    if word.is_empty() {
                        if c.is_alphabetic() || c == '_' {
                            word.push(c);
                        } else {
                            push_html_scape(&mut r, c);
                            lifetime |= c == '\'';
                        }
                    } else if c.is_alphanumeric() || c == '_' {
                        word.push(c);
                    } else {
                        push_word(&mut r, &word, lifetime);
                        push_html_scape(&mut r, c);
                        word.clear();
                        lifetime = false;
                    }
                }
                if !word.is_empty() {
                    push_word(&mut r, &word, lifetime);
                }
                if r.ends_with(' ') {
                    r.truncate(r.len() - 1);
                }
                r
            }

            fn push_word(r: &mut String, word: &str, lifetime: bool) {
                if lifetime {
                    r.push_str(word);
                    r.push(' ');
                } else {
                    match syn::parse_str::<syn::Ident>(word) {
                        Ok(_) => {
                            r.push_str("<span class='ident'>");
                            r.push_str(word);
                            r.push_str("</span>")
                        }
                        Err(_) => {
                            // Ident parse does not allow keywords.
                            r.push_str("<span class='kw'>");
                            r.push_str(word);
                            r.push_str("</span> ")
                        }
                    }
                }
            }

            fn push_html_scape(r: &mut String, c: char) {
                match c {
                    ' ' => {}
                    '<' => r.push_str("&lt;"),
                    '>' => r.push_str("&gt;"),
                    '"' => r.push_str("&quot;"),
                    '&' => r.push_str("&amp;"),
                    '\'' => r.push_str("&#x27;"),
                    ',' => r.push_str(", "),
                    '+' => r.push_str(" + "),
                    c => r.push(c),
                }
            }
        }
    }

    pub struct PropertyFns {
        pub set_attrs: Vec<Attribute>,
        pub generics: Vec<PropertyGenParam>,
        /// all property params (including the first child:imp UiNode).
        pub args: Vec<PropertyArg>,
        pub output: Box<Type>,
        pub block: Box<Block>,
        pub is_capture_only: bool,
        pub priority: Priority,
        pub allowed_in_when: bool,
        pub property_ident: Ident,
    }
    impl ToTokens for PropertyFns {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            // `set` function.

            let generics = if self.generics.is_empty() {
                None
            } else {
                let idents = self.generics.iter().map(|g| &g.ident);
                let bounds = self.generics.iter().map(|g| &g.bounds);
                Some(quote!(<#(#idents: #bounds),*>))
            };

            let output = &self.output;
            let block = &self.block;

            let args = &self.args;

            for attr in &self.set_attrs {
                attr.to_tokens(tokens);
            }
            if self.is_capture_only {
                tokens.extend(quote! { #[doc(hidden)] })
            } else {
                tokens.extend(quote! {
                    /// Set the property.
                    /// <style>a[href='fn.__.html']{ display: none; }</style>
                });
            }
            tokens.extend(quote! {
                pub fn set #generics (#(#args),*) -> #output #block
            });

            // `args` function.
            let args = self.args.iter().skip(1);
            let arg_idents = args.clone().map(|a| &a.ident);

            let generics = self.generics.iter().filter(|g| g.used_by_args);
            let idents = generics.clone().map(|g| &g.ident);
            let bounds = generics.map(|g| &g.bounds);
            let generics = quote!(<#(#idents: #bounds),*>);

            tokens.extend(quote! {
                /// Initializes a new [`Args`].
                /// <style>a[href='fn.__.html']{ display: none; }</style>
                #[inline]
                pub fn args #generics (#(#args),*) -> impl Args {
                    NamedArgs {
                        _phantom: std::marker::PhantomData,
                        #(#arg_idents,)*
                    }
                }
            });

            let generics = self.generics.iter().filter(|g| !g.used_by_args);
            let idents = generics.clone().map(|g| &g.ident);
            let bounds = generics.map(|g| &g.bounds);
            let generics = quote!(<#(#idents: #bounds),*>);

            let child = &self.args[0];
            let args: Vec<_> = self.args.iter().skip(1).map(|a| &a.ident).collect();
            let child_name = &child.ident;

            if self.is_capture_only {
                tokens.extend(quote! { #[doc(hidden)] })
            } else {
                tokens.extend(quote! {
                    /// Set the property with bundled [`Args`].
                    /// <style>a[href='fn.__.html']{ display: none; }</style>
                });
            }
            tokens.extend(quote! {
                #[inline]
                pub fn set_args #generics (#child, args: impl ArgsUnwrap) -> #output {
                    let (#(#args),*) = args.unwrap();
                    set(#child_name, #(#args),*)
                }
            });
            if !self.is_capture_only {
                let crate_ = zero_ui_crate_ident();
                let input_name = ident_spanned!(child.ty.span()=> "input");
                let message_ty = ident!("{}_must_support_any_UiNode", child_name);

                tokens.extend(quote! {
                    struct AssertInput<A>(A);
                    impl<#message_ty: #crate_::core::UiNode> AssertInput<#message_ty> {
                        fn assert_input(#input_name: #message_ty) {
                            fn t #generics (#child) { let _ = #child_name; }
                            t(#input_name)
                        }
                    }
                });
                let output_name = quote_spanned! {output.span()=> output};
                tokens.extend(quote! {
                    fn assert_output #generics (#output_name: #output) {
                        fn impl_UiNode(_: impl #crate_::core::UiNode) {}
                        impl_UiNode(#output_name)
                    }
                })
            }

            if cfg!(debug_assertions) {
                let crate_ = zero_ui_crate_ident();

                let priority = match self.priority {
                    Priority::Context(_) => quote!(Context),
                    Priority::Event(_) => quote!(Event),
                    Priority::Outer(_) => quote!(Outer),
                    Priority::Size(_) => quote!(Size),
                    Priority::Inner(_) => quote!(Inner),
                    Priority::CaptureOnly(_) => quote!(CaptureOnly),
                };

                let debug_args = if self.allowed_in_when {
                    quote! {
                        use #crate_::core::var::IntoVar;
                        use #crate_::core::debug::debug_var;
                        use std::clone::Clone;
                        Box::new([#(debug_var(IntoVar::into_var(Clone::clone(ArgsNamed::#args(args))))),*])
                    }
                } else {
                    let no_debug_vars = args.iter().map(|_| quote!(no_debug_var()));
                    quote! {
                        use #crate_::core::debug::no_debug_var;
                        let _ = args;
                        Box::new([
                            #(#no_debug_vars),*
                        ])
                    }
                };
                let arg_names = args.iter().map(|a| a.to_string());
                let property_name = self.property_ident.to_string();
                tokens.extend(quote! {
                    #[doc(hidden)]
                    #[cfg(debug_assertions)]
                    pub fn debug_args(args: &impl Args) -> #crate_::core::debug::DebugArgs {
                        #debug_args
                    }

                    #[doc(hidden)]
                    #[cfg(debug_assertions)]
                    pub fn debug_info(
                        node: Box<dyn #crate_::core::UiNode>,
                        debug_args: #crate_::core::debug::DebugArgs,
                        property_name: &'static str,
                        instance_location: #crate_::core::debug::SourceLocation,
                        user_assigned: bool
                    )
                    -> #crate_::core::debug::PropertyInfoNode {
                        #crate_::core::debug::PropertyInfoNode::new_v1(
                            node,
                            #crate_::core::debug::PropertyPriority::#priority,
                            #property_name,
                            #crate_::core::debug::source_location!(),
                            property_name,
                            instance_location,
                            &[#(#arg_names),*],
                            debug_args,
                            user_assigned
                        )
                    }
                });
            }
        }
    }

    pub struct PropertyGenParam {
        pub ident: Ident,
        pub bounds: Punctuated<TypeParamBound, Token![+]>,
        /// If this generic type is used by the property arguments (i.e. excluding the child and return types).
        pub used_by_args: bool,
    }
    impl ToTokens for PropertyArg {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.ident.to_tokens(tokens);
            self.colon_token.to_tokens(tokens);
            self.ty.to_tokens(tokens);
        }
    }

    pub struct PropertyTypes {
        pub generics: Vec<(Ident, Punctuated<TypeParamBound, Token![+]>)>,
        pub phantom_generics: Vec<Ident>,
        pub args: Vec<PropertyArg>,
        /// args.ty but with inter generics referenced updated to Self::T.
        pub args_tys_trait_decl: Vec<Punctuated<TypeParamBound, Token![+]>>,
        /// args.ty but with all generics updated to Self::T.
        pub args_tys_trait_return: Vec<Type>,
    }
    impl ToTokens for PropertyTypes {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let generic_idents: Vec<_> = self.generics.iter().map(|(id, _)| id).collect();
            let generic_bounds: Vec<_> = self.generics.iter().map(|(_, b)| b).collect();
            let phantom_generics = &self.phantom_generics;
            let args = &self.args;

            let named_args_generics = if generic_idents.is_empty() {
                None
            } else {
                Some(quote!(<#(#generic_idents: #generic_bounds),*>))
            };
            let named_args_idents = if generic_idents.is_empty() {
                None
            } else {
                Some(quote!(<#(#generic_idents),*>))
            };

            let args_numbered_idents: Vec<_> = (0..args.len()).map(|i| ident!("arg{}", i)).collect();
            let args_idents: Vec<_> = args.iter().map(|a| &a.ident).collect();
            let args_tys = &self.args_tys_trait_return;
            let generic_bounds_trait_style = &self.args_tys_trait_decl;

            tokens.extend(quote! {
                #[doc(hidden)]
                pub struct NamedArgs#named_args_generics {
                    pub _phantom: std::marker::PhantomData<(#(#phantom_generics),*)>,
                    #(pub #args,)*
                }

                /// Positional view of the property arguments.
                /// <style>a[href='fn.__.html']{ display: none; }</style>
                pub trait ArgsNumbered {
                    #(type #generic_idents: #generic_bounds_trait_style;)*

                    #(fn #args_numbered_idents(&self) -> &#args_tys;)*
                }

                /// Named view of the property arguments.
                /// <style>a[href='fn.__.html']{ display: none; }</style>
                pub trait ArgsNamed {
                    #(type #generic_idents: #generic_bounds_trait_style;)*

                    #(fn #args_idents(&self) -> &#args_tys;)*
                }

                /// Bundled args unwrap.
                /// <style>a[href='fn.__.html']{ display: none; }</style>
                pub trait ArgsUnwrap {
                    #(type #generic_idents: #generic_bounds_trait_style;)*

                    fn unwrap(self) -> (#(#args_tys),*);
                }

                /// Bundled property arguments.
                /// <style>a[href='fn.__.html']{ display: none; }</style>
                pub trait Args: ArgsNamed + ArgsNumbered + ArgsUnwrap { }

                impl#named_args_generics ArgsNumbered for NamedArgs#named_args_idents {
                    #(type #generic_idents = #generic_idents;)*

                    #(
                        fn #args_numbered_idents(&self) -> &#args_tys {
                            &self.#args_idents
                        }
                    )*
                }

                impl#named_args_generics ArgsNamed for NamedArgs#named_args_idents {
                    #(type #generic_idents = #generic_idents;)*

                    #(
                        fn #args_idents(&self) -> &#args_tys {
                            &self.#args_idents
                        }
                    )*
                }

                impl#named_args_generics ArgsUnwrap for NamedArgs#named_args_idents {
                    #(type #generic_idents = #generic_idents;)*

                    fn unwrap(self) -> (#(#args_tys),*) {
                        (#(self.#args_idents),*)
                    }
                }

                impl#named_args_generics Args for NamedArgs#named_args_idents { }
            });
        }
    }

    pub struct PropertyMacros {
        pub priority: Priority,
        pub allowed_in_when: bool,
        pub ident: Ident,
        pub can_export: bool,
        /// idents of property arguments, (not the child:impl UiNode param).
        pub arg_idents: Vec<Ident>,
    }
    impl ToTokens for PropertyMacros {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let pid = uuid(); // unique id of this property.

            // set_args!
            let set_args_ident = ident!("set_args_{}", pid);
            let priority = &self.priority;
            let set_args_rule = if self.priority.is_capture_only() {
                None
            } else if cfg!(debug_assertions) {
                let crate_ = zero_ui_crate_ident();
                Some(quote! {
                    (#priority, $property_path:path, $property_name:path, $node:ident, $args:ident, $user_assigned:tt) => {
                        let $node = {
                            use $property_path::{set_args, debug_args, debug_info};
                            let dbg_args = debug_args(&$args);
                            let $node = set_args($node, $args);
                            debug_info(
                                Box::new($node),
                                dbg_args,
                                stringify!($property_name),
                                #crate_::core::debug::source_location!(),
                                $user_assigned
                            )
                        };
                    };
                })
            } else {
                Some(quote! {
                    (#priority, $property_path:path, $property_name:path, $node:ident, $args:ident) => {
                        let $node = {
                            use $property_path::{set_args};
                            set_args($node, $args)
                        };
                    };
                })
            };
            tokens.extend(quote! {
                #[doc(hidden)]
                #[macro_export]
                macro_rules! #set_args_ident {
                    #set_args_rule
                    ($($ignore:tt)*) => {}
                }

                #[doc(hidden)]
                pub use #set_args_ident as set_args;
            });

            // assert!
            let assert_ident = ident!("assert_{}", pid);
            fn assert_compile_error(case: bool) -> Option<TokenStream> {
                if case {
                    None
                } else {
                    Some(quote! { compile_error!($msg); })
                }
            }
            let allowed_in_when_rule = assert_compile_error(self.allowed_in_when);
            let capture_only_rule = assert_compile_error(!self.priority.is_capture_only());
            tokens.extend(quote! {
                #[doc(hidden)]
                #[macro_export]
                macro_rules! #assert_ident {
                    (allowed_in_when, $msg:tt) => {
                        #allowed_in_when_rule
                    };
                    (!capture_only, $msg:tt) => {
                        #capture_only_rule
                    };
                }

                #[doc(hidden)]
                pub use #assert_ident as assert;
            });

            // if_export!
            let if_export_ident = ident!("if_export_{}", pid);
            let if_export_rule = if self.can_export { quote!( $($tt)* ) } else { quote!() };
            tokens.extend(quote! {
                #[doc(hidden)]
                #[macro_export]
                macro_rules! #if_export_ident {
                    ($($tt:tt)*) => {
                        #if_export_rule
                    };
                }
                #[doc(hidden)]
                pub use #if_export_ident as if_export;
            });

            // switch_args!
            let switch_args_ident = ident!("switch_args_{}", pid);
            let crate_ = zero_ui_crate_ident();
            let arg_idents = &self.arg_idents;
            let arg_n: Vec<_> = if arg_idents.len() == 1 {
                // we don't have a tuple for single arguments.
                vec![TokenStream::new()]
            } else {
                // tuple fields [.0, .1, ..]
                (0..arg_idents.len())
                    .map(|i| {
                        let i = Index::from(i);
                        quote!(.#i)
                    })
                    .collect()
            };
            let last_arg_i = arg_idents.len() - 1;
            let idx_clone = (0..arg_idents.len()).map(|i| if last_arg_i == i { None } else { Some(quote!(.clone())) });
            tokens.extend(quote! {
                #[doc(hidden)]
                #[macro_export]
                macro_rules! #switch_args_ident {
                    ($property_path:path, $idx:ident, $($arg:ident),*) => {{
                        use $property_path::{args, ArgsUnwrap};

                        $(let $arg = ArgsUnwrap::unwrap($arg);)*

                        #(let #arg_idents = #crate_::core::var::switch_var!($idx#idx_clone, $($arg#arg_n),*);)*

                        args(#(#arg_idents),*)
                    }};
                }

                #[doc(hidden)]
                pub use #switch_args_ident as switch_args;
            });

            // named_args!
            let named_args_ident = ident!("named_args_{}", pid);
            tokens.extend(quote! {
                #[doc(hidden)]
                #[macro_export]
                macro_rules! #named_args_ident {
                    ($property_path:path: { $($tt:tt)* }) => {{
                        use $property_path::{NamedArgs};
                        NamedArgs {
                            _phantom: std::marker::PhantomData,
                            $($tt)*
                        }
                    }};
                }

                #[doc(hidden)]
                pub use #named_args_ident as named_args;
            });
        }
    }

    /// Validations that required full compiler type information.
    pub struct PropertyAsserts {
        pub prefix: Prefix,
    }
    impl ToTokens for PropertyAsserts {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self.prefix {
                Prefix::State => {
                    let crate_ = zero_ui_crate_ident();
                    tokens.extend(quote! {
                        #[allow(unused)]
                        fn assert_is_state(args: impl ArgsUnwrap) -> #crate_::core::var::StateVar {
                            args.unwrap()
                        }
                    })
                }
                Prefix::Event => {}
                Prefix::None => {}
            }
        }
    }
}
