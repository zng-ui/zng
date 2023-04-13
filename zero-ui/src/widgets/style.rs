//! Style building blocks.

use std::sync::Arc;
use std::{fmt, ops};

use crate::prelude::new_widget::*;

/// Represents a set of properties that can be applied to any styleable widget.
///
/// This *widget* can be instantiated using the same syntax as any widget, but it produces a [`StyleBuilder`]
/// instance instead of an widget. Widgets that have [`StyleMixin<P>`] can be modified using properties
/// defined in a style, the properties are dynamically spliced into each widget instance.
///
/// Styles must only visually affect the styled widget, this is a semantic distinction only, any property can be set
/// in a style, so feel free to setup event handlers in styles, but only if they are used to affect the widget visually.
///
/// # Derived Styles
///
/// Note that you can declare a custom style *widget* using the same inheritance mechanism of normal widgets, as long
/// as they build to [`StyleBuilder`].
///
/// [`style_mixin`]: mod@style_mixin
#[widget($crate::widgets::Style)]
pub struct Style(WidgetBase);
impl Style {
    /// Build the style.
    pub fn build(&mut self) -> StyleBuilder {
        StyleBuilder::from_builder(self.take_builder())
    }
}

/// Styleable widget mix-in.
///
/// Widgets that inherit from this one have a `style_fn` property that can be set to a [`style_fn!`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
///
/// The style mix-in drastically affects the widget build process, only the `style_fn` property and `when` condition
/// properties that affects it are instantiated with the widget, all the other properties and intrinsic nodes are instantiated
/// on init, after the style is generated.
///
/// Styleable widgets usually have a more elaborate style setup that supports mixing multiple contextual styles, see
/// [`style_mixin::with_style_extension`] for a full styleable widget example.
#[widget_mixin]
pub struct StyleMix<P>(P);
impl<P: WidgetImpl> StyleMix<P> {
    #[widget(on_start)]
    fn on_start(&mut self) {
        self.base().builder().set_custom_build(StyleMix::<()>::custom_build);
    }
}
impl<P> StyleMix<P> {
    /// The custom build that is set on intrinsic by the mix-in.
    pub fn custom_build(mut wgt: WidgetBuilder) -> BoxedUiNode {
        // 1 - "split_off" the property `style`
        //     this moves the property and any `when` that affects it to a new widget builder.
        let style_id = property_id!(style_fn);
        let mut style_builder = WidgetBuilder::new(wgt.widget_type());
        wgt.split_off([style_id], &mut style_builder);

        if style_builder.has_properties() {
            // 2.a - There was a `style` property, build a "mini widget" that is only the style property
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
                b.set_child(StyleNode {
                    child: None,
                    builder: wgt.take().unwrap(),
                    style,
                });
            });
            // 4 - Build the "mini widget",
            //     if the `style` property was not affected by any `when` this just returns the `StyleNode`.
            style_builder.build()
        } else {
            // 2.b - There was not property `style`, this widget is not styleable, just build the default.
            wgt.build()
        }
    }
}
/// Style function used for the widget.
///
/// Properties and `when` conditions in the generated style are applied to the widget as
/// if they where set on it. Note that changing the style causes the widget info tree to rebuild,
/// prefer property binding and `when` conditions to cause visual changes that happen often.
///
/// The style property it-self can be affected by `when` conditions set on the widget, this works to a limited
/// extent as only the style and when condition properties is loaded to evaluate, so a when condition that depends
/// on the full widget context will not work.
///
/// Is `nil` by default.
#[property(WIDGET, capture, default(StyleFn::nil()), impl(StyleMix<P>))]
pub fn style_fn(_child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {}

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
/// #[widget($crate::foo)]
/// pub mod foo {
///     use super::*;
///
///     inherit!(widget_base::base);
///     inherit!(style_mixin);
///
///     properties! {
///         /// Foo style.
///         ///
///         /// The style is set to [`vis::STYLE_VAR`], settings this directly replaces the style.
///         /// You can use [`vis::replace_style`] and [`vis::extend_style`] to set or modify the
///         /// style for all `foo` in a context.
///         style_fn = vis::STYLE_VAR;
///     }
///
///     /// Foo style and visual properties.
///     pub mod vis {
///         use super::*;
///
///         context_var! {
///             /// Foo style.
///             pub static STYLE_VAR: StyleFn = style_fn!(|_args| {
///                 Style! {
///                     background_color = color_scheme_pair((colors::BLACK, colors::WHITE));
///                     cursor = CursorIcon::Crosshair;
///                 }
///             });
///         }
///
///         /// Replace the contextual [`STYLE_VAR`] with `style`.
///         #[property(CONTEXT, default(STYLE_VAR))]
///         pub fn replace_style(
///             child: impl UiNode,
///             style: impl IntoVar<StyleFn>
///         ) -> impl UiNode {
///             with_context_var(child, STYLE_VAR, style)
///         }
///
///         /// Extends the contextual [`STYLE_VAR`] with the `style` override.
///         #[property(CONTEXT, default(StyleFn::nil()))]
///         pub fn extend_style(
///             child: impl UiNode,
///             style: impl IntoVar<StyleFn>
///         ) -> impl UiNode {
///             style_mixin::with_style_extension(child, STYLE_VAR, style)
///         }
///     }
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

#[ui_node(struct StyleNode {
    child: Option<BoxedUiNode>,
    builder: WidgetBuilder,
    #[var] style: BoxedVar<StyleFn>,
})]
impl UiNode for StyleNode {
    fn init(&mut self) {
        self.auto_subs();
        if let Some(style) = self.style.get().call(&StyleArgs {}) {
            let mut builder = self.builder.clone();
            builder.extend(style.into_builder());
            self.child = Some(builder.default_build());
        } else {
            self.child = Some(self.builder.clone().default_build());
        }
        self.child.init();
    }

    fn deinit(&mut self) {
        self.child.deinit();
        self.child = None;
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        if self.style.is_new() {
            WIDGET.reinit();
            WIDGET.update_info().layout().render();
        } else {
            self.child.update(updates);
        }
    }
}

/// Represents a style instance.
///
/// Use the [`Style!`] *widget* to declare.
///
/// [`Style!`]: mod@style
#[derive(Debug)]
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

    /// New style from a widget builder.
    ///
    /// The importance index of properties is adjusted, any custom build or widget build action is ignored.
    pub fn from_builder(mut wgt: WidgetBuilder) -> StyleBuilder {
        wgt.clear_build_actions();
        wgt.clear_custom_build();
        for p in wgt.properties_mut() {
            *p.importance = match *p.importance {
                Importance::WIDGET => StyleBuilder::WIDGET_IMPORTANCE,
                Importance::INSTANCE => StyleBuilder::INSTANCE_IMPORTANCE,
                other => other,
            };
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
    /// [`extend`]: Style::extend
    pub fn with_extend(self, other: StyleFn) -> StyleFn {
        if self.is_nil() {
            other
        } else if other.is_nil() {
            self
        } else {
            StyleFn::new(move |args| {
                let mut r = self(args).unwrap();
                r.extend(other(args).unwrap());
                r
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
