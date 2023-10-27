//! Style building blocks.

use std::sync::Arc;
use std::{fmt, ops};

use crate::prelude::new_widget::*;

/// Represents a set of properties that can be applied to any styleable widget.
///
/// This *widget* can be instantiated using the same syntax as any widget, but it produces a [`StyleBuilder`]
/// instance instead of an widget. Widgets that have [`StyleMix<P>`] can be modified using properties
/// defined in a style, the properties are dynamically spliced into each widget instance.
///
/// Styles must only visually affect the styled widget, this is a semantic distinction only, any property can be set
/// in a style, so feel free to setup event handlers in styles, but only if they are used to affect the widget visually.
///
/// # Derived Styles
///
/// Note that you can declare a custom style *widget* using the same inheritance mechanism of normal widgets, as long
/// as they build to [`StyleBuilder`].
#[widget($crate::widgets::Style)]
pub struct Style(WidgetBase);
impl Style {
    /// Build the style.
    pub fn widget_build(&mut self) -> StyleBuilder {
        StyleBuilder::from_builder(self.widget_take())
    }
}

/// Styleable widget mixin.
///
/// Widgets that inherit from this one have a `style_fn` property that can be set to a [`style_fn!`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
///
/// The style mixin drastically affects the widget build process, only the `style_fn` property and `when` condition
/// properties that affects it are instantiated with the widget, all the other properties and intrinsic nodes are instantiated
/// on init, after the style is generated.
///
/// Styleable widgets usually have a more elaborate style setup that supports mixing multiple contextual styles, see
/// [`with_style_extension`] for a full styleable widget example.
#[widget_mixin]
pub struct StyleMix<P>(P);
impl<P: WidgetImpl> StyleMix<P> {
    fn widget_intrinsic(&mut self) {
        self.base().widget_builder().set_custom_build(StyleMix::<()>::custom_build);
    }
}
impl<P> StyleMix<P> {
    /// The custom build that is set on intrinsic by the mixin.
    pub fn custom_build(mut wgt: WidgetBuilder) -> BoxedUiNode {
        // 1 - "split_off" the property `style_fn`
        //     this moves the property and any `when` that affects it to a new widget builder.
        let style_id = property_id!(style_fn);
        let mut style_builder = WidgetBuilder::new(wgt.widget_type());
        wgt.split_off([style_id, style_id], &mut style_builder);

        if style_builder.has_properties() {
            // 2.a - There was a `style_fn` property, build a "mini widget" that is only the style property
            //       and when condition properties that affect it.

            #[cfg(trace_widget)]
            wgt.push_build_action(|wgt| {
                // avoid double trace as the style builder already inserts a widget tracer.
                wgt.disable_trace_widget();
            });

            let mut wgt = Some(wgt);
            style_builder.push_build_action(move |b| {
                // 3 - The actual StyleNode and builder is a child of the "mini widget".
                let style = b.capture_var::<StyleFn>(style_id).unwrap();
                b.set_child(style_node(None, wgt.take().unwrap(), style));
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

/// Replaces the widget's style with an style function.
///
/// Properties and `when` conditions in the generated style are applied to the widget as
/// if they where set on it. Note that changing the style causes the widget info tree to rebuild,
/// prefer property binding and `when` conditions to cause visual changes that happen often.
///
/// The style property it-self can be affected by `when` conditions set on the widget, this works to a limited
/// extent as only the style and when condition properties is loaded to evaluate, so a when condition that depends
/// on the full widget context will not work.
///
/// You can also set this property to an style instance directly, it will always work when you have an instance
/// of the style per widget instance, but if the style is used in multiple widgets properties with cloneable
/// values will be cloned, properties with node values will be moved to the last usage place, breaking the style
/// in previous instances. When in doubt use [`style_fn!`], it always works.
///
/// Is `nil` by default.
#[property(WIDGET, capture, default(StyleFn::nil()), widget_impl(StyleMix<P>))]
pub fn style_fn(style: impl IntoVar<StyleFn>) {}

/// Helper for declaring properties that [extend] a style set from a context var.
///
/// [extend]: StyleFn::with_extend
///
/// # Examples
///
/// Example styleable widget defining a `foo::vis::extend_style` property that extends the contextual style.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude::new_widget::*;
///
/// #[widget($crate::Foo)]
/// pub struct Foo(StyleMix<WidgetBase>);
/// impl Foo {
///     
///     fn widget_intrinsic(&mut self) {
///         widget_set! {
///             self;
///             style_fn = STYLE_VAR;
///         }
///     }
/// }
///
/// context_var! {
///    /// Foo style.
///    pub static STYLE_VAR: StyleFn = style_fn!(|_args| {
///        Style! {
///            background_color = color_scheme_pair((colors::BLACK, colors::WHITE));
///            cursor = CursorIcon::Crosshair;
///        }
///    });
///}
///
/// /// Replace the contextual [`STYLE_VAR`] with `style`.
/// #[property(CONTEXT, default(STYLE_VAR))]
/// pub fn replace_style(
///     child: impl UiNode,
///     style: impl IntoVar<StyleFn>
/// ) -> impl UiNode {
///     with_context_var(child, STYLE_VAR, style)
/// }
///
/// /// Extends the contextual [`STYLE_VAR`] with the `style` override.
/// #[property(CONTEXT, default(StyleFn::nil()))]
/// pub fn extend_style(
///     child: impl UiNode,
///     style: impl IntoVar<StyleFn>
/// ) -> impl UiNode {
///     style::with_style_extension(child, STYLE_VAR, style)
/// }
/// ```
pub fn with_style_extension(child: impl UiNode, style_context: ContextVar<StyleFn>, extension: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(
        child,
        style_context,
        merge_var!(style_context, extension.into_var(), |base, over| {
            base.clone().with_extend(over.clone())
        }),
    )
}

fn style_node(child: Option<BoxedUiNode>, builder: WidgetBuilder, style: BoxedVar<StyleFn>) -> impl UiNode {
    match_node_typed(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&style);

            if let Some(style) = style.get().call(&StyleArgs {}) {
                let mut builder = builder.clone();
                builder.extend(style.into_builder());
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
            if style.is_new() {
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
}
impl Default for StyleBuilder {
    fn default() -> Self {
        Self {
            builder: WidgetBuilder::new(Style::widget_type()),
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
        for p in wgt.properties_mut() {
            *p.importance = match *p.importance {
                Importance::WIDGET => StyleBuilder::WIDGET_IMPORTANCE,
                Importance::INSTANCE => StyleBuilder::INSTANCE_IMPORTANCE,
                other => other,
            };
            p.position.index = p.position.index.saturating_sub(Self::POSITION_OFFSET);
        }
        StyleBuilder { builder: wgt }
    }

    /// Unwrap the style dynamic widget.
    pub fn into_builder(self) -> WidgetBuilder {
        self.builder
    }

    /// Overrides `self` with `other`.
    pub fn extend(&mut self, other: StyleBuilder) {
        self.builder.extend(other.builder);
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
/// Empty in this version.
#[derive(Debug)]
pub struct StyleArgs {}

/// Boxed shared closure that generates a style instance for a given widget context.
///
/// You can also use the [`style_fn!`] macro, it has the advantage of being clone move.
#[derive(Clone)]
pub struct StyleFn(Option<Arc<dyn Fn(&StyleArgs) -> Option<StyleBuilder> + Send + Sync>>);
impl Default for StyleFn {
    fn default() -> Self {
        Self::nil()
    }
}
impl PartialEq for StyleFn {
    // can only fail by returning `false` in some cases where the value pointer is actually equal.
    // see: https://github.com/rust-lang/rust/issues/103763
    //
    // we are fine with this, worst case is just an extra var update
    #[allow(clippy::vtable_address_comparisons)]
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
            if style.is_empty() {
                None
            } else {
                Some(style)
            }
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
    /// use zero_ui::widgets::style::{StyleFn, StyleArgs};
    ///
    /// fn foo(func: &StyleFn) {
    ///     let a = func.call(&StyleArgs {});
    ///     let b = func(&StyleArgs {});
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
        if let Some(func) = &self.0 {
            &**func
        } else {
            &nil_func
        }
    }
}
fn nil_func(_: &StyleArgs) -> Option<StyleBuilder> {
    None
}

/// <span data-del-macro-root></span> Declares a style function closure.
///
/// The output type is a [`StyleFn`], the closure is [`clmv!`].
///
/// [`clmv!`]: crate::core::clmv
#[macro_export]
macro_rules! style_fn {
    ($($tt:tt)+) => {
        $crate::widgets::style::StyleFn::new($crate::core::clmv! {
            $($tt)+
        })
    }
}
#[doc(inline)]
pub use crate::style_fn;
