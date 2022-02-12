//! The [`implicit_base`](mod@implicit_base) and properties used in all or most widgets.

use std::{fmt, ops};

use crate::{
    context::{state_key, InfoContext, LayoutContext, RenderContext, StateMap, WidgetContext},
    event::EventUpdateArgs,
    impl_ui_node, property,
    render::{FrameBuilder, FrameUpdate},
    units::{AvailableSize, PxCornerRadius, PxRect, PxSize},
    var::*,
    widget_info::{UpdateMask, WidgetInfo, WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions},
    NilUiNode, UiNode, Widget, WidgetId,
};

/// Base widget inherited implicitly by all [widgets](widget!) that don't inherit from
/// any other widget.
#[zero_ui_proc_macros::widget_base($crate::widget_base::implicit_base)]
pub mod implicit_base {
    use std::cell::RefCell;

    use crate::{
        context::{OwnedStateMap, RenderContext},
        render::FrameBindingKey,
        units::RenderTransform,
        widget_info::{WidgetLayout, WidgetLayoutInfo, WidgetRenderInfo, WidgetSubscriptions},
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
        /// If interaction is enabled in the widget and descendants.
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
    pub fn new_child_fill(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_child_border(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_child_size(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_child_layout(child: impl UiNode) -> impl UiNode {
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
    pub fn new_fill(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Returns a node that wraps `child` and marks the [`WidgetLayout::with_inner`].
    pub fn new_border(child: impl UiNode) -> impl UiNode {
        struct WidgetInnerBoundsNode<T> {
            child: T,
            transform_key: FrameBindingKey<RenderTransform>,
            transform: RenderTransform,
            clip: (PxSize, PxCornerRadius),
        }
        #[impl_ui_node(child)]
        impl<T: UiNode> UiNode for WidgetInnerBoundsNode<T> {
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                self.transform = widget_layout.with_inner(ctx.metrics, final_size, |wl| self.child.arrange(ctx, wl, final_size));
                self.clip = (final_size, widget_layout.corner_radius());
            }
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                frame.push_inner(self.transform_key.bind(self.transform), |frame| {
                    match HitTestMode::get(ctx.vars) {
                        HitTestMode::RoundedBounds => {
                            let rect = PxRect::from_size(self.clip.0);
                            frame.push_clip_rounded_rect(rect, self.clip.1, false, |f| f.push_hit_test(rect));
                        }
                        HitTestMode::Bounds => {
                            frame.push_hit_test(PxRect::from_size(self.clip.0));
                        }
                        _ => {}
                    }
                    self.child.render(ctx, frame);
                });
            }
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                update.update_transform(self.transform_key.update(self.transform));
                self.child.render_update(ctx, update);
            }
        }
        WidgetInnerBoundsNode {
            child,
            transform_key: FrameBindingKey::new_unique(),
            transform: RenderTransform::identity(),
            clip: (PxSize::zero(), PxCornerRadius::zero()),
        }
    }

    /// No-op, returns `child`.
    pub fn new_size(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_layout(child: impl UiNode) -> impl UiNode {
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
            state: OwnedStateMap,
            child: T,
            outer_info: WidgetLayoutInfo,
            inner_info: WidgetLayoutInfo,
            render_info: WidgetRenderInfo,
            subscriptions: RefCell<WidgetSubscriptions>,
            #[cfg(debug_assertions)]
            inited: bool,
        }
        impl<T: UiNode> UiNode for WidgetNode<T> {
            #[inline(always)]
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::info` called in not inited widget {:?}", self.id);
                }

                ctx.with_widget(self.id, &self.state, |ctx| {
                    info.push_widget(
                        self.id,
                        self.outer_info.clone(),
                        self.inner_info.clone(),
                        self.render_info.clone(),
                        |info| self.child.info(ctx, info),
                    );
                });
            }
            #[inline(always)]
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                let mut wgt_subs = self.subscriptions.borrow_mut();
                *wgt_subs = WidgetSubscriptions::new();

                self.child.subscriptions(ctx, &mut wgt_subs);

                subscriptions.extend(&wgt_subs);
            }
            #[inline(always)]
            fn init(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::init` called in already inited widget {:?}", self.id);
                }

                ctx.widget_context(self.id, &mut self.state, |ctx| self.child.init(ctx));

                #[cfg(debug_assertions)]
                {
                    self.inited = true;
                }
            }
            #[inline(always)]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::deinit` called in not inited widget {:?}", self.id);
                }

                ctx.widget_context(self.id, &mut self.state, |ctx| self.child.deinit(ctx));

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
                    ctx.widget_context(self.id, &mut self.state, |ctx| self.child.update(ctx));
                }
            }
            #[inline(always)]
            fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::event::<{}>` called in not inited widget {:?}", std::any::type_name::<EU>(), self.id);
                }

                if self.subscriptions.borrow().event_contains(args) {
                    ctx.widget_context(self.id, &mut self.state, |ctx| self.child.event(ctx, args));
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

                let child_size = ctx.with_widget(self.id, &mut self.state, |ctx| self.child.measure(ctx, available_size));

                #[cfg(debug_assertions)]
                {}

                child_size
            }
            #[inline(always)]
            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                #[cfg(debug_assertions)]
                {
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::arrange` called in not inited widget {:?}", self.id);
                    }
                }

                ctx.with_widget(self.id, &mut self.state, |ctx| {
                    widget_layout.with_widget(self.id, &self.outer_info, &self.inner_info, final_size, |wo| {
                        self.child.arrange(ctx, wo, final_size);
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
                    frame.push_widget(self.id, &self.render_info, |frame| self.child.render(ctx, frame));
                });
            }
            #[inline(always)]
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::render_update` called in not inited widget {:?}", self.id);
                }

                self.child.render_update(ctx, update);
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
            fn outer_info(&self) -> &WidgetLayoutInfo {
                &self.outer_info
            }
            #[inline]
            fn inner_info(&self) -> &WidgetLayoutInfo {
                &self.inner_info
            }
            #[inline]
            fn render_info(&self) -> &WidgetRenderInfo {
                &self.render_info
            }
        }
        WidgetNode {
            id: id.into(),
            state: OwnedStateMap::default(),
            child,
            outer_info: WidgetLayoutInfo::new(),
            inner_info: WidgetLayoutInfo::new(),
            render_info: WidgetRenderInfo::new(),
            subscriptions: RefCell::default(),
            #[cfg(debug_assertions)]
            inited: false,
        }
    }
}

state_key! {
    struct EnabledState: bool;
    struct RegisteredDisabledFilter: ();
}

context_var! {
    struct IsEnabledVar: bool = true;
}

/// Extension method for accessing the [`enabled`](fn@enabled) state in [`WidgetInfo`].
pub trait WidgetEnabledExt {
    /// Returns if the widget was enabled when the info tree was build.
    ///
    /// If `false` the widget does not [`allow_interaction`] and visually indicates this.
    ///
    /// [`allow_interaction`]: crate::widget_info::WidgetInfo::allow_interaction
    fn is_enabled(&self) -> bool;
}
impl<'a> WidgetEnabledExt for WidgetInfo<'a> {
    fn is_enabled(&self) -> bool {
        self.self_and_ancestors()
            .all(|w| w.meta().get(EnabledState).copied().unwrap_or(true))
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

    /// Gets the update mask for [`WidgetSubscriptions`].
    ///
    /// [`WidgetSubscriptions`]: crate::widget_info::WidgetSubscriptions
    #[inline]
    pub fn update_mask<Vr: WithVarsRead>(vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|vars| IsEnabledVar::new().update_mask(vars))
    }
}

/// If interaction is allowed in the widget and its descendants.
///
/// This property sets the enabled state of the widget, to probe the enabled state in `when` clauses
/// use [`is_enabled`] or [`is_disabled`]. To probe from inside the implementation of widgets use [`IsEnabled::get`].
/// To probe the widget state use [`WidgetEnabledExt`].
///
/// # Interaction
///
/// A widget allows interaction only if [`WidgetInfo::allows_interaction`] returns `true`, this property pushes an interaction
/// filter that blocks interaction for the widget and all its descendants. Note that widgets can block interaction and
/// still be *enabled*, meaning that it behaves like a *disabled* widget but looks like an idle enabled widget, this can happen,
/// for example, when a *modal overlay* is open.
///
/// # Disabled Visual
///
/// Widgets that are expected to be interactive should visually indicate when they are not interactive, but **only** if interaction
/// was disabled by this property, widgets visual should not try to use [`WidgetInfo::allows_interaction`] directly.
///
/// The visual cue for the disabled state is usually a reduced contrast from content and background by *graying-out* the text and applying a
/// grayscale filter for image content.
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`Event`]: crate:core::event::Event
/// [`MouseDownEvent`]: crate::core::mouse::MouseDownEvent
/// [`WidgetInfo::allows_interaction`]: crate::widget_info::WidgetInfo::allows_interaction
#[property(context, default(true))]
pub fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    struct EnabledNode<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for EnabledNode<C> {
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if !IsEnabled::get(ctx) {
                info.meta().set(EnabledState, false);

                if !ctx.update_state.flag(RegisteredDisabledFilter) {
                    info.push_interaction_filter(move |args| args.info.is_enabled())
                }
            }
            self.child.info(ctx, info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.updates(&IsEnabled::update_mask(ctx));
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if IsEnabled::get_new(ctx).is_some() {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }
    }

    with_context_var(
        EnabledNode { child },
        IsEnabledVar,
        merge_var!(IsEnabledVar::new(), enabled.into_var(), |&a, &b| a && b),
    )
}

/// If interaction is allowed in the widget and its descendants.
///
/// This property *enables* and *disables* interaction with the widget and its descendants without causing
/// a visual change like [`enabled`].
///
/// [`enabled`]: fn@enabled
#[property(context, default(true))]
pub fn interactive(child: impl UiNode, interactive: impl IntoVar<bool>) -> impl UiNode {
    struct InteractiveNode<C, I> {
        child: C,
        interactive: I,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, I: Var<bool>> UiNode for InteractiveNode<C, I> {
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if self.interactive.copy(ctx) {
                let id = ctx.path.widget_id();
                info.push_interaction_filter(move |args| args.info.self_and_ancestors().all(|w| w.widget_id() != id));
            }
            self.child.info(ctx, info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.interactive);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.interactive.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }
    }
    InteractiveNode {
        child,
        interactive: interactive.into_var(),
    }
}

struct IsEnabledNode<C: UiNode> {
    child: C,
    state: StateVar,
    expected: bool,
}
impl<C: UiNode> IsEnabledNode<C> {
    fn update_state(&self, ctx: &mut WidgetContext) {
        let is_state = IsEnabled::get(ctx) == self.expected;
        self.state.set_ne(ctx.vars, is_state);
    }
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsEnabledNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.update_state(ctx);
    }

    fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
        self.child.info(ctx, widget);
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        subscriptions.updates(&IsEnabled::update_mask(ctx));
        self.child.subscriptions(ctx, subscriptions);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        self.update_state(ctx);
    }
}

/// If the widget is enabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// [`enabled`]: fn@enabled
/// [`WidgetInfo::allow_interaction`]: crate::widget_info::WidgetInfo::allow_interaction
#[property(event)]
pub fn is_enabled(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsEnabledNode {
        child,
        state,
        expected: true,
    }
}
/// If the widget is disabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// This is the same as `!self.is_enabled`.
///
/// [`enabled`]: fn@enabled
#[property(event)]
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
        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.visibility);
            self.child.subscriptions(ctx, subscriptions);
        }

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

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            if Visibility::Collapsed != self.visibility.copy(ctx) {
                self.child.arrange(ctx, widget_layout, final_size)
            } else {
                widget_layout.collapse(ctx.info_tree);
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if let Visibility::Visible = self.visibility.get(ctx) {
                self.child.render(ctx, frame);
            } else {
                frame.skip_render(ctx.info_tree);
            }
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            if let Visibility::Visible = self.visibility.get(ctx) {
                self.child.render_update(ctx, update);
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
/// the visibility of a parent widget affects all descendant widgets, you can inspect the visibility using the
/// [`WidgetInfo::visibility`] method.
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
    ctx.info_tree
        .find(ctx.path.widget_id())
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

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        subscriptions.event(crate::window::FrameImageReadyEvent);
        self.child.subscriptions(ctx, subscriptions);
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

/// Defines if and how an widget is hit-tested.
///
/// See [`hit_test_mode`](fn@hit_test_mode) for more details.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum HitTestMode {
    /// Widget is never hit.
    ///
    /// This mode is *sticky*, if set it disables hit-testing for the widget all its descendants.
    Disabled,
    /// Simplest mode, the widget is hit by any point that intersects the transformed inner bounds rectangle.
    Bounds,
    /// Default mode, the widget is hit by any point that intersects the transformed inner bounds rectangle except on the outside
    /// of rounded corners.
    RoundedBounds,
    /// Complex mode, every render primitive used for rendering the widget is hit-testable, the widget is hit only by
    /// points that intersect visible parts of the render primitives.
    Visual,
}
impl HitTestMode {
    /// Returns `true` if is any mode other then [`Disabled`].
    ///
    /// [`Disabled`]: Self::Disabled
    pub fn is_hit_testable(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Gets the hit-test mode of the current widget context.
    pub fn get<Vr: AsRef<VarsRead>>(vars: &Vr) -> &HitTestMode {
        HitTestModeVar::get(vars)
    }

    /// Gets the new hit-test mode of the current widget context.
    #[inline]
    pub fn get_new<Vw: AsRef<Vars>>(vars: &Vw) -> Option<&HitTestMode> {
        HitTestModeVar::get_new(vars)
    }

    /// Gets the update mask for [`WidgetSubscriptions`].
    ///
    /// [`WidgetSubscriptions`]: crate::widget_info::WidgetSubscriptions
    #[inline]
    pub fn update_mask<Vr: WithVarsRead>(vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|vars| HitTestModeVar::new().update_mask(vars))
    }
}
impl fmt::Debug for HitTestMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "HitTestMode::")?;
        }        
        match self {
            Self::Disabled => write!(f, "Disabled"),
            Self::Bounds => write!(f, "Bounds"),
            Self::RoundedBounds => write!(f, "RoundedBounds"),
            Self::Visual => write!(f, "Visual"),
        }
    }
}
impl Default for HitTestMode {
    fn default() -> Self {
        HitTestMode::RoundedBounds
    }
}
impl_from_and_into_var! {
    fn from(default_or_disabled: bool) -> HitTestMode {
        if default_or_disabled {
            HitTestMode::default()
        } else {
            HitTestMode::Disabled
        }
    }
}

context_var! {
    struct HitTestModeVar: HitTestMode = HitTestMode::default();
}

/// Defines how the widget is hit-tested.
///
/// Hit-testing determines if a point intersects with the widget, the most common hit-test point is the mouse pointer.
/// By default widgets are hit by any point inside the widget area, excluding the outer corners if [`corner_radius`] is set,
/// this is very efficient, but assumes that the widget is *filled*, if the widget has visual *holes* the user may be able
/// to see another widget underneath but be unable to click on it.
///
/// If you have a widget with a complex shape or with *holes*, set this property to [`HitTestMode::Visual`] to enable the full
/// hit-testing power where all render primitives and clips used to render the widget are considered during hit-testing.
///
/// [`hit_testable`]: fn@hit_testable
/// [`corner_radius`]: fn@crate::border::corner_radius
#[property(context, default(HitTestModeVar))]
pub fn hit_test_mode(child: impl UiNode, mode: impl IntoVar<HitTestMode>) -> impl UiNode {
    struct HitTestModeNode<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for HitTestModeNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.updates(&HitTestMode::update_mask(ctx));
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if HitTestMode::get_new(ctx).is_some() {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            match HitTestMode::get(ctx.vars) {
                HitTestMode::Disabled => {
                    frame.with_hit_tests_disabled(|frame| self.child.render(ctx, frame));
                }
                HitTestMode::Visual => frame.with_auto_hit_test(true, |frame| self.child.render(ctx, frame)),
                _ => frame.with_auto_hit_test(false, |frame| self.child.render(ctx, frame)),
            }
        }
    }

    with_context_var(
        HitTestModeNode { child },
        HitTestModeVar,
        merge_var!(HitTestModeVar::new(), mode.into_var(), |&a, &b| match (a, b) {
            (HitTestMode::Disabled, _) | (_, HitTestMode::Disabled) => HitTestMode::Disabled,
            (_, b) => b,
        }),
    )
}

/// If the widget is visible for hit-tests.
///
/// This property is used only for probing the state. You can set the state using
/// the [`hit_test_mode`] property.
///
/// [`hit_testable`]: fn@hit_testable
#[property(event)]
pub fn is_hit_testable(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsHitTestableNode<C: UiNode> {
        child: C,
        state: StateVar,
    }
    impl<C: UiNode> IsHitTestableNode<C> {
        fn update_state(&self, ctx: &mut WidgetContext) {
            let enabled = HitTestMode::get(ctx).is_hit_testable();
            self.state.set_ne(ctx.vars, enabled);
        }
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsHitTestableNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            self.update_state(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.updates(&HitTestMode::update_mask(ctx));
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            self.update_state(ctx);
        }
    }
    IsHitTestableNode { child, state }
}
