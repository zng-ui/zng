//! Style building blocks.

use std::{fmt, rc::Rc};

use crate::core::widget_builder::widget_mod;
use crate::prelude::new_widget::*;

/// Represents a set of properties that can be applied to any styleable widget.
///
/// This *widget* can be instantiated using the same syntax as any widget, but it produces a [`Style`]
/// instance instead of an widget. Widgets that inherit from [`style_mixin`] can be modified using properties
/// defined in a style, the properties are dynamically spliced into each widget instance.
///
/// Styles must only visually affect the styled widget, this is a semantic distinction only, any property can be set
/// in a style, so feel free to setup event handlers in styles, but only if they are used to affect the widget visually.
///
/// # Derived Styles
///
/// Note that you can declare a custom style *widget* using the same inheritance mechanism of normal widgets, as long
/// as any build override calls [`style::build`].
///
/// [`style_mixin`]: mod@style_mixin
#[widget($crate::widgets::style)]
pub mod style {
    use super::*;

    #[doc(inline)]
    pub use super::{style_generator, Style, StyleGenerator};

    /// style constructor.
    pub fn build(wgt: WidgetBuilder) -> Style {
        Style::from_builder(wgt)
    }

    // make `style` be a capture-only property too, this avoids import bugs caused by the same module name.
    #[doc(hidden)]
    #[property(context, capture, default(StyleGenerator::nil()))]
    pub fn style_property(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        child
    }
    #[doc(hidden)]
    pub use style_property::*;
}

/// Styleable widget mix-in.
///
/// Widgets that inherit from this one have a `style` property that can be set to a [`style_generator!`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
///
/// The style mix-in drastically affects the widget build process, only the `style` property and `when` condition
/// properties that affects it are instantiated with the widget, all the other properties and intrinsic nodes are instantiated
/// on init, after the style is generated.
///
/// Styleable widgets usually have a more elaborate style setup that supports mixing multiple contextual styles, see
/// [`style_mixin::with_style_extension`] for a full styleable widget example.
#[widget_mixin($crate::widgets::mixins::style_mixin)]
pub mod style_mixin {
    use super::*;

    properties! {
        /// Style generator used for the widget.
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
        pub super::style;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.set_custom_build(custom_build);
    }

    /// Helper for declaring properties that [extend] a style set from a context var.
    ///
    /// [extend]: StyleGenerator::with_extend
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
    ///         style = vis::STYLE_VAR;
    ///     }
    ///
    ///     /// Foo style and visual properties.
    ///     pub mod vis {
    ///         use super::*;
    ///
    ///         context_var! {
    ///             /// Foo style.
    ///             pub static STYLE_VAR: StyleGenerator = style_generator!(|ctx, _args| {
    ///                 style! {
    ///                     background_color = color_scheme_pair((colors::BLACK, colors::WHITE));
    ///                     cursor = CursorIcon::Crosshair;
    ///                 }
    ///             });
    ///         }
    ///
    ///         /// Replace the contextual [`STYLE_VAR`] with `style`.
    ///         #[property(context, default(STYLE_VAR))]
    ///         pub fn replace_style(
    ///             child: impl UiNode,
    ///             style: impl IntoVar<StyleGenerator>
    ///         ) -> impl UiNode {
    ///             with_context_var(child, STYLE_VAR, style)
    ///         }
    ///
    ///         /// Extends the contextual [`STYLE_VAR`] with the `style` override.
    ///         #[property(context, default(StyleGenerator::nil()))]
    ///         pub fn extend_style(
    ///             child: impl UiNode,
    ///             style: impl IntoVar<StyleGenerator>
    ///         ) -> impl UiNode {
    ///             style_mixin::with_style_extension(child, STYLE_VAR, style)
    ///         }
    ///     }
    /// }
    /// ```
    pub fn with_style_extension(
        child: impl UiNode,
        style_context: ContextVar<StyleGenerator>,
        extension: impl IntoVar<StyleGenerator>,
    ) -> impl UiNode {
        with_context_var(
            child,
            style_context,
            merge_var!(style_context, extension.into_var(), |base, over| {
                base.clone().with_extend(over.clone())
            }),
        )
    }

    ///Gets the custom build that is set on intrinsic by the mix-in.
    pub fn custom_build(mut wgt: WidgetBuilder) -> BoxedUiNode {
        // 1 - "split_off" the property `style`
        //     this moves the property and any `when` that affects it to a new widget builder.
        let style_id = property_id!(self.style);
        let mut style_builder = WidgetBuilder::new(wgt.widget_mod());
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
                let style = b.capture_var::<StyleGenerator>(style_id).unwrap();
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

    #[ui_node(struct StyleNode {
        child: Option<BoxedUiNode>,
        builder: WidgetBuilder,
        #[var] style: BoxedVar<StyleGenerator>,
    })]
    impl UiNode for StyleNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            if let Some(style) = self.style.get().generate(ctx, &StyleArgs {}) {
                let mut builder = self.builder.clone();
                builder.extend(style.into_builder());
                self.child = Some(builder.default_build());
            } else {
                self.child = Some(self.builder.clone().default_build());
            }
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.child = None;
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.style.is_new(ctx.vars) {
                self.deinit(ctx);
                self.init(ctx);
                ctx.updates.info_layout_and_render();
            } else {
                self.child.update(ctx, updates);
            }
        }
    }
}

/// Represents a style instance.
///
/// Use the [`style!`] *widget* to instantiate.
///
/// [`style!`]: mod@style
#[derive(Debug)]
pub struct Style {
    builder: WidgetBuilder,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            builder: WidgetBuilder::new(widget_mod!(style)),
        }
    }
}
impl Style {
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
    /// The importance index of properties is adjusted, any custom build or build action is ignored.
    pub fn from_builder(mut wgt: WidgetBuilder) -> Style {
        wgt.clear_build_actions();
        wgt.clear_custom_build();
        for p in wgt.properties_mut() {
            *p.importance = match *p.importance {
                Importance::WIDGET => Style::WIDGET_IMPORTANCE,
                Importance::INSTANCE => Style::INSTANCE_IMPORTANCE,
                other => other,
            };
        }
        Style { builder: wgt }
    }

    /// Unwrap the style dynamic widget.
    pub fn into_builder(self) -> WidgetBuilder {
        self.builder
    }

    /// Overrides `self` with `other`.
    pub fn extend(&mut self, other: Style) {
        self.builder.extend(other.builder);
    }

    /// If the style does nothing.
    pub fn is_empty(&self) -> bool {
        !self.builder.has_properties() && !self.builder.has_whens() && !self.builder.has_unsets()
    }
}
impl From<Style> for WidgetBuilder {
    fn from(t: Style) -> Self {
        t.into_builder()
    }
}
impl From<WidgetBuilder> for Style {
    fn from(p: WidgetBuilder) -> Self {
        Style::from_builder(p)
    }
}

/// Arguments for [`StyleGenerator`] closure.
///
/// Empty in this version.
#[derive(Debug)]
pub struct StyleArgs {}

/// Boxed shared closure that generates a style instance for a given widget context.
///
/// You can also use the [`style_generator!`] macro, it has the advantage of being clone move.
#[derive(Clone)]
pub struct StyleGenerator(Option<Rc<dyn Fn(&mut WidgetContext, &StyleArgs) -> Style>>);
impl Default for StyleGenerator {
    fn default() -> Self {
        Self::nil()
    }
}
impl StyleGenerator {
    /// Default generator, produces an empty style.
    pub fn nil() -> Self {
        Self(None)
    }

    /// If this generator represents no style.
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// New style generator, the `generate` closure is called for each styleable widget, before the widget is inited.
    pub fn new(generate: impl Fn(&mut WidgetContext, &StyleArgs) -> Style + 'static) -> Self {
        Self(Some(Rc::new(generate)))
    }

    /// Generate a style for the styleable widget in the context.
    ///
    /// Returns `None` if [`is_nil`] or empty, otherwise returns the style.
    ///
    /// [`is_nil`]: Self::is_nil
    pub fn generate(&self, ctx: &mut WidgetContext, args: &StyleArgs) -> Option<Style> {
        if let Some(g) = &self.0 {
            let style = g(ctx, args);
            if !style.is_empty() {
                return Some(style);
            }
        }
        None
    }

    /// New style generator that generates `self` and `other` and then [`extend`] `self` with `other`.
    ///
    /// [`extend`]: Style::extend
    pub fn with_extend(self, other: StyleGenerator) -> StyleGenerator {
        if self.is_nil() {
            other
        } else if other.is_nil() {
            self
        } else {
            StyleGenerator::new(move |ctx, args| {
                let mut r = self.generate(ctx, args).unwrap();
                r.extend(other.generate(ctx, args).unwrap());
                r
            })
        }
    }
}
impl fmt::Debug for StyleGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StyleGenerator(_)")
    }
}

/// <span data-del-macro-root></span> Declares a style generator closure.
///
/// The output type is a [`StyleGenerator`], the closure is [`clone_move!`].
///
/// [`clone_move!`]: crate::core::clone_move
#[macro_export]
macro_rules! style_generator {
    ($($tt:tt)+) => {
        $crate::widgets::style::StyleGenerator::new($crate::core::clone_move! {
            $($tt)+
        })
    }
}
#[doc(inline)]
pub use crate::style_generator;
