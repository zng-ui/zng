//! Theme building blocks.

use std::{cell::RefCell, fmt, mem, rc::Rc};

use crate::{core::NilUiNode, prelude::new_widget::*};

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
/// a constructor function you **must** delegate to the equivalent function defined in [`nodes`], each priority constructor captures
/// the properties and returns a placeholder node that is the child of the next priority.
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
        nodes::new_child()
    }

    fn new_child_layout(child: impl UiNode) -> impl UiNode {
        nodes::new_child_layout(child)
    }

    fn new_child_context(child: impl UiNode) -> impl UiNode {
        nodes::new_child_context(child)
    }

    fn new_fill(child: impl UiNode) -> impl UiNode {
        nodes::new_fill(child)
    }

    fn new_border(child: impl UiNode) -> impl UiNode {
        nodes::new_border(child)
    }

    fn new_size(child: impl UiNode) -> impl UiNode {
        nodes::new_size(child)
    }

    fn new_layout(child: impl UiNode) -> impl UiNode {
        nodes::new_layout(child)
    }

    fn new_event(child: impl UiNode) -> impl UiNode {
        nodes::new_event(child)
    }

    fn new_context(child: impl UiNode) -> impl UiNode {
        nodes::new_context(child);
        NilUiNode
    }

    fn new(_: impl UiNode) -> Theme {
        nodes::new()
    }

    /// Building blocks for custom theme.
    ///
    /// Each of node must be inserted in the constructor overrides of the same name.
    pub mod nodes {
        use super::*;

        /// Start constructing the theme.
        pub fn new_child() -> impl UiNode {
            Theme::new_child()
        }

        /// Captures the *child-layout* priority properties for the theme.
        pub fn new_child_layout(child: impl UiNode) -> impl UiNode {
            Theme::new_priority(child.boxed(), |t| &t.child_layout, |t| &t.child_context)
        }

        /// Captures the *child-context* priority properties for the theme.
        pub fn new_child_context(child: impl UiNode) -> impl UiNode {
            Theme::new_priority(child.boxed(), |t| &t.child_context, |t| &t.fill)
        }

        /// Captures the *fill* priority properties for the theme.
        pub fn new_fill(child: impl UiNode) -> impl UiNode {
            Theme::new_priority(child.boxed(), |t| &t.fill, |t| &t.border)
        }

        /// Captures the *border* priority properties for the theme.
        pub fn new_border(child: impl UiNode) -> impl UiNode {
            Theme::new_priority(child.boxed(), |t| &t.border, |t| &t.size)
        }

        /// Captures the *size* priority properties for the theme.
        pub fn new_size(child: impl UiNode) -> impl UiNode {
            Theme::new_priority(child.boxed(), |t| &t.size, |t| &t.layout)
        }

        /// Captures the *layout* priority properties for the theme.
        pub fn new_layout(child: impl UiNode) -> impl UiNode {
            Theme::new_priority(child.boxed(), |t| &t.layout, |t| &t.event)
        }

        /// Captures the *event* priority properties for the theme.
        pub fn new_event(child: impl UiNode) -> impl UiNode {
            Theme::new_priority(child.boxed(), |t| &t.event, |t| &t.context)
        }

        /// Captures the *context* priority properties for the theme.
        ///
        /// Note that the theme is completed here, nodes inserted after this call and before [`new`] are ignored
        /// in the theme widget, in the default `new_context` the [`NilUiNode`] is returned to fulfill the widget signature.
        pub fn new_context(child: impl UiNode) {
            Theme::new_context(child.boxed());
        }

        /// Finishes constructing the theme.
        ///
        /// Note that the theme already completed in [`new_context`] this just collects the [`Theme`] instance, in the
        /// default `new` the input node is discarded.
        pub fn new() -> Theme {
            Theme::new()
        }
    }
}

/// Themable widget mix-in.
///
/// Adds the `theme` property that can be set to a [`ThemeConstructor`] that generates properties that are dynamically injected
/// into the widget to alter its appearance.
#[widget_mixin($crate::widgets::mixins::theme_mixin)]
pub mod theme_mixin {
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
        
        #[doc(hidden)]
        properties::insert_child_layout = ();
        #[doc(hidden)]
        properties::insert_child_context = ();
        #[doc(hidden)]
        properties::insert_fill = ();
        #[doc(hidden)]
        properties::insert_border = ();
        #[doc(hidden)]
        properties::insert_size = ();
        #[doc(hidden)]
        properties::insert_layout = ();
        #[doc(hidden)]
        properties::on_insert_event = ();
    }

    /// Properties inserted by the mix-in.
    ///
    /// Only the `theme` property is doc visible, the others are implementation details.
    pub mod properties {
        use super::*;

        /// Theme generator used for the widget.
        ///
        /// Properties and `when` conditions in the generated theme are applied to the widget as
        /// if they where set on it. Note that changing the theme causes the widget info tree to rebuild,
        /// prefer property binding and `when` conditions to cause visual changes that happen often.
        #[property(context, default(ThemeGenerator::nil()))]
        pub fn theme(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
            let child = Theme::insert_priority(child.boxed(), |t| &t.context);

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

        /// Insert the *child-layout* priority properties from the theme.
        #[property(child_layout, allowed_in_when = false)]
        pub fn insert_child_layout(child: impl UiNode, _activate: ()) -> impl UiNode {
            Theme::insert_priority(child.boxed(), |t| &t.child_layout)
        }

        /// Insert the *child-context* priority properties from the theme.
        #[property(child_context, allowed_in_when = false)]
        pub fn insert_child_context(child: impl UiNode, _activate: ()) -> impl UiNode {
            Theme::insert_priority(child.boxed(), |t| &t.child_context)
        }

        /// Insert the *fill* priority properties from the theme.
        #[property(fill, allowed_in_when = false)]
        pub fn insert_fill(child: impl UiNode, _activate: ()) -> impl UiNode {
            Theme::insert_priority(child.boxed(), |t| &t.fill)
        }

        /// Insert the *border* priority properties from the theme.
        #[property(border, allowed_in_when = false)]
        pub fn insert_border(child: impl UiNode, _activate: ()) -> impl UiNode {
            Theme::insert_priority(child.boxed(), |t| &t.border)
        }

        /// Insert the *size* priority properties from the theme.
        #[property(size, allowed_in_when = false)]
        pub fn insert_size(child: impl UiNode, _activate: ()) -> impl UiNode {
            Theme::insert_priority(child.boxed(), |t| &t.size)
        }

        /// Insert the *layout* priority properties from the theme.
        #[property(layout, allowed_in_when = false)]
        pub fn insert_layout(child: impl UiNode, _activate: ()) -> impl UiNode {
            Theme::insert_priority(child.boxed(), |t| &t.layout)
        }

        /// Insert the *event* priority properties from the theme.
        #[property(event, allowed_in_when = false)]
        pub fn on_insert_event(child: impl UiNode, _activate: ()) -> impl UiNode {
            Theme::insert_priority(child.boxed(), |t| &t.event)
        }
    }
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

/// Represents a theme instance.
///
/// Use the [`theme!`] *widget* to instantiate.
///
/// [`theme!`]: mod@theme
pub struct Theme {
    child_layout: ThemePriority,
    child_context: ThemePriority,
    fill: ThemePriority,
    border: ThemePriority,
    size: ThemePriority,
    layout: ThemePriority,
    event: ThemePriority,
    context: ThemePriority,
}
thread_local! {
    static THEME: RefCell<Vec<Theme>> = RefCell::default();
}
impl Theme {
    fn new_child() -> impl UiNode {
        let child = THEME.with(|t| {
            let theme = Theme {
                child_layout: ThemePriority::default(),
                child_context: ThemePriority::default(),
                fill: ThemePriority::default(),
                border: ThemePriority::default(),
                size: ThemePriority::default(),
                layout: ThemePriority::default(),
                event: ThemePriority::default(),
                context: ThemePriority::default(),
            };
            let child = theme.child_layout.child.clone();
            t.borrow_mut().push(theme);
            child
        });

        struct ThemeChildNode {
            child: Rc<RefCell<BoxedUiNode>>,
        }
        #[impl_ui_node(
            delegate = self.child.borrow(),
            delegate_mut = self.child.borrow_mut(),
        )]
        impl UiNode for ThemeChildNode {}

        ThemeChildNode { child }
    }

    fn new_priority(
        theme_child: BoxedUiNode,
        priority: impl FnOnce(&Theme) -> &ThemePriority,
        next_priority: impl FnOnce(&Theme) -> &ThemePriority,
    ) -> impl UiNode {
        let child = THEME.with(move |t| {
            let t = t.borrow();
            let t = t.last().expect("no theme instantiating");

            let priority = priority(t);
            let next_priority = next_priority(t);

            *priority.properties.borrow_mut() = Some(theme_child);
            next_priority.child.clone()
        });

        struct ThemePriorityNode {
            child: Rc<RefCell<BoxedUiNode>>,
        }
        #[impl_ui_node(
            delegate = self.child.borrow(),
            delegate_mut = self.child.borrow_mut(),
        )]
        impl UiNode for ThemePriorityNode {}

        ThemePriorityNode { child }
    }

    fn new_context(theme_child: BoxedUiNode) {
        THEME.with(move |t| {
            let t = t.borrow();
            let t = t.last().expect("no theme instantiating");

            *t.context.properties.borrow_mut() = Some(theme_child);
        });
    }

    fn new() -> Theme {
        THEME.with(|t| t.borrow_mut().pop().expect("no theme instantiating"))
    }

    fn insert_priority(widget_child: BoxedUiNode, priority: impl Fn(&Theme) -> &ThemePriority + 'static) -> impl UiNode {
        struct ThemablePriorityNode<P> {
            priority: P,

            wgt_child: Option<Rc<RefCell<BoxedUiNode>>>,
            child: BoxedUiNode,
        }
        #[impl_ui_node(child)]
        impl<P> UiNode for ThemablePriorityNode<P>
        where
            P: Fn(&Theme) -> &ThemePriority + 'static,
        {
            fn init(&mut self, ctx: &mut WidgetContext) {
                debug_assert!(self.wgt_child.is_none());

                let t = ActualThemeVar::get(ctx.vars);
                if let (Some(id), Some(theme)) = (t.widget_id, &t.theme) {
                    if id == ctx.path.widget_id() {
                        // widget is themed, insert the theme

                        let t = (self.priority)(theme);

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

        ThemablePriorityNode {
            priority,
            wgt_child: None,
            child: widget_child,
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
