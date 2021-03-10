use quote::ToTokens;
use syn::parse_macro_input;

/// `widget_new!` expansion.
pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as input::WidgetNewInput);
    let output = analysis::generate(input);
    let output_stream = output.to_token_stream();
    output_stream.into()
}

mod input {
    pub use crate::widget_stage3::input::{
        InheritedProperty, InheritedWhen, PropertyAssign, PropertyBlock, PropertyValue as InputPropertyValue,
    };
    use proc_macro2::Ident;
    use syn::{
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        Error, Expr, Token,
    };

    mod keyword {
        pub use crate::widget_stage3::input::keyword::{default_child, new, new_child, when, whens};
        syn::custom_keyword!(user_input);
    }

    pub struct WidgetNewInput {
        pub name: Ident,
        pub default: Punctuated<InheritedProperty, Token![,]>,
        pub default_child: Punctuated<InheritedProperty, Token![,]>,
        pub whens: Punctuated<InheritedWhen, Token![,]>,
        pub new: Punctuated<Ident, Token![,]>,
        pub new_child: Punctuated<Ident, Token![,]>,
        pub user_input: UserInput,
    }
    impl Parse for WidgetNewInput {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            fn parse_block<T: Parse, R: Parse>(input: ParseStream) -> Punctuated<R, Token![,]> {
                input.parse::<T>().unwrap_or_else(|e| non_user_error!(e));
                let inner = non_user_braced!(input);
                Punctuated::parse_terminated(&inner).unwrap_or_else(|e| non_user_error!(e))
            }

            Ok(WidgetNewInput {
                name: input.parse().unwrap_or_else(|e| non_user_error!(e)),
                default: parse_block::<Token![default], InheritedProperty>(&input),
                default_child: parse_block::<keyword::default_child, InheritedProperty>(&input),
                whens: parse_block::<keyword::whens, InheritedWhen>(&input),
                new: parse_block::<keyword::new, Ident>(input),
                new_child: parse_block::<keyword::new_child, Ident>(input),
                user_input: input.parse()?,
            })
        }
    }

    pub struct UserInput {
        pub items: Vec<UserInputItem>,
    }
    impl Parse for UserInput {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            input.parse::<keyword::user_input>().unwrap_or_else(|e| non_user_error!(e));
            let input = non_user_braced!(input);

            let mut items = vec![];

            while !input.is_empty() {
                items.push(input.parse()?);
            }

            Ok(UserInput { items })
        }
    }

    pub enum UserInputItem {
        Property(PropertyAssign),
        ShortProperty(ShortPropertyAssign),
        When(WgtItemWhen),
        MetaProperty(MetaProperty),
    }
    impl Parse for UserInputItem {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            if input.peek2(Token![:]) {
                Ok(UserInputItem::Property(input.parse()?))
            } else if input.peek2(Token![;]) {
                Ok(UserInputItem::ShortProperty(input.parse()?))
            } else if input.peek(keyword::when) {
                Ok(UserInputItem::When(input.parse()?))
            } else if input.peek(Token![@]) {
                Ok(UserInputItem::MetaProperty(input.parse()?))
            } else {
                Err(Error::new(input.span(), "expected property assign or when block"))
            }
        }
    }

    pub struct MetaProperty {
        pub at_token: Token![@],
        pub ident: Ident,
        pub colon_token: Token![:],
        pub value: Box<Expr>,
        pub semi_token: Token![;],
    }
    impl Parse for MetaProperty {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(MetaProperty {
                at_token: input.parse()?,
                ident: input.parse()?,
                colon_token: input.parse()?,
                value: input.parse()?,
                semi_token: input.parse()?,
            })
        }
    }

    pub struct WgtItemWhen {
        pub when_token: keyword::when,
        pub condition: Box<Expr>,
        pub block: WhenBlock,
    }
    impl Parse for WgtItemWhen {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(WgtItemWhen {
                when_token: input.parse()?,
                condition: Box::new(Expr::parse_without_eager_brace(input)?),
                block: input.parse()?,
            })
        }
    }
    pub type WhenBlock = PropertyBlock<WhenPropertyAssign>;

    pub enum WhenPropertyAssign {
        Assign(PropertyAssign),
        Short(ShortPropertyAssign),
    }

    impl Parse for WhenPropertyAssign {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            if input.peek2(Token![:]) {
                Ok(WhenPropertyAssign::Assign(input.parse()?))
            } else if input.peek2(Token![;]) {
                Ok(WhenPropertyAssign::Short(input.parse()?))
            } else {
                Err(Error::new(input.span(), "expected property assign"))
            }
        }
    }

    pub struct ShortPropertyAssign {
        pub ident: Ident,
        pub semi_token: Token![;],
    }
    impl Parse for ShortPropertyAssign {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(ShortPropertyAssign {
                ident: input.parse()?,
                semi_token: input.parse()?,
            })
        }
    }
}

mod analysis {
    use super::{
        input::{self, InputPropertyValue, PropertyBlock, UserInputItem, WidgetNewInput},
        output::*,
    };
    use crate::widget_stage3::input::WgtItemWhen;
    use crate::{property::Prefix, util::Errors, widget_stage3::analysis::*};
    use proc_macro2::{Ident, Span};
    use std::collections::{HashMap, HashSet};
    use syn::{parse_quote, spanned::Spanned};

    pub fn generate(input: WidgetNewInput) -> WidgetNewOutput {
        let mut properties = vec![];
        let mut whens = vec![];
        let mut meta_properties = vec![];
        for item in input.user_input.items {
            match item {
                UserInputItem::Property(p) => properties.push(p),
                UserInputItem::ShortProperty(p) => properties.push(p.into()),
                UserInputItem::When(w) => whens.push(w.into()),
                UserInputItem::MetaProperty(m) => meta_properties.push(m),
            }
        }

        let mut errors = Errors::default();

        validate_property_assigns(&mut properties, &mut errors);
        validate_whens(&mut whens, &mut errors);

        // properties that are used in the new_child or new functions.
        let mut captured_properties = HashSet::new();
        for property in input.new.iter().chain(input.new_child.iter()) {
            captured_properties.insert(property.clone());
        }

        struct UserPropsIndex {
            binding: usize,
            assign: Option<usize>,
        }
        let mut user_properties = HashMap::new();
        let mut args_bindings = vec![];
        let mut unset_properties = HashMap::new();
        let mut mixed_props_assigns = vec![];

        #[derive(Clone, Copy)]
        enum AssignTarget {
            Child,
            Widget,
        }

        // process user properties.
        for property in properties {
            let is_captured = captured_properties.contains(&property.ident);

            let value = match property.value {
                InputPropertyValue::Fields(f) => PropertyValue::Fields(f),
                InputPropertyValue::Args(a) => PropertyValue::Args(a),
                InputPropertyValue::Unset(u) => {
                    if is_captured {
                        errors.push(format!("cannot unset captured property `{}`", property.ident), u.span())
                    } else {
                        unset_properties.insert(property.ident.clone(), u);
                    }
                    continue;
                }
            };

            args_bindings.push(ArgsBinding {
                widget: None,
                property: property.ident.clone(),
                value,
            });

            if !is_captured {
                mixed_props_assigns.push((
                    AssignTarget::Widget,
                    PropertyAssign {
                        is_from_widget: false,
                        user_assigned: true,
                        ident: property.ident.clone(),
                    },
                ));
            }

            user_properties.insert(
                property.ident,
                UserPropsIndex {
                    binding: args_bindings.len() - 1,
                    assign: if is_captured { None } else { Some(mixed_props_assigns.len() - 1) },
                },
            );
        }

        let mut widget_defaults = HashSet::new();
        let mut widget_properties = HashSet::new();

        // process widget properties.
        for (target, property) in input
            .default
            .into_iter()
            .map(|p| (AssignTarget::Widget, p))
            .chain(input.default_child.into_iter().map(|p| (AssignTarget::Child, p)))
        {
            widget_properties.insert(property.ident.clone());

            if let Some(&UserPropsIndex { binding: bi, assign: ai }) = user_properties.get(&property.ident) {
                // property already has user value, just change it to be found inside widget.
                args_bindings[bi].widget = Some(input.name.clone());
                if let Some(i) = ai {
                    mixed_props_assigns[i].1.is_from_widget = true;
                    mixed_props_assigns[i].0 = target;
                }
                continue;
            }

            use crate::widget_stage3::input::BuiltPropertyKind::*;
            match property.kind {
                Required => {
                    if let Some(u) = unset_properties.get(&property.ident) {
                        errors.push(format!("cannot unset required property `{}`", property.ident), u.span())
                    } else {
                        errors.push(format!("missing required property `{}`", property.ident), Span::call_site())
                    }
                }
                Local => {}
                Default => {
                    if unset_properties.get(&property.ident).is_none() {
                        widget_defaults.insert(property.ident.clone());
                        args_bindings.push(ArgsBinding {
                            widget: Some(input.name.clone()),
                            property: property.ident.clone(),
                            value: PropertyValue::Inherited,
                        });
                        if !captured_properties.contains(&property.ident) {
                            mixed_props_assigns.push((
                                target,
                                PropertyAssign {
                                    is_from_widget: true,
                                    user_assigned: false,
                                    ident: property.ident,
                                },
                            ));
                        }
                    }
                }
            }
        }

        let mut inited_properties = HashSet::with_capacity(mixed_props_assigns.len());
        let mut child_props_assigns = vec![];
        let mut props_assigns = vec![];
        for (target, p) in mixed_props_assigns {
            assert!(inited_properties.insert(p.ident.clone()));
            match target {
                AssignTarget::Child => child_props_assigns.push(p),
                AssignTarget::Widget => props_assigns.push(p),
            }
        }

        let mut state_bindings_done = HashSet::new();
        let mut state_bindings = vec![];
        let mut when_bindings = vec![];
        let mut when_index = 0;
        let mut property_indexes: HashMap<Ident, WhenPropertyIndex> = HashMap::new();
        let mut when_index_usage = HashMap::new();
        let mut when_switch_bindings: HashMap<Ident, WhenSwitchArgs> = HashMap::new();

        // process widget whens.
        'when_for: for when in input.whens {
            let mut is_bindings = vec![];

            for arg in when.args.iter() {
                if user_properties.contains_key(arg) || state_bindings_done.contains(arg) || widget_defaults.contains(arg) {
                    // user or widget already set arg or another when already uses the same property.
                    continue;
                } else if Prefix::new(&arg) == Prefix::State {
                    is_bindings.push(StateBinding {
                        widget: Some(input.name.clone()),
                        property: arg.clone(),
                    })
                } else if let Some(_u) = unset_properties.get(&arg) {
                    // TODO warning when API stabilizes.
                    continue 'when_for;
                } else {
                    unreachable!("when condition property has no initial value")
                }
            }

            // when will be used:

            when_bindings.push(WhenBinding {
                index: when_index,
                condition: WhenCondition::Inherited {
                    widget: input.name.clone(),
                    index: when_index,
                    properties: when.args.into_iter().collect(),
                },
            });

            for binding in is_bindings {
                state_bindings_done.insert(binding.property.clone());
                props_assigns.push(PropertyAssign {
                    is_from_widget: true,
                    user_assigned: false,
                    ident: binding.property.clone(),
                });
                state_bindings.push(binding);
            }

            for property in when.sets {
                let when_var = WhenConditionVar {
                    index: when_index,
                    can_move: false,
                };
                let count = when_index_usage.entry(when_index).or_insert(0);
                *count += 1;

                if let Some(entry) = property_indexes.get_mut(&property) {
                    entry.whens.push(when_var);
                } else {
                    property_indexes.insert(
                        property.clone(),
                        WhenPropertyIndex {
                            property: property.clone(),
                            whens: vec![when_var],
                        },
                    );
                }

                let when_value = WhenPropertyValue {
                    index: when_index,
                    value: PropertyValue::Inherited,
                };

                if let Some(entry) = when_switch_bindings.get_mut(&property) {
                    entry.whens.push(when_value);
                } else {
                    when_switch_bindings.insert(
                        property.clone(),
                        WhenSwitchArgs {
                            widget: Some(input.name.clone()),
                            property,
                            whens: vec![when_value],
                        },
                    );
                }
            }

            when_index += 1;
        }

        // process user whens.
        validate_whens_with_default(&mut whens, &mut errors, inited_properties);
        'when_for2: for when in whens {
            #[cfg(debug_assertions)]
            let expr_str = {
                let c = &when.condition;
                quote!(#c).to_string()
            };

            let when_analysis = match WhenConditionAnalysis::new(when.condition) {
                Ok(r) => r,
                Err(e) => {
                    errors.push_syn(e);
                    continue;
                }
            };

            let mut is_bindings = vec![];

            for arg in when_analysis.properties.iter().map(|p| &p.property) {
                if user_properties.contains_key(&arg) || state_bindings_done.contains(&arg) || widget_defaults.contains(&arg) {
                    // user or widget already set arg or another when already uses the same property.
                    continue;
                } else if Prefix::new(&arg) == Prefix::State {
                    is_bindings.push(StateBinding {
                        widget: None,
                        property: arg.clone(),
                    })
                } else if let Some(_u) = unset_properties.get(&arg) {
                    // TODO warning when API stabilizes.
                    continue 'when_for2;
                } else {
                    unreachable!("when condition property has no initial value")
                }
            }

            when_bindings.push(WhenBinding {
                index: when_index,
                condition: WhenCondition::Local {
                    widget: input.name.clone(),
                    properties: when_analysis.properties.iter().map(|p| p.property.clone()).collect(),
                    widget_properties: widget_properties.clone(),
                    expr: when_analysis.expr,
                    #[cfg(debug_assertions)]
                    expr_str,
                    #[cfg(debug_assertions)]
                    property_sets: when.block.properties.iter().map(|p| p.ident.clone()).collect(),
                },
            });

            for binding in is_bindings {
                state_bindings_done.insert(binding.property.clone());
                props_assigns.push(PropertyAssign {
                    is_from_widget: widget_properties.contains(&binding.property),
                    user_assigned: true,
                    ident: binding.property.clone(),
                });
                state_bindings.push(binding);
            }

            for property in when.block.properties {
                let when_var = WhenConditionVar {
                    index: when_index,
                    can_move: false,
                };
                let count = when_index_usage.entry(when_index).or_insert(0);
                *count += 1;

                if let Some(entry) = property_indexes.get_mut(&property.ident) {
                    entry.whens.push(when_var);
                } else {
                    property_indexes.insert(
                        property.ident.clone(),
                        WhenPropertyIndex {
                            property: property.ident.clone(),
                            whens: vec![when_var],
                        },
                    );
                }

                let when_value = WhenPropertyValue {
                    index: when_index,
                    value: match property.value {
                        InputPropertyValue::Fields(f) => PropertyValue::Fields(f),
                        InputPropertyValue::Args(a) => PropertyValue::Args(a),
                        InputPropertyValue::Unset(_) => unreachable!("error case removed early"),
                    },
                };

                if let Some(entry) = when_switch_bindings.get_mut(&property.ident) {
                    entry.whens.push(when_value);
                } else {
                    when_switch_bindings.insert(
                        property.ident.clone(),
                        WhenSwitchArgs {
                            widget: if widget_properties.contains(&property.ident) {
                                Some(input.name.clone())
                            } else {
                                None
                            },
                            property: property.ident,
                            whens: vec![when_value],
                        },
                    );
                }
            }

            when_index += 1;
        }

        let mut property_indexes: Vec<_> = property_indexes.into_iter().map(|(_, i)| i).collect();

        for pi in &mut property_indexes {
            for w in &mut pi.whens {
                let count = when_index_usage.get_mut(&w.index).unwrap();
                if *count == 1 {
                    debug_assert!(!w.can_move);
                    w.can_move = true;
                } else {
                    *count -= 1;
                }
            }
        }

        let when_switch_bindings = when_switch_bindings.into_iter().map(|(_, i)| i).collect();

        #[cfg(debug_assertions)]
        let debug_enabled = {
            let p_name = ident!("debug_enabled");
            if let Some(mp) = meta_properties.iter().find(|mp| mp.ident == p_name) {
                mp.value == parse_quote!(true)
            } else {
                true // default
            }
        };

        WidgetNewOutput {
            args_bindings: ArgsBindings {
                args: args_bindings,
                state_args: state_bindings,
            },
            when_bindings: WhenBindings {
                conditions: when_bindings,
                indexes: property_indexes,
                switch_args: when_switch_bindings,
                #[cfg(debug_assertions)]
                debug_enabled,
            },
            new_child_call: NewChildCall {
                widget_name: input.name.clone(),

                #[cfg(debug_assertions)]
                properties_user_assigned: input.new_child.iter().map(|p| user_properties.contains_key(p)).collect(),
                #[cfg(debug_assertions)]
                debug_enabled,

                properties: input.new_child.into_iter().collect(),
            },
            child_props_assigns: PropertyAssigns {
                widget_name: input.name.clone(),
                properties: child_props_assigns,
                #[cfg(debug_assertions)]
                debug_enabled,
            },
            props_assigns: PropertyAssigns {
                widget_name: input.name.clone(),
                properties: props_assigns,
                #[cfg(debug_assertions)]
                debug_enabled,
            },
            new_call: NewCall {
                widget_name: input.name,

                #[cfg(debug_assertions)]
                properties_user_assigned: input.new.iter().map(|p| user_properties.contains_key(p)).collect(),
                #[cfg(debug_assertions)]
                debug_enabled,

                properties: input.new.into_iter().collect(),
            },
            errors,
        }
    }

    impl From<input::ShortPropertyAssign> for input::PropertyAssign {
        fn from(p: input::ShortPropertyAssign) -> Self {
            input::PropertyAssign {
                colon_token: parse_quote![:],
                value: {
                    let ident = &p.ident;
                    parse_quote!(#ident)
                },
                ident: p.ident,
                semi_token: p.semi_token,
            }
        }
    }

    impl From<input::WgtItemWhen> for WgtItemWhen {
        fn from(w: input::WgtItemWhen) -> Self {
            WgtItemWhen {
                attrs: vec![],
                when_token: w.when_token,
                condition: w.condition,
                block: PropertyBlock {
                    brace_token: w.block.brace_token,
                    properties: w.block.properties.into_iter().map(From::from).collect(),
                },
            }
        }
    }

    impl From<input::WhenPropertyAssign> for input::PropertyAssign {
        fn from(p: input::WhenPropertyAssign) -> Self {
            match p {
                input::WhenPropertyAssign::Assign(a) => a,
                input::WhenPropertyAssign::Short(p) => p.into(),
            }
        }
    }
}

mod output {
    use crate::{
        property::Priority,
        util::{crate_core, Errors},
        widget_stage3::input::{PropertyArgs, PropertyFields},
        widget_stage3::output::{WhenConditionExpr, WhenPropertyRef},
    };
    use proc_macro2::{Ident, TokenStream};
    use quote::ToTokens;
    use std::collections::HashSet;

    pub struct WidgetNewOutput {
        pub args_bindings: ArgsBindings,
        pub when_bindings: WhenBindings,
        pub new_child_call: NewChildCall,
        pub child_props_assigns: PropertyAssigns,
        pub props_assigns: PropertyAssigns,
        pub new_call: NewCall,
        pub errors: Errors,
    }
    impl ToTokens for WidgetNewOutput {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.errors.to_tokens(tokens);
            let mut inner = TokenStream::new();
            self.args_bindings.to_tokens(&mut inner);
            self.when_bindings.to_tokens(&mut inner);
            self.new_child_call.to_tokens(&mut inner);
            self.child_props_assigns.to_tokens(&mut inner);
            self.props_assigns.to_tokens(&mut inner);
            self.new_call.to_tokens(&mut inner);

            tokens.extend(quote!({#inner}));
        }
    }

    pub struct ArgsBindings {
        pub args: Vec<ArgsBinding>,
        pub state_args: Vec<StateBinding>,
    }
    impl ToTokens for ArgsBindings {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            self.args.iter().for_each(|arg| arg.to_tokens(tokens));
            self.state_args.iter().for_each(|arg| arg.to_tokens(tokens));
        }
    }
    pub struct ArgsBinding {
        pub widget: Option<Ident>,
        pub property: Ident,
        pub value: PropertyValue,
    }
    impl ToTokens for ArgsBinding {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let var_name = ident!("{}_args", self.property);
            let property_path = || {
                let property = &self.property;
                if let Some(widget) = &self.widget {
                    quote!(#widget::properties::#property)
                } else {
                    property.to_token_stream()
                }
            };

            let out = match &self.value {
                PropertyValue::Args(args) => {
                    let property_path = property_path();
                    quote! {
                        let #var_name = #property_path::ArgsImpl::new(#args);
                    }
                }
                PropertyValue::Fields(fields) => {
                    let property_path = property_path();
                    quote! {
                        let #var_name = #property_path::code_gen! { named_new #property_path, __ArgsImpl { #fields } };
                    }
                }
                PropertyValue::Inherited => {
                    let property = &self.property;
                    let widget = self
                        .widget
                        .as_ref()
                        .unwrap_or_else(|| non_user_error!("widget required for inherited property value"));

                    quote! {
                        let #var_name = #widget::defaults::#property();
                    }
                }
            };

            tokens.extend(out)
        }
    }

    pub struct StateBinding {
        pub widget: Option<Ident>,
        pub property: Ident,
    }
    impl ToTokens for StateBinding {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let var_name = ident!("{}_args", self.property);
            let property = &self.property;
            let crate_ = crate_core();
            let mod_ = self.widget.as_ref().map(|widget| quote!(#widget::properties::));

            tokens.extend(quote! {let #var_name = #mod_#property::ArgsImpl::new(#crate_::var::state_var());})
        }
    }

    #[derive(Debug)]
    pub enum PropertyValue {
        Args(PropertyArgs),
        Fields(PropertyFields),
        Inherited,
    }
    impl ToTokens for PropertyArgs {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.0.to_tokens(tokens)
        }
    }
    impl ToTokens for PropertyFields {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.fields.to_tokens(tokens)
        }
    }

    pub struct WhenBindings {
        pub conditions: Vec<WhenBinding>,
        pub indexes: Vec<WhenPropertyIndex>,
        pub switch_args: Vec<WhenSwitchArgs>,
        #[cfg(debug_assertions)]
        pub debug_enabled: bool,
    }

    impl ToTokens for WhenBindings {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.conditions.iter().for_each(|c| c.to_tokens(tokens));

            #[cfg(debug_assertions)]
            if self.debug_enabled {
                let infos = self.conditions.iter().map(|c| c.debug_info_tokens());
                tokens.extend(quote! {
                    let debug_whens = vec![#(#infos),*];
                });
            }

            self.indexes.iter().for_each(|c| c.to_tokens(tokens));
            self.switch_args.iter().for_each(|c| c.to_tokens(tokens));
        }
    }

    pub struct WhenBinding {
        pub index: u32,
        pub condition: WhenCondition,
    }
    impl WhenBinding {
        fn var_name(&self) -> Ident {
            ident!("local_w{}", self.index)
        }

        #[cfg(debug_assertions)]
        fn debug_info_tokens(&self) -> TokenStream {
            let crate_ = crate_core();
            let var_name = self.var_name();
            let var_clone = quote! { #crate_::var::VarObj::boxed(std::clone::Clone::clone(&#var_name)) };

            match &self.condition {
                WhenCondition::Inherited { widget, index, .. } => {
                    let fn_name = ident!("w{}_info", index);

                    quote! {
                        #widget::whens::#fn_name(
                            #var_clone,
                            #crate_::debug::source_location!()
                        )
                    }
                }
                WhenCondition::Local {
                    property_sets, expr_str, ..
                } => {
                    let props_str = property_sets.iter().map(|p| p.to_string());
                    quote! {
                        #crate_::debug::WhenInfoV1 {
                            condition_expr: #expr_str,
                            condition_var: Some(#var_clone),
                            properties: vec![#(#props_str),*],
                            decl_location: #crate_::debug::source_location!(),
                            instance_location: #crate_::debug::source_location!(),
                            user_declared: true,
                        }
                    }
                }
            }
        }
    }
    impl ToTokens for WhenBinding {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let var_name = self.var_name();
            let condition = &self.condition;
            tokens.extend(quote! {
                let #var_name = {
                    #condition
                };
            })
        }
    }

    #[cfg_attr(debug_assertions, allow(clippy::large_enum_variant))]
    pub enum WhenCondition {
        Inherited {
            widget: Ident,
            index: u32,
            /// properties used by the condition.
            properties: Vec<Ident>,
        },
        Local {
            widget: Ident,
            /// properties used by the condition.
            properties: Vec<Ident>,
            widget_properties: HashSet<Ident>,
            expr: WhenConditionExpr,
            #[cfg(debug_assertions)]
            expr_str: String,
            #[cfg(debug_assertions)]
            property_sets: Vec<Ident>,
        },
    }
    impl ToTokens for WhenCondition {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self {
                WhenCondition::Inherited { widget, index, properties } => {
                    let fn_name = ident!("w{}", index);
                    let properties = properties.iter().map(|p| ident!("{}_args", p));
                    tokens.extend(quote! { #widget::whens::#fn_name(#(&#properties),*) })
                }
                WhenCondition::Local {
                    widget,
                    properties,
                    widget_properties,
                    expr,
                    ..
                } => {
                    for property in properties {
                        let not_allowed_msg = format!("property `{}` is not allowed in when condition", property);

                        let mod_ = if widget_properties.contains(property) {
                            Some(quote! {#widget::properties::})
                        } else {
                            None
                        };

                        tokens.extend(quote! {
                            #mod_#property::code_gen!(assert allowed_in_when=> #not_allowed_msg);
                        });
                    }
                    expr.to_local_tokens(widget, widget_properties, tokens)
                }
            }
        }
    }

    impl WhenConditionExpr {
        fn to_local_tokens(&self, widget: &Ident, widget_properties: &HashSet<Ident>, tokens: &mut TokenStream) {
            match self {
                WhenConditionExpr::Ref(let_name) => {
                    let name = let_name.name();
                    let let_name = let_name.to_local_tokens(widget, widget_properties);
                    tokens.extend(quote! {
                        #[allow(clippy::let_and_return)]
                        #let_name
                        #name
                    })
                }
                WhenConditionExpr::Map(let_name, expr) => {
                    let name = let_name.name();
                    let let_name = let_name.to_local_tokens(widget, widget_properties);
                    let crate_ = crate_core();
                    tokens.extend(quote! {
                        #let_name
                        #crate_::var::Var::into_map(#name, |#name|{#expr})
                    })
                }
                WhenConditionExpr::Merge(let_names, expr) => {
                    let names: Vec<_> = let_names.iter().map(|n| n.name()).collect();
                    let crate_ = crate_core();
                    let let_names = let_names.iter().map(|l| l.to_local_tokens(widget, widget_properties));
                    tokens.extend(quote! {
                        #(#let_names)*
                        #crate_::var::merge_var!(#(#names, )* |#(#names),*|{
                            #expr
                        })
                    })
                }
                WhenConditionExpr::Inherited(inh) => inh.to_tokens(tokens),
            }
        }
    }

    impl WhenPropertyRef {
        fn to_local_tokens(&self, widget: &Ident, widget_properties: &HashSet<Ident>) -> TokenStream {
            let crate_ = crate_core();
            let mod_ = if widget_properties.contains(&self.property) {
                Some(quote! {#widget::properties::})
            } else {
                None
            };
            let property = &self.property;
            let property_args = ident!("{}_args", property);
            let arg = &self.arg;
            let name = self.name();
            quote! {
                let #name = #crate_::var::IntoVar::into_var(std::clone::Clone::clone(#mod_#property::#arg(&#property_args)));
            }
        }
    }

    pub struct WhenPropertyIndex {
        pub property: Ident,
        pub whens: Vec<WhenConditionVar>,
    }
    impl ToTokens for WhenPropertyIndex {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let var_name = ident!("{}_index", self.property);

            let crate_ = crate_core();
            if self.whens.len() == 1 {
                let wn = ident!("local_w{}", self.whens[0].index);
                if self.whens[0].can_move {
                    tokens.extend(quote! {
                        let #var_name = #crate_::var::Var::into_map(#wn, |&#wn| if #wn { 1usize } else { 0usize });
                    });
                } else {
                    tokens.extend(quote! {
                        let #var_name = #crate_::var::Var::map(&#wn, |&#wn| if #wn { 1usize } else { 0usize });
                    });
                }
            } else {
                debug_assert!(!self.whens.is_empty());
                let wns: Vec<_> = self.whens.iter().map(|i| ident!("local_w{}", i.index)).collect();
                let wns_clone = self.whens.iter().map(|i| if i.can_move { None } else { Some(quote!(.clone())) });
                let wns_rev = wns.iter().rev();
                let wns_i = (1..=wns.len()).rev();
                tokens.extend(quote! {
                    let #var_name = #crate_::var::merge_var!(#(#wns #wns_clone,)* |#(&#wns),*|{
                        #(if #wns_rev { #wns_i })else*
                        else { 0usize }
                    });
                });
            }
        }
    }

    pub struct WhenConditionVar {
        pub index: u32,
        pub can_move: bool,
    }

    pub struct WhenSwitchArgs {
        /// Widget name if the property exists in the widget.
        pub widget: Option<Ident>,
        pub property: Ident,
        pub whens: Vec<WhenPropertyValue>,
    }

    impl ToTokens for WhenSwitchArgs {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let var_name = ident!("{}_args", self.property);

            let widget = &self.widget;
            let property = &self.property;

            let index_var_name = ident!("{}_index", property);

            let when_var_names: Vec<_> = self.whens.iter().map(|w| ident!("{}{}", property, w.index)).collect();

            let when_var_inits = self.whens.iter().map(|w| match &w.value {
                PropertyValue::Args(a) => {
                    let widget = widget.as_ref().map(|w| quote! { #w::properties:: });
                    quote! { #widget#property::ArgsImpl::new(#a) }
                }
                PropertyValue::Fields(fields) => {
                    let widget = widget.as_ref().map(|w| quote! { #w::properties:: });
                    quote! {
                        #widget#property::code_gen! { named_new #widget#property, __ArgsImpl { #fields } }
                    }
                }
                PropertyValue::Inherited => {
                    debug_assert!(
                        widget.is_some(),
                        "property default value is inherited, but no widget name was given"
                    );
                    let wi = ident!("w{}", w.index);
                    quote! { #widget::when_defaults::#wi::#property() }
                }
            });

            let property_path = if let Some(widget) = &self.widget {
                quote!(#widget::properties::#property)
            } else {
                property.to_token_stream()
            };

            tokens.extend(quote! {
                let #var_name = {
                    #(let #when_var_names = #when_var_inits;)*
                    #property_path::code_gen!(switch #property_path,
                        #index_var_name,
                        #var_name, #(#when_var_names),*
                    )
                };
            })
        }
    }

    pub struct WhenPropertyValue {
        pub index: u32,
        pub value: PropertyValue,
    }

    pub struct NewChildCall {
        pub widget_name: Ident,
        // properties captured by the new function
        pub properties: Vec<Ident>,

        #[cfg(debug_assertions)]
        pub properties_user_assigned: Vec<bool>,

        #[cfg(debug_assertions)]
        pub debug_enabled: bool,
    }
    impl ToTokens for NewChildCall {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let name = &self.widget_name;
            let args = self.properties.iter().map(|p| ident!("{}_args", p));

            #[cfg(debug_assertions)]
            if self.debug_enabled {
                let crate_ = crate_core();
                let p = &self.properties;
                let p_names = p.iter().map(|p| p.to_string());
                let p_locs = p.iter().map(|p| quote_spanned!(p.span()=> #crate_::debug::source_location!()));
                let p_assig = &self.properties_user_assigned;
                let args = args.clone();

                tokens.extend(quote! {
                    let mut debug_captured_new_child = {
                        vec![#(#name::properties::#p::captured_debug(&#args, #p_names, #p_locs, #p_assig)),*]
                    };
                });
            }

            tokens.extend(quote!( let node = #name::new_child(#(#args),*); ));

            #[cfg(debug_assertions)]
            if self.debug_enabled {
                let crate_ = crate_core();
                tokens.extend(quote! {
                    let node = #crate_::debug::NewChildMarkerNode::new_v1(
                        #crate_::UiNode::boxed(node)
                    );
                });
            }
        }
    }

    pub struct PropertyAssigns {
        pub widget_name: Ident,
        pub properties: Vec<PropertyAssign>,
        #[cfg(debug_assertions)]
        pub debug_enabled: bool,
    }
    impl ToTokens for PropertyAssigns {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let mod_ = {
                let name = &self.widget_name;
                quote!(#name::properties)
            };

            // property details (*_args, *, ::*)
            let properties: Vec<_> = self
                .properties
                .iter()
                .map(|p| {
                    let ident = &p.ident;
                    let args_ident = ident!("{}_args", ident);

                    let property = if p.is_from_widget { quote!(#mod_::#ident) } else { quote!(#ident) };
                    (args_ident, ident, property, p.user_assigned)
                })
                .collect();

            // assert property is not capture_only
            for (_, ident, property, _) in &properties {
                let msg = format!("cannot set capture_only property `{}`", ident);
                tokens.extend(quote! {
                    #property::code_gen!(assert !capture_only=> #msg);
                });
            }

            // set the property in their priority.
            for priority in &Priority::all_settable() {
                #[cfg(debug_assertions)]
                for (args_ident, property_name, property, user_assigned) in &properties {
                    let property_name = property_name.to_string();
                    if self.debug_enabled {
                        let crate_core = crate_core();
                        tokens.extend(quote_spanned! {args_ident.span()=>
                            #property::code_gen!(set #priority,
                                node,
                                #property,
                                #args_ident,
                                #property_name,
                                #crate_core::debug::source_location!(),
                                #user_assigned
                            );
                        });
                    } else {
                        tokens.extend(quote_spanned! {args_ident.span()=>
                            #property::code_gen!(set #priority,
                                node,
                                #property,
                                #args_ident
                            );
                        });
                    }
                }

                #[cfg(not(debug_assertions))]
                for (args_ident, _, property, _) in &properties {
                    tokens.extend(quote_spanned! {args_ident.span()=>
                        #property::code_gen!(set #priority, node, #args_ident);
                    });
                }
            }
        }
    }
    pub struct PropertyAssign {
        pub is_from_widget: bool,
        pub user_assigned: bool,
        pub ident: Ident,
    }
    impl Priority {
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

    pub struct NewCall {
        pub widget_name: Ident,
        // properties captured by the new function
        pub properties: Vec<Ident>,

        #[cfg(debug_assertions)]
        pub properties_user_assigned: Vec<bool>,

        #[cfg(debug_assertions)]
        pub debug_enabled: bool,
    }
    impl ToTokens for NewCall {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let name = &self.widget_name;
            let args = self.properties.iter().map(|p| ident!("{}_args", p));

            #[cfg(debug_assertions)]
            if self.debug_enabled {
                let crate_ = crate_core();
                let name_str = name.to_string();
                let p = &self.properties;
                let p_names = p.iter().map(|p| p.to_string());
                let p_locs = p.iter().map(|p| quote_spanned!(p.span()=> #crate_::debug::source_location!()));
                let p_assig = &self.properties_user_assigned;
                let args = args.clone();

                tokens.extend(quote! {
                    let node = #crate_::debug::WidgetInstanceInfoNode::new_v1(
                        #crate_::UiNode::boxed(node),
                        #name_str,
                        #name::decl_location(),
                        #crate_::debug::source_location!(),
                        debug_captured_new_child,
                        vec![#(#name::properties::#p::captured_debug(&#args, #p_names, #p_locs, #p_assig)),*],
                        debug_whens
                    );
                });
            }

            tokens.extend(quote!(#name::new(node, #(#args),*)));
        }
    }
}
