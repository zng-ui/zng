//! Theme building blocks.

use std::{cell::RefCell, fmt, rc::Rc};

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
            #[impl_ui_node(
                            delegate = self.child.borrow(),
                            delegate_mut = self.child.borrow_mut(),
                        )]
            impl UiNode for $Node {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    if let Some(theme) = ctx.widget_state.get_mut(ThemeKey) {
                        // theme init.
                        self.child = theme.$child.child.clone();
                        theme.$properties.properties = self.properties.take().unwrap();
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
                    theme.context.properties = self.properties.take().unwrap();
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

    pub use super::{Theme, ThemeGenerator};
}

/// Widget base that can by dynamically styled by a [`Theme`].
#[widget($crate::widgets::themable)]
pub mod themable {
    use super::*;

    use implicit_base::nodes as base_nodes;

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

    fn new_context(child: impl UiNode) -> impl UiNode {
        let child = nodes::insert_context(child);
        nodes::generate_themes(child, ThemeVar)
    }

    pub use super::{nodes, ThemeVar, Themes};
}

struct ThemePriority {
    child: Rc<RefCell<BoxedUiNode>>,
    properties: BoxedUiNode,
}
impl Default for ThemePriority {
    fn default() -> Self {
        Self {
            child: Rc::new(RefCell::new(NilUiNode.boxed())),
            properties: NilUiNode.boxed(),
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
#[derive(Clone)]
pub struct ThemeGenerator(Rc<dyn Fn(&mut InfoContext) -> Option<Theme>>);
impl Default for ThemeGenerator {
    fn default() -> Self {
        Self::nil()
    }
}
impl ThemeGenerator {
    /// Default generator, never produces a theme.
    pub fn nil() -> Self {
        Self::new(|_| None)
    }

    /// New theme generator, the `generate` closure is called for each potential themable widget, if it returns some
    /// theme it is applied on the themable.
    pub fn new(generate: impl Fn(&mut InfoContext) -> Option<Theme> + 'static) -> Self {
        Self(Rc::new(generate))
    }

    /// Generate a theme for the themable widget in the context.
    ///
    /// Returns `None` if no theme is provided for the widget.
    pub fn generate(&self, ctx: &mut InfoContext) -> Option<Theme> {
        (self.0)(ctx)
    }
}
impl fmt::Debug for ThemeGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ThemeGenerator(Rc<dyn Fn(&mut InfoContext) -> Option<Theme>>)")
    }
}

context_var! {
    /// Theme generators available in a context.
    ///
    /// Note that themable widgets can have custom theme tracking vars.
    pub struct ThemeVar: Vec<ThemeGenerator> = vec![];

    struct ActualThemesVar: ActualThemes = ActualThemes::default();
}

#[derive(Debug, Default)]
struct ActualThemes {
    themes: Themes,
}
impl Clone for ActualThemes {
    fn clone(&self) -> Self {
        // need clone to be `VarValue`, but we only use this type in
        // `ActualThemesVar` that we control and don't clone.
        unreachable!()
    }
}

/// Themes instances generated for an widget.
pub struct Themes {
    widget_id: Option<WidgetId>,
    themes: Vec<Theme>,
}
impl Default for Themes {
    fn default() -> Self {
        Themes::new()
    }
}
impl Themes {
    /// New default empty.
    pub fn new() -> Themes {
        Themes {
            widget_id: None,
            themes: vec![],
        }
    }

    /// Themable widget that is applying these themes.
    pub fn widget_id(&self) -> Option<WidgetId> {
        self.widget_id
    }

    /// If any theme was generated for the widget.
    pub fn is_any(&self) -> bool {
        self.themes.is_empty()
    }

    /// Gets the themes selected for the themable context.
    pub fn get(vars: &impl AsRef<VarsRead>) -> &Themes {
        &ActualThemesVar::get(vars).themes
    }
}
impl fmt::Debug for Themes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Themes")
            .field("widget_id", &self.widget_id)
            .field("is_any", &self.is_any())
            .finish_non_exhaustive()
    }
}

/// Nodes for building themable widgets.
pub mod nodes {
    use super::*;

    /// Merge the `theme` into the `themes` context.
    ///
    /// The `themes` can be the [`ThemeVar`] or a custom var.
    pub fn merge_theme<T: ContextVar<Type = Vec<ThemeGenerator>>>(
        child: impl UiNode,
        themes: T,
        theme: impl IntoVar<ThemeGenerator>,
    ) -> impl UiNode {
        with_context_var(
            child,
            themes,
            merge_var!(T::new(), theme.into_var(), |themes, theme| {
                let mut themes = themes.clone();
                themes.push(theme.clone());
                themes
            }),
        )
    }

    /// Insert the *child-layout* priority properties from the theme.
    pub fn insert_child_layout(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Insert the *child-context* priority properties from the theme.
    pub fn insert_child_context(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Insert the *fill* priority properties from the theme.
    pub fn insert_fill(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Insert the *border* priority properties from the theme.
    pub fn insert_border(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Insert the *size* priority properties from the theme.
    pub fn insert_size(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Insert the *layout* priority properties from the theme.
    pub fn insert_layout(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Insert the *event* priority properties from the theme.
    pub fn insert_event(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Insert the *context* priority properties from the theme.
    pub fn insert_context(child: impl UiNode) -> impl UiNode {
        struct ThemableInsertContext {
            child: BoxedUiNode,
        }
        #[impl_ui_node(child)]
        impl UiNode for ThemableInsertContext {
            fn init(&mut self, ctx: &mut WidgetContext) {
                let themes = Themes::get(ctx.vars);
                self.child.init(ctx);
            }
        }
        ThemableInsertContext { child: child.boxed() }
    }

    /// Generate the themes for the widget.
    ///
    /// The `themes` can be the [`ThemeVar`] or a custom var. This node (re)init the `child` with
    /// the theme instances available in [`Themes`] every time the `themes` updates.
    pub fn generate_themes<T: ContextVar<Type = Vec<ThemeGenerator>>>(child: impl UiNode, themes: T) -> impl UiNode {
        let _ = themes;

        struct ThemableContextNode<C, T>
        where
            T: ContextVar<Type = Vec<ThemeGenerator>>,
        {
            child: C,
            themes: ContextVarProxy<T>,
            actual_themes: ActualThemes,
        }
        impl<C, T> ThemableContextNode<C, T>
        where
            C: UiNode,
            T: ContextVar<Type = Vec<ThemeGenerator>>,
        {
            fn with_mut<R>(&mut self, vars: &Vars, f: impl FnOnce(&mut C) -> R) -> R {
                vars.with_context_var(ActualThemesVar, ContextVarData::fixed(&self.actual_themes), || f(&mut self.child))
            }

            fn with<R>(&self, vars: &VarsRead, f: impl FnOnce(&C) -> R) -> R {
                vars.with_context_var(ActualThemesVar, ContextVarData::fixed(&self.actual_themes), || f(&self.child))
            }
        }
        impl<C, T> UiNode for ThemableContextNode<C, T>
        where
            C: UiNode,
            T: ContextVar<Type = Vec<ThemeGenerator>>,
        {
            fn init(&mut self, ctx: &mut WidgetContext) {
                let themes = self.themes.get(ctx.vars);
                let mut info = ctx.as_info();
                let mut ts = vec![];
                for theme in themes {
                    if let Some(t) = theme.generate(&mut info) {
                        ts.push(t);
                    }
                }
                self.actual_themes = ActualThemes {
                    themes: Themes {
                        widget_id: Some(ctx.path.widget_id()),
                        themes: ts,
                    },
                };

                self.with_mut(ctx.vars, |c| {
                    c.init(ctx);
                })
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.with_mut(ctx.vars, |c| {
                    c.deinit(ctx);
                });
                self.actual_themes = ActualThemes::default();
            }

            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                self.with(ctx.vars, |c| {
                    c.info(ctx, info);
                })
            }

            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                subs.var(ctx, &self.themes);

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
                if let Some(themes) = self.themes.get_new(ctx.vars) {
                    let mut info = ctx.as_info();
                    let mut ts = vec![];
                    for theme in themes {
                        if let Some(t) = theme.generate(&mut info) {
                            ts.push(t);
                        }
                    }
                    self.actual_themes = ActualThemes {
                        themes: Themes {
                            widget_id: Some(ctx.path.widget_id()),
                            themes: ts,
                        },
                    };

                    todo!("(re)init");
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
            themes: T::new(),
            actual_themes: ActualThemes::default(),
        }
    }
}
