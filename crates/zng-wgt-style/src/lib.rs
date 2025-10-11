#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Style building blocks.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]

use zng_app::widget::builder::{Importance, PropertyId};
use zng_wgt::prelude::*;

#[doc(hidden)]
pub use zng_wgt::prelude::clmv as __clmv;

use std::sync::Arc;
use std::{fmt, ops};

/// Represents a set of properties that can be applied to any styleable widget.
///
/// Style can be instantiated using the same syntax as any widget, but it produces a [`StyleBuilder`]
/// instance instead of a widget. Widgets that have [`StyleMix<P>`] can be modified using properties
/// defined in a style, the properties are dynamically spliced into each widget instance.
///
/// # Extend/Replace
///
/// Style instances extend the contextual style by default, meaning all properties set on the style are inserted over
/// the parent style, so properties set on the contextual style that are not reset in the new style are retained. You
/// can set [`replace`](fn@replace) on a style to `true` to fully remove all contextual properties and only use the
/// new style properties.
///
/// ## Named Styles
///
/// Styleable widgets have one contextual style by default, usually defined by [`impl_style_fn!`] the `style_fn` property
/// implements the extend/replace mixing of the style, tracked by a `STYLE_FN_VAR`.
///
/// This same pattern can be used to define alternate named styles, these styles set [`named_style_fn`](fn@named_style_fn) to another
/// context variable that defines the style context, on widget instantiation this other context will be used instead of the default one.
/// You can use [`impl_named_style_fn!`] to declare most of the boilerplate.
///
/// # Inherit Style
///
/// Note that you can declare a custom style *widget* using the same inheritance mechanism of normal widgets, as long
/// as they build to [`StyleBuilder`]. This is different from the *extend/replace* mechanism as it operates on the style
/// type, not the instances. A style that inherits a `named_style_fn` will not inherit that named context either, each named
/// context property is strongly associated with a single style type only.
#[widget($crate::Style)]
pub struct Style(zng_app::widget::base::NonWidgetBase);
impl Style {
    /// Build the style.
    pub fn widget_build(&mut self) -> StyleBuilder {
        StyleBuilder::from_builder(self.widget_take())
    }
}

/// Fully replace the contextual style.
///
/// This is not enabled by default, if set to `true` the contextual style properties are removed.
#[property(WIDGET, default(false), widget_impl(Style))]
pub fn replace(wgt: &mut WidgetBuilding, replace: impl IntoValue<bool>) {
    let _ = replace;
    wgt.expect_property_capture();
}

/// Set in the default properties of a named style to define the contextual variable for that style.
///
/// During widget instantiation, if this is set by default in a style the contextual style is used as the *defaults* and only the
/// properties set on the style instance *replace* them.
///
/// This property is part of the *named styles* pattern, see [`impl_named_style_fn!`] for more details.
///
/// Note that this property expects a `ContextVar<StyleFn>` as a value, not a variable directly, it will also only work if
/// set in the default properties of a style type.
#[property(WIDGET, widget_impl(Style))]
pub fn named_style_fn(wgt: &mut WidgetBuilding, name: impl IntoValue<NamedStyleVar>) {
    let _ = name;
    wgt.expect_property_capture();
}

/// Represents a `ContextVar<StyleFn>` that defines a named style.
///
/// See [`named_style_fn`](fn@named_style_fn) for more details.
#[derive(Clone, Copy)]
pub struct NamedStyleVar(ContextVar<StyleFn>);
impl fmt::Debug for NamedStyleVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NamedStyleVar").finish_non_exhaustive()
    }
}
impl PartialEq for NamedStyleVar {
    fn eq(&self, other: &Self) -> bool {
        self.0.var_eq(&other.0)
    }
}
impl_from_and_into_var! {
    fn from(var: ContextVar<StyleFn>) -> NamedStyleVar {
        NamedStyleVar(var)
    }
}
impl IntoVar<StyleFn> for NamedStyleVar {
    fn into_var(self) -> Var<StyleFn> {
        self.0.into_var()
    }
}
impl ops::Deref for NamedStyleVar {
    type Target = ContextVar<StyleFn>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Styleable widget mixin.
///
/// Widgets that inherit this mixin have a `style_fn` property that can be set to a [`style_fn!`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
///
/// The style mixin drastically affects the widget build process, only the `style_fn` and `when` condition
/// properties that affects it are instantiated with the widget, all the other properties and intrinsic nodes are instantiated
/// on init, after the style is generated.
///
/// Widgets that inherit this mixin must call [`style_intrinsic`] in their own `widget_intrinsic`, if the call is missing
/// the widget will log an error on instantiation. You can use the [`impl_style_fn!`] macro to generate the style var and property.
///
/// [`style_intrinsic`]: StyleMix::style_intrinsic
#[widget_mixin]
pub struct StyleMix<P>(P);
impl<P: WidgetImpl> StyleMix<P> {
    fn widget_intrinsic(&mut self) {
        self.base()
            .widget_builder()
            .set_custom_build(|b| StyleMix::<()>::custom_build(b, None));
    }

    /// Setup the style build.
    pub fn style_intrinsic(&mut self, style_var: ContextVar<StyleFn>, style_fn: PropertyId) {
        self.base()
            .widget_builder()
            .set_custom_build(move |b| StyleMix::<()>::custom_build(b, Some((style_var, style_fn))));
    }
}
impl<P> StyleMix<P> {
    /// The custom build that is set on intrinsic by the mixin.
    pub fn custom_build(mut wgt: WidgetBuilder, cfg: Option<(ContextVar<StyleFn>, PropertyId)>) -> UiNode {
        let (style_var, style_id) = cfg.unwrap_or_else(|| {
            tracing::error!("missing `style_intrinsic` in `{}`", wgt.widget_type().path);
            (MISSING_STYLE_VAR, property_id!(self::missing_style_fn))
        });

        // 1 - "split_off" the property `style_fn`
        //     this moves the properties and any `when` that affects them to a new widget builder.
        let mut style_builder = WidgetBuilder::new(wgt.widget_type());
        wgt.split_off([style_id], &mut style_builder);

        // 2 - build a "mini widget" that is only the intrinsic default style var,
        //     `style_fn` property and when condition properties that affect `style_fn`.

        #[cfg(feature = "trace_widget")]
        wgt.push_build_action(|wgt| {
            // avoid double trace as the style builder already inserts a widget tracer.
            wgt.disable_trace_widget();
        });

        let mut wgt = Some(wgt);
        style_builder.push_build_action(move |b| {
            // 3 - The actual style_node and builder is a child of the "mini widget".

            let style = b.capture_var::<StyleFn>(style_id).unwrap_or_else(|| const_var(StyleFn::nil()));

            b.set_child(style_node(UiNode::nil(), wgt.take().unwrap(), style_var, style));
        });
        // 4 - Build the "mini widget"
        style_builder.build()
    }
}

#[doc(hidden)]
pub mod __impl_style_context_util {
    pub use pastey::paste;
    pub use zng_wgt::prelude::{IntoUiNode, IntoVar, UiNode, context_var, property};
}

/// Implements the contextual `STYLE_FN_VAR` and `style_fn`.
///
/// This is a helper for [`StyleMix<P>`](struct@StyleMix) implementers, see the `zng::style` module level
/// documentation for more details.
#[macro_export]
macro_rules! impl_style_fn {
    ($Widget:path, $DefaultStyle:path) => {
        $crate::__impl_style_context_util::paste! {
            $crate::__impl_style_context_util::context_var! {
                /// Contextual style variable.
                ///
                /// Use [`style_fn`](fn@style_fn) to set.
                ///
                #[doc = "Is `" $DefaultStyle "!` by default."]
                pub static STYLE_FN_VAR: $crate::StyleFn = $crate::style_fn!(|_| $DefaultStyle!());
            }
        }

        /// Extends or replaces the widget style.
        ///
        /// Properties and `when` conditions in the style are applied to the widget. Style instances extend the contextual style
        /// by default, you can set `replace` on a style to `true` to fully replace.
        #[$crate::__impl_style_context_util::property(WIDGET, default($crate::StyleFn::nil()), widget_impl($Widget))]
        pub fn style_fn(
            child: impl $crate::__impl_style_context_util::IntoUiNode,
            style_fn: impl $crate::__impl_style_context_util::IntoVar<$crate::StyleFn>,
        ) -> $crate::__impl_style_context_util::UiNode {
            $crate::with_style_fn(child, STYLE_FN_VAR, style_fn)
        }
    };
}

/// Implements a `NAMED_STYLE_FN_VAR`, `named_style_fn` and `NamedStyle!` items.
///
/// This is a helper for declaring *named styles* that can be modified contextually, just like the default style.
///
/// # Examples
///
/// The example bellow declares a `FooStyle` manually, this is a normal definition for a named style. This macro generates
/// a `foo_style_fn` property and a `FOO_STYLE_FN_VAR` context var. Note that the manual style implementation must set the
/// [`named_style_fn`](fn@named_style_fn), otherwise the style will not inherit from the correct *name*.
///
/// ```
/// # macro_rules! example { () => {
/// /// Foo style.
/// #[widget($crate::FooStyle)]
/// pub struct FooStyle(Style);
/// impl FooStyle {
///     fn widget_intrinsic(&mut self) {
///         widget_set! {
///             self;
///             style_fn_var = FOO_STYLE_FN_VAR;
///
///             // .. style properties here
///         }
///     }
/// }
/// impl_named_style_fn!(foo, FooStyle);
/// # };}
/// ```
#[macro_export]
macro_rules! impl_named_style_fn {
    ($name:ident, $NamedStyle:ty) => {
        $crate::__impl_style_context_util::paste! {
            $crate::__impl_style_context_util::context_var! {
                /// Contextual style variable.
                ///
                #[doc = "Use [`" $name "_style_fn`](fn@" $name "_style_fn) to set."]
                pub static [<$name:upper _STYLE_FN_VAR>]: $crate::StyleFn = $crate::style_fn!(|_| $NamedStyle!());
            }

            #[doc = "Extends or replaces the [`" $NamedStyle "!`](struct@" $NamedStyle ")."]
            ///
            /// Properties and `when` conditions set here are applied to widgets using the style.
            ///
            /// Note that style instances extend the contextual style by default,
            /// you can set `replace` on a style to `true` to fully replace.
            #[$crate::__impl_style_context_util::property(WIDGET, default($crate::StyleFn::nil()))]
            pub fn [<$name _style_fn>](
                child: impl $crate::__impl_style_context_util::IntoUiNode,
                style_fn: impl $crate::__impl_style_context_util::IntoVar<$crate::StyleFn>,
            ) -> $crate::__impl_style_context_util::UiNode {
                $crate::with_style_fn(child, [<$name:upper _STYLE_FN_VAR>], style_fn)
            }
        }
    };
}

/// Helper for declaring the `style_fn` property.
///
/// The [`impl_style_fn!`] and [`impl_named_style_fn!`] macros uses this function as the implementation of `style_fn`.
pub fn with_style_fn(child: impl IntoUiNode, style_context: ContextVar<StyleFn>, style: impl IntoVar<StyleFn>) -> UiNode {
    with_context_var(
        child,
        style_context,
        merge_var!(style_context, style.into_var(), |base, over| {
            base.clone().with_extend(over.clone())
        }),
    )
}

fn style_node(child: UiNode, widget_builder: WidgetBuilder, style_var: ContextVar<StyleFn>, captured_style: Var<StyleFn>) -> UiNode {
    let style_vars = [style_var.into_var(), captured_style];
    let mut style_fn_var_styles = vec![];
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            // the final style builder
            let mut style_builder = StyleBuilder::default();
            for var in &style_vars {
                // each style var is subscribed and extend/replaces the previous
                WIDGET.sub_var(var);

                if let Some(mut style) = var.get().call(&StyleArgs {}) {
                    // style var was set

                    let named_style_fn = property_id!(named_style_fn);
                    if let Some(p) = style.builder.property(named_style_fn) {
                        if p.importance != StyleBuilder::WIDGET_IMPORTANCE {
                            tracing::warn!("ignoring `named_style_fn` not set as default")
                        } else {
                            // style is *named*, the contextual named style is used, only the items explicitly set
                            // on the style override the contextual named style.

                            let named_style = style
                                .builder
                                .capture_value::<NamedStyleVar>(named_style_fn)
                                .unwrap()
                                .current_context();

                            if let Some(mut from) = named_style.get().call(&StyleArgs {}) {
                                // contextual named style is set
                                let _ = from.builder.capture_value::<NamedStyleVar>(named_style_fn); // cleanup capture-only property

                                // only override instance set properties/whens

                                from.extend_named(style);
                                style = from;
                            }

                            // subscribe to the contextual named style
                            let handle = named_style.subscribe(UpdateOp::Update, WIDGET.id());
                            style_fn_var_styles.push((named_style, handle));
                        }
                    }

                    // extend/replace
                    style_builder.extend(style);
                }
            }

            if !style_builder.is_empty() {
                // apply style items and build actual widget
                let mut builder = widget_builder.clone();
                builder.extend(style_builder.into_builder());
                *c.node() = builder.default_build();
            } else {
                // no styles set, just build widget directly
                *c.node() = widget_builder.clone().default_build();
            }
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
            style_fn_var_styles.clear();
        }
        UiNodeOp::Update { .. } => {
            if style_vars.iter().any(|v| v.is_new()) || style_fn_var_styles.iter().any(|(n, _)| n.is_new()) {
                WIDGET.reinit();
                WIDGET.update_info().layout().render();
                c.delegated();
            }
        }
        _ => {}
    })
}

/// Represents a style instance.
///
/// Use the [`Style!`] *widget* to declare.
///
/// [`Style!`]: struct@Style
#[derive(Debug, Clone)]
pub struct StyleBuilder {
    builder: WidgetBuilder,
    replace: bool,
}
impl Default for StyleBuilder {
    fn default() -> Self {
        Self {
            builder: WidgetBuilder::new(Style::widget_type()),
            replace: false,
        }
    }
}
impl StyleBuilder {
    /// Importance of style properties set by default in style widgets.
    ///
    /// Is `Importance::WIDGET - 10`.
    pub const WIDGET_IMPORTANCE: Importance = Importance(Importance::WIDGET.0 - 10);

    /// Importance of style properties set in style instances.
    ///
    /// Is `Importance::INSTANCE - 10`.
    pub const INSTANCE_IMPORTANCE: Importance = Importance(Importance::INSTANCE.0 - 10);

    /// Negative offset on the position index of style properties.
    ///
    /// Is `1`.
    pub const POSITION_OFFSET: u16 = 1;

    /// New style from a widget builder.
    ///
    /// The importance and position index of properties are adjusted,
    /// any custom build or widget build action is ignored.
    pub fn from_builder(mut wgt: WidgetBuilder) -> StyleBuilder {
        wgt.clear_build_actions();
        wgt.clear_custom_build();
        let replace = wgt.capture_value_or_default(property_id!(self::replace));
        for p in wgt.properties_mut() {
            *p.importance = match *p.importance {
                Importance::WIDGET => StyleBuilder::WIDGET_IMPORTANCE,
                Importance::INSTANCE => StyleBuilder::INSTANCE_IMPORTANCE,
                other => other,
            };
            p.position.index = p.position.index.saturating_sub(Self::POSITION_OFFSET);
        }
        StyleBuilder { builder: wgt, replace }
    }

    /// Unwrap the style dynamic widget.
    pub fn into_builder(self) -> WidgetBuilder {
        self.builder
    }

    /// Override or replace `self` with `other`.
    pub fn extend(&mut self, other: StyleBuilder) {
        if other.is_replace() {
            *self = other;
        } else {
            self.builder.extend(other.builder);
        }
    }

    /// Override `self` with items set in the instance of `other` if both are the same type. Otherwise replace `self` with `other`.
    ///
    /// `self` is the [`named_style_fn`] and `other` is the style instance set in `style_fn`.
    ///
    /// [`named_style_fn`]: fn@named_style_fn
    pub fn extend_named(&mut self, other: StyleBuilder) {
        if self.builder.widget_type() != other.builder.widget_type() {
            *self = other;
        } else {
            self.builder.extend_important(other.builder, Self::INSTANCE_IMPORTANCE);
        }
    }

    /// if the style removes all contextual properties.
    pub fn is_replace(&self) -> bool {
        self.replace
    }

    /// If the style does nothing.
    pub fn is_empty(&self) -> bool {
        !self.builder.has_properties() && !self.builder.has_whens() && !self.builder.has_unsets()
    }
}
impl From<StyleBuilder> for WidgetBuilder {
    fn from(t: StyleBuilder) -> Self {
        t.into_builder()
    }
}
impl From<WidgetBuilder> for StyleBuilder {
    fn from(p: WidgetBuilder) -> Self {
        StyleBuilder::from_builder(p)
    }
}
impl_from_and_into_var! {
    /// Singleton.
    fn from(style: StyleBuilder) -> StyleFn {
        StyleFn::singleton(style)
    }
}

/// Arguments for [`StyleFn`] closure.
///
/// Empty struct, there are no style args in the current release, this struct is declared so that if
/// args may be introduced in the future with minimal breaking changes.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct StyleArgs {}

/// Boxed shared closure that generates a style instance for a given widget context.
///
/// You can also use the [`style_fn!`] macro to instantiate.
#[derive(Clone)]
pub struct StyleFn(Option<Arc<dyn Fn(&StyleArgs) -> Option<StyleBuilder> + Send + Sync>>);
impl Default for StyleFn {
    fn default() -> Self {
        Self::nil()
    }
}
impl PartialEq for StyleFn {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl StyleFn {
    /// Default function, produces an empty style.
    pub fn nil() -> Self {
        Self(None)
    }

    /// If this function represents no style.
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// New style function, the `func` closure is called for each styleable widget, before the widget is inited.
    pub fn new(func: impl Fn(&StyleArgs) -> StyleBuilder + Send + Sync + 'static) -> Self {
        Self(Some(Arc::new(move |a| {
            let style = func(a);
            if style.is_empty() { None } else { Some(style) }
        })))
    }

    /// New style function that returns clones of `style`.
    ///
    /// Note that if the `style` contains properties with node values the nodes will be moved to
    /// the last usage of the style, as nodes can't be cloned.
    ///
    /// Also note that the `style` will stay in memory for the lifetime of the `StyleFn`.
    pub fn singleton(style: StyleBuilder) -> Self {
        Self::new(move |_| style.clone())
    }

    /// Call the function to create a style for the styleable widget in the context.
    ///
    /// Returns `None` if [`is_nil`] or empty, otherwise returns the style.
    ///
    /// Note that you can call the style function directly:
    ///
    /// ```
    /// # use zng_wgt_style::{StyleFn, StyleArgs};
    /// fn foo(func: &StyleFn) {
    ///     let a = func.call(&StyleArgs::default());
    ///     let b = func(&StyleArgs::default());
    /// }
    /// ```
    ///
    /// In the example above `a` and `b` are both calls to the style function.
    ///
    /// [`is_nil`]: Self::is_nil
    pub fn call(&self, args: &StyleArgs) -> Option<StyleBuilder> {
        self.0.as_ref()?(args)
    }

    /// New style function that instantiates `self` and `other` and then [`extend`] `self` with `other`.
    ///
    /// [`extend`]: StyleBuilder::extend
    pub fn with_extend(self, other: StyleFn) -> StyleFn {
        if self.is_nil() {
            other
        } else if other.is_nil() {
            self
        } else {
            StyleFn::new(move |args| match (self(args), other(args)) {
                (Some(mut a), Some(b)) => {
                    a.extend(b);
                    a
                }
                (Some(r), None) | (None, Some(r)) => r,
                _ => StyleBuilder::default(),
            })
        }
    }
}
impl fmt::Debug for StyleFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StyleFn(_)")
    }
}
impl ops::Deref for StyleFn {
    type Target = dyn Fn(&StyleArgs) -> Option<StyleBuilder>;

    fn deref(&self) -> &Self::Target {
        if let Some(func) = &self.0 { &**func } else { &nil_func }
    }
}
fn nil_func(_: &StyleArgs) -> Option<StyleBuilder> {
    None
}

/// <span data-del-macro-root></span> Declares a style function closure.
///
/// The output type is a [`StyleFn`], the input can be a function name path or a closure,
/// with input type `&StyleArgs`. The closure syntax is clone-move ([`clmv!`]).
///
/// # Examples
///
/// The example below declares a closure that prints every time it is used, the closure captures `cloned` by clone-move
/// and `moved` by move. The closure ignores the [`StyleArgs`] because it is empty.
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::prelude::*;
/// # use zng_wgt_style::*;
/// # fn main() {
/// let cloned = var(10u32);
/// let moved = var(20u32);
/// let style_fn = style_fn!(cloned, |_| {
///     println!(
///         "style instantiated in {:?}, with captured values, {} and {}",
///         WIDGET.try_id(),
///         cloned.get(),
///         moved.get()
///     );
///
///     Style! {
///         // ..
///     }
/// });
/// # }
/// ```
///
/// [`clmv!`]: zng_wgt::prelude::clmv
#[macro_export]
macro_rules! style_fn {
    ($fn:path) => {
        $crate::StyleFn::new($fn)
    };
    ($($tt:tt)+) => {
        $crate::StyleFn::new($crate::__clmv! {
            $($tt)+
        })
    };
    () => {
        $crate::StyleFn::nil()
    };
}

context_var! {
    static MISSING_STYLE_VAR: StyleFn = StyleFn::nil();
}
#[property(WIDGET)]
fn missing_style_fn(child: impl IntoUiNode, _s: impl IntoVar<StyleFn>) -> UiNode {
    child.into_node()
}
