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

    let uuid = crate::util::uuid(&input);

    let fn_ = match syn::parse::<input::PropertyFn>(input.clone()) {
        Ok(p) => p,
        Err(e) => {
            // in case of major parsing error, like item not being a function.
            let mut r = proc_macro::TokenStream::from(e.to_compile_error());
            r.extend(input);
            return r;
        }
    };

    let output = analysis::generate(args, fn_, uuid);

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
        syn::custom_keyword!(layout);
        syn::custom_keyword!(size);
        syn::custom_keyword!(border);
        syn::custom_keyword!(fill);
        syn::custom_keyword!(child_context);
        syn::custom_keyword!(child_layout);
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
        Layout(keyword::layout),
        Size(keyword::size),
        Border(keyword::border),
        Fill(keyword::fill),
        ChildContext(keyword::child_context),
        ChildLayout(keyword::child_layout),
        CaptureOnly(keyword::capture_only),
    }
    impl Priority {
        pub fn is_event(self) -> bool {
            matches!(self, Priority::Event(_))
        }
        pub fn is_capture_only(self) -> bool {
            matches!(self, Priority::CaptureOnly(_))
        }
        pub fn all_settable() -> [Self; 8] {
            use crate::property::keyword::*;
            [
                Priority::ChildLayout(child_layout::default()),
                Priority::ChildContext(child_context::default()),
                Priority::Fill(fill::default()),
                Priority::Border(border::default()),
                Priority::Size(size::default()),
                Priority::Layout(layout::default()),
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
            } else if lookahead.peek(keyword::layout) {
                input.parse().map(Priority::Layout)
            } else if lookahead.peek(keyword::size) {
                input.parse().map(Priority::Size)
            } else if lookahead.peek(keyword::border) {
                input.parse().map(Priority::Border)
            } else if lookahead.peek(keyword::fill) {
                input.parse().map(Priority::Fill)
            } else if lookahead.peek(keyword::child_context) {
                input.parse().map(Priority::ChildContext)
            } else if lookahead.peek(keyword::child_layout) {
                input.parse().map(Priority::ChildLayout)
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
                    Priority::Layout(_) => write!(f, "Layout"),
                    Priority::Size(_) => write!(f, "Size"),
                    Priority::Border(_) => write!(f, "Border"),
                    Priority::Fill(_) => write!(f, "Fill"),
                    Priority::ChildContext(_) => write!(f, "ChildContext"),
                    Priority::ChildLayout(_) => write!(f, "ChildLayout"),
                    Priority::CaptureOnly(_) => write!(f, "CaptureOnly"),
                }
            } else {
                match self {
                    Priority::Context(_) => write!(f, "context"),
                    Priority::Event(_) => write!(f, "event"),
                    Priority::Layout(_) => write!(f, "layout"),
                    Priority::Size(_) => write!(f, "size"),
                    Priority::Border(_) => write!(f, "border"),
                    Priority::Fill(_) => write!(f, "fill"),
                    Priority::ChildContext(_) => write!(f, "child_context"),
                    Priority::ChildLayout(_) => write!(f, "child_layout"),
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

    use crate::util::{crate_core, Attributes, Errors};

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

    pub fn generate(args: input::MacroArgs, fn_: input::PropertyFn, uuid: u64) -> output::Output {
        let input::PropertyFn { attrs, mut fn_ } = fn_;

        let mut errors = Errors::default();
        let crate_core = crate_core();

        // if Output must only expand to the function and errors.
        let mut fn_and_errors_only = false;

        let prefix = Prefix::new(&fn_.sig.ident);
        let attrs = Attributes::new(attrs);

        let mut allowed_in_when = match args.allowed_in_when {
            Some(b) => b.3.value,
            None => match prefix {
                Prefix::State | Prefix::None => true,
                Prefix::Event => false,
            },
        };

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
        if args.priority.is_event() && prefix != Prefix::Event && prefix != Prefix::State {
            errors.push("property marked `event` does not have prefix `on_` or `is_`", fn_.sig.ident.span());
        }

        // validate return type.
        let mut output_assert_data = None;
        let mut output_is_impl_node = false;

        if args.priority.is_capture_only() {
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
        } else {
            // properties not capture_only:
            // rust will validate because we call fn_ in ArgsImpl.set(..) -> impl UiNode.
            // we only need the span so that the error highlights the right code.
            if let syn::ReturnType::Type(_, t) = &fn_.sig.output {
                output_assert_data = Some(t.span());
                output_is_impl_node = is_impl_node(t);
            }
        };

        // patch signature to continue validation:
        let mut args_are_valid = true;
        if args.priority.is_capture_only() {
            if fn_.sig.inputs.is_empty() {
                if let Prefix::State = prefix {
                    fn_.sig.inputs.push(parse_quote!(_missing_param: #crate_core::var::StateVar));
                } else {
                    fn_.sig.inputs.push(parse_quote!(_missing_param: ()));
                }
                args_are_valid = false;
            }
        } else {
            if fn_.sig.inputs.is_empty() {
                fn_.sig.inputs.push(parse_quote!( _missing_child: impl #crate_core::UiNode ));
                args_are_valid = false;
            }
            if fn_.sig.inputs.len() == 1 {
                if let Prefix::State = prefix {
                    fn_.sig.inputs.push(parse_quote!(_missing_param: #crate_core::var::StateVar));
                } else {
                    fn_.sig.inputs.push(parse_quote!(_missing_param: ()));
                }
                args_are_valid = false;
            }
        }
        let args_are_valid = args_are_valid;

        // collect normal generics.
        let mut generic_types = vec![]; // Vec<TypeParam>
        for gen in fn_.sig.generics.type_params() {
            generic_types.push(gen.clone());
        }
        if allowed_in_when {
            for e in &generic_types {
                errors.push(
                    "`allowed_in_when = true` cannot have named type params, only `impl Trait`",
                    e.span(),
                );
                allowed_in_when = false;
            }
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
        let mut invalid_idents = || {
            let next = ident!("_invalid{invalid_n}");
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
                        arg_idents[last_i] = ident_spanned! {arg_idents[last_i].span()=> "__{}{count}", arg_idents[last_i]};
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

        // number of args generics that cannot be set by property::<T> because they where impl.
        let anon_generics_len = embedded_impl_types.types.len() + impl_types.len();

        generic_types.extend(embedded_impl_types.types);
        generic_types.extend(impl_types);
        let generic_types = generic_types;

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

        let macro_ident = ident!("{}_{}", fn_.sig.ident, uuid);

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

        let mut wgt_capture_only_reexport = TokenStream::new();
        let mut wgt_capture_only_reexport_use = TokenStream::new();
        if fn_attrs.is_wgt_capture_only {
            let cfg = &attrs.cfg;
            let mut fn_ = fn_.clone();
            let vis = &fn_.vis;
            let mod_ident = ident!("__wgt_cap_{}", fn_.sig.ident);
            fn_.sig.ident = ident!("wgt_cap_export");
            wgt_capture_only_reexport = quote! {
                #cfg
                #[doc(hidden)]
                #vis mod #mod_ident {
                    use super::*;

                    /// **`property`**
                    #fn_
                }
            };

            wgt_capture_only_reexport_use = quote! {
                #cfg
                #vis use super::#mod_ident::wgt_cap_export;
            }
        }

        // Build time optimization:
        //
        // Refactors the function to redirect to a private "property_impl" that is called using `UiNode::cfg_boxed`.
        //
        // This only applies if the property function is valid, is not capture-only, the child input is a simple `ident: impl UiNode`
        // and the return is also `impl UiNode`
        let mut actual_fn = None;
        if !fn_and_errors_only && invalid_n == 0 && output_is_impl_node {
            if let Some(syn::FnArg::Typed(t)) = fn_.sig.inputs.first() {
                if let syn::Pat::Ident(child_ident) = &*t.pat {
                    if is_impl_node(&t.ty) {
                        let mut fn_impl = fn_.clone();

                        let impl_ident = ident!("__{}_impl", fn_.sig.ident);

                        fn_.block = parse_quote! {{
                            fn box_fix(child: impl #crate_core::UiNode) -> impl #crate_core::UiNode {
                                #crate_core::UiNode::cfg_boxed(child)
                            }

                            let out = #impl_ident(box_fix(#child_ident), #(#arg_idents),*);
                            box_fix(out)
                        }};

                        fn_impl.sig.ident = impl_ident;
                        fn_impl.vis = syn::Visibility::Inherited;
                        actual_fn = Some(fn_impl);
                    }
                }
            }
        }

        let is_state = matches!(prefix, Prefix::State);

        output::Output {
            errors,
            fn_and_errors_only,
            fn_attrs,
            types: output::OutputTypes {
                cfg: attrs.cfg.clone(),
                ident: fn_.sig.ident.clone(),
                generics: generic_types,
                allowed_in_when,
                is_state,
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
                allowed_in_when,
                is_state,
                args_are_valid,
                macro_ident: macro_ident.clone(),
                args_ident: ident!("{}_Args", fn_.sig.ident),
                args_impl_ident: ident!("{}_ArgsImpl", fn_.sig.ident),
                property_type_ident: ident!("{}_PropertyType", fn_.sig.ident),
                has_default_value,
                wgt_capture_only_reexport,
                wgt_capture_only_reexport_use,
            },
            macro_: output::OutputMacro {
                cfg: attrs.cfg,
                macro_ident,
                export,
                priority: args.priority,
                allowed_in_when,
                is_state,
                arg_idents,
                has_default_value,
                anon_generics_len,
            },
            fn_,
            actual_fn,
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

    /// Best effort matches `impl opt_path::UiNode`.
    fn is_impl_node(ty: &syn::Type) -> bool {
        if let syn::Type::ImplTrait(ty) = ty {
            if ty.bounds.len() == 1 {
                let ty = ty.bounds.first().unwrap();
                if let syn::TypeParamBound::Trait(ty) = ty {
                    if ty.lifetimes.is_none() && matches!(&ty.modifier, syn::TraitBoundModifier::None) {
                        if let Some(seg) = ty.path.segments.last() {
                            if seg.arguments.is_empty() {
                                return seg.ident == "UiNode";
                            }
                        }
                    }
                }
            }
        }
        false
    }
}

mod output {
    use proc_macro2::{Ident, TokenStream};
    use quote::ToTokens;
    use syn::{spanned::Spanned, Attribute, ItemFn, TraitItemType, Type, TypeParam, Visibility};

    use crate::util::{crate_core, Errors};

    use super::input::Priority;

    pub struct Output {
        pub errors: Errors,
        pub fn_attrs: OutputAttributes,
        pub fn_: ItemFn,
        pub actual_fn: Option<ItemFn>,
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
                if let Some(actual_fn) = &self.actual_fn {
                    self.fn_attrs.to_tokens_no_docs(tokens);
                    actual_fn.to_tokens(tokens);
                }
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
                doc_extend!(tokens, "**`property`** ");
                for attr in &self.docs {
                    attr.to_tokens(tokens);
                }
            }
            if self.is_capture_only {
                tokens.extend(quote! {
                    ///
                    /// This property is `capture_only`, it can only be used in widget declarations
                    /// to define a property that is captured by the widget.
                });
            } else if !wgt {
                tokens.extend(quote! {
                    /// # As Function
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

            self.cfg.to_tokens(tokens);

            self.inline.to_tokens(tokens);
            for attr in &self.attrs {
                attr.to_tokens(tokens);
            }
        }

        pub fn to_tokens_no_docs(&self, tokens: &mut TokenStream) {
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
        pub is_state: bool,

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
            let args_impl_ident = ident!("{ident}_ArgsImpl");
            let args_ident = ident!("{ident}_Args");
            let arg_locals: Vec<_> = arg_idents.iter().enumerate().map(|(i, id)| ident!("__{i}_{id}")).collect();
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

            let arg_debug_vars = {
                let args_len = arg_locals.len();
                // generate debug_var calls with the arg type span in case the type does
                // not implement Clone and (IntoVar or Var or Debug) which generates a compile error.
                let debug_var_calls = arg_locals.iter().zip(arg_types).map(|(lid, ty)| {
                    let mut lid = lid.clone();
                    lid.set_span(ty.span());
                    quote_spanned! {ty.span()=>
                         {
                            use #crate_core::inspector::v1::debug_var_util::*;
                            (&&&&&Wrap(#lid)).debug_var()
                        },
                    }
                });

                let arg_idents_str = arg_idents.iter().map(|i| i.to_string());

                quote! {
                    let arg_debug_vars = {
                        let ( #(#arg_locals),* ) = self_.unwrap_ref();
                        #[allow(clippy::needless_borrow)]
                        let __r: [_; #args_len] = [
                            #(
                                #crate_core::inspector::v1::PropertyArg {
                                    name: #arg_idents_str,
                                    value: #debug_var_calls
                                },
                            )*
                        ];
                        Box::new(__r)
                    };
                }
            };

            let mut child_ty_span = proc_macro2::Span::call_site();
            let child_assert = if let Some((child_ty, ty_params)) = &self.child_assert {
                child_ty_span = child_ty.span();
                let assert_ident = ident!("__{ident}_arg0_assert");
                quote_spanned! {child_ty.span()=>
                    fn #assert_ident<#(#ty_params),*>(child: #child_ty) -> impl #crate_core::UiNode {
                        child
                    }
                }
            } else {
                TokenStream::default()
            };
            let set_child_span = child_ty_span;

            let mut set = TokenStream::new();

            if !self.priority.is_capture_only() {
                // set span for error when child type does not take impl UiNode.
                let child_arg = quote_spanned! {set_child_span=>
                    child: impl #crate_core::UiNode
                };
                let child_arg_use = quote_spanned! {set_child_span=>
                    box_fix(child)
                };

                let set_ident = ident!("__{ident}_set");
                {
                    let output_span = self.output_assert.unwrap_or_else(proc_macro2::Span::call_site);
                    let output_ty = quote_spanned! {output_span=>
                        impl #crate_core::UiNode
                    };
                    let out_ident = ident_spanned!(output_span=> "out");
                    set.extend(quote! {
                        #[doc(hidden)]
                        pub fn #set_ident(self_: impl #args_ident, #child_arg) -> #output_ty {
                            fn box_fix(node: impl #crate_core::UiNode) -> #output_ty {
                                #crate_core::UiNode::cfg_boxed(node)
                            }
                            let ( #(#arg_locals),* ) = self_.unwrap();
                            let #out_ident = #ident(#child_arg_use, #( #arg_locals ),*);
                            box_fix(#out_ident)
                        }
                    });
                }

                {
                    let set_inspect_ident = ident!("__{ident}_set_inspect");
                    let ident_str = ident.to_string();

                    let priority = match self.priority {
                        Priority::Context(_) => quote!(Context),
                        Priority::Event(_) => quote!(Event),
                        Priority::Layout(_) => quote!(Layout),
                        Priority::Size(_) => quote!(Size),
                        Priority::Border(_) => quote!(Border),
                        Priority::Fill(_) => quote!(Fill),
                        Priority::ChildContext(_) => quote!(ChildContext),
                        Priority::ChildLayout(_) => quote!(ChildLayout),
                        Priority::CaptureOnly(_) => quote!(CaptureOnly),
                    };
                    set.extend(quote! {
                        #crate_core::core_cfg_inspector! {
                            #[doc(hidden)]
                            pub fn #set_inspect_ident(
                                self_: impl #args_ident,
                                #child_arg,
                                property_name: &'static str,
                                instance_location: #crate_core::inspector::v1::SourceLocation,
                                user_assigned: bool,
                            ) -> #crate_core::BoxedUiNode {
                                #arg_debug_vars

                                fn box_fix(node: impl #crate_core::UiNode) -> #crate_core::BoxedUiNode {
                                    #crate_core::UiNode::boxed(node)
                                }
                                let node = box_fix(#set_ident(self_, #child_arg_use));

                                #crate_core::inspector::v1::inspect_property(
                                    node,
                                    #crate_core::inspector::v1::PropertyInstanceMeta {
                                        priority: #crate_core::inspector::v1::PropertyPriority::#priority,
                                        original_name: #ident_str,
                                        decl_location: #crate_core::inspector::v1::source_location!(),
                                        property_name,
                                        instance_location,
                                        user_assigned
                                    },
                                    arg_debug_vars,
                                )
                            }
                        }
                    });
                }
            };

            let cap_debug = {
                let cap_ident = ident!("__{ident}_captured_inspect");

                quote! {
                    #crate_core::core_cfg_inspector! {
                        #[doc(hidden)]
                        pub fn #cap_ident(
                            self_: &impl #args_ident,
                            property_name: &'static str,
                            instance_location: #crate_core::inspector::v1::SourceLocation,
                            user_assigned: bool
                        ) -> #crate_core::inspector::v1::CapturedPropertyInfo {
                            #arg_debug_vars
                            #crate_core::inspector::v1::CapturedPropertyInfo {
                                property_name,
                                instance_location,
                                user_assigned,
                                args: arg_debug_vars,
                            }
                        }
                    }
                }
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

            let named_arg_mtds: Vec<_> = arg_idents.iter().map(|a| ident!("__{a}")).collect();
            let numbered_arg_mtds: Vec<_> = (0..arg_idents.len()).map(|a| ident!("__{a}")).collect();

            let default_fn = if default_value.is_empty() {
                TokenStream::default()
            } else {
                let default_fn_ident = ident!("__{ident}_default_args");
                quote! {

                    #[doc(hidden)]
                    #[allow(non_snake_case)]
                    pub fn #default_fn_ident() -> impl #args_ident {
                        #default_value
                    }
                }
            };

            let allowed_in_when_assert = if self.allowed_in_when && self.args_are_valid {
                let assert_ident = ident!("__{ident}_assert_allowed_in_when");
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
                TokenStream::new()
            };

            let dyn_ctor = if self.allowed_in_when && self.args_are_valid && !matches!(self.priority, Priority::CaptureOnly(_)) {
                let arg_n = 0..arg_idents.len();
                let ident_dyn_ctor = ident!("__{ident}_dyn_ctor");

                let var_idents: Vec<_> = named_arg_mtds
                    .iter()
                    .zip(arg_types.iter())
                    .map(|(a, t)| {
                        let mut a = a.clone();
                        a.set_span(t.span());
                        a
                    })
                    .collect();

                if self.is_state {
                    let var_ident = &var_idents[0];
                    quote! {
                        #[doc(hidden)]
                        pub fn #ident_dyn_ctor(child__: #crate_core::BoxedUiNode, args__: &#crate_core::DynPropertyArgs)
                        -> std::result::Result<#crate_core::BoxedUiNode, (#crate_core::BoxedUiNode, #crate_core::DynPropError)>
                        {
                            let #var_ident = match args__.get_state() {
                                Ok(r) => r,
                                Err(e) => return Err((child__, e))
                            };
                            let r__ = #ident(child__, #var_ident);
                            Ok(#crate_core::UiNode::boxed(r__))
                        }
                    }
                } else {
                    let dyn_args = ident!("__{ident}_dyn_args");
                    let dyn_when_args = ident!("__{ident}_dyn_when_args");

                    let dyn_args_decl = arg_locals.iter().zip(arg_types.iter()).map(|(arg_local, arg_ty)| {
                        let span = arg_ty.span();
                        let mut r = quote_spanned! {span=>
                            #crate_core::var::AnyVar::into_any(
                                #crate_core::var::Var::boxed(
                                    #crate_core::var::IntoVar::allowed_in_when_property_requires_IntoVar_members(
                                        #arg_local
                                    )
                                )
                            ),
                        };
                        crate::util::set_span(&mut r, span);
                        r
                    });

                    let dyn_when_args_t: Vec<_> = arg_idents.iter().map(|i| ident!("T_{i}")).collect();
                    let dyn_when_args_tuple_ty = dyn_when_args_t.iter().zip(arg_types.iter()).map(|(arg_t, arg_ty)| {
                        let span = arg_ty.span();
                        let mut r = quote_spanned! {span=>
                            &#crate_core::var::types::RcWhenVar<#arg_t>
                        };
                        crate::util::set_span(&mut r, span);
                        r
                    });

                    quote! {
                        #[doc(hidden)]
                        pub fn #ident_dyn_ctor(child__: #crate_core::BoxedUiNode, args__: &#crate_core::DynPropertyArgs)
                        -> std::result::Result<#crate_core::BoxedUiNode, (#crate_core::BoxedUiNode, #crate_core::DynPropError)>
                        {
                            #(
                                let #var_idents = match args__.get(#arg_n) {
                                    Ok(r) => r,
                                    Err(e) => return Err((child__, e))
                                };
                            )*
                            let r__ = #ident(child__, #(#var_idents),*);
                            Ok(#crate_core::UiNode::boxed(r__))
                        }

                        #[doc(hidden)]
                        pub fn #dyn_args(args__: &impl #args_ident) -> std::vec::Vec<std::boxed::Box<dyn #crate_core::var::AnyVar>> {
                            let ( #(#arg_locals),* ) = args__.unwrap_ref();
                            std::vec![#(#dyn_args_decl)*]
                        }

                        #[doc(hidden)]
                        pub fn #dyn_when_args<#(#dyn_when_args_t: #crate_core::var::VarValue),*>(args__: (#(#dyn_when_args_tuple_ty),*)) -> std::vec::Vec<#crate_core::var::types::AnyWhenVarBuilder> {
                            let ( #(#arg_locals),* ) = args__;
                            std::vec![
                                #(
                                    #crate_core::var::types::AnyWhenVarBuilder::from_var(#arg_locals),
                                )*
                            ]
                        }
                    }
                }
            } else {
                let ident_dyn_ctor = ident!("__{ident}_dyn_ctor");
                quote! {
                    #[doc(hidden)]
                    pub use #crate_core::not_allowed_in_when_dyn_ctor as #ident_dyn_ctor;
                }
            };

            let property_type_ident = ident!("{ident}_PropertyType");

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
                pub enum #property_type_ident { }

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

                    pub fn new(#( #arg_idents: #arg_types ),*) -> impl #args_ident {
                        Self {
                            #phantom_init
                            #(#arg_idents,)*
                        }
                    }


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

                        fn #named_arg_mtds(&self) -> &#arg_return_types {
                            &self.#arg_idents
                        }


                        fn #numbered_arg_mtds(&self) -> &#arg_return_types {
                            &self.#arg_idents
                        }
                    )*


                    fn unwrap(self) -> #unwrap_ty {
                        #unwrap_expr
                    }


                    fn unwrap_ref(&self) -> #unwrap_ty_ref {
                        #unwrap_expr_ref
                    }
                }

                #child_assert
                #allowed_in_when_assert
                #dyn_ctor
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
                Priority::Layout(kw) => kw.to_tokens(tokens),
                Priority::Size(kw) => kw.to_tokens(tokens),
                Priority::Border(kw) => kw.to_tokens(tokens),
                Priority::Fill(kw) => kw.to_tokens(tokens),
                Priority::ChildContext(kw) => kw.to_tokens(tokens),
                Priority::ChildLayout(kw) => kw.to_tokens(tokens),
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
        pub is_state: bool,

        pub arg_idents: Vec<Ident>,

        pub has_default_value: bool,

        pub anon_generics_len: usize,
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
                quote! {
                    (set #priority, $node:ident, $property_path:path, $args:ident,
                        $property_name:expr, $source_location:expr, $user_assigned:tt, $__set:ident) => {
                            let $node = {
                                use $property_path::{core_cfg_inspector as __core_cfg_inspector};
                                __core_cfg_inspector! {
                                    use $property_path::{set_inspect as $__set};
                                    $__set($args, $node, $property_name, $source_location, $user_assigned)
                                }
                                __core_cfg_inspector! {@NOT
                                    use $property_path::{set as $__set};
                                    $__set($args, $node)
                                }
                            };
                    };
                    (set $other:ident, $($ignore:tt)+) => { };
                }
            };

            let set_dyn = if priority.is_capture_only() {
                quote! {
                    (set_dyn $($tt:tt)*) => {};
                }
            } else {
                let mut r = TokenStream::new();

                if self.allowed_in_when {
                    if self.is_state {
                        r.extend(quote! {
                            (set_dyn #priority, $node:ident, $property_path:path, $args:ident,
                                $property_name:expr, $source_location:expr, $user_assigned:tt, $priority_index:expr, $__set:ident,
                                $dyn_wgt_part:ident) => {
                                    let ($node, dyn_prop__) = $dyn_wgt_part.begin_property();
                                    let dyn_state__ = {
                                        use $property_path::{Args as __Args};
                                        std::clone::Clone::clone(__Args::__0(&$args))
                                    };
                                    let $node = {
                                        use $property_path::{core_cfg_inspector as __core_cfg_inspector};
                                        __core_cfg_inspector! {
                                            use $property_path::{set_inspect as $__set};
                                            $__set($args, $node, $property_name, $source_location, $user_assigned)
                                        }
                                        __core_cfg_inspector! {@NOT
                                            use $property_path::{set as $__set};
                                            $__set($args, $node)
                                        }
                                    };
                                    let (property_type_id__, dyn_ctor__) = {
                                        use $property_path::{PropertyType as __PropertyType, dyn_ctor as __dyn_ctor};

                                        (std::any::TypeId::of::<__PropertyType>(), __dyn_ctor)
                                    };
                                    $dyn_wgt_part.finish_property_state(
                                        dyn_prop__, $node, $property_name, property_type_id__,
                                        $user_assigned, $priority_index, dyn_ctor__, dyn_state__
                                    );
                            };
                        });
                    } else {
                        // extract args variables.
                        r.extend(quote! {
                            (set_dyn #priority, $node:ident, $property_path:path, $args:ident,
                                $property_name:expr, $source_location:expr, $user_assigned:tt, $priority_index:expr, $__set:ident,
                                $dyn_wgt_part:ident) => {
                                    let ($node, dyn_prop__) = $dyn_wgt_part.begin_property();
                                    let dyn_args__ = {
                                        use $property_path::{dyn_args as __dyn_args};
                                        __dyn_args(&$args)
                                    };
                                    let $node = {
                                        use $property_path::{core_cfg_inspector as __core_cfg_inspector};
                                        __core_cfg_inspector! {
                                            use $property_path::{set_inspect as $__set};
                                            $__set($args, $node, $property_name, $source_location, $user_assigned)
                                        }
                                        __core_cfg_inspector! {@NOT
                                            use $property_path::{set as $__set};
                                            $__set($args, $node)
                                        }
                                    };
                                    let (property_type_id__, dyn_ctor__) = {
                                        use $property_path::{PropertyType as __PropertyType, dyn_ctor as __dyn_ctor};
                                        (std::any::TypeId::of::<__PropertyType>(), __dyn_ctor)
                                    };
                                    $dyn_wgt_part.finish_property_allowed_in_when(
                                        dyn_prop__, $node, $property_name, property_type_id__,
                                        $user_assigned, $priority_index, dyn_ctor__, dyn_args__
                                    );
                            };
                        });
                        // when mode, extract when variables, convert into "any" builders.
                        r.extend(quote! {
                            (set_dyn #priority when, $node:ident, $property_path:path, $args:ident,
                                $property_name:expr, $source_location:expr, $user_assigned:tt, $priority_index:expr, $__set:ident,
                                $dyn_wgt_part:ident, $default_set:expr) => {
                                    let ($node, dyn_prop__) = $dyn_wgt_part.begin_property();
                                    let dyn_args__ = {
                                        use $property_path::{dyn_when_args as __dyn_args, Args as __Args};
                                        __dyn_args(__Args::unwrap_ref(&$args))
                                    };
                                    let $node = {
                                        use $property_path::{core_cfg_inspector as __core_cfg_inspector};
                                        __core_cfg_inspector! {
                                            use $property_path::{set_inspect as $__set};
                                            $__set($args, $node, $property_name, $source_location, $user_assigned)
                                        }
                                        __core_cfg_inspector! {@NOT
                                            use $property_path::{set as $__set};
                                            $__set($args, $node)
                                        }
                                    };
                                    let property_type_id__, dyn_ctor__ = {
                                        use $property_path::{PropertyType as __PropertyType, dyn_ctor as __dyn_ctor};
                                        (std::any::TypeId::of::<__PropertyType>(), __dyn_ctor)
                                    };
                                    $dyn_wgt_part.finish_property_with_when(
                                        dyn_prop__, $node, $property_name, property_type_id__,
                                        $user_assigned, $priority_index, dyn_ctor__, dyn_args__, $default_set
                                    );
                            };
                        });
                    }
                } else {
                    // not allowed in when.
                    r.extend(quote! {
                        (set_dyn #priority, $node:ident, $property_path:path, $args:ident,
                            $property_name:expr, $source_location:expr, $user_assigned:tt, $priority_index:expr, $__set:ident,
                            $dyn_wgt_part:ident) => {
                                let ($node, dyn_prop__) = $dyn_wgt_part.begin_property();
                                let $node = {
                                    use $property_path::{core_cfg_inspector as __core_cfg_inspector};
                                    __core_cfg_inspector! {
                                        use $property_path::{set_inspect as $__set};
                                        $__set($args, $node, $property_name, $source_location, $user_assigned)
                                    }
                                    __core_cfg_inspector! {@NOT
                                        use $property_path::{set as $__set};
                                        $__set($args, $node)
                                    }
                                };
                                let property_type_id__ = {
                                    use $property_path::{PropertyType as __PropertyType};
                                    std::any::TypeId::of::<__PropertyType>()
                                };
                                $dyn_wgt_part.finish_property_not_allowed_in_when(
                                    dyn_prop__, $node, $property_name, property_type_id__,
                                    $user_assigned, $priority_index
                                );
                        };
                    });
                }

                // ignore other priorities or when configured in not_allowed_in_when.
                r.extend(quote! {
                    (set_dyn $other:ident, $($ignore:tt)+) => { };
                });

                r
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

            let arg_locals: Vec<_> = arg_idents.iter().enumerate().map(|(i, id)| ident!("__{i}_{id}")).collect();

            let whens = if arg_locals.len() == 1 {
                let arg = &arg_locals[0];
                quote! {
                    let #arg = __when_var! {
                        $(
                            $(#[$meta])*
                            use($cfg_macro) std::clone::Clone::clone(&$condition) => $args,
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

            let mut generics_extra = TokenStream::new();
            for _ in 0..self.anon_generics_len {
                generics_extra.extend(quote!(,_));
            }

            tokens.extend(quote! {
                #cfg
                #[doc(hidden)]
                #[macro_export]
                macro_rules! #macro_ident {
                    // named_new property::path, __ArgsImpl ::<[T, U]>? { a: a, b: b }
                    (named_new $property_path:path, $ArgsImpl:ident ::<[$($type_args:tt)*]> $fields_block:tt) => {
                        {
                            use $property_path::{__property_new};
                            __property_new! {
                                property_path { $property_path }
                                args_impl_spanned { $ArgsImpl }
                                arg_idents { #(#arg_idents)* }
                                ty_args { $($type_args)* }
                                generics_extra { #generics_extra }
                                named_input { $fields_block }
                            }
                        }
                    };
                    // named_new property::path, __ArgsImpl { a: a, b: b }
                    (named_new $property_path:path, $ArgsImpl:ident $fields_block:tt) => {
                        {
                            use $property_path::{__property_new};
                            __property_new! {
                                property_path { $property_path }
                                args_impl_spanned { $ArgsImpl }
                                arg_idents { #(#arg_idents)* }
                                ty_args { }
                                generics_extra { #generics_extra }
                                named_input { $fields_block }
                            }
                        }
                    };

                    // unnamed_new property::path, __ArgsImpl ::<[T, U]> a, b
                    (unnamed_new $property_path:path, $ArgsImpl:ident ::<[$($type_args:tt)+]> $($fields:tt)*) => {
                        {
                            use $property_path::{__property_new};
                            __property_new! {
                                property_path { $property_path }
                                args_impl_spanned { $ArgsImpl }
                                arg_idents { #(#arg_idents)* }
                                ty_args { $($type_args)* }
                                generics_extra { #generics_extra }
                                unnamed_input { $($fields)* }
                            }
                        }
                    };
                    // unnamed_new property::path, __ArgsImpl a, b
                    (unnamed_new $property_path:path, $ArgsImpl:ident $($fields:tt)*) => {
                        {
                            use $property_path::{__property_new};
                            __property_new! {
                                property_path { $property_path }
                                args_impl_spanned { $ArgsImpl }
                                arg_idents { #(#arg_idents)* }
                                ty_args { }
                                generics_extra { #generics_extra }
                                unnamed_input { $($fields)* }
                            }
                        }
                    };

                    #set
                    #set_dyn

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
                            use($cfg_macro:path) $condition:ident => $args:ident,
                        )+
                        _ => $default_args:ident,
                    }) => {
                        {
                            use $property_path::{ArgsImpl as __ArgsImpl, Args as __Args, when_var as __when_var};
                            $(
                                $cfg_macro! {
                                    $(#[$meta])*
                                    let $args = __Args::unwrap($args);
                                }
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
        pub allowed_in_when: bool,
        pub is_state: bool,
        pub args_are_valid: bool,
        pub has_default_value: bool,
        pub ident: Ident,
        pub macro_ident: Ident,
        pub args_ident: Ident,
        pub args_impl_ident: Ident,
        pub property_type_ident: Ident,
        pub wgt_capture_only_reexport: TokenStream,
        pub wgt_capture_only_reexport_use: TokenStream,
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
                property_type_ident,
                wgt_capture_only_reexport,
                wgt_capture_only_reexport_use,
                ..
            } = self;

            let crate_core = crate_core();

            let default_export = if self.has_default_value {
                let default_fn_ident = ident!("__{ident}_default_args");
                quote! {
                    #default_fn_ident as default_args,
                }
            } else {
                TokenStream::new()
            };

            let (set_export, set_inspect_export) = if self.is_capture_only {
                (TokenStream::new(), TokenStream::new())
            } else {
                let set_ident = ident!("__{ident}_set");
                let set_dbg_ident = ident!("__{ident}_set_inspect");

                (
                    quote! {
                        #set_ident as set,
                    },
                    quote! {
                        #set_dbg_ident as set_inspect,
                    },
                )
            };

            let cap_ident = ident!("__{ident}_captured_inspect");
            let cap_export = quote! {
                #cap_ident as captured_inspect,
            };

            let dyn_ctor_ident = ident!("__{ident}_dyn_ctor");

            let dyn_args_export = if self.allowed_in_when && !self.is_state && !self.is_capture_only && self.args_are_valid {
                let dyn_args_ident = ident!("__{ident}_dyn_args");
                let dyn_when_args_ident = ident!("__{ident}_dyn_when_args");

                quote! {
                    #dyn_args_ident as dyn_args,
                    #dyn_when_args_ident as dyn_when_args,
                }
            } else {
                TokenStream::new()
            };

            tokens.extend(quote! {
                #wgt_capture_only_reexport

                #cfg
                #[doc(hidden)]
                #vis mod #ident {
                    #vis use super::{
                        #ident as export,
                    };
                    #wgt_capture_only_reexport_use
                    pub use super::{
                        #args_impl_ident as ArgsImpl,
                        #property_type_ident as PropertyType,
                        #args_ident as Args,
                        #dyn_ctor_ident as dyn_ctor,
                        #dyn_args_export
                        #default_export
                        #set_export
                    };

                    #crate_core::core_cfg_inspector! {
                        pub use super::{
                            #set_inspect_export
                            #cap_export
                        };
                    }

                    pub use #macro_ident as code_gen;
                    pub use #crate_core::var::{when_var, switch_var};
                    #[doc(hidden)]
                    pub use #crate_core::{property_new as __property_new, core_cfg_inspector};
                }
            })
        }
    }
}
