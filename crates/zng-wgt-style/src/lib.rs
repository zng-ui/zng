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
/// can set [`replace`](#replace) on a style to `true` to fully remove all contextual properties and only use the
/// new style properties.
///
/// # Inherit Style
///
/// Note that you can declare a custom style *widget* using the same inheritance mechanism of normal widgets, as long
/// as they build to [`StyleBuilder`]. This is different from the *extend/replace* mechanism as it operates on the style
/// type, not the instances.
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
#[property(WIDGET, capture, default(false), widget_impl(Style))]
pub fn replace(replace: impl IntoValue<bool>) {}

/// Styleable widget mixin.
///
/// Widgets that inherit this mix-in have a `style_fn` property that can be set to a [`style_fn!`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
///
/// The style mixin drastically affects the widget build process, only the `style_base_fn`, `style_fn` and `when` condition
/// properties that affects these are instantiated with the widget, all the other properties and intrinsic nodes are instantiated
/// on init, after the style is generated.
///
/// Widgets that inherit this mix-in must call [`style_intrinsic`] in their own `widget_intrinsic`, the call is missing
/// the widget will log an error on instantiation and only the `style_base_fn` will be used. You can use the [`impl_style_fn!`]
/// macro to generate the style var and property.
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
    pub fn custom_build(mut wgt: WidgetBuilder, cfg: Option<(ContextVar<StyleFn>, PropertyId)>) -> BoxedUiNode {
        let (style_var, style_id) = cfg.unwrap_or_else(|| {
            tracing::error!("missing `style_intrinsic` in `{}`", wgt.widget_type().path);
            (MISSING_STYLE_VAR, property_id!(self::missing_style_fn))
        });

        // 1 - "split_off" the properties `style_base_fn` and `style_fn`
        //     this moves the properties and any `when` that affects them to a new widget builder.
        let style_base_id = property_id!(style_base_fn);
        let mut style_builder = WidgetBuilder::new(wgt.widget_type());
        wgt.split_off([style_base_id, style_id], &mut style_builder);

        if style_builder.has_properties() {
            // 2.a - There was a `style_fn` property, build a "mini widget" that is only the style property
            //       and when condition properties that affect it.

            #[cfg(feature = "trace_widget")]
            wgt.push_build_action(|wgt| {
                // avoid double trace as the style builder already inserts a widget tracer.
                wgt.disable_trace_widget();
            });

            let mut wgt = Some(wgt);
            style_builder.push_build_action(move |b| {
                // 3 - The actual style_node and builder is a child of the "mini widget".

                let style_base = b
                    .capture_var::<StyleFn>(style_base_id)
                    .unwrap_or_else(|| LocalVar(StyleFn::nil()).boxed());
                let style = b
                    .capture_var::<StyleFn>(style_id)
                    .unwrap_or_else(|| LocalVar(StyleFn::nil()).boxed());

                b.set_child(style_node(None, wgt.take().unwrap(), style_base, style_var, style));
            });
            // 4 - Build the "mini widget",
            //     if the `style` property was not affected by any `when` this just returns the `StyleNode`.
            style_builder.build()
        } else {
            // 2.b - There was no `style_fn` property, this widget is not styleable, just build the default.
            wgt.build()
        }
    }
}

#[doc(hidden)]
pub mod __impl_style_context_util {
    pub use zng_wgt::prelude::{IntoVar, UiNode, context_var, property};
}

/// Implements the contextual `STYLE_FN_VAR` and `style_fn`.
///
/// This is a helper for [`StyleMix<P>`](struct@StyleMix) implementers, see the `zng::style` module level
/// documentation for more details.
#[macro_export]
macro_rules! impl_style_fn {
    ($Widget:ty) => {
        $crate::__impl_style_context_util::context_var! {
            /// Contextual style variable.
            ///
            /// Use [`style_fn`](fn@style_fn) to set.
            pub static STYLE_FN_VAR: $crate::StyleFn = $crate::StyleFn::nil();
        }

        /// Extends or replaces the widget style.
        ///
        /// Properties and `when` conditions in the style are applied to the widget. Style instances extend the contextual style
        /// by default, you can set `replace` on a style to `true` to fully replace.
        #[$crate::__impl_style_context_util::property(WIDGET, default($crate::StyleFn::nil()), widget_impl($Widget))]
        pub fn style_fn(
            child: impl $crate::__impl_style_context_util::UiNode,
            style_fn: impl $crate::__impl_style_context_util::IntoVar<$crate::StyleFn>,
        ) -> impl $crate::__impl_style_context_util::UiNode {
            $crate::with_style_fn(child, STYLE_FN_VAR, style_fn)
        }
    };
}

/// Widget's base style. All other styles set using `style_fn` are applied over this style.
///
/// Is `nil` by default.
#[property(WIDGET, capture, default(StyleFn::nil()), widget_impl(StyleMix<P>))]
pub fn style_base_fn(style: impl IntoVar<StyleFn>) {}

/// Helper for declaring the `style_fn` property.
///
/// The [`impl_style_fn!`] macro uses this function as the implementation of `style_fn`.
pub fn with_style_fn(child: impl UiNode, style_context: ContextVar<StyleFn>, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(
        child,
        style_context,
        merge_var!(style_context, style.into_var(), |base, over| {
            base.clone().with_extend(over.clone())
        }),
    )
}

fn style_node(
    child: Option<BoxedUiNode>,
    builder: WidgetBuilder,
    captured_style_base: BoxedVar<StyleFn>,
    style_var: ContextVar<StyleFn>,
    captured_style: BoxedVar<StyleFn>,
) -> impl UiNode {
    let style_vars = [captured_style_base, style_var.boxed(), captured_style];
    match_node_typed(child, move |c, op| match op {
        UiNodeOp::Init => {
            let mut style_builder = StyleBuilder::default();
            for var in &style_vars {
                WIDGET.sub_var(var);

                if let Some(style) = var.get().call(&StyleArgs {}) {
                    style_builder.extend(style);
                }
            }

            if !style_builder.is_empty() {
                let mut builder = builder.clone();
                builder.extend(style_builder.into_builder());
                *c.child() = Some(builder.default_build());
            } else {
                *c.child() = Some(builder.clone().default_build());
            }
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = None;
        }
        UiNodeOp::Update { .. } => {
            if style_vars.iter().any(|v| v.is_new()) {
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
fn missing_style_fn(child: impl UiNode, _s: impl IntoVar<StyleFn>) -> impl UiNode {
    child
}
