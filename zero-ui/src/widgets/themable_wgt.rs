//! Theme building blocks.

use std::{fmt, rc::Rc};

use crate::{
    core::{DynPropImportance, DynWidget, DynWidgetNode, DynWidgetSnapshot, NilUiNode},
    prelude::new_widget::*,
};

/// Represents a set of properties that can be applied to any themable widget.
///
/// This *widget* can be instantiated using the same syntax as any widget, but it produces a [`Theme`]
/// instance instead of an widget. Widgets that inherit from [`themable`] can be modified using properties
/// defined in a theme, the properties are dynamically spliced into each widget instance.
///
/// Themes must only visually affect the themed widget, this is a semantic distinction only, any property can be set
/// in a theme, so feel free to setup event handlers in themes, but only if they are used to affect the widget visually.
///
/// # Derived Themes
///
/// Note that you can declare a custom theme *widget* using the same inheritance mechanism of normal widgets, all widget
/// constructors are no-op and can be ignored, except the [`new_dyn`].
///
/// [`themable`]: mod@themable
#[widget($crate::widgets::theme)]
pub mod theme {
    use super::*;

    use crate::core::window::WindowTheme;
    use crate::widgets::window::nodes::WINDOW_THEME_VAR;

    #[doc(inline)]
    pub use super::{theme_generator, Theme, ThemeGenerator};

    properties! {
        remove { id; visibility; enabled }
    }

    fn new_child() -> NilUiNode {
        NilUiNode
    }

    fn new_child_layout(child: impl UiNode) -> impl UiNode {
        child
    }

    fn new_child_context(child: impl UiNode) -> impl UiNode {
        child
    }

    fn new_fill(child: impl UiNode) -> impl UiNode {
        child
    }

    fn new_border(child: impl UiNode) -> impl UiNode {
        child
    }

    fn new_size(child: impl UiNode) -> impl UiNode {
        child
    }

    fn new_layout(child: impl UiNode) -> impl UiNode {
        child
    }

    fn new_event(child: impl UiNode) -> impl UiNode {
        child
    }

    fn new_context(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Theme constructor.
    pub fn new_dyn(widget: DynWidget) -> Theme {
        Theme::from_dyn_widget(widget)
    }

    /// Declare a dark and light value that is selected depending on the window theme.
    ///
    /// This is a [`merge_var!`] that matches the [`WINDOW_THEME_VAR`] to select the value.
    pub fn pair<T: VarValue>(dark: impl IntoVar<T>, light: impl IntoVar<T>) -> impl Var<T> {
        merge_var!(WINDOW_THEME_VAR, dark.into_var(), light.into_var(), |w, d, l| {
            match w {
                WindowTheme::Dark => d.clone(),
                WindowTheme::Light => l.clone(),
            }
        })
    }

    /// Represents a dark and light *color*.
    #[derive(Debug, Clone, Copy, PartialEq, Hash)]
    pub struct ColorPair {
        /// Color used when [`WindowTheme::Dark`].
        pub dark: Rgba,
        /// Color used when [`WindowTheme::Light`].
        pub light: Rgba,
    }
    impl_from_and_into_var! {
        /// From `(dark, light)` tuple.
        fn from<D: Into<Rgba> + Clone, L: Into<Rgba> + Clone>((dark, light): (D, L)) -> ColorPair {
            ColorPair {
                dark: dark.into(),
                light: light.into(),
            }
        }
    }
    impl ColorPair {
        /// Overlay white with `highlight` amount as alpha over the [`dark`] color.
        ///
        /// [`dark`]: ColorPair::dark
        pub fn highlight_dark(self, hightlight: impl Into<Factor>) -> Rgba {
            colors::WHITE.with_alpha(hightlight.into()).mix_normal(self.dark)
        }

        /// Overlay black with `highlight` amount as alpha over the [`light`] color.
        ///
        /// [`light`]: ColorPair::light
        pub fn highlight_light(self, hightlight: impl Into<Factor>) -> Rgba {
            colors::BLACK.with_alpha(hightlight.into()).mix_normal(self.light)
        }
    }

    /// Declare a variable that selects the color from a [`ColorPair`] depending on the current [`WINDOW_THEME_VAR`].
    pub fn color(pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
        merge_var!(WINDOW_THEME_VAR, pair.into_var(), |&theme, &pair| {
            match theme {
                WindowTheme::Dark => pair.dark,
                WindowTheme::Light => pair.light,
            }
        })
    }

    /// Declare a variable that selects the color from a [`ColorPair`] depending on the current [`WINDOW_THEME_VAR`]
    /// mixin a `highlight`.
    ///
    /// See [`ColorPair::highlight_dark`] and [`ColorPair::highlight_light`] for more details.
    pub fn color_highlight(color_pair: impl IntoVar<ColorPair>, hightlight: impl IntoVar<Factor>) -> impl Var<Rgba> {
        merge_var!(
            WINDOW_THEME_VAR,
            color_pair.into_var(),
            hightlight.into_var(),
            |&theme, &pair, &hightlight| {
                match theme {
                    WindowTheme::Dark => pair.highlight_dark(hightlight),
                    WindowTheme::Light => pair.highlight_light(hightlight),
                }
            }
        )
    }

    /// Declare a dark and light *color* modified to highlight *hover*.
    pub fn color_hovered(color_pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
        color_highlight(color_pair, 0.08)
    }

    /// Declare a dark and light *color* modified to highlight *pressed*.
    pub fn color_pressed(color_pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
        color_highlight(color_pair, 0.16)
    }
}

/// Themable widget base.
///
/// Widgets that inherit from this one have a `theme` property that can be set to a [`ThemeGenerator`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
/// You can also use the [`theme::pair`] to set `theme` to set properties that toggle depending on the [`WindowTheme`].
///
/// Themable widgets usually have a more elaborate theme setup that supports mixing multiple contextual themes, see [`themable::with_theme_extension`]
/// for a full themable widget example.
///
/// # Derived Widgets
///
/// Widgets that inherit from this one must use the dynamic constructors and delegate to [`nodes::new_priority`], custom nodes
/// can be inserted just like in a normal widget declaration, the [`nodes::new_priority`] is the insert point for the dynamic
/// properties from the widget and theme.
#[widget($crate::widgets::themable)]
pub mod themable {
    use super::*;

    properties! {
        /// Theme generator used for the widget.
        ///
        /// Properties and `when` conditions in the generated theme are applied to the widget as
        /// if they where set on it. Note that changing the theme causes the widget info tree to rebuild,
        /// prefer property binding and `when` conditions to cause visual changes that happen often.
        ///
        /// Is `nil` by default.
        theme(impl IntoVar<ThemeGenerator>) = ThemeGenerator::nil();
    }

    /// Themable `new`, captures the `id` and `theme` properties.
    pub fn new_dyn(widget: DynWidget, id: impl IntoValue<WidgetId>, theme: impl IntoVar<ThemeGenerator>) -> impl Widget {
        struct ThemableNode<T> {
            child: DynWidgetNode,
            snapshot: Option<DynWidgetSnapshot>,
            theme: T,
        }
        #[impl_ui_node(child)]
        impl<T: Var<ThemeGenerator>> UiNode for ThemableNode<T> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                if let Some(theme) = self.theme.get(ctx.vars).generate(ctx, &ThemeArgs {}) {
                    self.snapshot = Some(self.child.snapshot());
                    self.child.extend(theme.into_node());
                }
                self.child.init(ctx);
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.child.deinit(ctx);
                if let Some(snap) = self.snapshot.take() {
                    self.child.restore(snap).unwrap();
                }
            }

            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                subs.var(ctx, &self.theme);
                self.child.subscriptions(ctx, subs);
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                if self.theme.is_new(ctx.vars) {
                    self.deinit(ctx);
                    self.init(ctx);
                    ctx.updates.info_layout_and_render();
                } else {
                    self.child.update(ctx);
                }
            }
        }
        let child = ThemableNode {
            child: widget.into_node(true),
            snapshot: None,
            theme: theme.into_var(),
        };
        implicit_base::new(child, id)
    }

    /// Helper for declaring properties that [extend] a theme set from a context var.
    ///
    /// [extend]: ThemeGenerator::with_extend
    ///
    /// # Examples
    ///
    /// Example themable widget defining a `foo::vis::extend_theme` property that extends the contextual theme.
    ///
    /// ```
    /// # fn main() { }
    /// use zero_ui::prelude::new_widget::*;
    ///
    /// #[widget($crate::foo)]
    /// pub mod foo {
    ///     use super::*;
    ///
    ///     inherit!(themable);
    ///
    ///     properties! {
    ///         /// Foo theme.
    ///         ///
    ///         /// The theme is set to [`vis::THEME_VAR`], settings this directly replaces the theme.
    ///         /// You can use [`vis::replace_theme`] and [`vis::extend_theme`] to set or modify the
    ///         /// theme for all `foo` in a context.
    ///         theme = vis::THEME_VAR;
    ///     }
    ///
    ///     /// Foo theme and visual properties.
    ///     pub mod vis {
    ///         use super::*;
    ///
    ///         context_var! {
    ///             /// Foo theme.
    ///             pub static THEME_VAR: ThemeGenerator = theme_generator!(|ctx, _args| {
    ///                 theme! {
    ///                     background_color = theme::pair(colors::BLACK, colors::WHITE);
    ///                     cursor = CursorIcon::Crosshair;
    ///                 }
    ///             });
    ///         }
    ///
    ///         /// Replace the contextual [`THEME_VAR`] with the `theme`.
    ///         #[property(context, default(THEME_VAR))]
    ///         pub fn replace_theme(
    ///             child: impl UiNode,
    ///             theme: impl IntoVar<ThemeGenerator>
    ///         ) -> impl UiNode {
    ///             with_context_var(child, THEME_VAR, theme)
    ///         }
    ///
    ///         /// Extends the contextual [`THEME_VAR`] with the `theme` override.
    ///         #[property(context, default(ThemeGenerator::nil()))]
    ///         pub fn extend_theme(
    ///             child: impl UiNode,
    ///             theme: impl IntoVar<ThemeGenerator>
    ///         ) -> impl UiNode {
    ///             themable::with_theme_extension(child, THEME_VAR, theme)
    ///         }
    ///     }
    /// }
    /// ```
    pub fn with_theme_extension(
        child: impl UiNode,
        theme_context: ContextVar<ThemeGenerator>,
        extension: impl IntoVar<ThemeGenerator>,
    ) -> impl UiNode {
        with_context_var(
            child,
            theme_context,
            merge_var!(theme_context, extension.into_var(), |base, over| {
                base.clone().with_extend(over.clone())
            }),
        )
    }
}

/// Represents a theme instance.
///
/// Use the [`theme!`] *widget* to instantiate.
///
/// [`theme!`]: mod@theme
#[derive(Default, Debug)]
pub struct Theme {
    node: DynWidgetNode,
}
impl Theme {
    /// Importance of theme properties set by default in theme widgets.
    ///
    /// Is `DynPropImportance::WIDGET - 10`.
    pub const WIDGET_IMPORTANCE: DynPropImportance = DynPropImportance(DynPropImportance::WIDGET.0 - 10);

    /// Importance of theme properties set in theme instances.
    ///
    /// Is `DynPropImportance::INSTANCE - 10`.
    pub const INSTANCE_IMPORTANCE: DynPropImportance = DynPropImportance(DynPropImportance::INSTANCE.0 - 10);

    /// Cast the node to `Theme` if it is the same type.
    ///
    /// Note that each theme constructor function returns `Theme`, so the input child of the next constructor is
    /// `Theme`, unless an override changed a constructor.
    pub fn downcast(node: impl UiNode) -> Option<Theme> {
        let node = node.boxed();
        #[cfg(inspector)]
        let node = crate::core::inspector::unwrap_constructor(node);

        node.downcast_unbox().ok()
    }

    /// Properties and when blocks of this theme.
    pub fn node(&self) -> &DynWidgetNode {
        &self.node
    }

    /// Mutable reference to the properties and when blocks of this theme.
    pub fn node_mut(&mut self) -> &mut DynWidgetNode {
        &mut self.node
    }

    /// Unwrap the theme properties.
    pub fn into_node(self) -> DynWidgetNode {
        self.node
    }

    /// New theme from dynamic widget input.
    ///
    /// The importance index of properties is adjusted, the intrinsic constructor and child nodes are discarded.
    pub fn from_dyn_widget(mut wgt: DynWidget) -> Theme {
        for part in &mut wgt.parts {
            for p in &mut part.properties {
                p.importance = match p.importance {
                    DynPropImportance::WIDGET => Theme::WIDGET_IMPORTANCE,
                    DynPropImportance::INSTANCE => Theme::INSTANCE_IMPORTANCE,
                    custom => custom,
                };
            }
        }
        wgt.into_node(false).into()
    }

    /// New theme from built dynamic widget.
    pub fn from_node(node: DynWidgetNode) -> Theme {
        Self { node }
    }

    /// Overrides `self` with `other`.
    pub fn extend(&mut self, other: Theme) {
        self.node.extend(other.node);
    }
}
#[impl_ui_node(
    delegate = &self.node,
    delegate_mut = &mut self.node,
)]
impl UiNode for Theme {}
impl From<Theme> for DynWidgetNode {
    fn from(t: Theme) -> Self {
        t.into_node()
    }
}
impl From<DynWidgetNode> for Theme {
    fn from(p: DynWidgetNode) -> Self {
        Theme::from_node(p)
    }
}
impl From<DynWidget> for Theme {
    fn from(p: DynWidget) -> Self {
        Theme::from_dyn_widget(p)
    }
}

/// Arguments for [`ThemeGenerator`] closure.
///
/// Currently no arguments.
#[derive(Debug)]
pub struct ThemeArgs {}

/// Boxed shared closure that generates a theme instance for a given widget context.
///
/// You can also use the [`theme_generator!`] macro, it has the advantage of being clone move.
#[derive(Clone)]
pub struct ThemeGenerator(Option<Rc<dyn Fn(&mut WidgetContext, &ThemeArgs) -> Theme>>);
impl Default for ThemeGenerator {
    fn default() -> Self {
        Self::nil()
    }
}
impl ThemeGenerator {
    /// Default generator, produces an empty theme.
    pub fn nil() -> Self {
        Self(None)
    }

    /// If this generator produces an empty theme.
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// New theme generator, the `generate` closure is called for each themable widget, before the widget is inited.
    pub fn new(generate: impl Fn(&mut WidgetContext, &ThemeArgs) -> Theme + 'static) -> Self {
        Self(Some(Rc::new(generate)))
    }

    /// Generate a theme for the themable widget in the context.
    ///
    /// Returns `None` if [`is_nil`], otherwise returns the theme.
    ///
    /// [`is_nil`]: Self::is_nil
    pub fn generate(&self, ctx: &mut WidgetContext, args: &ThemeArgs) -> Option<Theme> {
        self.0.as_ref().map(|g| g(ctx, args))
    }

    /// New theme generator that generates `self` and `other` and then [`extend`] `self` with `other`.
    ///
    /// [`extend`]: Theme::extend
    pub fn with_extend(self, other: ThemeGenerator) -> ThemeGenerator {
        if self.is_nil() {
            other
        } else if other.is_nil() {
            self
        } else {
            ThemeGenerator::new(move |ctx, args| {
                let mut r = self.generate(ctx, args).unwrap();
                r.extend(other.generate(ctx, args).unwrap());
                r
            })
        }
    }
}
impl fmt::Debug for ThemeGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ThemeGenerator(_)")
    }
}

/// <span data-del-macro-root></span> Declares a theme generator closure.
///
/// The output type is a [`ThemeGenerator`], the closure is [`clone_move!`].
///
/// [`clone_move!`]: crate::core::clone_move
#[macro_export]
macro_rules! theme_generator {
    ($($tt:tt)+) => {
        $crate::widgets::theme::ThemeGenerator::new($crate::core::clone_move! {
            $($tt)+
        })
    }
}
#[doc(inline)]
pub use crate::theme_generator;
