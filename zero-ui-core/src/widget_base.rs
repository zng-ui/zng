//! The [`implicit_base`](mod@implicit_base) and properties used in all or most widgets.

use std::{fmt, ops};

use crate::event::EventUpdateArgs;
use crate::var::{context_var, impl_from_and_into_var, IntoValue, IntoVar, StateVar, Var, VarsRead, WithVars, WithVarsRead};
use crate::widget_info::{WidgetInfo, WidgetInfoBuilder, WidgetOffset};
use crate::{
    context::{state_key, LayoutContext, StateMap, WidgetContext},
    units::{AvailableSize, PxSize},
};
use crate::{
    context::{InfoContext, RenderContext},
    render::{FrameBuilder, FrameUpdate, WidgetTransformKey},
};
use crate::{impl_ui_node, property, NilUiNode, UiNode, Widget, WidgetId};

/// Base widget inherited implicitly by all [widgets](widget!) that don't inherit from
/// any other widget.
#[zero_ui_proc_macros::widget_base($crate::widget_base::implicit_base)]
pub mod implicit_base {
    use std::cell::RefCell;

    use zero_ui_view_api::units::PxRect;

    use crate::{
        context::{OwnedStateMap, RenderContext},
        widget_info::{BoundsRect, WidgetOffset, WidgetRendered, WidgetSubscriptions},
    };

    use super::*;

    properties! {
        /// Widget id. Set to a new id by default.
        ///
        /// Can also be set to an `&'static str` unique name.
        #[allowed_in_when = false]
        id(impl IntoValue<WidgetId>) = WidgetId::new_unique();
    }

    properties! {
        /// If events are enabled in the widget and descendants.
        ///
        /// Widgets are enabled by default, you can set this to `false` to disable.
        enabled;

        /// Widget visibility.
        ///
        /// Widgets are visible by default, you can set this to [`Collapsed`]
        /// to remove the widget from layout & render or to [`Hidden`] to only remove it from render.
        ///
        /// Note that the widget visibility is computed from its outer-bounds and render
        ///
        /// [`Collapsed`]: crate::widget_base::Visibility::Collapsed
        /// [`Hidden`]: crate::widget_base::Visibility::Hidden
        visibility;
    }

    /// Implicit `new_child`, does nothing, returns the [`NilUiNode`].
    pub fn new_child() -> impl UiNode {
        NilUiNode
    }

    /// No-op, returns `child`.
    pub fn new_child_inner(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_child_size(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_child_outer(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_child_event(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_child_context(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Returns a node that wraps `child` and marks the [`WidgetOffset::with_inner`].
    pub fn new_inner(child: impl UiNode) -> impl UiNode {
        struct WidgetInnerBoundsNode<T> {
            child: T,
        }
        #[impl_ui_node(child)]
        impl<T: UiNode> UiNode for WidgetInnerBoundsNode<T> {
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                widget_offset.with_inner(final_size, |wo| self.child.arrange(ctx, wo, final_size))
            }
        }
        WidgetInnerBoundsNode { child }
    }

    /// No-op, returns `child`.
    pub fn new_size(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_outer(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_event(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_context(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Implicit `new`, captures the `id` property.
    ///
    /// Returns a [`Widget`] node that introduces a new widget context. The node calls
    /// [`WidgetContext::widget_context`], [`LayoutContext::with_widget`] and [`FrameBuilder::push_widget`]
    /// to define the widget.
    ///
    /// [`WidgetContext::widget_context`]: crate::context::WidgetContext::widget_context
    /// [`LayoutContext::widget_context`]: crate::context::LayoutContext::widget_context
    /// [`FrameBuilder::push_widget`]: crate::render::FrameBuilder::push_widget
    pub fn new(child: impl UiNode, id: impl IntoValue<WidgetId>) -> impl Widget {
        struct WidgetNode<T> {
            id: WidgetId,
            transform_key: WidgetTransformKey,
            state: OwnedStateMap,
            child: T,
            outer_bounds: BoundsRect,
            inner_bounds: BoundsRect,
            rendered: WidgetRendered,
            subscriptions: RefCell<WidgetSubscriptions>,
            #[cfg(debug_assertions)]
            inited: bool,
        }
        impl<T: UiNode> UiNode for WidgetNode<T> {
            #[inline(always)]
            fn init(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::init` called in already inited widget {:?}", self.id);
                }

                let child = &mut self.child;
                ctx.widget_context(self.id, &mut self.state, |ctx| child.init(ctx));

                #[cfg(debug_assertions)]
                {
                    self.inited = true;
                }
            }
            #[inline(always)]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::info` called in not inited widget {:?}", self.id);
                }

                ctx.with_widget(self.id, &self.state, |ctx| {
                    info.push_widget(
                        self.id,
                        self.outer_bounds.clone(),
                        self.inner_bounds.clone(),
                        self.rendered.clone(),
                        &mut self.subscriptions.borrow_mut(),
                        |info| self.child.info(ctx, info),
                    );
                });
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::deinit` called in not inited widget {:?}", self.id);
                }

                let child = &mut self.child;
                ctx.widget_context(self.id, &mut self.state, |ctx| child.deinit(ctx));

                #[cfg(debug_assertions)]
                {
                    self.inited = false;
                }
            }
            #[inline(always)]
            fn update(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::update` called in not inited widget {:?}", self.id);
                }

                if self.subscriptions.borrow().update_intersects(ctx.updates) {
                    let child = &mut self.child;
                    ctx.widget_context(self.id, &mut self.state, |ctx| child.update(ctx));
                }
            }
            #[inline(always)]
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::event::<{}>` called in not inited widget {:?}", std::any::type_name::<EU>(), self.id);
                }

                if self.subscriptions.borrow().event_contains(args) {
                    let child = &mut self.child;
                    ctx.widget_context(self.id, &mut self.state, |ctx| child.event(ctx, args));
                }
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                #[cfg(debug_assertions)]
                {
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::measure` called in not inited widget {:?}", self.id);
                    }
                }

                let child = &mut self.child;
                let child_size = ctx.with_widget(self.id, &mut self.state, |ctx| child.measure(ctx, available_size));

                #[cfg(debug_assertions)]
                {}

                child_size
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
                #[cfg(debug_assertions)]
                {
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::arrange` called in not inited widget {:?}", self.id);
                    }
                }

                let child = &mut self.child;
                ctx.with_widget(self.id, &mut self.state, |ctx| {
                    widget_offset.with_widget(&self.outer_bounds, &self.inner_bounds, final_size, |wo| {
                        child.arrange(ctx, wo, final_size);
                    });
                });
            }
            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::render` called in not inited widget {:?}", self.id);
                }

                ctx.with_widget(self.id, &self.state, |ctx| {
                    frame.push_widget(self.id, self.transform_key, self.outer_bounds.get().size, &self.rendered, |frame| {
                        self.child.render(ctx, frame)
                    });
                });
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::render_update` called in not inited widget {:?}", self.id);
                }

                ctx.with_widget(self.id, &self.state, |ctx| {
                    update.update_widget(self.id, self.transform_key, &self.child, ctx);
                });
            }
        }
        impl<T: UiNode> Widget for WidgetNode<T> {
            #[inline]
            fn id(&self) -> WidgetId {
                self.id
            }
            #[inline]
            fn state(&self) -> &StateMap {
                &self.state.0
            }
            #[inline]
            fn state_mut(&mut self) -> &mut StateMap {
                &mut self.state.0
            }
            #[inline]
            fn outer_bounds(&self) -> PxRect {
                self.outer_bounds.get()
            }
            #[inline]
            fn inner_bounds(&self) -> PxRect {
                self.inner_bounds.get()
            }
            #[inline]
            fn visibility(&self) -> Visibility {
                if self.rendered.get() {
                    Visibility::Visible
                } else if self.outer_bounds.get().size == PxSize::zero() {
                    Visibility::Collapsed
                } else {
                    Visibility::Hidden
                }
            }
        }

        WidgetNode {
            id: id.into(),
            transform_key: WidgetTransformKey::new_unique(),
            state: OwnedStateMap::default(),
            child,
            outer_bounds: BoundsRect::new(),
            inner_bounds: BoundsRect::new(),
            rendered: WidgetRendered::new(),
            subscriptions: RefCell::default(),
            #[cfg(debug_assertions)]
            inited: false,
        }
    }
}

state_key! {
    struct EnabledState: bool;
}

context_var! {
    /// Don't use this directly unless you read all the enabled related
    /// source code here and in core/window.rs
    #[doc(hidden)]
    pub struct IsEnabledVar: bool = true;
}

/// Extension method for accessing the [`enabled`](fn@enabled) state of widgets.
pub trait WidgetEnabledExt {
    /// Gets the widget enabled state.
    ///
    /// The implementation for [`StateMap`] and [`Widget`] only get the state configured
    /// in the widget, if a parent widget is disabled that does not show here. Use [`IsEnabled`]
    /// to get the inherited state from inside a widget.
    ///
    /// The implementation for [`WidgetInfo`] gets if the widget and all ancestors are enabled.
    fn enabled(&self) -> bool;
}
impl WidgetEnabledExt for StateMap {
    fn enabled(&self) -> bool {
        self.get(EnabledState).copied().unwrap_or(true)
    }
}
impl<W: Widget> WidgetEnabledExt for W {
    fn enabled(&self) -> bool {
        self.state().enabled()
    }
}
impl<'a> WidgetEnabledExt for WidgetInfo<'a> {
    fn enabled(&self) -> bool {
        self.meta().enabled() && self.parent().map(|p| p.enabled()).unwrap_or(true)
    }
}

/// Contextual [`enabled`](fn@enabled) accessor.
pub struct IsEnabled;
impl IsEnabled {
    /// Gets the enabled state in the current `vars` context.
    #[inline]
    pub fn get<Vr: WithVarsRead>(vars: &Vr) -> bool {
        vars.with_vars_read(|vars| *IsEnabledVar::get(vars))
    }

    /// Gets the new enabled state in the current `vars` context.
    #[inline]
    pub fn get_new<Vw: WithVars>(vars: &Vw) -> Option<bool> {
        vars.with_vars(|vars| IsEnabledVar::get_new(vars).copied())
    }
}

/// If events are enabled in the widget and its descendants.
///
/// This property sets the enabled state of the widget, to probe the enabled state in `when` clauses
/// use [`is_enabled`] or [`is_disabled`]. To probe from inside the implementation of widgets use [`IsEnabled::get`].
/// To probe the widget state use [`WidgetEnabledExt`].
///
/// # Events
///
/// Most `on_<event>` properties do not fire when the widget is disabled. The event properties that ignore
/// the enabled status should mention this in their documentation.
///
/// Most app events ([`Event`]) still get generated by the app extensions.
/// [`MouseDownEvent`] for example is emitted for a click in a disabled widget.
/// The enabled parents of the disabled widget can handle this event.
///
/// # Focus
///
/// Disabled widgets are not focusable. The focus manager skips disabled widgets.
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`Event`]: crate:core::event::Event
/// [`MouseDownEvent`]: crate::core::mouse::MouseDownEvent
#[property(context, default(true))]
pub fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    struct EnabledNode<C, E> {
        child: C,
        enabled: E,
    }
    impl<C: UiNode, E: Var<bool>> EnabledNode<C, E> {
        fn with_context(&mut self, vars: &VarsRead, f: impl FnOnce(&mut C)) {
            if IsEnabled::get(vars) {
                if *self.enabled.get(vars) {
                    // context already enabled
                    f(&mut self.child);
                } else {
                    // we are disabling
                    let child = &mut self.child;
                    vars.with_context_bind(IsEnabledVar, &self.enabled, || f(child));
                }
            } else {
                // context already disabled
                f(&mut self.child);
            }
        }
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for EnabledNode<C, E> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            if !self.enabled.copy(ctx) {
                ctx.widget_state.set(EnabledState, false);
            }
            self.with_context(ctx.vars, |c| c.init(ctx));
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.with_context(ctx.vars, |c| c.deinit(ctx));
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(&state) = self.enabled.get_new(ctx) {
                ctx.widget_state.set(EnabledState, state);
                ctx.updates.info();
            }
            self.with_context(ctx.vars, |c| c.update(ctx));
        }

        fn event<U: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &U)
        where
            Self: Sized,
        {
            self.with_context(ctx.vars, |c| c.event(ctx, args));
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if !self.enabled.copy(ctx) {
                info.meta().set(EnabledState, false);
            }
            self.child.info(ctx, info);
        }
    }
    EnabledNode {
        child,
        enabled: enabled.into_var(),
    }
}

struct IsEnabledNode<C: UiNode> {
    child: C,
    state: StateVar,
    expected: bool,
}
impl<C: UiNode> IsEnabledNode<C> {
    fn update_state(&self, ctx: &mut WidgetContext) {
        let enabled = IsEnabled::get(ctx) && ctx.widget_state.enabled();
        let is_state = enabled == self.expected;
        self.state.set_ne(ctx.vars, is_state);
    }
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsEnabledNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.update_state(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        self.update_state(ctx);
    }
}

/// If the widget is enabled for receiving events.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// [`enabled`]: fn@enabled
#[property(context)]
pub fn is_enabled(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsEnabledNode {
        child,
        state,
        expected: true,
    }
}
/// If the widget is disabled for receiving events.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// This is the same as `!self.is_enabled`.
///
/// [`enabled`]: fn@enabled
#[property(context)]
pub fn is_disabled(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsEnabledNode {
        child,
        state,
        expected: false,
    }
}

/// Sets the widget visibility.
///
/// This property causes the widget to have the `visibility`, the widget actual visibility is computed, for example,
/// widgets that don't render anything are considered `Hidden` even if the visibility property is not set, this property
/// only forces the widget to layout and render according to the specified visibility.
///
/// To probe the visibility state of an widget in `when` clauses use [`is_visible`], [`is_hidden`] or [`is_collapsed`] in `when` clauses,
/// to probe a widget state use [`Widget::visibility`] or [`WidgetInfo::visibility`].
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`is_visible`]: fn@is_visible
/// [`is_hidden`]: fn@is_hidden
/// [`is_collapsed`]: fn@is_collapsed
/// [`WidgetInfo::visibility`]: crate::widget_info::WidgetInfo::visibility
#[property(context, default(true))]
pub fn visibility(child: impl UiNode, visibility: impl IntoVar<Visibility>) -> impl UiNode {
    struct VisibilityNode<C, V> {
        child: C,
        prev_vis: Visibility,
        visibility: V,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, V: Var<Visibility>> UiNode for VisibilityNode<C, V> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.prev_vis = self.visibility.copy(ctx);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(vis) = self.visibility.copy_new(ctx) {
                use Visibility::*;
                match (self.prev_vis, vis) {
                    (Collapsed, Visible) | (Visible, Collapsed) => ctx.updates.layout_and_render(),
                    (Hidden, Visible) | (Visible, Hidden) => ctx.updates.render(),
                    (Collapsed, Hidden) | (Hidden, Collapsed) => ctx.updates.layout(),
                    _ => {}
                }
                self.prev_vis = vis;
            }
            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            match self.visibility.copy(ctx) {
                Visibility::Collapsed => PxSize::zero(),
                _ => self.child.measure(ctx, available_size),
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
            if Visibility::Collapsed != self.visibility.copy(ctx) {
                self.child.arrange(ctx, widget_offset, final_size)
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if let Visibility::Visible = self.visibility.get(ctx) {
                self.child.render(ctx, frame);
            } else {
                frame
                    .cancel_widget()
                    .expect("visibility not set before `FrameBuilder::open_widget_display`");
            }
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            if let Visibility::Visible = self.visibility.get(ctx) {
                self.child.render_update(ctx, update);
            } else {
                update.cancel_widget();
            }
        }
    }
    VisibilityNode {
        child,
        prev_vis: Visibility::Visible,
        visibility: visibility.into_var(),
    }
}

/// Widget visibility.
///
/// The visibility status of a widget is computed from its outer-bounds in the last layout and if it rendered anything,
/// the visibility of a parent widget affects all descendant widgets, you can inspect the visibility using [`WidgetInfo::visibility`]
/// or the [`Widget::visibility`] methods.
///
/// You can use  the [`visibility`] property to explicitly set the visibility of a widget, this property causes the widget to
/// layout and render according to specified visibility.
///
/// [`WidgetInfo::visibility`]: crate::widget_info::WidgetInfo::visibility
/// [`visibility`]: fn@visibility
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Visibility {
    /// The widget is visible, this is default.
    Visible,
    /// The widget is not visible, but still affects layout.
    ///
    /// Hidden widgets measure and reserve space in their parent but are not rendered.
    Hidden,
    /// The widget is not visible and does not affect layout.
    ///
    /// Collapsed widgets always measure to zero and are not rendered.
    Collapsed,
}
impl fmt::Debug for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Visibility::")?;
        }
        match self {
            Visibility::Visible => write!(f, "Visible"),
            Visibility::Hidden => write!(f, "Hidden"),
            Visibility::Collapsed => write!(f, "Collapsed"),
        }
    }
}
impl Default for Visibility {
    /// [` Visibility::Visible`]
    fn default() -> Self {
        Visibility::Visible
    }
}
impl ops::BitOr for Visibility {
    type Output = Self;

    /// `Collapsed` | `Hidden` | `Visible` short circuit from left to right.
    fn bitor(self, rhs: Self) -> Self::Output {
        use Visibility::*;
        match (self, rhs) {
            (Collapsed, _) | (_, Collapsed) => Collapsed,
            (Hidden, _) | (_, Hidden) => Hidden,
            _ => Visible,
        }
    }
}
impl ops::BitOrAssign for Visibility {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}
impl_from_and_into_var! {
    /// * `true` -> `Visible`
    /// * `false` -> `Collapsed`
    fn from(visible: bool) -> Visibility {
        if visible { Visibility::Visible } else { Visibility::Collapsed }
    }
}

struct IsVisibilityNode<C: UiNode> {
    child: C,
    state: StateVar,
    expected: Visibility,
}
fn current_vis(ctx: &mut WidgetContext) -> Visibility {
    ctx.services
        .get::<crate::window::Windows>()
        .and_then(|w| w.widget_tree(ctx.path.window_id()).ok())
        .and_then(|t| t.find(ctx.path.widget_id()))
        .map(|w| w.visibility())
        .unwrap_or(Visibility::Visible)
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsVisibilityNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);

        let vis = current_vis(ctx);
        self.state.set_ne(ctx, vis != self.expected);
    }

    fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
        if let Some(args) = crate::window::FrameImageReadyEvent.update(args) {
            let vis = current_vis(ctx);
            self.state.set_ne(ctx, vis != self.expected);

            self.child.event(ctx, args);
        } else {
            self.child.event(ctx, args);
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.child.deinit(ctx);
        self.state.set_ne(ctx, self.expected == Visibility::Collapsed);
    }
}
/// If the widget is [`Visible`](Visibility::Visible).
#[property(context)]
pub fn is_visible(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Visible,
    }
}
/// If the widget is [`Hidden`](Visibility::Hidden).
#[property(context)]
pub fn is_hidden(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Hidden,
    }
}
/// If the widget is [`Collapsed`](Visibility::Collapsed).
#[property(context)]
pub fn is_collapsed(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Collapsed,
    }
}
