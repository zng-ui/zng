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
    use super::{input::WidgetNewInput, output::*};
    use crate::util::Errors;
    use proc_macro2::Span;
    use std::collections::HashMap;
    use syn::{parse_quote, spanned::Spanned};

    pub fn generate(input: WidgetNewInput) -> WidgetNewOutput {
        let mut properties = vec![];
        let mut whens = vec![];
        let mut contents = vec![];
        for item in input.user_input.items {
            use super::input::UserInputItem::*;
            match item {
                super::input::UserInputItem::Property(p) => properties.push(p),
                super::input::UserInputItem::When(w) => whens.push(w),
                super::input::UserInputItem::Content(c) => contents.push(c),
            }
        }

        //validate items
        let mut errors = Errors::default();

        for extra_contents in contents.iter().skip(1) {
            errors.push("widget content already set", extra_contents.fat_arrow_token.span())
        }
        let content_block = if let Some(content) = contents.into_iter().next() {
            content.block
        } else {
            errors.push("missing widget content (=> {})", Span::call_site());
            parse_quote!({ () })
        };

        let mut user_properties = HashMap::new();
        let mut args_bindings = vec![];
        let mut unsetted_properties = HashMap::new();
        let mut state_bindings = vec![];

        for property in properties {
            let value = match property.value {
                InputPropertyValue::Fields(f) => PropertyValue::Fields(f),
                InputPropertyValue::Args(a) => PropertyValue::Args(a),
                InputPropertyValue::Unset(u) => {
                    unsetted_properties.insert(property.ident.clone(), u);
                    continue;
                }
            };

            user_properties.insert(property.ident.clone(), args_bindings.len());
            args_bindings.push(ArgsBinding {
                widget: None,
                property: property.ident,
                value,
            });
        }

        for property in input.default {
            if let Some(&i) = user_properties.get(&property.ident) {
                args_bindings[i].widget = Some(input.name.clone());
                continue;
            }

            use crate::widget_stage3::input::BuiltPropertyKind::*;
            match property.kind {
                Required => {
                    if let Some(u) = unsetted_properties.get(&property.ident) {
                        errors.push(format!("cannot unset required property `{}`", property.ident), u.span())
                    } else {
                        errors.push(format!("missing required property `{}`", property.ident), Span::call_site())
                    }
                }
                Local => {}
                Default => todo!("property inits"),
            }
        }

        WidgetNewOutput {
            args_bindings: ArgsBindings { args: (), state_args: () },
            when_bindings: WhenBindings {
                conditions: (),
                indexes: (),
                switch_args: (),
            },
            content_binding: ContentBinding { content: content_block },
            child_props_assigns: PropertyAssigns {
                widget_name: (),
                properties: (),
            },
            new_child_call: NewCall {
                widget_name: (),
                is_new_child: (),
                args: (),
            },
            props_assigns: PropertyAssigns {
                widget_name: (),
                properties: (),
            },
            new_call: NewCall {
                widget_name: (),
                is_new_child: (),
                args: (),
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
            properties: Vec<Ident>,
        },
        Local {
            widget: Ident,
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
        pub widget: Ident,
        pub property: Ident,
        pub whens: Vec<WhenPropertyValue>,
    }

    impl ToTokens for WhenSwitchArgs {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let var_name = ident!("{}_args", self.property);

            let widget = &self.widget;
            let property = &self.property;
            let property_path = quote!(#widget::properties::#property);

            let index_var_name = ident!("{}_index", property);

            let when_var_names: Vec<_> = self.whens.iter().map(|w| ident!("{}{}", property, w.index)).collect();
            let when_var_inits = self.whens.iter().map(|w| match &w.value {
                PropertyValue::Args(a) => a.to_token_stream(),
                PropertyValue::Fields(f) => f.to_token_stream(),
                PropertyValue::Inherited => {
                    let wi = ident!("w{}", w.index);
                    quote! { #widget::when_defaults::#wi::#property() }
                }
            });

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
            let ns = {
                let name = &self.widget_name;
                quote!(#name::properties)
            };

            for priority in &Priority::all() {
                for property in &self.properties {
                    let ident = &property.ident;
                    let args_ident = &property.args_ident;

                    let set_args = if property.is_known {
                        quote!( #ns::#ident::set_args)
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
        pub is_known: bool,
        pub ident: Ident,
        pub args_ident: Ident,
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
        // arg var names
        pub args: Vec<Ident>,
    }
    impl ToTokens for NewCall {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let name = &self.widget_name;
            let new_token = if self.is_new_child { quote!(new_child) } else { quote!(new) };
            let args = &self.args;

            let call = quote!(#name::#new_token(node, #(#args),*));

            if self.is_new_child {
                tokens.extend(quote!(let node = #call;));
            } else {
                tokens.extend(call);
            }
        }
    }
}
