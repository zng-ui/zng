//! Theme building blocks.

use std::{cell::RefCell, fmt, rc::Rc};

use crate::{
    core::{DynPropImportance, DynPropPriority, DynProperties, DynPropertiesSnapshot, DynProperty},
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

    use crate::core::window::WindowTheme;
    use crate::widgets::window::nodes::WindowThemeVar;

    #[doc(inline)]
    pub use super::{theme_generator, Theme, ThemeGenerator};

    properties! {
        remove { id; visibility; enabled }
    }

    /// Theme `new_child`.
    ///
    /// Returns the [`Theme`] that will be modified by the other widget constructor functions.
    pub fn new_child() -> Theme {
        Theme::default()
    }

    /// Theme `new_child_layout_dyn`.
    ///
    /// Returns the [`Theme`] with the `properties` inserted.
    fn new_child_layout_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> Theme {
        nodes::new_priority(child, DynPropPriority::ChildLayout, properties)
    }

    /// Theme `new_child_context_dyn`.
    ///
    /// Returns the [`Theme`] with the `properties` inserted.
    fn new_child_context_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> Theme {
        nodes::new_priority(child, DynPropPriority::ChildContext, properties)
    }

    /// Theme `new_fill_dyn`.
    ///
    /// Returns the [`Theme`] with the `properties` inserted.
    fn new_fill_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> Theme {
        nodes::new_priority(child, DynPropPriority::Fill, properties)
    }

    /// Theme `new_border_dyn`.
    ///
    /// Returns the [`Theme`] with the `properties` inserted.
    fn new_border_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> Theme {
        nodes::new_priority(child, DynPropPriority::Border, properties)
    }

    /// Theme `new_size_dyn`.
    ///
    /// Returns the [`Theme`] with the `properties` inserted.
    fn new_size_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> Theme {
        nodes::new_priority(child, DynPropPriority::Size, properties)
    }

    /// Theme `new_layout_dyn`.
    ///
    /// Returns the [`Theme`] with the `properties` inserted.
    fn new_layout_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> Theme {
        nodes::new_priority(child, DynPropPriority::Layout, properties)
    }

    /// Theme `new_event_dyn`.
    ///
    /// Returns the [`Theme`] with the `properties` inserted.
    fn new_event_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> Theme {
        nodes::new_priority(child, DynPropPriority::Event, properties)
    }

    /// Theme `new_context_dyn`.
    ///
    /// Returns the [`Theme`] with the `properties` inserted.
    fn new_context_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> Theme {
        nodes::new_priority(child, DynPropPriority::Context, properties)
    }

    /// Theme `new_child`.
    ///
    /// Finishes the
    pub fn new(child: impl UiNode) -> Theme {
        Theme::downcast(child).expect("expected `Theme` node in `new` constructor")
    }

    /// Declare a dark and light theme that is selected depending on the window theme.
    ///
    /// This is a [`merge_var!`] that matches the [`WindowThemeVar`] to select the theme.
    pub fn pair(dark_theme: impl IntoVar<ThemeGenerator>, light_theme: impl IntoVar<ThemeGenerator>) -> impl Var<ThemeGenerator> {
        merge_var!(WindowThemeVar::new(), dark_theme.into_var(), light_theme.into_var(), |w, d, l| {
            match w {
                WindowTheme::Dark => d.clone(),
                WindowTheme::Light => l.clone(),
            }
        })
    }

    /// Nodes used for building the theme.
    pub mod nodes {
        use super::*;

        /// Default `theme::new_*_dyn` constructor.
        pub fn new_priority(child: impl UiNode, priority: DynPropPriority, mut properties: Vec<DynProperty>) -> Theme {
            let mut theme = Theme::downcast(child).unwrap_or_else(|| {
                tracing::error!("expected `Theme` node in `{priority:?}` constructor");
                Theme::default()
            });
            for p in &mut properties {
                p.importance = match p.importance {
                    DynPropImportance::WIDGET => Theme::WIDGET_IMPORTANCE,
                    DynPropImportance::INSTANCE => Theme::INSTANCE_IMPORTANCE,
                    custom => custom,
                };
            }
            theme.properties.insert(priority, properties);
            theme
        }
    }
}

/// Themable widget base.
///
/// Widgets that inherit from this one have a `theme` property that can be set to a [`ThemeGenerator`]
/// that generates properties that are dynamically injected into the widget to alter its appearance.
///
/// You can also use the [`theme::pair`] to set `theme` to two themes, dark and light, that is selected according
/// to the system or window preference.
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

    /// Themable `new_child_layout_dyn`.
    ///
    /// Returns [`implicit_base::new_child`].
    pub fn new_child() -> impl UiNode {
        implicit_base::new_child()
    }

    /// Themable `new_child_layout_dyn`.
    ///
    /// Introduces the [`nodes::insert_priority`] node for [`DynPropPriority::ChildLayout`] and returns [`implicit_base::new_child_layout`].
    pub fn new_child_layout_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        let child = nodes::insert_priority(child, DynPropPriority::ChildLayout, properties);
        implicit_base::new_child_layout(child)
    }

    /// Themable `new_child_context_dyn`.
    ///
    /// Introduces the [`nodes::insert_priority`] node for [`DynPropPriority::ChildContext`] and returns [`implicit_base::new_child_context`].
    pub fn new_child_context_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        let child = nodes::insert_priority(child, DynPropPriority::ChildContext, properties);
        implicit_base::new_child_context(child)
    }

    /// Themable `new_fill_dyn`.
    ///
    /// Introduces the [`nodes::insert_priority`] node for [`DynPropPriority::Fill`] and returns [`implicit_base::new_fill`].
    pub fn new_fill_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        let child = nodes::insert_priority(child, DynPropPriority::Fill, properties);
        implicit_base::new_fill(child)
    }

    /// Themable `new_border_dyn`.
    ///
    /// Introduces the [`nodes::insert_priority`] node for [`DynPropPriority::Border`] and returns [`implicit_base::new_border`].
    pub fn new_border_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        let child = nodes::insert_priority(child, DynPropPriority::Border, properties);
        implicit_base::new_border(child)
    }

    /// Themable `new_size_dyn`.
    ///
    /// Introduces the [`nodes::insert_priority`] node for [`DynPropPriority::Size`] and returns [`implicit_base::new_size`].
    pub fn new_size_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        let child = nodes::insert_priority(child, DynPropPriority::Size, properties);
        implicit_base::new_size(child)
    }

    /// Themable `new_layout_dyn`.
    ///
    /// Introduces the [`nodes::insert_priority`] node for [`DynPropPriority::Layout`] and returns [`implicit_base::new_layout`].
    pub fn new_layout_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        let child = nodes::insert_priority(child, DynPropPriority::Layout, properties);
        implicit_base::new_layout(child)
    }

    /// Themable `new_event_dyn`.
    ///
    /// Introduces the [`nodes::insert_priority`] node for [`DynPropPriority::Event`] and returns [`implicit_base::new_event`].
    pub fn new_event_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        let child = nodes::insert_priority(child, DynPropPriority::Event, properties);
        implicit_base::new_event(child)
    }

    /// Themable `new_context_dyn`.
    ///
    /// Introduces the [`nodes::insert_priority`] node for [`DynPropPriority::Context`] and returns [`implicit_base::new_context`].
    pub fn new_context_dyn(child: impl UiNode, properties: Vec<DynProperty>) -> impl UiNode {
        let child = nodes::insert_priority(child, DynPropPriority::Context, properties);
        implicit_base::new_context(child)
    }

    /// Themable `new`, captures the `id` and `theme` properties.
    ///
    /// Introduces the [`nodes::generate_theme`] and returns [`implicit_base::new`].
    pub fn new(child: impl UiNode, id: impl IntoValue<WidgetId>, theme: impl IntoVar<ThemeGenerator>) -> impl Widget {
        let child = nodes::generate_theme(child, theme);
        implicit_base::new(child, id)
    }

    /// Nodes used for building the themable.
    pub mod nodes {
        use super::*;

        /// Generates the theme that is used by the [`insert_priority`] nodes on the same widget.
        pub fn generate_theme(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
            struct GenerateThemeNode<C, T> {
                child: C,
                theme: T,
                actual_theme: ActualTheme,
            }
            impl<C, T> GenerateThemeNode<C, T> {
                fn with_mut<R>(&mut self, vars: &Vars, f: impl FnOnce(&mut C) -> R) -> R {
                    vars.with_context_var(ActualThemeVar, ContextVarData::fixed(&self.actual_theme), || f(&mut self.child))
                }

                fn with<R>(&self, vars: &VarsRead, f: impl FnOnce(&C) -> R) -> R {
                    vars.with_context_var(ActualThemeVar, ContextVarData::fixed(&self.actual_theme), || f(&self.child))
                }
            }
            impl<C, T> UiNode for GenerateThemeNode<C, T>
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
                            .generate(ctx, &ThemeArgs {})
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
                            parts: theme.generate(ctx, &ThemeArgs {}).map(|t| t.split_priority()).unwrap_or_default(),
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

            GenerateThemeNode {
                child,
                theme: theme.into_var(),
                actual_theme: ActualTheme::default(),
            }
        }

        /// Inserts the theme properties for the priority.
        pub fn insert_priority(child: impl UiNode, priority: DynPropPriority, properties: Vec<DynProperty>) -> impl UiNode {
            struct ThemableNode {
                wgt_snapshot: Option<DynPropertiesSnapshot>,
                properties: DynProperties,
                priority: DynPropPriority,
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
                                self.properties.insert_all(theme);
                            }
                        }
                    }

                    self.properties.init(ctx);
                }
                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.properties.deinit(ctx);
                    if let Some(snap) = self.wgt_snapshot.take() {
                        self.properties.restore(snap).unwrap();
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
#[derive(Default, Debug)]
pub struct Theme {
    properties: DynProperties,
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
        let node = crate::core::inspector::unwrap_new_fn(node);

        node.downcast_unbox().ok()
    }

    /// Properties of this theme.
    pub fn properties(&self) -> &DynProperties {
        &self.properties
    }

    /// Mutable reference to the properties of this theme.
    pub fn properties_mut(&mut self) -> &mut DynProperties {
        &mut self.properties
    }

    /// Unwrap the theme properties.
    pub fn into_properties(self) -> DynProperties {
        self.properties
    }

    /// New theme from dynamic properties.
    pub fn from_properties(properties: DynProperties) -> Theme {
        Self { properties }
    }

    /// Overrides `self` with `other`.
    pub fn insert_all(&mut self, other: Theme) {
        self.properties.insert_all(other.properties);
    }

    fn split_priority(self) -> [RefCell<Option<DynProperties>>; DynPropPriority::LEN] {
        let mut parts = self.properties.split_priority().into_iter();
        std::array::from_fn(|_| RefCell::new(Some(parts.next().unwrap())))
    }
}
#[impl_ui_node(
    delegate = &self.properties,
    delegate_mut = &mut self.properties,
)]
impl UiNode for Theme {}
impl From<Theme> for DynProperties {
    fn from(t: Theme) -> Self {
        t.into_properties()
    }
}
impl From<DynProperties> for Theme {
    fn from(p: DynProperties) -> Self {
        Theme::from_properties(p)
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

    /// New theme generator that generates `self` overridden with `other`.
    pub fn with_override(self, other: ThemeGenerator) -> ThemeGenerator {
        if self.is_nil() {
            other
        } else if other.is_nil() {
            self
        } else {
            ThemeGenerator::new(move |ctx, args| {
                let mut r = self.generate(ctx, args).unwrap();
                r.insert_all(other.generate(ctx, args).unwrap());
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

context_var! {
    struct ActualThemeVar: ActualTheme = ActualTheme::default();
}

#[derive(Default)]
struct ActualTheme {
    widget_id: Option<WidgetId>,

    parts: [RefCell<Option<DynProperties>>; DynPropPriority::LEN],
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
        let parts = self.parts.iter().map(|p| p.borrow()).collect::<Vec<_>>();
        let parts = parts.iter().map(|p| &**p).collect::<Vec<_>>();
        let parts = <[&Option<DynProperties>; DynPropPriority::LEN]>::try_from(parts).unwrap();
        f.debug_struct("ActualTheme")
            .field("widget_id", &self.widget_id)
            .field("parts", &parts)
            .finish_non_exhaustive()
    }
}
