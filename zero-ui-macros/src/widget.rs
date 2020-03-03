use crate::util;
use crate::widget_new::BuiltPropertyKind;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::{parse::*, punctuated::Punctuated, *};

/// `widget!` implementation
#[allow(clippy::cognitive_complexity)]
pub fn expand_widget(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // arguments can be in two states:
    let args = parse_macro_input!(input as WidgetArgs);
    let mut input = match args {
        // 1 - Is in a recursive expansion that is including the widget_new! info
        // from the inherited widgets.
        WidgetArgs::IncludeInherits { inherits_todo, rest } => {
            let inherits: Vec<_> = inherits_todo.into_iter().map(|i| ident!("__{}", i)).collect();
            let inherit = &inherits[0]; //TODO: multiple inherits.
            let r = quote! { #inherit! { inherit #rest } };
            return r.into();
        }
        // 2 - Has collected the inherited widget_new! info or does not have any
        // the rest of this function expands the widget.
        WidgetArgs::Declare(input) => *input,
    };

    // we get the item level docs
    let (mut docs, attrs) = util::split_doc_other(&mut input.attrs);
    // end insert the header termination html because we will
    // generate custom sections to the item docs page.
    push_item_docs_close(&mut docs);

    let (export, pub_) = if input.export {
        (quote!(#[macro_export]), quote!(pub))
    } else {
        (quote!(), quote!())
    };

    let widget_name = input.ident;

    // Collect `new_child` and what properties are required by it.
    let new_child_properties;
    let new_child;
    if let Some(c) = input.new_child {
        let attrs = c.attrs;
        let child = c.child;
        let output = c.output;
        let block = c.block;
        let ps = c.properties;
        new_child = quote! {
            #(#attrs)*
            pub fn new_child(#child: impl zero_ui::core::UiNode, #(#ps: impl ps::#ps::Args),*) -> #output
            #block
        };
        new_child_properties = ps;
    } else {
        let fn_doc = doc!("Manually initializes a new [`{0}`](self) content.", widget_name);
        new_child = quote!(
            #fn_doc
            #[inline]
            pub fn new_child<C: zero_ui::core::UiNode>(child: C) -> C {
                zero_ui::core::default_new_widget_child(child)
            }
        );
        new_child_properties = vec![];
    };

    // Collect `new` and what properties are required by it.
    let new_properties;
    let new;
    if let Some(n) = input.new {
        let attrs = n.attrs;
        let child = n.child;
        let output = n.output;
        let block = n.block;
        let ps = n.properties;

        new = quote! {
            #(#attrs)*
            pub fn new(#child: impl zero_ui::core::UiNode, #(#ps: impl ps::#ps::Args),*) -> #output
            #block
        };
        new_properties = ps;
    } else {
        new_properties = vec![ident!["id"]];
        let fn_doc = doc!("Manually initializes a new [`{0}`](self).", widget_name);
        new = quote!(
            #fn_doc
            #[inline]
            pub fn new(child: impl zero_ui::core::UiNode, id: impl ps::id::Args) -> impl zero_ui::core::UiNode {
                zero_ui::core::default_new_widget(child, id)
            }
        );
    };

    let mut defined_props = HashSet::new();

    // flatten property declarations
    let mut dft_child: Vec<_> = input.default_child.into_iter().flat_map(|d| d.properties).collect();
    let mut dft_self: Vec<_> = input.default_self.into_iter().flat_map(|d| d.properties).collect();

    // Collect property info from local definitions:
    // 1 - Property use clauses and defaults.
    let mut use_props = vec![];
    let mut fn_prop_dfts = vec![];
    // 2 - Generate widget_new! property metadata.
    let mut built_child = vec![];
    let mut built_self = vec![];
    let mut built_child_docs = vec![];
    let mut built_self_docs = vec![];
    // 3 - Separate the property documentation. Each vector contains (DefaultBlockTarget, &mut PropertyDeclaration).
    let mut required_docs = vec![];
    let mut default_docs = vec![];
    let mut other_docs = vec![];

    let mut default_blocks = [
        (DefaultBlockTarget::Child, &mut dft_child),
        (DefaultBlockTarget::Self_, &mut dft_self),
    ];

    for (target, properties) in &mut default_blocks {
        let target = *target;
        for p in properties.iter_mut() {
            let (prop_docs, other_attrs) = util::split_doc_other(&mut p.attrs);

            if let Some(invalid) = other_attrs.into_iter().next() {
                abort!(invalid.span(), "only #[doc] attributes are allowed here")
            }

            let is_required = p.is_required();

            let ident = &p.ident;
            if !defined_props.insert(ident) {
                abort!(ident.span(), "property named `{}` already declared", ident);
            }

            // 1
            let mut unset = false;
            if let Some(dft) = &p.default_value {
                match dft {
                    PropertyDefaultValue::Args(args) => {
                        fn_prop_dfts.push(quote! {
                            #[inline]
                            pub fn #ident() -> impl ps::#ident::Args {
                                ps::#ident::args(#args)
                            }
                        });
                    }
                    PropertyDefaultValue::Fields(fields) => {
                        fn_prop_dfts.push(quote! {
                            #[inline]
                            pub fn #ident() -> impl ps::#ident::Args {
                                ps::ident::NamedArgs {
                                    _phantom: std::marker::PhantomData,
                                    #fields
                                }
                            }
                        });
                    }
                    PropertyDefaultValue::Unset => {
                        unset = true;
                    }
                    PropertyDefaultValue::Required => {}
                }
            }
            if !unset {
                // 1
                if let Some(maps_to) = &p.maps_to {
                    use_props.push(quote!(pub use super::#maps_to as #ident;))
                } else {
                    use_props.push(quote!(pub use super::#ident;))
                }

                // 2
                let (built, built_docs) = match target {
                    DefaultBlockTarget::Child => (&mut built_child, &mut built_child_docs),
                    DefaultBlockTarget::Self_ => (&mut built_self, &mut built_self_docs),
                };
                if is_required {
                    built.push(quote!(r #ident));
                } else if p.default_value.is_some() {
                    built.push(quote!(d #ident));
                } else {
                    built.push(quote!(l #ident));
                }
                built_docs.push(quote! {#(#prop_docs)*});

                // 3
                let docs = if is_required {
                    &mut required_docs
                } else if p.default_value.is_some() {
                    &mut default_docs
                } else {
                    &mut other_docs
                };
                push_property_docs(docs, target, ident, &p.maps_to, prop_docs);
            }
        }
    }

    // Collect property info from inherits:
    // 1 - Property use clauses and defaults.
    let mut i_use_props = vec![];
    let mut i_fn_prop_dfts = vec![];
    // 2 - Generate widget_new! property metadata.
    let mut i_built_child = vec![];
    let mut i_built_self = vec![];
    let mut i_built_child_docs = vec![];
    let mut i_built_self_docs = vec![];
    // 3 - Separate the property documentation. Each vector contains (DefaultBlockTarget, &mut PropertyDeclaration).
    let mut i_required_docs = vec![];
    let mut i_default_docs = vec![];
    let mut i_other_docs = vec![];

    let mut i_default_blocks = vec![];
    for inherit in &mut input.inherits {
        let widget_name = &inherit.ident;
        for child_prop in &mut inherit.default_child.properties {
            i_default_blocks.push((widget_name, DefaultBlockTarget::Child, child_prop));
        }
        for self_prop in &mut inherit.default_self.properties {
            i_default_blocks.push((widget_name, DefaultBlockTarget::Self_, self_prop));
        }
    }
    for (widget_name, target, prop) in i_default_blocks {
        let ident = &prop.ident;
        if !defined_props.insert(ident) {
            continue; // inherited property overridden
        }

        //1
        if prop.kind == BuiltPropertyKind::Default {
            i_fn_prop_dfts.push(quote! {
                #[inline]
                pub fn #ident() -> impl ps::#ident::Args {
                    #widget_name::df::#ident()
                }
            });
        }
        i_use_props.push(quote! {
            pub use super::#widget_name::ps::#ident;
        });

        //2
        let (built, built_docs) = match target {
            DefaultBlockTarget::Child => (&mut i_built_child, &mut i_built_child_docs),
            DefaultBlockTarget::Self_ => (&mut i_built_self, &mut i_built_self_docs),
        };
        match prop.kind {
            BuiltPropertyKind::Required => built.push(quote!(r #ident)),
            BuiltPropertyKind::Default => built.push(quote!(d #ident)),
            BuiltPropertyKind::Local => built.push(quote!(l #ident)),
        }
        let prop_docs = std::mem::replace(&mut prop.docs, Vec::default());
        built_docs.push(quote! {#(#prop_docs)*});

        //3
        let docs = match prop.kind {
            BuiltPropertyKind::Required => &mut i_required_docs,
            BuiltPropertyKind::Default => &mut i_default_docs,
            BuiltPropertyKind::Local => &mut i_other_docs,
        };

        push_inherited_property_docs(docs, target, ident, widget_name, prop_docs);
    }

    // validate property captures.
    if let Some(p) = new_child_properties.iter().find(|p| !defined_props.contains(p)) {
        abort!(p.span(), "`new_child` cannot capture undefined property `{}`", p);
    }
    if let Some(p) = new_properties.iter().find(|p| !defined_props.contains(p)) {
        abort!(p.span(), "`new` cannot capture undefined property `{}`", p);
    }

    // make property documentation sections.
    if !i_required_docs.is_empty() || !required_docs.is_empty() {
        push_docs_section_open(&mut docs, "required-properties", "Required properties");
        docs.extend(i_required_docs);
        docs.extend(required_docs);
        push_docs_section_close(&mut docs);
    }
    if !i_default_docs.is_empty() || !default_docs.is_empty() {
        push_docs_section_open(&mut docs, "provided-properties", "Provided properties");
        docs.extend(i_default_docs);
        docs.extend(default_docs);
        push_docs_section_close(&mut docs);
    }
    push_docs_section_open(&mut docs, "other-properties", "Other properties");
    docs.extend(i_other_docs);
    docs.extend(other_docs);
    push_docs_all_other_props(&mut docs);
    push_docs_section_close(&mut docs);

    // ident of a doc(hidden) macro that is the actual macro implementation.
    // This macro is needed because we want to have multiple match arms, but
    // the widget macro needs to take $($tt:tt)*.
    let inner_wgt_name = ident!("__{}", widget_name);

    let widget_new_tokens = quote! {
        m #widget_name
        c { #(#i_built_child,)* #(#built_child),* }
        s { #(#i_built_self,)* #(#built_self),* }
        n (#(#new_child_properties),*) (#(#new_properties),*)
    };

    let widget_inherit_tokens = quote! {
        m #widget_name
        c { #(#i_built_child,)* #(#built_child_docs #built_child),* }
        s { #(#i_built_self,)* #(#built_self_docs #built_self),* }
    };

    let r = quote! {
        // widget macro. Is hidden until macro 2.0 comes around.
        // For now this simulates name-spaced macros because we
        // require a use widget_mod for this to work.
        #[doc(hidden)]
        #(#attrs)*
        #export
        macro_rules! #widget_name {
            ($($input:tt)*) => {
                // call the new variant.
                #inner_wgt_name!{new $($input)*}
            };
        }

        #[doc(hidden)]
        #export
        macro_rules! #inner_wgt_name {
            // new widget instance.
            (new $($input:tt)*) => {
                widget_new! {
                    #widget_new_tokens
                    i { $($input)* }
                }
            };
            // recursive callback to widget! but this time including
            // the widget_new! info from this widget in an inherit block.
            (inherit $($widget_declaration:tt)*) => {
                widget! {
                    inherit {
                        #widget_inherit_tokens
                    }

                    $($widget_declaration)*
                }
            };
        }

        // the widget module, also the public face of the widget in the documentation.
        #(#docs)*
        #pub_ mod #widget_name {
            #[doc(hidden)]
            pub use super::*;

            #new_child
            #new

            // Properties used in widget.
            #[doc(hidden)]
            pub mod ps {
                #(#i_use_props)*
                #(#use_props)*
            }

            // Default values from the widget.
            #[doc(hidden)]
            pub mod df {
                use super::*;

                #(#i_fn_prop_dfts)*
                #(#fn_prop_dfts)*
            }
        }
    };

    r.into()
}

fn push_item_docs_close(docs: &mut Vec<Attribute>) {
    docs.push(doc!(
        "\n</div><style>span.wgprop p {{ display: inline; margin-left:-1ch; }}</style><script>{}</script>",
        include_str!("widget_docs_ext.js")
    )); // finish item level docs.
}

fn push_docs_section_open(docs: &mut Vec<Attribute>, id: &str, title: &str) {
    docs.push(doc!(
        r##"<h2 id="{0}" class="small-section-header">{1}<a href="#{0}" class="anchor"></a></h2>
        <div class="methods" style="display: block;">"##,
        id,
        title
    ));
}

fn push_property_docs(
    docs: &mut Vec<Attribute>,
    target: DefaultBlockTarget,
    ident: &Ident,
    maps_to: &Option<Ident>,
    pdocs: Vec<Attribute>,
) {
    docs.push(doc!(
        r##"<h3 id="wgproperty.{0}" class="method"><code id='{0}.v'><a href='#wgproperty.{0}' class='fnname'>{0}</a> -> <span title="applied to {1}">{1}</span>.<span class='wgprop'>"##,
        ident,
        match target {
            DefaultBlockTarget::Self_ => "self",
            DefaultBlockTarget::Child => "child",
        },
    ));
    docs.push(doc!("\n[<span class='mod'>{0}</span>]({0})\n", maps_to.as_ref().unwrap_or(&ident),));
    docs.push(doc!("<ul style='display:none;'></ul></span></code></h3>"));

    if !pdocs.is_empty() {
        docs.push(doc!("<div class='docblock'>\n"));
        docs.extend(pdocs);
        docs.push(doc!("\n</div>"));
    }
}

fn push_inherited_property_docs(
    docs: &mut Vec<Attribute>,
    target: DefaultBlockTarget,
    ident: &Ident,
    source_widget: &Ident,
    pdocs: Vec<Attribute>,
) {
    docs.push(doc!(
        r##"<h3 id="wgproperty.{0}" class="method"><code id='{0}.v'><a href='#wgproperty.{0}' class='fnname'>{0}</a> -> <span title="applied to {1}">{1}</span>.<span class='wgprop'>"##,
        ident,
        match target {
            DefaultBlockTarget::Self_ => "self",
            DefaultBlockTarget::Child => "child",
        },
    ));
    docs.push(doc!(
        "\n[<span class='mod' data-inherited>{0}</span>](self::{1})\n",
        ident,
        source_widget
    ));
    docs.push(doc!("<ul style='display:none;'></ul></span></code></h3>"));

    docs.push(doc!("<div class='docblock'>\n"));
    docs.extend(pdocs);
    docs.push(doc!("\nInherited from [`{0}`](self::{0}).", source_widget));
    docs.push(doc!("\n</div>"));
}

fn push_docs_all_other_props(docs: &mut Vec<Attribute>) {
    docs.push(doc!(r##"<h3 id="wgall" class="method"><code><a href="#wgall" class="fnname">*</a> -> <span title="applied to self">self</span>.<span class='wgprop'>"##));
    docs.push(doc!("\n[<span class='mod'>*</span>](zero_ui::properties)\n"));
    docs.push(doc!(r##"<ul style='display:none;'></ul></span></code></h3><div class="docblock">Widgets are open-ended, all properties are accepted.</div>"##));
}

fn push_docs_section_close(docs: &mut Vec<Attribute>) {
    docs.push(doc!("</div>"));
}

pub mod keyword {
    syn::custom_keyword!(child);
    syn::custom_keyword!(required);
    syn::custom_keyword!(unset);
    syn::custom_keyword!(when);
    syn::custom_keyword!(new_child);
    syn::custom_keyword!(new);
    syn::custom_keyword!(inherit);
}

enum WidgetArgs {
    IncludeInherits {
        inherits_todo: Punctuated<Ident, Token![+]>,
        rest: TokenStream,
    },
    Declare(Box<WidgetInput>),
}
impl Parse for WidgetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut inherits = vec![];
        while input.peek(keyword::inherit) {
            input.parse::<keyword::inherit>().expect(util::NON_USER_ERROR);
            let inner = util::non_user_braced(input);
            inherits.push(inner.parse().expect(util::NON_USER_ERROR));
        }

        let attrs = Attribute::parse_outer(input)?;

        let export = input.peek(Token![pub]);
        if export {
            input.parse::<Token![pub]>()?;
        }

        let ident = input.parse()?;
        let inherits_todo = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Punctuated::parse_separated_nonempty(input)?
        } else {
            Punctuated::new()
        };
        input.parse::<Token![;]>()?;

        if !inherits_todo.is_empty() {
            let rest: TokenStream = input.parse().unwrap();
            let pub_ = if export { quote!(pub) } else { quote!() };
            return Ok(WidgetArgs::IncludeInherits {
                inherits_todo,
                rest: quote! {
                    #(#attrs)*
                    #pub_ #ident;
                    #rest
                },
            });
        }

        let mut default_child = vec![];
        let mut default_self = vec![];
        let mut whens = vec![];
        let mut new_child = None;
        let mut new = None;
        while !input.is_empty() {
            let mut attrs = Attribute::parse_outer(input)?;

            let lookahead = input.lookahead1();

            if attrs.is_empty() && lookahead.peek(Token![default]) {
                let block: DefaultBlock = input.parse()?;
                match block.target {
                    DefaultBlockTarget::Self_ => {
                        default_self.push(block);
                    }
                    DefaultBlockTarget::Child => {
                        default_child.push(block);
                    }
                }
            } else if lookahead.peek(keyword::when) {
                let mut when: WhenBlock = input.parse()?;
                // extend outer with inner
                attrs.extend(when.attrs.drain(..));
                when.attrs = attrs;

                whens.push(when);
            } else if lookahead.peek(Token![fn]) {
                let mut fn_: NewFn = input.parse()?;
                attrs.extend(fn_.attrs.drain(..));
                fn_.attrs = attrs;

                match fn_.target {
                    DefaultBlockTarget::Self_ => {
                        if new.is_some() {
                            return Err(Error::new(fn_.ident.span(), "function `new` can only be defined once"));
                        }
                        new = Some(fn_);
                    }
                    DefaultBlockTarget::Child => {
                        if new_child.is_some() {
                            return Err(Error::new(fn_.ident.span(), "function `new_child` can only be defined once"));
                        }
                        new_child = Some(fn_);
                    }
                }
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(WidgetArgs::Declare(Box::new(WidgetInput {
            attrs,
            export,
            ident,
            inherits,
            default_child,
            default_self,
            whens,
            new_child,
            new,
        })))
    }
}

struct WidgetInput {
    attrs: Vec<Attribute>,
    export: bool,
    ident: Ident,
    inherits: Vec<InheritBlock>,
    default_child: Vec<DefaultBlock>,
    default_self: Vec<DefaultBlock>,
    whens: Vec<WhenBlock>,
    new_child: Option<NewFn>,
    new: Option<NewFn>,
}

struct InheritBlock {
    ident: Ident,
    default_child: InheritedDefaultBlock,
    default_self: InheritedDefaultBlock,
}

impl Parse for InheritBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        use crate::widget_new::keyword;

        input.parse::<keyword::m>().expect(util::NON_USER_ERROR);
        let ident = input.parse().expect(util::NON_USER_ERROR);

        input.parse::<keyword::c>().expect(util::NON_USER_ERROR);
        let default_child = input.parse().expect(util::NON_USER_ERROR);

        input.parse::<keyword::s>().expect(util::NON_USER_ERROR);
        let default_self = input.parse().expect(util::NON_USER_ERROR);

        Ok(InheritBlock {
            ident,
            default_child,
            default_self,
        })
    }
}

struct InheritedDefaultBlock {
    properties: Punctuated<InheritedProperty, Token![,]>,
}
impl Parse for InheritedDefaultBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        braced!(inner in input);
        let properties = Punctuated::parse_terminated(&inner)?;
        Ok(InheritedDefaultBlock { properties })
    }
}

struct InheritedProperty {
    docs: Vec<Attribute>,
    kind: BuiltPropertyKind,
    ident: Ident,
}

impl Parse for InheritedProperty {
    fn parse(input: ParseStream) -> Result<Self> {
        let docs = Attribute::parse_outer(input)?;
        let kind = input.parse()?;
        let ident = input.parse()?;
        Ok(InheritedProperty { docs, kind, ident })
    }
}

#[derive(Debug)]
pub struct DefaultBlock {
    pub target: DefaultBlockTarget,
    pub properties: Vec<PropertyDeclaration>,
}
impl Parse for DefaultBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![default]>()?;

        let inner;
        parenthesized!(inner in input);
        let target = inner.parse()?;

        let inner;
        braced!(inner in input);
        let mut properties = vec![];
        while !inner.is_empty() {
            properties.push(inner.parse()?);
        }

        Ok(DefaultBlock { target, properties })
    }
}

pub struct WhenBlock {
    attrs: Vec<Attribute>,
    pub condition: Expr,
    pub properties: Vec<PropertyAssign>,
}
impl Parse for WhenBlock {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<keyword::when>()?;

        let condition = input.parse()?;

        let inner;
        braced!(inner in input);

        let attrs = Attribute::parse_inner(input)?;

        let mut properties = vec![];
        while !inner.is_empty() {
            properties.push(inner.parse()?);
        }

        Ok(WhenBlock {
            attrs,
            condition,
            properties,
        })
    }
}
impl ToTokens for WhenBlock {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let condition = &self.condition;
        let properties = &self.properties;

        tokens.extend(quote! {
            when(#condition) {
                #(#properties)*
            }
        })
    }
}

pub struct NewFn {
    attrs: Vec<Attribute>,
    target: DefaultBlockTarget,
    ident: Ident,
    child: Ident,
    properties: Vec<Ident>,
    output: Type,
    block: Block,
}
impl Parse for NewFn {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![fn]>()?;

        let lookahread = input.lookahead1();

        let target;
        let ident;
        if lookahread.peek(keyword::new) {
            ident = input.parse()?;
            target = DefaultBlockTarget::Self_;
        } else if lookahread.peek(keyword::new_child) {
            input.parse::<keyword::new_child>()?;
            ident = input.parse()?;
            target = DefaultBlockTarget::Child;
        } else {
            return Err(lookahread.error());
        };

        let inner;
        parenthesized!(inner in input);
        let args: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(&inner)?;
        if args.is_empty() {
            return Err(Error::new(input.span(), "expected at least one input (child)"));
        }
        let mut properties: Vec<_> = args.into_iter().collect();
        let child = properties.remove(0);

        input.parse::<Token![->]>()?;
        let output = input.parse()?;

        let block = input.parse()?;

        let attrs = vec![];

        Ok(NewFn {
            attrs,
            ident,
            target,
            child,
            properties,
            output,
            block,
        })
    }
}

#[derive(Debug)]
pub struct PropertyDeclaration {
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub maps_to: Option<Ident>,
    pub default_value: Option<PropertyDefaultValue>,
}
impl Parse for PropertyDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let ident = input.parse()?;
        let mut maps_to = None;
        let mut default_value = None;

        let lookahead = input.lookahead1();
        if lookahead.peek(Token![->]) {
            // is property alias.
            input.parse::<Token![->]>()?;
            maps_to = Some(input.parse()?);

            let lookahead = input.lookahead1();
            if lookahead.peek(Token![:]) {
                // alias does not need default value but this one has it too.
                input.parse::<Token![:]>()?;
                default_value = Some(input.parse()?);
            } else if lookahead.peek(Token![;]) {
                // no value and added the required ;.
                input.parse::<Token![;]>()?;
            } else {
                // invalid did not finish the declaration with ;.
                return Err(lookahead.error());
            }
        } else if lookahead.peek(Token![:]) {
            // is not property alias but has default value.
            input.parse::<Token![:]>()?;
            default_value = Some(input.parse()?);
        } else {
            // invalid, no alias and no value.
            return Err(lookahead.error());
        }

        Ok(PropertyDeclaration {
            attrs,
            ident,
            maps_to,
            default_value,
        })
    }
}
impl ToTokens for PropertyDeclaration {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ts = match (&self.ident, &self.maps_to, &self.default_value) {
            (ident, None, Some(default_value)) => quote!(#ident: #default_value;),
            (ident, Some(maps_to), Some(default_value)) => quote!(#ident -> #maps_to: #default_value;),
            (ident, Some(maps_to), None) => quote!(#ident -> #maps_to;),
            _ => unreachable!(),
        };
        tokens.extend(ts)
    }
}
impl PropertyDeclaration {
    pub fn is_required(&self) -> bool {
        self.default_value.as_ref().map(|v| v.is_required()).unwrap_or_default()
    }
}

pub struct PropertyAssign {
    pub ident: Ident,
    pub value: PropertyValue,
}
impl Parse for PropertyAssign {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;
        Ok(PropertyAssign { ident, value })
    }
}
impl ToTokens for PropertyAssign {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let value = &self.value;
        tokens.extend(quote!(#ident: #value;))
    }
}

#[derive(Debug)]
pub enum PropertyDefaultValue {
    Fields(Punctuated<FieldValue, Token![,]>),
    Args(Punctuated<Expr, Token![,]>),
    Unset,
    Required,
}
impl Parse for PropertyDefaultValue {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Brace) {
            use syn::parse::discouraged::Speculative;

            let fields_fork = input.fork();
            let inner;
            braced!(inner in fields_fork);
            if let Ok(fields) = Punctuated::parse_terminated(&inner) {
                input.advance_to(&fields_fork);
                input.parse::<Token![;]>()?;

                Ok(PropertyDefaultValue::Fields(fields))
            } else if let Ok(args) = Punctuated::parse_separated_nonempty(&input) {
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                input.parse::<Token![;]>()?;

                Ok(PropertyDefaultValue::Args(args))
            } else {
                Err(Error::new(inner.span(), "expected one of: args, named args"))
            }
        } else if input.peek(keyword::unset) && input.peek2(Token![!]) {
            input.parse::<keyword::unset>()?;
            input.parse::<Token![!]>()?;
            input.parse::<Token![;]>()?;
            Ok(PropertyDefaultValue::Unset)
        } else if input.peek(keyword::required) && input.peek2(Token![!]) {
            input.parse::<keyword::required>()?;
            input.parse::<Token![!]>()?;
            input.parse::<Token![;]>()?;
            Ok(PropertyDefaultValue::Required)
        } else if let Ok(args) = Punctuated::parse_separated_nonempty(input) {
            input.parse::<Token![;]>()?;
            Ok(PropertyDefaultValue::Args(args))
        } else {
            Err(Error::new(input.span(), "expected one of: args, named args, `required!`, `unset!`"))
        }
    }
}
impl ToTokens for PropertyDefaultValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            PropertyDefaultValue::Fields(f) => tokens.extend(quote!({#f})),
            PropertyDefaultValue::Args(a) => a.to_tokens(tokens),
            PropertyDefaultValue::Unset => tokens.extend(quote!(unset!)),
            PropertyDefaultValue::Required => tokens.extend(quote!(required!)),
        }
    }
}
impl PropertyDefaultValue {
    pub fn is_required(&self) -> bool {
        match self {
            PropertyDefaultValue::Required => true,
            _ => false,
        }
    }
}

pub enum PropertyValue {
    /// Named arguments.
    Fields(Punctuated<FieldValue, Token![,]>),
    /// Unnamed arguments.
    Args(Punctuated<Expr, Token![,]>),
    /// unset!.
    Unset,
}
impl Parse for PropertyValue {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Brace) {
            use syn::parse::discouraged::Speculative;

            let fields_fork = input.fork();
            let inner;
            braced!(inner in fields_fork);
            if let Ok(fields) = Punctuated::parse_terminated(&inner) {
                input.advance_to(&fields_fork);
                input.parse::<Token![;]>()?;

                Ok(PropertyValue::Fields(fields))
            } else if let Ok(args) = Punctuated::parse_separated_nonempty(&input) {
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                input.parse::<Token![;]>()?;

                Ok(PropertyValue::Args(args))
            } else {
                Err(Error::new(inner.span(), "expected one of: args, named args"))
            }
        } else if input.peek(keyword::unset) && input.peek2(Token![!]) {
            input.parse::<keyword::unset>()?;
            input.parse::<Token![!]>()?;
            input.parse::<Token![;]>()?;
            Ok(PropertyValue::Unset)
        } else if let Ok(args) = Punctuated::parse_separated_nonempty(input) {
            input.parse::<Token![;]>()?;
            Ok(PropertyValue::Args(args))
        } else {
            Err(Error::new(input.span(), "expected one of: args, named args, `unset!`, `todo!(..)`"))
        }
    }
}
impl ToTokens for PropertyValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            PropertyValue::Fields(f) => tokens.extend(quote!({#f})),
            PropertyValue::Args(a) => a.to_tokens(tokens),
            PropertyValue::Unset => tokens.extend(quote!(unset!)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultBlockTarget {
    Self_,
    Child,
}
impl Parse for DefaultBlockTarget {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![self]) {
            input.parse::<Token![self]>()?;

            Ok(DefaultBlockTarget::Self_)
        } else if lookahead.peek(keyword::child) {
            input.parse::<keyword::child>()?;

            Ok(DefaultBlockTarget::Child)
        } else {
            Err(lookahead.error())
        }
    }
}
impl ToTokens for DefaultBlockTarget {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            DefaultBlockTarget::Self_ => tokens.extend(quote!(self)),
            DefaultBlockTarget::Child => tokens.extend(quote!(child)),
        }
    }
}
