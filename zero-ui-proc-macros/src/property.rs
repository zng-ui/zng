use quote::ToTokens;

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = match syn::parse::<input::MacroArgs>(args) {
        Ok(a) => a,
        Err(e) => {
            // in case of incorrect args, like unknown priority, we give the args error
            // but do not remove the function.
            let mut r = proc_macro::TokenStream::from(e.to_compile_error());
            r.extend(input);
            return r;
        }
    };

    let fn_ = match syn::parse::<input::PropertyFn>(input.clone()) {
        Ok(p) => p,
        Err(e) => {
            // in case of major parsing error, like item not being a function.
            let mut r = proc_macro::TokenStream::from(e.to_compile_error());
            r.extend(input);
            return r;
        }
    };

    let output = analysis::generate(args, fn_);

    let tokens = output.to_token_stream();

    tokens.into()
}

pub use analysis::Prefix;
pub use input::keyword;
pub use input::Priority;

mod input {
    use std::fmt;

    use syn::{parse::*, punctuated::Punctuated, spanned::Spanned, *};

    pub mod keyword {
        syn::custom_keyword!(context);
        syn::custom_keyword!(event);
        syn::custom_keyword!(outer);
        syn::custom_keyword!(size);
        syn::custom_keyword!(inner);
        syn::custom_keyword!(capture_only);
        syn::custom_keyword!(allowed_in_when);
    }

    pub struct MacroArgs {
        pub priority: Priority,
        //", allowed_in_when = true"
        pub allowed_in_when: Option<(Token![,], keyword::allowed_in_when, Token![=], LitBool)>,
        pub default_: Option<(Token![,], Token![default], token::Paren, ArgsDefault)>,
        // trailing comma
        pub comma_token: Option<Token![,]>,
    }
    impl Parse for MacroArgs {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(MacroArgs {
                priority: input.parse()?,
                allowed_in_when: {
                    if input.peek(Token![,]) && input.peek2(keyword::allowed_in_when) {
                        let comma = input.parse().unwrap();
                        let allowed_in_when = input.parse::<keyword::allowed_in_when>().unwrap();

                        if input.is_empty() {
                            return Err(syn::Error::new(allowed_in_when.span(), "expected `allowed_in_when = <bool>`"));
                        }
                        let equal = input.parse::<Token![=]>()?;

                        if input.is_empty() {
                            return Err(syn::Error::new(equal.span(), "expected `= <bool>`"));
                        }
                        let bool_ = input.parse()?;

                        Some((comma, allowed_in_when, equal, bool_))
                    } else {
                        None
                    }
                },
                default_: {
                    if input.peek(Token![,]) && input.peek2(Token![default]) {
                        let comma = input.parse().unwrap();
                        let default_ = input.parse::<Token![default]>().unwrap();

                        if input.is_empty() {
                            return Err(syn::Error::new(
                                default_.span(),
                                "expected `default(\"arg1\", ..)` or `default(arg1: \"arg1\", ..)`",
                            ));
                        }

                        let inner;
                        let paren = parenthesized!(inner in input);

                        if inner.is_empty() {
                            return Err(syn::Error::new(
                                paren.span,
                                "expected `default(\"arg1\", ..)` or `default(arg1: \"arg1\", ..)`",
                            ));
                        }

                        Some((comma, default_, paren, inner.parse()?))
                    } else {
                        None
                    }
                },
                comma_token: input.parse()?,
            })
        }
    }

    pub enum ArgsDefault {
        Unamed(Punctuated<Expr, Token![,]>),
        Named(Punctuated<FieldValue, Token![,]>),
    }
    impl Parse for ArgsDefault {
        fn parse(input: ParseStream) -> Result<Self> {
            if input.peek(Ident) && input.peek2(Token![:]) && !input.peek3(Token![:]) {
                Ok(ArgsDefault::Named(Punctuated::parse_terminated(input)?))
            } else {
                Ok(ArgsDefault::Unamed(Punctuated::parse_terminated(input)?))
            }
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
            matches!(self, Priority::Event(_))
        }
        pub fn is_capture_only(self) -> bool {
            matches!(self, Priority::CaptureOnly(_))
        }
        pub fn is_context(self) -> bool {
            matches!(self, Priority::Context(_))
        }
        pub fn all_settable() -> [Self; 5] {
            use crate::property::keyword::*;
            [
                Priority::Inner(inner::default()),
                Priority::Size(size::default()),
                Priority::Outer(outer::default()),
                Priority::Event(event::default()),
                Priority::Context(context::default()),
            ]
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
    impl fmt::Display for Priority {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if f.alternate() {
                match self {
                    Priority::Context(_) => write!(f, "Context"),
                    Priority::Event(_) => write!(f, "Event"),
                    Priority::Outer(_) => write!(f, "Outer"),
                    Priority::Size(_) => write!(f, "Size"),
                    Priority::Inner(_) => write!(f, "Inner"),
                    Priority::CaptureOnly(_) => write!(f, "CaptureOnly"),
                }
            } else {
                match self {
                    Priority::Context(_) => write!(f, "context"),
                    Priority::Event(_) => write!(f, "event"),
                    Priority::Outer(_) => write!(f, "outer"),
                    Priority::Size(_) => write!(f, "size"),
                    Priority::Inner(_) => write!(f, "inner"),
                    Priority::CaptureOnly(_) => write!(f, "capture_only"),
                }
            }
        }
    }

    /// An [`ItemFn`] with outer attributes detached.
    pub struct PropertyFn {
        pub attrs: Vec<Attribute>,
        pub fn_: ItemFn,
    }
    impl Parse for PropertyFn {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(PropertyFn {
                attrs: Attribute::parse_outer(input)?,
                fn_: input.parse()?,
            })
        }
    }
}

mod analysis {
    use std::collections::{HashMap, HashSet};

    use proc_macro2::{Ident, TokenStream};
    use syn::{parse_quote, spanned::Spanned, visit::Visit, visit_mut::VisitMut, TypeParam};

    use crate::util::{self, crate_core, Attributes, Errors};

    use super::{input, output};

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
    }

    pub fn generate(args: input::MacroArgs, fn_: input::PropertyFn) -> output::Output {
        let input::PropertyFn { attrs, mut fn_ } = fn_;

        let mut errors = Errors::default();

        // if Output must only expand to the function and errors.
        let mut fn_and_errors_only = false;

        let prefix = Prefix::new(&fn_.sig.ident);
        let attrs = Attributes::new(attrs);

        // validate prefix
        let args_len = fn_.sig.inputs.len();
        let args_span = fn_.sig.paren_token.span;
        if args.priority.is_capture_only() {
            match prefix {
                Prefix::State => {
                    if args_len != 1 {
                        errors.push("is_* capture_only properties must have 1 parameter, `IsStateVar`", args_span);
                    }
                }
                Prefix::Event => {
                    if args_len != 1 {
                        // TODO: validate that the parameter type actually is FnMut
                        errors.push("on_* capture_only properties must have 1 parameter, `FnMut`", args_span);
                    }
                }
                Prefix::None => {
                    if args_len == 0 {
                        errors.push("capture_only properties must have at least 1 parameter", args_span);
                    }
                }
            }
        } else {
            match prefix {
                Prefix::State => {
                    if args_len != 2 {
                        errors.push(
                            "is_* properties functions must have 2 parameters, `UiNode` and `IsStateVar`",
                            args_span,
                        );
                    }
                }
                Prefix::Event => {
                    if args_len != 2 {
                        errors.push("on_* properties must have 2 parameters, `UiNode` and `FnMut`", args_span);
                    }
                    if !args.priority.is_event() {
                        errors.push(
                            "only `event` or `capture_only` priority properties can have the prefix `on_`",
                            fn_.sig.ident.span(),
                        )
                    }
                }
                Prefix::None => {
                    if args_len < 2 {
                        errors.push(
                            "properties must have at least 2 parameters, `UiNode` and one or more values",
                            args_span,
                        );
                    }
                }
            }
        }
        if args.priority.is_event() && prefix != Prefix::Event {
            errors.push("property marked `event` does not have prefix `on_`", fn_.sig.ident.span());
        }

        // validate return type.
        let output_assert_data = if args.priority.is_capture_only() {
            let mut fix = false;
            match &fn_.sig.output {
                syn::ReturnType::Default => {
                    errors.push(
                        "capture_only properties must have return type `-> !`",
                        // TODO change this to span of the last parenthesis when
                        // [proc_macro_span](https://github.com/rust-lang/rust/issues/54725) is stable.
                        args.priority.span(),
                    );
                    fix = true;
                }
                syn::ReturnType::Type(_, t) => {
                    if !matches!(&**t, syn::Type::Never(_)) {
                        errors.push("capture_only properties must have return type `!`", t.span());
                        fix = true;
                    }
                }
            }
            if fix {
                fn_.sig.output = parse_quote!( -> ! );
            }

            None
        } else {
            // properties not capture_only:
            // rust will validate because we call fn_ in ArgsImpl.set(..) -> impl UiNode.
            // we only need the span so that the error highlights the right code.
            match &fn_.sig.output {
                syn::ReturnType::Default => None,
                syn::ReturnType::Type(_, t) => Some(t.span()),
            }
        };

        // patch signature to continue validation:
        let mut args_are_valid = true;
        if args.priority.is_capture_only() {
            if fn_.sig.inputs.is_empty() {
                if let Prefix::State = prefix {
                    let crate_core = crate_core();
                    fn_.sig.inputs.push(parse_quote!(_missing_param: #crate_core::var::StateVar));
                    args_are_valid = false;
                } else {
                    fn_.sig.inputs.push(parse_quote!(_missing_param: ()));
                    args_are_valid = false;
                }
            }
        } else {
            if fn_.sig.inputs.is_empty() {
                let crate_core = crate_core();
                fn_.sig.inputs.push(parse_quote!( _missing_child: impl #crate_core::UiNode ));
                args_are_valid = false;
            }
            if fn_.sig.inputs.len() == 1 {
                if let Prefix::State = prefix {
                    let crate_core = crate_core();
                    fn_.sig.inputs.push(parse_quote!(_missing_param: #crate_core::var::StateVar));
                    args_are_valid = false;
                } else {
                    fn_.sig.inputs.push(parse_quote!(_missing_param: ()));
                    args_are_valid = false;
                }
            }
        }
        let args_are_valid = args_are_valid;

        // collect normal generics.
        let mut generic_types = vec![]; // Vec<TypeParam>
        for gen in fn_.sig.generics.type_params() {
            generic_types.push(gen.clone());
        }
        // move where clauses to normal generics.
        if let Some(where_) = &fn_.sig.generics.where_clause {
            for pre in where_.predicates.iter() {
                if let syn::WherePredicate::Type(pt) = pre {
                    if let syn::Type::Path(ti) = &pt.bounded_ty {
                        if ti.qself.is_none() {
                            if let Some(t_ident) = ti.path.get_ident() {
                                // T : bounds
                                if let Some(gen) = generic_types.iter_mut().find(|t| &t.ident == t_ident) {
                                    // found T
                                    gen.bounds.extend(pt.bounds.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut child_assert_data = None;

        if !args.priority.is_capture_only() {
            // validate the child arg.
            let child = &fn_.sig.inputs[0];
            match child {
                syn::FnArg::Typed(t) => {
                    if let syn::Pat::Ident(id) = &*t.pat {
                        // `self: T`
                        if id.ident == "self" {
                            errors.push("methods cannot be property functions", id.ident.span());
                            fn_and_errors_only = true; // we can't expand struct and trait declarations inside an impl.
                        }
                    }

                    let mut visitor = CollectUsedGenerics::new(&generic_types);
                    visitor.visit_fn_arg(child);
                    let child_generics: Vec<_> = generic_types.iter().filter(|t| visitor.used.contains(&t.ident)).cloned().collect();
                    child_assert_data = Some((t.ty.clone(), child_generics));
                }
                syn::FnArg::Receiver(invalid) => {
                    // `self`
                    errors.push("methods cannot be property functions", invalid.span());
                    fn_and_errors_only = true; // we can't expand struct and trait declarations inside an impl.
                }
            }

            // collect only generics used in property inputs (not in first child arg).
            let used = {
                let mut visitor = CollectUsedGenerics::new(&generic_types);
                for input in fn_.sig.inputs.iter().skip(1) {
                    visitor.visit_fn_arg(input);
                }

                visitor.used
            };
            // removes generics used in only the first child arg.
            generic_types.retain(|t| used.contains(&t.ident));
        }

        // validate input patterns and collect arg_idents and arg_types and impl_types (for generic_types):
        let mut arg_idents = vec![]; // Vec<Ident>
        let mut arg_types = vec![]; // Vec<Type>

        let mut embedded_impl_types = PatchEmbededImplTrait::default();
        let mut impl_types = vec![]; // Vec<TypeParam>

        let inputs = if args.priority.is_capture_only() {
            fn_.sig.inputs.iter().skip(0)
        } else {
            fn_.sig.inputs.iter().skip(1)
        };

        let mut invalid_n = 0;
        let mut invalid_idents = move || {
            let next = ident!("_invalid{}", invalid_n);
            invalid_n += 1;
            next
        };
        let mut unique_names = HashMap::new();
        for input in inputs {
            match input {
                syn::FnArg::Typed(t) => {
                    // any pat : ty
                    arg_types.push((&*t.ty).clone());
                    match &*t.pat {
                        syn::Pat::Ident(ident_pat) => {
                            if ident_pat.ident == "self" {
                                // self : type
                                errors.push("methods cannot be property functions", ident_pat.ident.span());
                                arg_idents.push(invalid_idents());
                            } else {
                                // VALID
                                // ident: type
                                arg_idents.push(ident_pat.ident.clone());
                            }
                        }
                        invalid => {
                            // any_pat no type ascription
                            errors.push("only `field: T` pattern can be property arguments", invalid.span());
                            arg_idents.push(invalid_idents());
                        }
                    }

                    let last_i = arg_types.len() - 1;
                    // resolve name conflicts so that only a rust error shows up when
                    // the user declares two or more inputs with the same name
                    let count: &mut u32 = unique_names.entry(arg_idents[last_i].clone()).or_default();
                    if *count > 0 {
                        arg_idents[last_i] = ident_spanned! {arg_idents[last_i].span()=> "__{}{}", arg_idents[last_i], count};
                    }
                    *count += 1;

                    // convert `impl Trait` to normal generics:
                    if let syn::Type::ImplTrait(impl_) = &arg_types[last_i] {
                        // impl at the *top* level gets a readable name

                        let t_ident = ident_spanned!(impl_.span()=> "T_{}", arg_idents[last_i]);

                        // the bounds can have nested impl Traits.
                        let mut bounds = impl_.bounds.clone();
                        for bound in bounds.iter_mut() {
                            embedded_impl_types.visit_type_param_bound_mut(bound);
                        }

                        impl_types.push(parse_quote! {
                            #t_ident : #bounds
                        });
                        arg_types[last_i] = parse_quote!( #t_ident );
                    } else {
                        embedded_impl_types.visit_type_mut(&mut arg_types[last_i]);
                    }
                }

                syn::FnArg::Receiver(invalid) => {
                    // `self`
                    errors.push("methods cannot be property functions", invalid.span());
                    fn_and_errors_only = true; // we can't expand struct and trait declarations inside an impl.
                }
            }
        }
        drop(unique_names);
        generic_types.extend(embedded_impl_types.types);
        generic_types.extend(impl_types);

        // convert `T:? bounds?` to `type T:? Self::?bounds?;`
        let mut to_assoc = GenericToAssocTypes {
            t_idents: generic_types.iter().map(|t| t.ident.clone()).collect(),
        };
        let mut assoc_types = vec![];
        for gen in &generic_types {
            let ident = &gen.ident;
            let mut bounds = gen.bounds.clone();
            if bounds.is_empty() {
                assoc_types.push(parse_quote! { type #ident; });
            } else {
                for bound in bounds.iter_mut() {
                    to_assoc.visit_type_param_bound_mut(bound);
                }
                assoc_types.push(parse_quote! {
                    type #ident : #bounds;
                });
            }
        }

        // convert arg types to be a return type in the Args trait methods.
        let mut arg_return_types = arg_types.clone();
        for ty in &mut arg_return_types {
            if let syn::Type::Path(tp) = ty {
                if let Some(ident) = tp.path.get_ident() {
                    if to_assoc.t_idents.contains(ident) {
                        // is one of the generic types, change to Self::T
                        *ty = parse_quote!( Self::#ident );
                        continue;
                    }
                }
            }
            to_assoc.visit_type_mut(ty);
        }

        // collect phantom type idents.
        let mut phantom_idents = to_assoc.t_idents;
        for arg_ty in &arg_types {
            if let syn::Type::Path(tp) = arg_ty {
                if let Some(ident) = tp.path.get_ident() {
                    if let Some(i) = phantom_idents.iter().position(|id| id == ident) {
                        phantom_idents.swap_remove(i);
                    }
                }
            }
        }

        // more signature validation.
        if let Some(async_) = &fn_.sig.asyncness {
            errors.push("property functions cannot be `async`", async_.span());
            fn_and_errors_only = true; // we don't call .await in set and we don't implement UiNode for Future
        }
        if let Some(unsafe_) = &fn_.sig.unsafety {
            errors.push("property functions cannot be `unsafe`", unsafe_.span());
            fn_and_errors_only = true; // we don't want to support unsafe set
        }
        if let Some(abi) = &fn_.sig.abi {
            errors.push("property functions cannot be `extern`", abi.span());
            fn_and_errors_only = true; // we don't want to support unsafe set
        }
        if let Some(lifetime) = fn_.sig.generics.lifetimes().next() {
            errors.push("property functions cannot declare lifetimes", lifetime.span());
            fn_and_errors_only = true; // we don't support lifetimes in the Args trait and ArgsImpl struct
        }
        if let Some(const_) = fn_.sig.generics.const_params().next() {
            errors.push("property functions do not support `const` generics", const_.span());
            fn_and_errors_only = true; // we don't support const generics yet.
        }

        if args.priority.is_capture_only() {
            let msg = if fn_and_errors_only {
                // we are in a context where we only want to expand to the code
                // the user wrote + errors, but the code for a capture_only function
                // is not valid, so we patch it into a valid function to minimize
                // misleading errors.
                "invalid property declaration".to_owned()
            } else {
                format!("property `{}` cannot be set because it is capture-only", fn_.sig.ident)
            };

            // set capture_only standard error.
            fn_.block = parse_quote! {
                { panic!(#msg) }
            };
            // allow unused property fields.
            fn_.attrs.push(parse_quote! { #[allow(unused_variables)] });
        }

        let allowed_in_when = match args.allowed_in_when {
            Some(b) => b.3.value,
            None => match prefix {
                Prefix::State | Prefix::None => true,
                Prefix::Event => false,
            },
        };

        let default_value = if let Some((_, _, paren, default_)) = args.default_ {
            let mut property_name = fn_.sig.ident.clone();
            property_name.set_span(paren.span);
            match default_ {
                input::ArgsDefault::Unamed(args) => {
                    quote_spanned! {paren.span=>
                        #property_name::ArgsImpl::new(#args)
                    }
                }
                input::ArgsDefault::Named(fields) => {
                    quote_spanned! {paren.span=>
                        #property_name::code_gen! { named_new #property_name, __ArgsImpl { #fields } }
                    }
                }
            }
        } else if matches!(prefix, Prefix::State) {
            let property_name = &fn_.sig.ident;
            if arg_idents.len() == 1 {
                let crate_core = util::crate_core();
                quote! {
                    #property_name::ArgsImpl::new(
                        #crate_core::var::state_var()
                    )
                }
            } else {
                // A compile error was generated for this case already.
                TokenStream::default()
            }
        } else {
            TokenStream::default()
        };

        let has_default_value = !default_value.is_empty();

        let macro_ident = ident!("{}_{}", fn_.sig.ident, util::uuid());

        let export = !matches!(&fn_.vis, syn::Visibility::Inherited);

        let is_capture_only = args.priority.is_capture_only();
        let fn_attrs = output::OutputAttributes {
            docs: attrs.docs,
            inline: attrs.inline,
            cfg: attrs.cfg.clone(),
            attrs: attrs.others.into_iter().chain(attrs.lints).collect(),
            is_capture_only,
            is_wgt_capture_only: is_capture_only && fn_.sig.ident.to_string().starts_with("__p_"),
        };

        // create a *real-alias* with docs that are inlined in widgets
        // we need to do this because of https://github.com/rust-lang/rust/issues/83976
        let mut alias_tokens = TokenStream::new();
        if export && !fn_and_errors_only {
            let mut docs_copy = TokenStream::new();
            fn_attrs.to_tokens(&mut docs_copy, true);

            let vis = &fn_.vis;
            let mut alias_fn = fn_.clone();
            let id = alias_fn.sig.ident;
            let alias_ident = ident_spanned!(id.span()=> "__wgt_{}", id);

            alias_fn.sig.ident = ident!("wgt_docs_export");
            alias_fn.block = parse_quote!({});
            alias_fn.sig.output = syn::ReturnType::Default;

            let cfg = &attrs.cfg;

            alias_tokens.extend(quote! {
                #cfg
                #[doc(hidden)]
                #[allow(unused)]
                #vis mod #alias_ident {
                    use super::*;

                    #docs_copy
                    #alias_fn
                }
            });
        }

        output::Output {
            errors,
            fn_and_errors_only,
            fn_attrs,
            types: output::OutputTypes {
                cfg: attrs.cfg.clone(),
                ident: fn_.sig.ident.clone(),
                generics: generic_types,
                allowed_in_when,
                args_are_valid,
                phantom_idents,
                arg_idents: arg_idents.clone(),
                priority: args.priority,
                arg_types,
                assoc_types,
                arg_return_types,
                default_value,
                child_assert: child_assert_data,
                output_assert: output_assert_data,
            },
            mod_: output::OutputMod {
                cfg: attrs.cfg.clone(),
                vis: fn_.vis.clone(),
                ident: fn_.sig.ident.clone(),
                is_capture_only: args.priority.is_capture_only(),
                macro_ident: macro_ident.clone(),
                args_ident: ident!("{}_Args", fn_.sig.ident),
                args_impl_ident: ident!("{}_ArgsImpl", fn_.sig.ident),
                has_default_value,
                alias_fn: alias_tokens,
            },
            macro_: output::OutputMacro {
                cfg: attrs.cfg,
                macro_ident,
                export,
                priority: args.priority,
                allowed_in_when,
                arg_idents,
                has_default_value,
            },
            fn_,
        }
    }

    #[derive(Default)]
    struct PatchEmbededImplTrait {
        types: Vec<TypeParam>,
    }
    impl VisitMut for PatchEmbededImplTrait {
        fn visit_type_mut(&mut self, i: &mut syn::Type) {
            syn::visit_mut::visit_type_mut(self, i);

            if let syn::Type::ImplTrait(impl_trait) = i {
                let t_ident = ident!("T_impl_{}", self.types.len());
                let bounds = &impl_trait.bounds;
                self.types.push(parse_quote! {
                    #t_ident : #bounds
                });
                *i = parse_quote!(#t_ident);
            }
        }
    }

    struct GenericToAssocTypes {
        t_idents: Vec<Ident>,
    }
    impl VisitMut for GenericToAssocTypes {
        fn visit_type_mut(&mut self, i: &mut syn::Type) {
            if let syn::Type::Path(tp) = i {
                if let Some(ident) = tp.path.get_ident() {
                    if self.t_idents.contains(ident) {
                        // if
                        *i = parse_quote!( Self::#ident );
                        return;
                    }
                }
            }

            // else
            syn::visit_mut::visit_type_mut(self, i);
        }
    }

    struct CollectUsedGenerics<'g> {
        generics: &'g [TypeParam],
        used: HashSet<Ident>,
    }
    impl<'g> CollectUsedGenerics<'g> {
        fn new(generics: &'g [TypeParam]) -> Self {
            CollectUsedGenerics {
                used: HashSet::new(),
                generics,
            }
        }
    }
    impl<'g, 'v> Visit<'v> for CollectUsedGenerics<'g> {
        fn visit_type(&mut self, i: &'v syn::Type) {
            if let syn::Type::Path(tp) = i {
                if let Some(ident) = tp.path.get_ident() {
                    if let Some(gen) = self.generics.iter().find(|g| &g.ident == ident) {
                        // uses generic.
                        if self.used.insert(ident.clone()) {
                            // because of this it uses the generic bounds too.
                            for bound in gen.bounds.iter() {
                                self.visit_type_param_bound(bound);
                            }
                        }
                    }
                }
            }
            syn::visit::visit_type(self, i);
        }
    }
}

mod output {
    use proc_macro2::{Ident, TokenStream};
    use quote::ToTokens;
    use syn::{spanned::Spanned, Attribute, ItemFn, TraitItemType, Type, TypeParam, Visibility};

    use crate::util::{crate_core, docs_with_first_line_js, Errors};

    use super::input::Priority;

    pub struct Output {
        pub errors: Errors,
        pub fn_attrs: OutputAttributes,
        pub fn_: ItemFn,
        pub fn_and_errors_only: bool,
        pub types: OutputTypes,
        pub macro_: OutputMacro,
        pub mod_: OutputMod,
    }
    impl ToTokens for Output {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.errors.to_tokens(tokens);
            if self.fn_and_errors_only {
                self.fn_attrs.to_tokens_fn_only(tokens);
                self.fn_.to_tokens(tokens);
            } else {
                self.fn_attrs.to_tokens(tokens, false);
                self.fn_.to_tokens(tokens);
                self.types.to_tokens(tokens);
                self.macro_.to_tokens(tokens);
                self.mod_.to_tokens(tokens);
            }
        }
    }

    pub struct OutputAttributes {
        pub docs: Vec<Attribute>,
        pub inline: Option<Attribute>,
        pub cfg: Option<Attribute>,
        pub attrs: Vec<Attribute>,
        pub is_capture_only: bool,
        pub is_wgt_capture_only: bool,
    }

    impl OutputAttributes {
        fn to_tokens_fn_only(&self, tokens: &mut TokenStream) {
            self.inline.to_tokens(tokens);
            self.cfg.to_tokens(tokens);
            for attr in self.attrs.iter().chain(&self.docs) {
                attr.to_tokens(tokens);
            }
        }

        pub fn to_tokens(&self, tokens: &mut TokenStream, wgt: bool) {
            if wgt {
                for attr in &self.docs {
                    attr.to_tokens(tokens);
                }
            } else if self.is_wgt_capture_only {
                tokens.extend(quote! { #[doc(hidden)] });
            } else {
                docs_with_first_line_js(tokens, &self.docs, js!("property_header.js"));
            }
            if self.is_capture_only {
                tokens.extend(quote! {
                    ///
                    /// This property is `capture_only`, it can only be used in widget declarations
                    /// to define a property that is captured by the widget.
                });
            } else if !wgt {
                tokens.extend(quote! {
                    /// </div>
                    /// <h2 id='function' class='small-section-header'>Function<a href='#function' class='anchor'></a></h2>
                    /// <pre id='ffn' class='rust fn'></pre>
                    /// <div class='docblock'>
                    ///
                    /// Properties are functions that can be called directly.
                    ///
                    /// The property is ***set*** around the first input [`UiNode`],
                    /// the other inputs are the property arguments. The function output is a new [`UiNode`] that
                    /// includes the property behavior.
                    ///
                    /// [`UiNode`]: zero_ui::core::UiNode
                });
            }

            doc_extend!(
                tokens,
                "<script>{}property({})</script>",
                if wgt {
                    js!("property_wgt_export.js")
                } else {
                    js!("property_full.js")
                },
                self.is_capture_only
            );

            self.cfg.to_tokens(tokens);

            self.inline.to_tokens(tokens);
            for attr in &self.attrs {
                attr.to_tokens(tokens);
            }
        }
    }

    pub struct OutputTypes {
        pub cfg: Option<Attribute>,

        pub ident: Ident,

        pub priority: Priority,
        pub allowed_in_when: bool,

        pub generics: Vec<TypeParam>,
        pub phantom_idents: Vec<Ident>,

        pub arg_idents: Vec<Ident>,
        pub arg_types: Vec<Type>,
        pub args_are_valid: bool,

        pub assoc_types: Vec<TraitItemType>,
        pub arg_return_types: Vec<Type>,

        pub default_value: TokenStream,

        pub child_assert: Option<(Box<syn::Type>, Vec<syn::TypeParam>)>,
        pub output_assert: Option<proc_macro2::Span>,
    }
    impl ToTokens for OutputTypes {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let OutputTypes {
                cfg,
                ident,
                generics,
                assoc_types,
                phantom_idents: phantom,
                arg_idents,
                arg_types,
                arg_return_types,
                default_value,
                ..
            } = self;
            let args_impl_ident = ident!("{}_ArgsImpl", ident);
            let args_ident = ident!("{}_Args", ident);
            let arg_locals: Vec<_> = arg_idents.iter().enumerate().map(|(i, id)| ident!("__{}_{}", i, id)).collect();
            let crate_core = crate_core();

            let (phantom_decl, phantom_init) = if phantom.is_empty() {
                (TokenStream::new(), TokenStream::new())
            } else {
                (
                    quote! {
                        pub _phantom: std::marker::PhantomData<( #(#phantom),* )>,
                    },
                    quote! {
                        _phantom: std::marker::PhantomData,
                    },
                )
            };

            let (generic_decl, generic_use) = if generics.is_empty() {
                (TokenStream::new(), TokenStream::new())
            } else {
                let generic_idents = generics.iter().map(|t| &t.ident);
                (quote! { < #(#generics),* > }, quote! { < #(#generic_idents),* > })
            };

            let assoc_connect = if generics.is_empty() {
                TokenStream::new()
            } else {
                let mut co = TokenStream::new();
                for gen in assoc_types {
                    let ident = &gen.ident;
                    co.extend(quote! { type #ident = #ident; });
                }
                co
            };
            #[cfg(debug_assertions)]
            let arg_debug_vars = {
                let args_len = arg_locals.len();
                // generate debug_var calls with the arg type span in case the type does
                // not implement Clone and (IntoVar or Var or Debug) which generates a compile error.
                let debug_var_calls = arg_locals.iter().zip(arg_types).map(|(lid, ty)| {
                    let mut lid = lid.clone();
                    lid.set_span(ty.span());
                    quote_spanned! {ty.span()=>
                        #[allow(clippy::needless_borrow)] {
                            use #crate_core::debug::debug_var_util::*;
                            (&&&&Wrap(#lid)).debug_var()
                        },
                    }
                });
                quote! {
                    let arg_debug_vars = {
                        let ( #(#arg_locals),* ) = self_.unwrap_ref();
                        let __r: [_; #args_len] = [
                            #(#debug_var_calls)*
                        ];
                        Box::new(__r)
                    };
                }
            };

            let mut child_ty_span = proc_macro2::Span::call_site();
            let child_assert = if let Some((child_ty, ty_params)) = &self.child_assert {
                child_ty_span = child_ty.span();
                let assert_ident = ident!("__{}_arg0_assert", ident);
                quote_spanned! {child_ty.span()=>
                    fn #assert_ident<#(#ty_params),*>(child: #child_ty) -> impl #crate_core::UiNode {
                        child
                    }
                }
            } else {
                TokenStream::default()
            };
            let set_child_span = child_ty_span;

            let set = if self.priority.is_capture_only() {
                TokenStream::new()
            } else {
                // set span for error when child type does not take impl UiNode.
                let child_arg = quote_spanned! {set_child_span=>
                    child: impl #crate_core::UiNode
                };
                let child_arg_use = quote_spanned! {set_child_span=>
                    child
                };
                let output_span = self.output_assert.unwrap_or_else(proc_macro2::Span::call_site);
                let output_ty = quote_spanned! {output_span=>
                    impl #crate_core::UiNode
                };

                let set_ident = ident!("__{}_set", ident);
                #[cfg(debug_assertions)]
                {
                    let set_debug_ident = ident!("__{}_set_debug", ident);
                    let ident_str = ident.to_string();
                    let arg_idents_str = arg_idents.iter().map(|i| i.to_string());
                    let priority = match self.priority {
                        Priority::Context(_) => quote!(Context),
                        Priority::Event(_) => quote!(Event),
                        Priority::Outer(_) => quote!(Outer),
                        Priority::Size(_) => quote!(Size),
                        Priority::Inner(_) => quote!(Inner),
                        Priority::CaptureOnly(_) => quote!(CaptureOnly),
                    };
                    quote! {
                        #[doc(hidden)]
                        #[inline]
                        pub fn #set_ident(self_: impl #args_ident, #child_arg) -> #output_ty {
                            let ( #(#arg_locals),* ) = self_.unwrap();
                            #ident(#child_arg_use, #( #arg_locals ),*)
                        }

                        #[doc(hidden)]
                        #[inline]
                        pub fn #set_debug_ident(
                            self_: impl #args_ident,
                            #child_arg,
                            property_name: &'static str,
                            instance_location: #crate_core::debug::SourceLocation,
                            child_priority: bool,
                            user_assigned: bool,
                        ) -> #crate_core::debug::PropertyInfoNode {
                            #arg_debug_vars

                            fn box_fix(node: impl #crate_core::UiNode) -> #crate_core::BoxedUiNode {
                                #crate_core::UiNode::boxed(node)
                            }
                            let node = box_fix(#set_ident(self_, #child_arg_use));

                            #crate_core::debug::PropertyInfoNode::new_v1(
                                node,
                                #crate_core::debug::PropertyPriority::#priority,
                                child_priority,
                                #ident_str,
                                #crate_core::debug::source_location!(),
                                property_name,
                                instance_location,
                                &[#( #arg_idents_str ),*],
                                arg_debug_vars,
                                user_assigned
                            )
                        }
                    }
                }

                #[cfg(not(debug_assertions))]
                quote! {
                    #[doc(hidden)]
                    #[inline]
                    pub fn #set_ident(self_: impl #args_ident, #child_arg) -> #output_ty {
                        let ( #(#arg_locals),* ) = self_.unwrap();
                        #ident(#child_arg_use, #( #arg_locals ),*)
                    }
                }
            };

            let cap_debug = {
                #[cfg(debug_assertions)]
                {
                    let cap_ident = ident!("__{}_captured_debug", ident);
                    let arg_idents_str = arg_idents.iter().map(|i| i.to_string());

                    quote! {
                        #[doc(hidden)]
                        #[inline]
                        pub fn #cap_ident(
                            self_: &impl #args_ident,
                            property_name: &'static str,
                            instance_location: #crate_core::debug::SourceLocation,
                            user_assigned: bool
                        ) -> #crate_core::debug::CapturedPropertyV1 {
                            #arg_debug_vars
                            #crate_core::debug::CapturedPropertyV1 {
                                property_name,
                                instance_location,
                                arg_names: &[#( #arg_idents_str ),*],
                                arg_debug_vars,
                                user_assigned,
                            }
                        }
                    }
                }
                #[cfg(not(debug_assertions))]
                TokenStream::new()
            };

            let (unwrap_ty, unwrap_expr) = if arg_return_types.len() == 1 {
                let ty = arg_return_types[0].to_token_stream();
                let single_arg = &arg_idents[0];
                let expr = quote! { self.#single_arg };
                (ty, expr)
            } else {
                (
                    quote! {
                        ( #( #arg_return_types ),* )
                    },
                    quote! {
                        ( #( self.#arg_idents ),* )
                    },
                )
            };
            let (unwrap_ty_ref, unwrap_expr_ref) = if arg_return_types.len() == 1 {
                let single_ty = &arg_return_types[0];
                let ty = quote! { &#single_ty };
                let single_arg = &arg_idents[0];
                let expr = quote! { &self.#single_arg };
                (ty, expr)
            } else {
                (quote! { ( #( &#arg_return_types ),* ) }, quote! { ( #( &self.#arg_idents ),* ) })
            };

            let named_arg_mtds: Vec<_> = arg_idents.iter().map(|a| ident!("__{}", a)).collect();
            let numbered_arg_mtds: Vec<_> = (0..arg_idents.len()).map(|a| ident!("__{}", a)).collect();

            let default_fn = if default_value.is_empty() {
                TokenStream::default()
            } else {
                let default_fn_ident = ident!("__{}_default_args", ident);
                quote! {
                    #[inline]
                    #[doc(hidden)]
                    #[allow(non_snake_case)]
                    pub fn #default_fn_ident() -> impl #args_ident {
                        #default_value
                    }
                }
            };

            let allowed_in_when_assert = if self.allowed_in_when && self.args_are_valid {
                let assert_ident = ident!("__{}_assert_allowed_in_when", ident);
                let var_idents: Vec<_> = arg_idents
                    .iter()
                    .zip(arg_types.iter())
                    .map(|(a, t)| {
                        let mut a = a.clone();
                        a.set_span(t.span());
                        a
                    })
                    .collect();
                let declarations = var_idents
                    .iter()
                    .zip(named_arg_mtds.iter())
                    .zip(arg_types.iter())
                    .map(|((a, m), t)| {
                        let span = t.span();
                        let mut r = quote_spanned! {span=>
                            let #a = #crate_core::var::IntoVar::allowed_in_when_property_requires_IntoVar_members(
                                #args_ident::#m(&__args)
                            );
                        };
                        crate::util::set_span(&mut r, span);
                        r
                    });

                quote! {
                    fn #assert_ident(__args: impl #args_ident) {
                        #(#declarations)*
                        let _ = #args_impl_ident::new(#(#var_idents),*);
                    }
                }
            } else {
                TokenStream::default()
            };

            tokens.extend(quote! {
                #cfg
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                pub struct #args_impl_ident #generic_decl {
                    #phantom_decl
                    #(pub #arg_idents: #arg_types,)*
                }

                #cfg
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                pub trait #args_ident {
                    #(#assoc_types)*

                    #(
                        fn #named_arg_mtds(&self) -> &#arg_return_types;
                        fn #numbered_arg_mtds(&self) -> &#arg_return_types;
                    )*

                    fn unwrap(self) -> #unwrap_ty;
                    fn unwrap_ref(&self) -> #unwrap_ty_ref;
                }

                #cfg
                #[allow(missing_docs)]
                #[allow(non_camel_case_types)]
                impl #generic_decl #args_impl_ident #generic_use {
                    #[inline]
                    pub fn new(#( #arg_idents: #arg_types ),*) -> impl #args_ident {
                        Self {
                            #phantom_init
                            #(#arg_idents,)*
                        }
                    }

                    #[inline]
                    pub fn args(self) -> impl #args_ident {
                        self
                    }
                }

                #default_fn

                #cfg
                #[allow(missing_docs)]
                #[allow(non_camel_case_types)]
                impl #generic_decl #args_ident for #args_impl_ident #generic_use {
                    #assoc_connect

                    #(
                        #[inline]
                        fn #named_arg_mtds(&self) -> &#arg_return_types {
                            &self.#arg_idents
                        }

                        #[inline]
                        fn #numbered_arg_mtds(&self) -> &#arg_return_types {
                            &self.#arg_idents
                        }
                    )*

                    #[inline]
                    fn unwrap(self) -> #unwrap_ty {
                        #unwrap_expr
                    }

                    #[inline]
                    fn unwrap_ref(&self) -> #unwrap_ty_ref {
                        #unwrap_expr_ref
                    }
                }

                #child_assert
                #allowed_in_when_assert
                #set
                #cap_debug
            })
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

    pub struct OutputMacro {
        pub cfg: Option<Attribute>,

        pub macro_ident: Ident,
        pub export: bool,

        pub priority: Priority,

        pub allowed_in_when: bool,

        pub arg_idents: Vec<Ident>,

        pub has_default_value: bool,
    }
    impl ToTokens for OutputMacro {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let OutputMacro {
                cfg,
                macro_ident,
                priority,
                arg_idents,
                ..
            } = self;

            let set = if priority.is_capture_only() {
                quote! {
                    (set $($tt:tt)*) => {};
                }
            } else {
                // we need $__set as input because some errors of type mismatch
                // on assign also highlight the __set function, so we take it as
                // input and in widget_new! we set the ident span to be value span.
                #[cfg(debug_assertions)]
                quote! {
                    (set #priority, $node:ident, $property_path: path, $args:ident,
                        $property_name:expr, $source_location:expr, $child_priority:tt, $user_assigned:tt, $__set:ident) => {
                            let $node = {
                                use $property_path::{set_debug as $__set};
                                $__set($args, $node, $property_name, $source_location, $child_priority, $user_assigned)
                            };
                    };
                    (set #priority, $node:ident, $property_path: path, $args:ident, $__set:ident) => {
                        let $node = {
                            use $property_path::{set as $__set};
                            $__set($args, $node)
                        };
                    };
                    (set $other:ident, $($ignore:tt)+) => { };
                }
                #[cfg(not(debug_assertions))]
                quote! {
                    (set #priority, $node:ident, $property_path: path, $args:ident, $__set:ident) => {
                        let $node = {
                            use $property_path::{set as __set};
                            $__set($args, $node)
                        };
                    };
                    (set $other:ident, $($ignore:tt)+) => { };
                }
            };

            let allowed_in_when = if self.allowed_in_when {
                quote! {
                    (if allowed_in_when=> $($tt:tt)*) => {
                        $($tt)*
                    };
                    (if !allowed_in_when=> $($tt:tt)*) => { };
                }
            } else {
                quote! {
                    (if allowed_in_when=> $($tt:tt)*) => { };
                    (if !allowed_in_when=> $($tt:tt)*) => {
                        $($tt)*
                    };
                }
            };

            let capture_only = if !priority.is_capture_only() {
                quote! {
                    (if capture_only=> $($tt:tt)*) => { };
                }
            } else {
                quote! {
                    (if capture_only=> $($tt:tt)*) => {
                        $($tt)*
                    };
                }
            };

            let if_pub = if self.export {
                quote! {
                    (if export=> $($tt:tt)*) => {
                        $($tt)*
                    };
                }
            } else {
                quote! {
                    (if export=> $($tt:tt)*) => { };
                }
            };

            let if_default = if self.has_default_value {
                quote! {
                    (if default=> $($tt:tt)*) => {
                        $($tt)*
                    };
                    (if !default=> $($tt:tt)*) => { };
                }
            } else {
                quote! {
                    (if default=> $($tt:tt)*) => { };
                    (if !default=> $($tt:tt)*) => {
                        $($tt)*
                    };
                }
            };

            let arg_locals: Vec<_> = arg_idents.iter().enumerate().map(|(i, id)| ident!("__{}_{}", i, id)).collect();

            let whens = if arg_locals.len() == 1 {
                let arg = &arg_locals[0];
                quote! {
                    let #arg = __when_var! {
                        $(
                            $(#[$meta])*
                            std::clone::Clone::clone(&$condition) => $args,
                        )*
                        _ => $default_args,
                    };
                }
            } else {
                let n = (0..arg_locals.len()).map(syn::Index::from);
                quote! {
                    #(
                        let #arg_locals = __when_var! {
                            $(
                                $(#[$meta])*
                                std::clone::Clone::clone(&$condition) => $args.#n,
                            )*
                            _ => $default_args.#n,
                        };
                    )*
                }
            };

            tokens.extend(quote! {
                #cfg
                #[doc(hidden)]
                #[macro_export]
                macro_rules! #macro_ident {
                    // named_new property::path, __ArgsImpl { a: a, b: b }
                    (named_new $property_path:path, $ArgsImpl:ident $fields_block:tt) => {
                        {
                            use $property_path::{__property_new};
                            __property_new! {
                                property_path { $property_path }
                                args_impl_spanned { $ArgsImpl }
                                arg_idents { #(#arg_idents)* }

                                $fields_block
                            }
                        }
                    };

                    #set

                    #allowed_in_when

                    #capture_only

                    #if_pub

                    #if_default

                    (if resolved=> $($tt:tt)*) => {
                        $($tt)*
                    };

                    (when $property_path:path {
                        $(
                            $(#[$meta:meta])*
                            $condition:ident => $args:ident,
                        )+
                        _ => $default_args:ident,
                    }) => {
                        {
                            use $property_path::{ArgsImpl as __ArgsImpl, Args as __Args, when_var as __when_var};
                            $(
                                $(#[$meta])*
                                let $args = __Args::unwrap($args);
                            )+
                            let $default_args = __Args::unwrap($default_args);
                            #whens
                            __ArgsImpl::new(#(#arg_locals),*)
                        }
                    };
                }
            })
        }
    }

    pub struct OutputMod {
        pub cfg: Option<Attribute>,
        pub vis: Visibility,
        pub is_capture_only: bool,
        pub has_default_value: bool,
        pub ident: Ident,
        pub macro_ident: Ident,
        pub args_ident: Ident,
        pub args_impl_ident: Ident,
        pub alias_fn: TokenStream,
    }
    impl ToTokens for OutputMod {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            let OutputMod {
                cfg,
                vis,
                ident,
                macro_ident,
                args_ident,
                args_impl_ident,
                alias_fn,
                ..
            } = self;

            let crate_core = crate_core();

            let default_export = if self.has_default_value {
                let default_fn_ident = ident!("__{}_default_args", ident);
                quote! {
                    #default_fn_ident as default_args,
                }
            } else {
                TokenStream::new()
            };

            let set_export = if self.is_capture_only {
                TokenStream::new()
            } else {
                let set_ident = ident!("__{}_set", ident);

                #[cfg(debug_assertions)]
                {
                    let set_dbg_ident = ident!("__{}_set_debug", ident);
                    quote! {
                        #set_ident as set,
                        #set_dbg_ident as set_debug,
                    }
                }
                #[cfg(not(debug_assertions))]
                quote! {
                    #set_ident as set,
                }
            };

            let cap_export = {
                #[cfg(debug_assertions)]
                {
                    let cap_ident = ident!("__{}_captured_debug", ident);
                    quote! {
                        #cap_ident as captured_debug,
                    }
                }
                #[cfg(not(debug_assertions))]
                TokenStream::new()
            };

            let alias_reexport = if !alias_fn.is_empty() {
                let ident = ident!("__wgt_{}", ident);
                quote! {
                    #[doc(inline)]
                    #vis use super::#ident::wgt_docs_export;
                }
            } else {
                quote! {
                    #[doc(hidden)]
                    #vis fn wgt_docs_export() {}
                }
            };

            tokens.extend(quote! {
                #alias_fn

                #cfg
                #[doc(hidden)]
                #vis mod #ident {
                    #vis use super::{
                        #ident as export,
                    };
                    pub use super::{
                        #args_impl_ident as ArgsImpl,
                        #args_ident as Args,
                        #default_export
                        #set_export
                        #cap_export
                    };
                    pub use #macro_ident as code_gen;
                    pub use #crate_core::var::{when_var, switch_var};
                    #[doc(hidden)]
                    pub use #crate_core::property_new as __property_new;

                    #alias_reexport
                }
            })
        }
    }
}
