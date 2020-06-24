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
    pub use crate::{
        util::{non_user_braced, non_user_parenthesized},
        widget_stage3::input::{InheritedProperty, InheritedWhen, PropertyAssign, PropertyValue as InputPropertyValue, WgtItemWhen},
    };
    use proc_macro2::Ident;
    use syn::{
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        spanned::Spanned,
        Block, Error, Token,
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
                let inner = non_user_braced(input);
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
            let input = non_user_braced(input);

            let mut items = vec![];

            while !input.is_empty() {
                items.push(input.parse()?);
            }

            Ok(UserInput { items })
        }
    }

    pub enum UserInputItem {
        Property(PropertyAssign),
        When(WgtItemWhen),
        Content(UserContent),
    }
    impl Parse for UserInputItem {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            if input.peek2(Token![:]) {
                Ok(UserInputItem::Property(input.parse()?))
            } else if input.peek(keyword::when) {
                Ok(UserInputItem::When(input.parse()?))
            } else if input.peek(Token![=>]) {
                Ok(UserInputItem::Content(input.parse()?))
            } else {
                Err(Error::new(
                    input.span(),
                    "expected property assign, when block or widget content (=>)",
                ))
            }
        }
    }

    pub struct UserContent {
        pub fat_arrow_token: Token![=>],
        pub block: Block,
    }
    impl Parse for UserContent {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(UserContent {
                fat_arrow_token: input.parse()?,
                block: input.parse()?,
            })
        }
    }
    impl Spanned for UserContent {
        fn span(&self) -> proc_macro2::Span {
            let span = self.fat_arrow_token.span();
            span.join(self.block.span()).unwrap_or(span)
        }
    }
}

mod analysis {
    use super::input::InputPropertyValue;
    use super::input::UserInputItem;
    use super::{input::WidgetNewInput, output::*};
    use crate::{property::input::Prefix, util::Errors, widget_stage3::analysis::*};
    use proc_macro2::{Ident, Span};
    use std::collections::{HashMap, HashSet};
    use syn::{parse_quote, spanned::Spanned};

    pub fn generate(input: WidgetNewInput) -> WidgetNewOutput {
        let mut properties = vec![];
        let mut whens = vec![];
        let mut contents = vec![];
        for item in input.user_input.items {
            match item {
                UserInputItem::Property(p) => properties.push(p),
                UserInputItem::When(w) => whens.push(w),
                UserInputItem::Content(c) => contents.push(c),
            }
        }

        let mut errors = Errors::default();

        // validate `=> {}`
        for extra_contents in contents.iter().skip(1) {
            errors.push("widget content already set", extra_contents.fat_arrow_token.span())
        }
        let content_block = if let Some(content) = contents.into_iter().next() {
            content.block
        } else {
            errors.push("missing widget content (=> {})", Span::call_site());
            parse_quote!({ () })
        };
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

        // process widget properties.
        for (target, property) in input
            .default
            .into_iter()
            .map(|p| (AssignTarget::Widget, p))
            .chain(input.default_child.into_iter().map(|p| (AssignTarget::Child, p)))
        {
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

            for arg in when.args {
                if user_properties.contains_key(&arg) || state_bindings_done.contains(&arg) || widget_defaults.contains(&arg) {
                    // user or widget already set arg or another when already uses the same property.
                    continue;
                } else if Prefix::new(&arg) == Prefix::State {
                    is_bindings.push(StateBinding {
                        widget: input.name.clone(),
                        property: arg,
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
                    properties: is_bindings.iter().map(|b| b.property.clone()).collect(),
                },
            });

            for binding in is_bindings {
                state_bindings_done.insert(binding.property.clone());
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
        for when in whens {
            let when_analysis = match WhenConditionAnalysis::new(when.condition) {
                Ok(r) => r,
                Err(e) => {
                    errors.push_syn(e);
                    continue;
                }
            };

            when_bindings.push(WhenBinding {
                index: when_index,
                condition: WhenCondition::Local {
                    widget: input.name.clone(),
                    properties: when_analysis.properties.iter().map(|p| p.property.clone()).collect(),
                    expr: when_analysis.expr,
                },
            });

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
                            widget: Some(input.name.clone()),
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

        WidgetNewOutput {
            args_bindings: ArgsBindings {
                args: args_bindings,
                state_args: state_bindings,
            },
            when_bindings: WhenBindings {
                conditions: when_bindings,
                indexes: property_indexes,
                switch_args: when_switch_bindings,
            },
            content_binding: ContentBinding { content: content_block },
            child_props_assigns: PropertyAssigns {
                widget_name: input.name.clone(),
                properties: child_props_assigns,
            },
            new_child_call: NewCall {
                widget_name: input.name.clone(),
                is_new_child: true,
                properties: input.new_child.into_iter().collect(),
            },
            props_assigns: PropertyAssigns {
                widget_name: input.name.clone(),
                properties: props_assigns,
            },
            new_call: NewCall {
                widget_name: input.name,
                is_new_child: false,
                properties: input.new.into_iter().collect(),
            },
            errors,
        }
    }
}

mod output {
    use crate::{
        property::input::Priority,
        util::{zero_ui_crate_ident, Errors},
        widget_stage3::input::{PropertyArgs, PropertyFields},
        widget_stage3::output::WhenConditionExpr,
    };
    use proc_macro2::{Ident, TokenStream};
    use quote::ToTokens;
    use syn::Block;

    pub struct WidgetNewOutput {
        pub args_bindings: ArgsBindings,
        pub when_bindings: WhenBindings,
        pub content_binding: ContentBinding,
        pub child_props_assigns: PropertyAssigns,
        pub new_child_call: NewCall,
        pub props_assigns: PropertyAssigns,
        pub new_call: NewCall,
        pub errors: Errors,
    }
    impl ToTokens for WidgetNewOutput {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.errors.to_tokens(tokens);
            let mut inner = TokenStream::new();
            self.args_bindings.to_tokens(&mut inner);
            self.content_binding.to_tokens(&mut inner);
            self.child_props_assigns.to_tokens(&mut inner);
            self.new_child_call.to_tokens(&mut inner);
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
                        let #var_name = #property_path::args(#args);
                    }
                }
                PropertyValue::Fields(fields) => {
                    let property_path = property_path();
                    quote! {
                        let #var_name = #property_path::NamedArgs {
                            _phantom: std::marker::PhantomData,
                            #fields
                        };
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
        pub widget: Ident,
        pub property: Ident,
    }
    impl ToTokens for StateBinding {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let var_name = ident!("{}_args", self.property);
            let widget = &self.widget;
            let property = &self.property;
            let crate_ = zero_ui_crate_ident();
            tokens.extend(quote! {let #var_name = #widget::properties::#property::args(#crate_::core::var::state_var());})
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
    }

    impl ToTokens for WhenBindings {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.conditions.iter().for_each(|c| c.to_tokens(tokens));
            self.indexes.iter().for_each(|c| c.to_tokens(tokens));
            self.switch_args.iter().for_each(|c| c.to_tokens(tokens));
        }
    }

    pub struct WhenBinding {
        pub index: u32,
        pub condition: WhenCondition,
    }
    impl ToTokens for WhenBinding {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let var_name = ident!("local_w{}", self.index);
            let condition = &self.condition;
            tokens.extend(quote! {
                let #var_name = {
                    #condition
                };
            })
        }
    }

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
            expr: WhenConditionExpr,
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
                    properties: p,
                    expr,
                } => {
                    let not_allowed_msg = p.iter().map(|p| format!("property `{}` is not allowed in when condition", p));
                    tokens.extend(quote! {
                        #(#widget::properties::#p::assert!(allowed_in_when, #not_allowed_msg);)*
                        #expr
                    })
                }
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
            let crate_ = zero_ui_crate_ident();
            if self.whens.len() == 1 {
                let wn = ident!("local_w{}", self.whens[0].index);
                if self.whens[0].can_move {
                    tokens.extend(quote! {
                        let #var_name = #crate_::core::var::Var::into_map(#wn, |&#wn| if #wn { 1usize } else { 0usize });
                    });
                } else {
                    tokens.extend(quote! {
                        let #var_name = #crate_::core::var::Var::map(&#wn, |&#wn| if #wn { 1usize } else { 0usize });
                    });
                }
            } else {
                debug_assert!(!self.whens.is_empty());
                let wns: Vec<_> = self.whens.iter().map(|i| ident!("local_w{}", i.index)).collect();
                let wns_clone = self.whens.iter().map(|i| if i.can_move { None } else { Some(quote!(.clone())) });
                let wns_rev = wns.iter().rev();
                let wns_i = (1..=wns.len()).rev();
                tokens.extend(quote! {
                    let #var_name = #crate_::core::var::merge_var!(#(#wns #wns_clone,)* |#(&#wns),*|{
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
                PropertyValue::Args(a) => a.to_token_stream(),
                PropertyValue::Fields(f) => f.to_token_stream(),
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
                    #property_path::switch_args!(#property_path, #index_var_name, #var_name, #(#when_var_names),*)
                };
            })
        }
    }

    pub struct WhenPropertyValue {
        pub index: u32,
        pub value: PropertyValue,
    }

    pub struct ContentBinding {
        pub content: Block,
    }
    impl ToTokens for ContentBinding {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            let content = &self.content;
            tokens.extend(quote! {
                let node = #content;
            });
        }
    }

    pub struct PropertyAssigns {
        pub widget_name: Ident,
        pub properties: Vec<PropertyAssign>,
    }
    impl ToTokens for PropertyAssigns {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let mod_ = {
                let name = &self.widget_name;
                quote!(#name::properties)
            };

            for priority in &Priority::all() {
                for property in &self.properties {
                    let ident = &property.ident;
                    let args_ident = ident!("{}_args", ident);

                    let set_args = if property.is_from_widget {
                        quote!(#mod_::#ident::set_args)
                    } else {
                        quote!(#ident::set_args)
                    };

                    tokens.extend(quote! {
                        #set_args!(#priority, #set_args, node, #args_ident);
                    });
                }
            }
        }
    }
    pub struct PropertyAssign {
        pub is_from_widget: bool,
        pub ident: Ident,
    }
    impl Priority {
        pub fn all() -> [Self; 5] {
            use crate::property::input::keyword::*;
            [
                Priority::Context(context::default()),
                Priority::Event(event::default()),
                Priority::Outer(outer::default()),
                Priority::Size(size::default()),
                Priority::Inner(inner::default()),
            ]
        }
    }

    pub struct NewCall {
        pub widget_name: Ident,
        pub is_new_child: bool,
        // properties captured by the new function
        pub properties: Vec<Ident>,
    }
    impl ToTokens for NewCall {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let name = &self.widget_name;
            let new_token = if self.is_new_child { quote!(new_child) } else { quote!(new) };
            let args = self.properties.iter().map(|p| ident!("{}_args", p));

            let call = quote!(#name::#new_token(node, #(#args),*));

            if self.is_new_child {
                tokens.extend(quote!(let node = #call;));
            } else {
                tokens.extend(call);
            }
        }
    }
}
