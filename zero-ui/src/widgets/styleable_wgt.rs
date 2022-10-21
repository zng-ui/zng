//! Style building blocks.

use std::{fmt, rc::Rc};

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
/// [`styleable`]: mod@styleable
#[widget($crate::widgets::style)]
pub mod style {
    use super::*;

    #[doc(inline)]
    pub use super::{style_generator, Style, StyleGenerator};

    /// style constructor.
    pub fn build(wgt: WidgetBuilder) -> Style {
        Style::from_dyn_widget(wgt)
    }
}

/// Styleable widget mix-in.
///
/// Widgets that inherit from this one have a `style` property that can be set to a [`StyleGenerator`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
///
/// Styleable widgets usually have a more elaborate style setup that supports mixing multiple contextual styles, see
/// [`style_mixin::with_style_extension`] for a full styleable widget example.
#[widget_mixin($crate::widgets::style_mixin)]
pub mod style_mixin {
    use super::*;

    properties! {
        pub self::style;
    }

    /// Styleable `new`, captures the `id` and `style` properties.
    pub fn new_dyn(widget: DynWidget, id: impl IntoValue<WidgetId>, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        #[ui_node(struct StyleableNode {
            child: DynWidgetNode,
            snapshot: Option<DynWidgetSnapshot>,
            #[var] style: impl Var<StyleGenerator>,
        })]
        impl UiNode for StyleableNode {
            fn init(&mut self, ctx: &mut WidgetContext) {
                if let Some(style) = self.style.get().generate(ctx, &StyleArgs {}) {
                    self.snapshot = Some(self.child.snapshot());
                    self.child.extend(style.into_node());
                }
                self.child.init(ctx);
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.child.deinit(ctx);
                if let Some(snap) = self.snapshot.take() {
                    self.child.restore(snap).unwrap();
                }
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
        let child = StyleableNode {
            child: widget.into_node(true),
            snapshot: None,
            style: style.into_var(),
        };
        implicit_base::new(child, id)
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
    ///     inherit!(styleable);
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
    ///             styleable::with_style_extension(child, STYLE_VAR, style)
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

    /// Style generator used for the widget.
    ///
    /// Properties and `when` conditions in the generated style are applied to the widget as
    /// if they where set on it. Note that changing the style causes the widget info tree to rebuild,
    /// prefer property binding and `when` conditions to cause visual changes that happen often.
    ///
    /// Is `nil` by default.
    #[property(context, default(StyleGenerator::nil()))]
    pub fn style(child: impl UiNode, generator: impl IntoVar<StyleGenerator>) -> impl UiNode {
        let _ = generator;
        tracing::error!("property `style` must be captured");
        child
    }
}

/// Represents a style instance.
///
/// Use the [`style!`] *widget* to instantiate.
///
/// [`style!`]: mod@style
#[derive(Default, Debug)]
pub struct Style {
    node: DynWidgetNode,
}
impl Style {
    /// Importance of style properties set by default in style widgets.
    ///
    /// Is `DynPropImportance::WIDGET - 10`.
    pub const WIDGET_IMPORTANCE: DynPropImportance = DynPropImportance(DynPropImportance::WIDGET.0 - 10);

    /// Importance of style properties set in style instances.
    ///
    /// Is `DynPropImportance::INSTANCE - 10`.
    pub const INSTANCE_IMPORTANCE: DynPropImportance = DynPropImportance(DynPropImportance::INSTANCE.0 - 10);

    /// Properties and when blocks of this style.
    pub fn node(&self) -> &DynWidgetNode {
        &self.node
    }

    /// Mutable reference to the properties and when blocks of this style.
    pub fn node_mut(&mut self) -> &mut DynWidgetNode {
        &mut self.node
    }

    /// Unwrap the style dynamic widget.
    pub fn into_node(self) -> DynWidgetNode {
        self.node
    }

    /// New style from dynamic widget input.
    ///
    /// The importance index of properties is adjusted, the intrinsic constructor and child nodes are discarded.
    pub fn from_dyn_widget(mut wgt: DynWidget) -> Style {
        for part in &mut wgt.parts {
            for p in &mut part.properties {
                p.importance = match p.importance {
                    DynPropImportance::WIDGET => Style::WIDGET_IMPORTANCE,
                    DynPropImportance::INSTANCE => Style::INSTANCE_IMPORTANCE,
                    custom => custom,
                };
            }
        }
        wgt.into_node(false).into()
    }

    /// New style from built dynamic widget.
    pub fn from_node(node: DynWidgetNode) -> Style {
        Self { node }
    }

    /// Overrides `self` with `other`.
    pub fn extend(&mut self, other: Style) {
        self.node.extend(other.node);
    }
}
#[ui_node(
    delegate = &self.node,
    delegate_mut = &mut self.node,
)]
impl UiNode for Style {}
impl From<Style> for DynWidgetNode {
    fn from(t: Style) -> Self {
        t.into_node()
    }
}
impl From<DynWidgetNode> for Style {
    fn from(p: DynWidgetNode) -> Self {
        Style::from_node(p)
    }
}
impl From<DynWidget> for Style {
    fn from(p: DynWidget) -> Self {
        Style::from_dyn_widget(p)
    }
}

/// Arguments for [`StyleGenerator`] closure.
///
/// Currently no arguments.
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
    /// Returns `None` if [`is_nil`], otherwise returns the style.
    ///
    /// [`is_nil`]: Self::is_nil
    pub fn generate(&self, ctx: &mut WidgetContext, args: &StyleArgs) -> Option<Style> {
        self.0.as_ref().map(|g| g(ctx, args))
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
