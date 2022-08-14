//! Theme building blocks.

use std::{cell::RefCell, fmt, mem, rc::Rc};

use crate::{core::NilUiNode, prelude::new_widget::*};

/// Represents a property bundle that can be applied to any [`themable`] widget.
///
/// This *widget* can be instantiated using the same syntax as any widget, but it produces a [`Theme`]
/// instance instead of an widget. Widgets that inherit from [`themable`] can be modified using properties
/// defined in a theme, the properties are dynamically spliced into each widget instance.
///
/// Themes must only visually affect the themed widget, this is a semantic distinction only, any property can be set
/// in a theme, so feel free to setup event handlers in themes, but only if they are used to affect the widget visually.
///
/// [`themable`]: mod@themable
#[widget($crate::widgets::theme)]
pub mod theme {
    use super::*;

    fn new_child() -> impl UiNode {
        struct ThemeChildNode {
            child: Rc<RefCell<BoxedUiNode>>,
        }
        #[impl_ui_node(
            delegate = self.child.borrow(),
            delegate_mut = self.child.borrow_mut(),
        )]
        impl UiNode for ThemeChildNode {
            fn init(&mut self, ctx: &mut WidgetContext) {
                if let Some(theme) = ctx.widget_state.get_mut(ThemeKey) {
                    // theme init.
                    self.child = theme.child_layout.child.clone();
                } else {
                    // themable init.
                    self.child.borrow_mut().init(ctx);
                }
            }
        }
        ThemeChildNode {
            child: Rc::new(RefCell::new(NilUiNode.boxed())),
        }
    }

    macro_rules! theme_node {
        ($Node:ident { $node:ident, $child:ident, $properties:ident, }) => {
            struct $Node {
                child: Rc<RefCell<BoxedUiNode>>,
                properties: Option<BoxedUiNode>,
            }
            #[impl_ui_node(delegate = self.child.borrow(), delegate_mut = self.child.borrow_mut())]
            impl UiNode for $Node {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    if let Some(theme) = ctx.widget_state.get_mut(ThemeKey) {
                        // theme init.
                        self.child = theme.$child.child.clone();
                        *theme.$properties.properties.borrow_mut() = self.properties.take();
                    } else {
                        // themable init.
                        self.child.borrow_mut().init(ctx);
                    }
                }
            }
            $Node {
                child: Rc::new(RefCell::new(NilUiNode.boxed())),
                properties: Some($node.boxed()),
            }
        };
    }

    fn new_child_layout(child: impl UiNode) -> impl UiNode {
        theme_node! {
            ThemeChildLayoutNode {
                child,
                child_context,
                child_layout,
            }
        }
    }

    fn new_child_context(child: impl UiNode) -> impl UiNode {
        theme_node! {
            ThemeChildContextNode {
                child,
                fill,
                child_context,
            }
        }
    }

    fn new_fill(child: impl UiNode) -> impl UiNode {
        theme_node! {
            ThemeChildContextNode {
                child,
                border,
                fill,
            }
        }
    }

    fn new_border(child: impl UiNode) -> impl UiNode {
        theme_node! {
            ThemeChildContextNode {
                child,
                size,
                border,
            }
        }
    }

    fn new_size(child: impl UiNode) -> impl UiNode {
        theme_node! {
            ThemeChildContextNode {
                child,
                layout,
                size,
            }
        }
    }

    fn new_layout(child: impl UiNode) -> impl UiNode {
        theme_node! {
            ThemeLayoutNode {
                child,
                event,
                layout,
            }
        }
    }

    fn new_event(child: impl UiNode) -> impl UiNode {
        theme_node! {
            ThemeEventNode {
                child,
                context,
                event,
            }
        }
    }

    fn new_context(child: impl UiNode) -> impl UiNode {
        struct ThemeContextNode {
            properties: Option<BoxedUiNode>,
        }
        #[impl_ui_node(none)]
        impl UiNode for ThemeContextNode {
            fn init(&mut self, ctx: &mut WidgetContext) {
                if let Some(theme) = ctx.widget_state.get_mut(ThemeKey) {
                    // theme init.
                    *theme.context.properties.borrow_mut() = self.properties.take();
                } else {
                    unreachable!()
                }
            }
        }
        ThemeContextNode {
            properties: Some(child.boxed()),
        }
    }

    fn new(child: impl UiNode) -> Theme {
        Theme {
            init: Some(child.boxed()),
            theme: ThemeProperties::default(),
        }
    }

    pub use super::{theme_generator, Theme, ThemeGenerator};
}

/// Widget base that can by dynamically styled by a [`Theme`].
#[widget($crate::widgets::themable)]
pub mod themable {
    use super::*;

    use implicit_base::nodes as base_nodes;

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

    fn new_child_layout(child: impl UiNode) -> impl UiNode {
        let child = nodes::insert_child_layout(child);
        base_nodes::child_layout(child)
    }

    fn new_child_context(child: impl UiNode) -> impl UiNode {
        nodes::insert_child_context(child)
    }

    fn new_fill(child: impl UiNode) -> impl UiNode {
        nodes::insert_fill(child)
    }

    fn new_border(child: impl UiNode) -> impl UiNode {
        let child = nodes::insert_border(child);
        base_nodes::inner(child)
    }

    fn new_size(child: impl UiNode) -> impl UiNode {
        nodes::insert_size(child)
    }

    fn new_layout(child: impl UiNode) -> impl UiNode {
        nodes::insert_layout(child)
    }

    fn new_event(child: impl UiNode) -> impl UiNode {
        nodes::insert_event(child)
    }

    fn new_context(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        let child = nodes::insert_context(child);
        nodes::generate_theme(child, theme)
    }

    pub use super::nodes;
}

struct ThemePriority {
    child: Rc<RefCell<BoxedUiNode>>,
    properties: RefCell<Option<BoxedUiNode>>,
}
impl Default for ThemePriority {
    fn default() -> Self {
        Self {
            child: Rc::new(RefCell::new(NilUiNode.boxed())),
            properties: RefCell::new(None),
        }
    }
}

#[derive(Default)]
struct ThemeProperties {
    child_layout: ThemePriority,
    child_context: ThemePriority,
    fill: ThemePriority,
    border: ThemePriority,
    size: ThemePriority,
    layout: ThemePriority,
    event: ThemePriority,
    context: ThemePriority,
}
state_key! {
    struct ThemeKey: ThemeProperties;
}

/// Represents a theme instance.
///
/// Use the [`theme!`] *widget* to instantiate.
///
/// [`theme!`]: mod@theme
pub struct Theme {
    init: Option<BoxedUiNode>,
    theme: ThemeProperties,
}
impl Theme {
    fn init(&mut self, ctx: &mut WidgetContext) {
        if let Some(mut builder) = self.init.take() {
            let mut state = OwnedStateMap::new();
            state.borrow_mut().set(ThemeKey, ThemeProperties::default());
            ctx.widget_context(ctx.path.widget_id(), ctx.widget_info, &mut state, |ctx| {
                builder.init(ctx);
            });
            self.theme = state.remove(ThemeKey).unwrap();
        }
    }
}

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
        if let Some(generate) = &self.0 {
            let mut theme = generate(ctx);
            theme.init(ctx);
            Some(theme)
        } else {
            None
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
    theme: Option<Theme>,
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

/// Nodes for building themable widgets.
pub mod nodes {
    use super::*;

    macro_rules! insert_node {
        ($Node:ident { $wgt_child:ident, $priority:ident, }) => {
            struct $Node {
                wgt_child: Option<Rc<RefCell<BoxedUiNode>>>,
                child: BoxedUiNode,
            }
            #[impl_ui_node(child)]
            impl UiNode for $Node {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    debug_assert!(self.wgt_child.is_none());

                    let t = ActualThemeVar::get(ctx.vars);
                    if let (Some(id), Some(theme)) = (t.widget_id, &t.theme) {
                        if id == ctx.path.widget_id() {
                            // widget is themed, insert the theme

                            let t = &theme.theme.$priority;

                            if let Some(properties) = t.properties.borrow_mut().take() {
                                // child becomes the properties
                                let prev_child = mem::replace(&mut self.child, properties);
                                // prev_child becomes the theme child
                                *t.child.borrow_mut() = prev_child;

                                // preserve the original widget child so that we can remove the theme on deinit.
                                self.wgt_child = Some(t.child.clone());
                            }
                        }
                    }

                    self.child.init(ctx);
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.child.deinit(ctx);

                    if let Some(w) = self.wgt_child.take() {
                        // restore to not-themed state, where child is the widget child.
                        self.child = mem::replace(&mut *w.borrow_mut(), NilUiNode.boxed());
                    }
                }
            }
            $Node {
                child: $wgt_child.boxed(),
                wgt_child: None,
            }
        };
    }

    /// Insert the *child-layout* priority properties from the theme.
    pub fn insert_child_layout(child: impl UiNode) -> impl UiNode {
        insert_node! {
            InsertChildLayoutNode {
                child,
                child_layout,
            }
        }
    }

    /// Insert the *child-context* priority properties from the theme.
    pub fn insert_child_context(child: impl UiNode) -> impl UiNode {
        insert_node! {
            InsertChildContextNode {
                child,
                child_context,
            }
        }
    }

    /// Insert the *fill* priority properties from the theme.
    pub fn insert_fill(child: impl UiNode) -> impl UiNode {
        insert_node! {
            InsertFillNode {
                child,
                fill,
            }
        }
    }

    /// Insert the *border* priority properties from the theme.
    pub fn insert_border(child: impl UiNode) -> impl UiNode {
        insert_node! {
            InsertBorderNode {
                child,
                border,
            }
        }
    }

    /// Insert the *size* priority properties from the theme.
    pub fn insert_size(child: impl UiNode) -> impl UiNode {
        insert_node! {
            InsertSizeNode {
                child,
                size,
            }
        }
    }

    /// Insert the *layout* priority properties from the theme.
    pub fn insert_layout(child: impl UiNode) -> impl UiNode {
        insert_node! {
            InsertLayoutNode {
                child,
                layout,
            }
        }
    }

    /// Insert the *event* priority properties from the theme.
    pub fn insert_event(child: impl UiNode) -> impl UiNode {
        insert_node! {
            InsertEventNode {
                child,
                event,
            }
        }
    }

    /// Insert the *context* priority properties from the theme.
    pub fn insert_context(child: impl UiNode) -> impl UiNode {
        insert_node! {
            InsertContextNode {
                child,
                context,
            }
        }
    }

    /// Generate the theme for the widget.
    pub fn generate_theme(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
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
                    theme: self.theme.get(ctx.vars).generate(ctx),
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
                        theme: theme.generate(ctx),
                    };

                    if self.actual_theme.theme.is_some() || actual_theme.theme.is_some() {
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
}
