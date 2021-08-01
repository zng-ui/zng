//! The [`implicit_base`](mod@implicit_base) and properties used in all or most widgets.

use std::{fmt, ops};

use crate::event::EventUpdateArgs;
use crate::var::{context_var, impl_from_and_into_var, IntoVar, WithVars, WithVarsRead};
use crate::{
    context::RenderContext,
    render::{FrameBuilder, FrameUpdate, WidgetInfo, WidgetTransformKey},
};
use crate::{
    context::{state_key, LayoutContext, StateMap, WidgetContext},
    units::LayoutPoint,
    WidgetList,
};
use crate::{impl_ui_node, property, NilUiNode, UiNode, Widget, WidgetId};
use crate::{
    units::LayoutSize,
    var::{Var, VarsRead},
};

#[cfg(debug_assertions)]
use crate::units::PixelGridExt;

/// Base widget inherited implicitly by all [widgets](widget!) that don't inherit from
/// any other widget.
#[zero_ui_proc_macros::widget_base($crate::widget_base::implicit_base)]
pub mod implicit_base {
    use crate::{
        context::{OwnedStateMap, RenderContext},
        var::IntoValue,
    };

    use super::*;

    properties! {
        /// Widget id. Set to an [`new_unique`] by default.
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
        /// Widgets are visible by default, you can set this to [`Collapsed`](zero_ui_core::widget_base::Visibility::Collapsed)
        /// to remove the widget from layout & render.
        visibility;

        /// If the widget is visible during hit-testing.
        ///
        /// Widgets are hit-testable by default, you can set this to `false` to make the widget transparent to
        /// hit-tests.
        hit_testable;
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

    /// No-op, returns `child`.
    pub fn new_inner(child: impl UiNode) -> impl UiNode {
        child
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
            size: LayoutSize,
            #[cfg(debug_assertions)]
            inited: bool,
        }
        impl<T: UiNode> UiNode for WidgetNode<T> {
            #[inline(always)]
            fn init(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if self.inited {
                    log::error!(target: "widget_base", "`UiNode::init` called in already inited widget {:?}", self.id);
                }

                let child = &mut self.child;
                ctx.widget_context(self.id, &mut self.state, |ctx| child.init(ctx));

                #[cfg(debug_assertions)]
                {
                    self.inited = true;
                }
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    log::error!(target: "widget_base", "`UiNode::deinit` called in not inited widget {:?}", self.id);
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
                    log::error!(target: "widget_base", "`UiNode::update` called in not inited widget {:?}", self.id);
                }

                let child = &mut self.child;
                ctx.widget_context(self.id, &mut self.state, |ctx| child.update(ctx));
            }
            #[inline(always)]
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    log::error!(target: "widget_base", "`UiNode::event::<{}>` called in not inited widget {:?}", std::any::type_name::<EU>(), self.id);
                }

                let child = &mut self.child;
                ctx.widget_context(self.id, &mut self.state, |ctx| child.event(ctx, args));
            }
            #[inline(always)]
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
                #[cfg(debug_assertions)]
                {
                    if !self.inited {
                        log::error!(target: "widget_base", "`UiNode::measure` called in not inited widget {:?}", self.id);
                    }

                    fn valid_measure(f: f32) -> bool {
                        f.is_finite() || crate::units::is_layout_any_size(f)
                    }

                    if !valid_measure(available_size.width) || !valid_measure(available_size.height) {
                        log::error!(
                            target: "widget_base",
                            "{:?} `UiNode::measure` called with invalid `available_size: {:?}`, must be finite or `LAYOUT_ANY_SIZE`",
                            self.id,
                            available_size
                        );
                    }
                }

                let child = &mut self.child;
                let child_size = ctx.with_widget(self.id, &mut self.state, |ctx| child.measure(ctx, available_size));

                #[cfg(debug_assertions)]
                {
                    if !child_size.width.is_finite() || !child_size.height.is_finite() {
                        log::error!(target: "widget_base", "{:?} `UiNode::measure` result is not finite: `{:?}`", self.id, child_size);
                    } else if !child_size.is_aligned_to(ctx.metrics.pixel_grid) {
                        let snapped = child_size.snap_to(ctx.metrics.pixel_grid);
                        log::error!(
                            target: "widget_base",
                            "{:?} `UiNode::measure` result not aligned, was: `{:?}`, expected: `{:?}`",
                            self.id,
                            child_size,
                            snapped
                        );
                        return snapped;
                    }
                }

                child_size
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
                self.size = final_size;

                #[cfg(debug_assertions)]
                {
                    if !self.inited {
                        log::error!(target: "widget_base", "`UiNode::arrange` called in not inited widget {:?}", self.id);
                    }

                    if !final_size.width.is_finite() || !final_size.height.is_finite() {
                        log::error!(
                            target: "widget_base",
                            "{:?} `UiNode::arrange` called with invalid `final_size: {:?}`, must be finite",
                            self.id,
                            final_size
                        );
                    } else if !final_size.is_aligned_to(ctx.metrics.pixel_grid) {
                        self.size = final_size.snap_to(ctx.metrics.pixel_grid);
                        log::error!(
                            target: "widget_base",
                            "{:?} `UiNode::arrange` called with not aligned value, was: `{:?}`, expected: `{:?}`",
                            self.id,
                            final_size,
                            self.size
                        );
                    }
                }

                let final_size = self.size;
                let child = &mut self.child;
                ctx.with_widget(self.id, &mut self.state, |ctx| {
                    child.arrange(ctx, final_size);
                });
            }
            #[inline(always)]
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    log::error!(target: "widget_base", "`UiNode::render` called in not inited widget {:?}", self.id);
                }

                ctx.with_widget(self.id, &self.state, |ctx| {
                    frame.push_widget(self.id, self.transform_key, self.size, &self.child, ctx);
                });
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    log::error!(target: "widget_base", "`UiNode::render_update` called in not inited widget {:?}", self.id);
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
            fn size(&self) -> LayoutSize {
                self.size
            }
        }

        WidgetNode {
            id: id.into(),
            transform_key: WidgetTransformKey::new_unique(),
            state: OwnedStateMap::default(),
            child,
            size: LayoutSize::zero(),
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
    pub struct IsEnabledVar: bool = return &true;
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
/// use [`is_enabled`]. To probe from inside the implementation of widgets use [`IsEnabled::get`].
/// To probe the widget state use [`WidgetEnabledExt`].
///
/// # Events
///
/// Most `on_<event>` properties do not fire when the widget is disabled. The event properties that ignore
/// the enabled status mention this in their documentation.
///
/// Most app events ([`Event`](crate:core::event::Event)) still get generated by the app extensions.
/// [`MouseDownEvent`](crate::core::mouse::MouseDownEvent) for example is emitted for a click in a disabled widget.
/// The enabled parents of the disabled widget can handle this event.
///
/// # Focus
///
/// Disabled widgets are not focusable. The focus manager skips disabled widgets.
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
                ctx.updates.render(); // TODO meta updates without a new frame?
            }
            self.with_context(ctx.vars, |c| c.update(ctx));
        }

        fn event<U: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &U)
        where
            Self: Sized,
        {
            self.with_context(ctx.vars, |c| c.event(ctx, args));
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if !self.enabled.copy(ctx) {
                frame.meta().set(EnabledState, false);
            }
            self.child.render(ctx, frame);
        }
    }
    EnabledNode {
        child,
        enabled: enabled.into_var(),
    }
}

/// Sets the widget visibility.
#[property(context, default(true))]
pub fn visibility(child: impl UiNode, visibility: impl IntoVar<Visibility>) -> impl UiNode {
    struct VisibilityNode<C, V> {
        child: C,
        visibility: V,
    }
    impl<C: UiNode, V: Var<Visibility>> VisibilityNode<C, V> {
        fn with_context(&mut self, vars: &VarsRead, f: impl FnOnce(&mut C)) {
            match *VisibilityVar::get(vars) {
                // parent collapsed => all descendants collapsed
                Visibility::Collapsed => f(&mut self.child),
                // parent hidden =>
                Visibility::Hidden => {
                    // if we are collapsed
                    if let Visibility::Collapsed = self.visibility.get(vars) {
                        // our branch is collapsed
                        let child = &mut self.child;
                        vars.with_context_bind(VisibilityVar, &self.visibility, || f(child));
                    } else {
                        // otherwise same as parent
                        f(&mut self.child)
                    }
                }
                // parent visible =>
                Visibility::Visible => {
                    if let Visibility::Visible = self.visibility.get(vars) {
                        // and we are also visible, same as parent
                        f(&mut self.child)
                    } else {
                        // or, our visibility is different
                        let child = &mut self.child;
                        vars.with_context_bind(VisibilityVar, &self.visibility, || f(child));
                    }
                }
            }
        }
    }
    impl<C: UiNode, V: Var<Visibility>> UiNode for VisibilityNode<C, V> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let vis = self.visibility.copy(ctx);
            ctx.widget_state.set(VisibilityState, vis);

            self.with_context(ctx.vars, |c| c.init(ctx));
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.with_context(ctx.vars, |c| c.deinit(ctx));
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(&vis) = self.visibility.get_new(ctx) {
                ctx.widget_state.set(VisibilityState, vis);
                ctx.updates.layout();
            }
            self.with_context(ctx.vars, |c| c.update(ctx));
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
            match self.visibility.copy(ctx) {
                Visibility::Visible | Visibility::Hidden => self.child.measure(ctx, available_size),
                Visibility::Collapsed => LayoutSize::zero(),
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            if let Visibility::Visible = self.visibility.get(ctx) {
                self.child.arrange(ctx, final_size)
            }
        }

        fn event<U: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &U)
        where
            Self: Sized,
        {
            self.with_context(ctx.vars, |c| c.event(ctx, args));
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
        visibility: visibility.into_var(),
    }
}

/// Widget visibility.
///
/// The visibility value affects the widget and its descendants.
///
/// # Inheritance
///
/// In a UI tree the visibility of widgets combine with that of their parents.
///
/// * If the parent is collapsed all descendants are collapsed.
///
/// * If the parent is hidden some descendants can still be collapsed and affect the layout.
///
/// * If the parent is visible the descendants can have the other visibility modes.
///
/// This combination of visibility is implemented as a *bit OR* (`|`) operation.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Visibility {
    /// The widget is visible, this is default.
    Visible,
    /// The widget is not visible, but still affects layout.
    ///
    /// Hidden widgets measure and reserve space in their parent but are not present
    /// in the rendered frames.
    Hidden,
    /// The widget is not visible and does not affect layout.
    ///
    /// Collapsed widgets always measure to zero and are not included in the rendered frames.
    ///
    /// Layout widgets can also consider this value.
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

state_key! { struct VisibilityState: Visibility; }

context_var! {
    /// Don't use this directly unless you read all the visibility related
    /// source code here and in core/window.rs
    #[doc(hidden)]
    pub struct VisibilityVar: Visibility = return &Visibility::Visible;
}

/// Extension method for accessing the [`Visibility`] of widgets.
pub trait WidgetVisibilityExt {
    /// Gets the widget visibility.
    ///
    /// This gets only the visibility configured in the widget, if a parent widget
    /// is not visible that does not show here. Use [`VisibilityContext`] to get the inherited
    /// visibility from inside a widget.
    fn visibility(&self) -> Visibility;
}
impl WidgetVisibilityExt for StateMap {
    fn visibility(&self) -> Visibility {
        self.get(VisibilityState).copied().unwrap_or_default()
    }
}

/// Extension methods for filtering an [`WidgetList`] by [`Visibility`].
pub trait WidgetListVisibilityExt: WidgetList {
    /// Counts the widgets that are not collapsed.
    fn count_not_collapsed(&self) -> usize;

    /// Render widgets, calls `origin` only for widgets that are not collapsed.
    fn render_not_collapsed<O: FnMut(usize) -> LayoutPoint>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder);
}

impl<U: WidgetList> WidgetListVisibilityExt for U {
    fn count_not_collapsed(&self) -> usize {
        self.count(|_, s| s.visibility() != Visibility::Collapsed)
    }

    fn render_not_collapsed<O: FnMut(usize) -> LayoutPoint>(&self, mut origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.render_filtered(
            |i, s| {
                if s.visibility() != Visibility::Collapsed {
                    Some(origin(i))
                } else {
                    None
                }
            },
            ctx,
            frame,
        )
    }
}

/// Contextual [`Visibility`] accessor.
pub struct VisibilityContext;
impl VisibilityContext {
    /// Gets the visibility state in the current `vars` context.
    #[inline]
    pub fn get<Vr: WithVarsRead>(vars: &Vr) -> Visibility {
        vars.with_vars_read(|vars| *VisibilityVar::get(vars))
    }
}

/// If the widget and its descendants are visible during hit-testing.
///
/// This property sets the hit-test visibility of the widget, to probe the state in `when` clauses
/// use [`is_hit_testable`](fn@is_hit_testable). To probe from inside the implementation of widgets use [`IsHitTestable::get`].
/// To probe the widget state use [`WidgetHitTestableExt`].
///
/// Widgets are hit-testable by default, so setting this property to `true` has the same effect as unsetting it.
///
/// # Events
///
/// Events that use hit-testing to work are effectively disabled by setting this to `false`. That includes
/// all mouse and touch events.
#[property(context, default(true))]
pub fn hit_testable(child: impl UiNode, hit_testable: impl IntoVar<bool>) -> impl UiNode {
    struct HitTestableNode<U, H> {
        child: U,
        hit_testable: H,
    }
    impl<U: UiNode, H: Var<bool>> HitTestableNode<U, H> {
        fn with_context(&mut self, vars: &VarsRead, f: impl FnOnce(&mut U)) {
            if IsHitTestable::get(vars) {
                if *self.hit_testable.get(vars) {
                    // context already hit-testable
                    f(&mut self.child);
                } else {
                    // we are disabling
                    let child = &mut self.child;
                    vars.with_context_bind(IsHitTestableVar, &self.hit_testable, || f(child));
                }
            } else {
                // context already not hit-testable
                f(&mut self.child);
            }
        }
    }
    #[impl_ui_node(child)]
    impl<U: UiNode, H: Var<bool>> UiNode for HitTestableNode<U, H> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            if !self.hit_testable.copy(ctx) {
                ctx.widget_state.set(HitTestableState, false);
            }
            self.with_context(ctx.vars, |c| c.init(ctx));
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.with_context(ctx.vars, |c| c.deinit(ctx));
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(&state) = self.hit_testable.get_new(ctx) {
                ctx.widget_state.set(HitTestableState, state);
                ctx.updates.render();
            }
            self.with_context(ctx.vars, |c| c.update(ctx));
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU)
        where
            Self: Sized,
        {
            self.with_context(ctx.vars, |c| c.event(ctx, args));
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if !self.hit_testable.copy(ctx) {
                frame.push_not_hit_testable(|frame| self.child.render(ctx, frame));
            } else {
                self.child.render(ctx, frame);
            }
        }
    }
    HitTestableNode {
        child,
        hit_testable: hit_testable.into_var(),
    }
}

context_var! {
    struct IsHitTestableVar: bool = return &true;
}

/// Contextual [`hit_testable`](fn@hit_testable) accessor.
pub struct IsHitTestable;
impl IsHitTestable {
    /// Gets the hit-testable state in the current `vars` context.
    pub fn get<Vr: WithVarsRead>(vars: &Vr) -> bool {
        vars.with_vars_read(|vars| *IsHitTestableVar::get(vars))
    }
}

state_key! {
    struct HitTestableState: bool;
}

/// Extension method for accessing the [`hit_testable`](fn@hit_testable) state of widgets.
pub trait WidgetHitTestableExt {
    /// Gets the widget hit-test visibility.
    ///
    /// The implementation for [`StateMap`] only get the state configured
    /// in `self`, if a parent widget is not hit-testable that does not show here. Use [`IsHitTestable`]
    /// to get the inherited state from inside a widget.
    ///
    /// The implementation for [`WidgetInfo`] gets if the widget and all ancestors are hit-test visible.
    fn hit_testable(&self) -> bool;
}
impl WidgetHitTestableExt for StateMap {
    fn hit_testable(&self) -> bool {
        self.get(HitTestableState).copied().unwrap_or(true)
    }
}
impl<'a> WidgetHitTestableExt for WidgetInfo<'a> {
    fn hit_testable(&self) -> bool {
        self.meta().hit_testable() && self.parent().map(|p| p.hit_testable()).unwrap_or(true)
    }
}
