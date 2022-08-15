//! Theme building blocks.

use std::{cell::RefCell, fmt, rc::Rc};

use crate::{
    core::{DynProperties, DynPropertiesSnapshot, DynProperty, DynPropertyPriority, DynPropertySource},
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
/// Note that you can declare a custom theme *widget* using the same inheritance mechanism of normal widgets, if you override
/// a constructor function you **must** delegate to the equivalent function defined in [`Theme::new_priority`], note that the
/// dynamic constructors must be used.
///
/// [`themable`]: mod@themable
#[widget($crate::widgets::theme)]
pub mod theme {
    use super::*;

    #[doc(inline)]
    pub use super::{theme_generator, Theme, ThemeGenerator};

    properties! {
        remove { id; visibility; enabled }
    }

    fn new_child() -> impl UiNode {
        Theme::new_child()
    }

    fn new_child_layout_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        Theme::new_priority(child, DynPropertyPriority::ChildLayout, properties)
    }

    fn new_child_context_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        Theme::new_priority(child, DynPropertyPriority::ChildContext, properties)
    }

    fn new_fill_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        Theme::new_priority(child, DynPropertyPriority::Fill, properties)
    }

    fn new_border_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        Theme::new_priority(child, DynPropertyPriority::Border, properties)
    }

    fn new_size_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        Theme::new_priority(child, DynPropertyPriority::Size, properties)
    }

    fn new_layout_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        Theme::new_priority(child, DynPropertyPriority::Layout, properties)
    }

    fn new_event_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        Theme::new_priority(child, DynPropertyPriority::Event, properties)
    }

    fn new_context_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        Theme::new_priority(child, DynPropertyPriority::Context, properties)
    }

    fn new(child: impl UiNode) -> Theme {
        Theme::new(child)
    }
}

/// Themable widget base.
///
/// Widgets that inherit from this one have a `theme` and `theme_pair` property that can be set to a [`ThemeGenerator`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
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
        properties::theme;

        /// Theme generators, dark and light, used for the widget.
        ///
        /// Is `nil` by default.
        properties::theme_pair;
    }

    /// Properties inserted by the mix-in.
    ///
    /// Only the `theme` property is doc visible, the others are implementation details.
    pub mod properties {
        use super::*;

        use crate::core::window::WindowTheme;
        use crate::widgets::window::nodes::WindowThemeVar;

        /// Theme generator pair used for the widget.
        ///
        /// This sets a dark or light theme depending of the [`WindowThemeVar`] property.
        #[property(context, default(ThemeGenerator::nil(), ThemeGenerator::nil()))]
        pub fn theme_pair(
            child: impl UiNode,
            dark_theme: impl IntoVar<ThemeGenerator>,
            light_theme: impl IntoVar<ThemeGenerator>,
        ) -> impl UiNode {
            theme(
                child,
                merge_var!(WindowThemeVar::new(), dark_theme.into_var(), light_theme.into_var(), |w, d, l| {
                    match w {
                        WindowTheme::Dark => d.clone(),
                        WindowTheme::Light => l.clone(),
                    }
                }),
            )
        }

        /// Theme generator used for the widget.
        ///
        /// Properties and `when` conditions in the generated theme are applied to the widget as
        /// if they where set on it. Note that changing the theme causes the widget info tree to rebuild,
        /// prefer property binding and `when` conditions to cause visual changes that happen often.
        #[property(context, default(ThemeGenerator::nil()))]
        pub fn theme(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
            struct ThemableContextNode<C, T> {
                child: C,
                theme: T,
                actual_theme: ActualTheme,
            }
            impl<C, T> ThemableContextNode<C, T> {
                fn with_mut<R>(&mut self, vars: &Vars, f: impl FnOnce(&mut C) -> R) -> R {
                    vars.with_context_var(ActualThemeVar, ContextVarData::fixed(&self.actual_theme), || f(&mut self.child))
                }

                fn with<R>(&self, vars: &VarsRead, f: impl FnOnce(&C) -> R) -> R {
                    vars.with_context_var(ActualThemeVar, ContextVarData::fixed(&self.actual_theme), || f(&self.child))
                }
            }
            impl<C, T> UiNode for ThemableContextNode<C, T>
            where
                C: UiNode,
                T: Var<ThemeGenerator>,
            {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    self.actual_theme = ActualTheme {
                        widget_id: Some(ctx.path.widget_id()),
                        parts: self
                            .theme
                            .get(ctx.vars)
                            .generate(ctx)
                            .map(|t| t.split_priority())
                            .unwrap_or_default(),
                    };

                    self.with_mut(ctx.vars, |c| {
                        c.init(ctx);
                    })
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.with_mut(ctx.vars, |c| {
                        c.deinit(ctx);
                    });
                    self.actual_theme = ActualTheme::default();
                }

                fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                    self.with(ctx.vars, |c| {
                        c.info(ctx, info);
                    })
                }

                fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                    subs.var(ctx, &self.theme);

                    self.with(ctx.vars, |c| {
                        c.subscriptions(ctx, subs);
                    })
                }

                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    self.with_mut(ctx.vars, |c| {
                        c.event(ctx, args);
                    })
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if let Some(theme) = self.theme.get_new(ctx.vars) {
                        let actual_theme = ActualTheme {
                            widget_id: Some(ctx.path.widget_id()),
                            parts: theme.generate(ctx).map(|t| t.split_priority()).unwrap_or_default(),
                        };

                        if self.actual_theme.is_some() || actual_theme.is_some() {
                            self.child.deinit(ctx);
                            self.actual_theme = actual_theme;
                            self.child.init(ctx);

                            ctx.updates.info_layout_and_render();
                        }
                    } else {
                        self.with_mut(ctx.vars, |c| {
                            c.update(ctx);
                        })
                    }
                }

                fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                    self.with(ctx.vars, |c| c.measure(ctx))
                }

                fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                    self.with_mut(ctx.vars, |c| c.layout(ctx, wl))
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    self.with(ctx.vars, |c| c.render(ctx, frame));
                }

                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    self.with(ctx.vars, |c| c.render_update(ctx, update));
                }
            }

            ThemableContextNode {
                child,
                theme: theme.into_var(),
                actual_theme: ActualTheme::default(),
            }
        }

        /// The `new_*_dyn` constructors delegates to this.
        pub fn new_priority(child: impl UiNode, priority: DynPropertyPriority, properties: Vec<DynProperty>) -> impl UiNode {
            struct ThemableNode {
                wgt_snapshot: Option<DynPropertiesSnapshot>,
                properties: DynProperties,
                priority: DynPropertyPriority,
            }
            #[impl_ui_node(
                delegate = &self.properties,
                delegate_mut = &mut self.properties,
            )]
            impl UiNode for ThemableNode {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let theme = ActualThemeVar::get(ctx.vars);
                    if theme.widget_id == Some(ctx.path.widget_id()) {
                        let theme = theme.parts[self.priority as usize].borrow_mut().take();
                        if let Some(theme) = theme {
                            if !theme.is_empty() {
                                self.wgt_snapshot = Some(self.properties.snapshot());
                                self.properties.insert_all(theme, DynPropertySource::Widget);
                            }
                        }
                    }

                    self.properties.init(ctx);
                }
                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.properties.deinit(ctx);
                    if let Some(snap) = self.wgt_snapshot.take() {
                        self.properties.restore(snap);
                    }
                }
            }

            let mut properties = DynProperties::new(priority, properties);
            properties.replace_child(child.boxed());

            ThemableNode {
                properties,
                priority,
                wgt_snapshot: None,
            }
        }
    }
}

/// Represents a theme instance.
///
/// Use the [`theme!`] *widget* to instantiate.
///
/// [`theme!`]: mod@theme
#[derive(Default)]
pub struct Theme {
    properties: DynProperties,
}
impl Theme {
    /// Cast the node to `Theme` if it is the same type.
    ///
    /// Note that each theme constructor function returns `Theme`, so the input child of the next constructor is
    /// `Theme`, unless an override changed a constructor.
    pub fn downcast(node: impl UiNode) -> Option<Theme> {
        node.downcast_unbox().ok()
    }

    /// Default `theme::new_child` constructor.
    pub fn new_child() -> Theme {
        Self::default()
    }

    /// Default `theme::new_*_dyn` constructor.
    pub fn new_priority(child: impl UiNode, priority: DynPropertyPriority, properties: Vec<DynProperty>) -> Theme {
        let mut theme = Self::downcast(child).unwrap_or_else(|| {
            tracing::error!("theme constructor function for `{priority:?}` did not receive a `Theme` in the first parameter");
            Theme::default()
        });
        theme.properties.insert(priority, properties, DynPropertySource::Widget);
        theme
    }

    /// Default `theme::new` constructor.
    pub fn new(child: impl UiNode) -> Theme {
        Self::downcast(child).unwrap_or_else(|| {
            tracing::error!("theme constructor function `new` did not receive a `Theme` in the first parameter");
            Theme::default()
        })
    }

    fn split_priority(self) -> [RefCell<Option<DynProperties>>; DynPropertyPriority::LEN] {
        let mut parts = self.properties.split_priority().into_iter();
        std::array::from_fn(|_| RefCell::new(Some(parts.next().unwrap())))
    }
}
#[impl_ui_node(
    delegate = &self.properties,
    delegate_mut = &mut self.properties,
)]
impl UiNode for Theme {}

/// Boxed shared closure that generates a theme instance for a given widget context.
///
/// You can also use the [`theme_generator!`] macro, it has the advantage of being clone move.
#[derive(Clone)]
pub struct ThemeGenerator(Option<Rc<dyn Fn(&mut WidgetContext) -> Theme>>);
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
    pub fn new(generate: impl Fn(&mut WidgetContext) -> Theme + 'static) -> Self {
        Self(Some(Rc::new(generate)))
    }

    /// Generate a theme for the themable widget in the context.
    ///
    /// Returns `None` if [`is_nil`], otherwise returns the theme.
    ///
    /// [`is_nil`]: Self::is_nil
    pub fn generate(&self, ctx: &mut WidgetContext) -> Option<Theme> {
        self.0.as_ref().map(|g| g(ctx))
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

context_var! {
    struct ActualThemeVar: ActualTheme = ActualTheme::default();
}

#[derive(Default)]
struct ActualTheme {
    widget_id: Option<WidgetId>,

    parts: [RefCell<Option<DynProperties>>; DynPropertyPriority::LEN],
}
impl ActualTheme {
    fn is_some(&self) -> bool {
        for part in &self.parts {
            if let Some(part) = &*part.borrow() {
                if !part.is_empty() {
                    return true;
                }
            }
        }
        false
    }
}
impl Clone for ActualTheme {
    fn clone(&self) -> Self {
        // need clone to be `VarValue`, but we only use this type in
        // `ActualThemesVar` that we control and don't clone.
        unreachable!()
    }
}
impl fmt::Debug for ActualTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActualTheme")
            .field("widget_id", &self.widget_id)
            .finish_non_exhaustive()
    }
}
