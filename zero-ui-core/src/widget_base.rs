//! The [`implicit_base`](mod@implicit_base) and properties used in all or most widgets.

use std::{fmt, mem};

use crate::{
    context::{
        state_map, InfoContext, LayoutContext, MeasureContext, OwnedStateMap, RenderContext, StateMapMut, StateMapRef, WidgetContext,
        WidgetUpdates,
    },
    event::EventUpdateArgs,
    impl_ui_node, property,
    render::{FrameBuilder, FrameUpdate, FrameValueKey, ReuseRange, SpatialFrameId},
    units::{PxCornerRadius, PxRect, PxSize, PxTransform},
    var::*,
    widget_info::{
        Interactivity, LayoutPassId, UpdateMask, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetContextInfo, WidgetInfoBuilder,
        WidgetLayout, WidgetSubscriptions,
    },
    window::WidgetInfoChangedEvent,
    FillUiNode, UiNode, Widget, WidgetId,
};

/// Base widget inherited implicitly by all [widgets](widget!) that don't inherit from
/// any other widget.
#[zero_ui_proc_macros::widget_base($crate::widget_base::implicit_base)]
pub mod implicit_base {
    use std::cell::{Cell, RefCell};

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

    /// Implicit `new_child`, does nothing, returns the [`FillUiNode`].
    pub fn new_child() -> impl UiNode {
        FillUiNode
    }

    /// Implicit `new_child_layout`, returns [`nodes::child_layout`].
    pub fn new_child_layout(child: impl UiNode) -> impl UiNode {
        nodes::child_layout(child)
    }

    /// No-op, returns `child`.
    pub fn new_child_context(child: impl UiNode) -> impl UiNode {
        child
    }

    /// No-op, returns `child`.
    pub fn new_fill(child: impl UiNode) -> impl UiNode {
        child
    }

    /// Implicit `new_border`, returns [`nodes::inner`].
    pub fn new_border(child: impl UiNode) -> impl UiNode {
        nodes::inner(child)
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
    /// Returns [`nodes::widget`].
    pub fn new(child: impl UiNode, id: impl IntoValue<WidgetId>) -> impl Widget {
        nodes::widget(child, id)
    }

    /// UI nodes used for implementing all widgets.
    pub mod nodes {
        use super::*;

        /// Returns a node that wraps `panel` and applies *child_layout* transforms to it.
        ///
        /// This node should wrap the inner most *child* node of panel widgets in the [`new_child`] constructor.
        ///
        /// [`new_child`]: super::new_child
        pub fn children_layout(panel: impl UiNode) -> impl UiNode {
            struct ChildrenLayoutNode<P> {
                panel: P,
                spatial_id: SpatialFrameId,
                translation_key: FrameValueKey<PxTransform>,
            }
            #[impl_ui_node(
                delegate = &self.panel,
                delegate_mut = &mut self.panel,
            )]
            impl<P: UiNode> UiNode for ChildrenLayoutNode<P> {
                fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                    self.panel.measure(ctx)
                }
                fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                    wl.with_children(ctx, |ctx, wl| self.panel.layout(ctx, wl))
                }
                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    let transform = PxTransform::from(ctx.widget_info.bounds.child_offset());
                    frame.push_reference_frame(self.spatial_id, self.translation_key.bind(transform, true), true, false, |frame| {
                        self.panel.render(ctx, frame)
                    });
                }
                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    let transform = PxTransform::from(ctx.widget_info.bounds.child_offset());
                    update.with_transform(self.translation_key.update(transform, true), false, |update| {
                        self.panel.render_update(ctx, update);
                    });
                }
            }
            ChildrenLayoutNode {
                panel: panel.cfg_boxed(),
                spatial_id: SpatialFrameId::new_unique(),
                translation_key: FrameValueKey::new_unique(),
            }
            .cfg_boxed()
        }

        /// Returns a node that wraps `child` and potentially applies child transforms if the `child` turns out
        /// to not be a full [`Widget`]. This is important for making properties like *padding* or *content_align* work
        /// for container widgets that accept any [`UiNode`] as content.
        ///
        /// This node should wrap the outer-most border node in the [`new_child_layout`] constructor, the implicit
        /// implementation already does this.
        ///
        /// [`new_child_layout`]: super::new_child_layout
        pub fn child_layout(child: impl UiNode) -> impl UiNode {
            struct ChildLayoutNode<C> {
                child: C,
                id: Option<(SpatialFrameId, FrameValueKey<PxTransform>)>,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for ChildLayoutNode<C> {
                fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                    self.child.measure(ctx)
                }
                fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                    let (size, needed) = wl.with_child(ctx, |ctx, wl| self.child.layout(ctx, wl));
                    if self.id.is_none() {
                        if needed {
                            // start rendering.
                            self.id = Some((SpatialFrameId::new_unique(), FrameValueKey::new_unique()));
                            ctx.updates.render();
                        }
                    } else if !needed {
                        self.id = None;
                        ctx.updates.render();
                    }
                    size
                }
                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    if let Some((id, key)) = &self.id {
                        let transform = PxTransform::from(ctx.widget_info.bounds.child_offset());
                        frame.push_reference_frame(*id, key.bind(transform, true), true, false, |frame| self.child.render(ctx, frame));
                    } else {
                        self.child.render(ctx, frame);
                    }
                }

                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    if let Some((_, key)) = &self.id {
                        let transform = PxTransform::from(ctx.widget_info.bounds.child_offset());
                        update.with_transform(key.update(transform, true), false, |update| self.child.render_update(ctx, update));
                    } else {
                        self.child.render_update(ctx, update);
                    }
                }
            }
            ChildLayoutNode {
                child: child.cfg_boxed(),
                id: None,
            }
            .cfg_boxed()
        }

        /// Returns a node that wraps `child` and marks the [`WidgetLayout::with_inner`] and [`FrameBuilder::push_inner`].
        ///
        /// This node renders the inner transform and implements the [`HitTestMode`] for the widget.
        ///
        /// This node should wrap the outer-most border node in the [`new_border`] constructor, the implicit implementation already does this.
        ///
        /// [`new_border`]: super::new_border
        pub fn inner(child: impl UiNode) -> impl UiNode {
            struct InnerNode<C> {
                child: C,
                transform_key: FrameValueKey<PxTransform>,
                hits_clip: (PxSize, PxCornerRadius),
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for InnerNode<C> {
                fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                    subs.updates(&HitTestMode::update_mask(ctx));
                    self.child.subscriptions(ctx, subs);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if HitTestMode::is_new(ctx) {
                        ctx.updates.layout();
                    }
                    self.child.update(ctx);
                }

                fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                    self.child.measure(ctx)
                }
                fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                    let size = wl.with_inner(ctx, |ctx, wl| self.child.layout(ctx, wl));

                    match HitTestMode::get(ctx.vars) {
                        HitTestMode::RoundedBounds => {
                            let clip = (size, ctx.widget_info.border.corner_radius());
                            if self.hits_clip != clip {
                                self.hits_clip = clip;
                                ctx.updates.render();
                            }
                        }
                        HitTestMode::Bounds => {
                            if self.hits_clip.0 != size {
                                self.hits_clip.0 = size;
                                ctx.updates.render();
                            }
                        }
                        _ => {}
                    }

                    size
                }
                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    frame.push_inner(ctx, self.transform_key, true, |ctx, frame| {
                        match HitTestMode::get(ctx.vars) {
                            HitTestMode::RoundedBounds => {
                                let rect = PxRect::from_size(self.hits_clip.0);
                                frame.hit_test().push_rounded_rect(rect, self.hits_clip.1);
                            }
                            HitTestMode::Bounds => {
                                frame.hit_test().push_rect(PxRect::from_size(self.hits_clip.0));
                            }
                            _ => {}
                        }
                        self.child.render(ctx, frame);
                    });
                }
                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    update.update_inner(ctx, self.transform_key, true, |ctx, update| self.child.render_update(ctx, update));
                }
            }
            InnerNode {
                child: child.cfg_boxed(),
                transform_key: FrameValueKey::new_unique(),
                hits_clip: (PxSize::zero(), PxCornerRadius::zero()),
            }
            .cfg_boxed()
        }

        /// Create a [`Widget`] node that wraps `child` and introduces a new widget context. The node calls
        /// [`WidgetContext::widget_context`], [`LayoutContext::with_widget`] and [`FrameBuilder::push_widget`]
        /// to define the widget.
        ///
        /// This node should wrap the outer-most context node in the [`new`] constructor, the implicit implementation already does this.
        ///
        /// [`new`]: super::new
        pub fn widget(child: impl UiNode, id: impl IntoValue<WidgetId>) -> impl Widget {
            struct WidgetNode<C> {
                id: WidgetId,
                state: OwnedStateMap<state_map::Widget>,
                child: C,
                info: WidgetContextInfo,
                subscriptions: RefCell<WidgetSubscriptions>,

                #[cfg(debug_assertions)]
                inited: bool,
                pending_updates: RefCell<WidgetUpdates>,
                offsets_pass: Cell<LayoutPassId>,

                reuse: RefCell<Option<ReuseRange>>,
            }
            impl<C: UiNode> UiNode for WidgetNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    #[cfg(debug_assertions)]
                    if self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::init` called in already inited widget {:?}", self.id);
                    }

                    ctx.widget_context(self.id, &self.info, &mut self.state, |ctx| self.child.init(ctx));
                    *self.pending_updates.get_mut() = WidgetUpdates::all();

                    #[cfg(debug_assertions)]
                    {
                        self.inited = true;
                    }
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::deinit` called in not inited widget {:?}", self.id);
                    }

                    ctx.widget_context(self.id, &self.info, &mut self.state, |ctx| self.child.deinit(ctx));
                    *self.pending_updates.get_mut() = WidgetUpdates::none();

                    #[cfg(debug_assertions)]
                    {
                        self.inited = false;
                    }
                }

                fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::info` called in not inited widget {:?}", self.id);
                    }

                    ctx.with_widget(self.id, &self.info, &self.state, |ctx| {
                        if mem::take(&mut self.pending_updates.borrow_mut().info) {
                            info.push_widget(self.id, self.info.bounds.clone(), self.info.border.clone(), |info| {
                                self.child.info(ctx, info)
                            });
                        } else {
                            info.push_widget_reuse(ctx);
                        }
                    });
                }

                fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::subscriptions` called in not inited widget {:?}", self.id);
                    }

                    if mem::take(&mut self.pending_updates.borrow_mut().subscriptions) {
                        let mut wgt_subs = self.subscriptions.borrow_mut();
                        *wgt_subs = WidgetSubscriptions::new();

                        ctx.with_widget(self.id, &self.info, &self.state, |ctx| {
                            self.child.subscriptions(ctx, &mut wgt_subs);
                        });

                        subs.extend(&wgt_subs);
                    } else {
                        subs.extend(&*self.subscriptions.borrow());
                    }
                }

                fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::event::<{}>` called in not inited widget {:?}", std::any::type_name::<EU>(), self.id);
                    }

                    if self.subscriptions.borrow().event_contains(args) {
                        let (_, updates) = ctx.widget_context(self.id, &self.info, &mut self.state, |ctx| {
                            args.with_widget(ctx, |ctx| {
                                self.child.event(ctx, args);
                            });
                        });
                        *self.pending_updates.get_mut() |= updates;
                    }
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::update` called in not inited widget {:?}", self.id);
                    }

                    if self.subscriptions.borrow().update_intersects(ctx.updates) {
                        let (_, updates) = ctx.widget_context(self.id, &self.info, &mut self.state, |ctx| self.child.update(ctx));
                        *self.pending_updates.get_mut() |= updates;
                    }
                }

                fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::measure` called in not inited widget {:?}", self.id);
                    }

                    let reuse = !self.pending_updates.borrow().layout;

                    ctx.with_widget(self.id, &self.info, &self.state, reuse, |ctx| self.child.measure(ctx))
                }

                fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::layout` called in not inited widget {:?}", self.id);
                    }

                    let reuse = !mem::take(&mut self.pending_updates.get_mut().layout);

                    let (child_size, updates) = ctx.with_widget(self.id, &self.info, &mut self.state, |ctx| {
                        wl.with_widget(ctx, reuse, |ctx, wl| self.child.layout(ctx, wl))
                    });
                    *self.pending_updates.get_mut() |= updates;

                    child_size
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::render` called in not inited widget {:?}", self.id);
                    }

                    let mut reuse_range = self.reuse.borrow_mut();
                    if !self.pending_updates.borrow_mut().render.take().is_none()
                        || self.offsets_pass.get() != self.info.bounds.offsets_pass()
                    {
                        // cannot reuse.
                        *reuse_range = None;
                        self.offsets_pass.set(self.info.bounds.offsets_pass());
                    }

                    ctx.with_widget(self.id, &self.info, &self.state, |ctx| {
                        frame.push_widget(ctx, &mut *reuse_range, |ctx, frame| self.child.render(ctx, frame));
                    });
                }

                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::render_update` called in not inited widget {:?}", self.id);
                    }

                    let mut reuse = true;
                    if !self.pending_updates.borrow_mut().render.take().is_none()
                        || self.offsets_pass.get() != self.info.bounds.offsets_pass()
                    {
                        reuse = false;
                        self.offsets_pass.set(self.info.bounds.offsets_pass());
                    }

                    ctx.with_widget(self.id, &self.info, &self.state, |ctx| {
                        update.update_widget(ctx, reuse, |ctx, update| self.child.render_update(ctx, update));
                    });
                }

                fn is_widget(&self) -> bool {
                    true
                }

                fn try_id(&self) -> Option<WidgetId> {
                    Some(self.id())
                }

                fn try_state(&self) -> Option<StateMapRef<state_map::Widget>> {
                    Some(self.state())
                }

                fn try_state_mut(&mut self) -> Option<StateMapMut<state_map::Widget>> {
                    Some(self.state_mut())
                }

                fn try_bounds_info(&self) -> Option<&WidgetBoundsInfo> {
                    Some(self.bounds_info())
                }

                fn try_border_info(&self) -> Option<&WidgetBorderInfo> {
                    Some(self.border_info())
                }

                fn into_widget(self) -> crate::BoxedWidget
                where
                    Self: Sized,
                {
                    self.boxed_wgt()
                }
            }
            impl<T: UiNode> Widget for WidgetNode<T> {
                fn id(&self) -> WidgetId {
                    self.id
                }

                fn state(&self) -> StateMapRef<state_map::Widget> {
                    self.state.borrow()
                }

                fn state_mut(&mut self) -> StateMapMut<state_map::Widget> {
                    self.state.borrow_mut()
                }

                fn bounds_info(&self) -> &WidgetBoundsInfo {
                    &self.info.bounds
                }

                fn border_info(&self) -> &WidgetBorderInfo {
                    &self.info.border
                }
            }
            WidgetNode {
                id: id.into(),
                state: OwnedStateMap::default(),
                child: child.cfg_boxed(),
                info: WidgetContextInfo::default(),
                subscriptions: RefCell::default(),
                #[cfg(debug_assertions)]
                inited: false,
                pending_updates: RefCell::default(),
                offsets_pass: Cell::default(),
                reuse: RefCell::default(),
            }
            .cfg_boxed_wgt()
        }
    }
}

context_var! {
    static IS_ENABLED_VAR: bool = true;
}

/// If default interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`ENABLED`] or [`DISABLED`], to probe the enabled state in `when` clauses
/// use [`is_enabled`] or [`is_disabled`]. To probe the a widget's state use [`interactivity`] value.
///
/// # Interactivity
///
/// Every widget has an [`interactivity`] value, it defines two *tiers* of disabled, the normal disabled blocks the default actions
/// of the widget, but still allows some interactions, such as a different cursor on hover or event an error tool-tip on click, the
/// second tier blocks all interaction with the widget. This property controls the *normal* disabled, to fully block interaction use
/// the [`interactive`] property.
///
/// # Disabled Visual
///
/// Widgets that are interactive should visually indicate when the normal interactions are disabled, you can use the [`is_disabled`]
/// state property in a when block to implement the *visually disabled* appearance of a widget.
///
/// The visual cue for the disabled state is usually a reduced contrast from content and background by *graying-out* the text and applying a
/// grayscale filter for image content. You should also consider adding *disabled interactions* that inform the user when the widget will be
/// enabled.
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
/// [`interactivity`]: crate::widget_info::WidgetInfo::interactivity
/// [`interactive`]: fn@interactive
/// [`is_enabled`]: fn@is_enabled
/// [`is_disabled`]: fn@is_disabled
#[property(context, default(true))]
pub fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    struct EnabledNode<C, E> {
        child: C,
        local_enabled: E,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for EnabledNode<C, E> {
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if !self.local_enabled.copy(ctx) {
                info.push_interactivity(Interactivity::DISABLED);
            }
            self.child.info(ctx, info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.local_enabled);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.local_enabled.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }
    }

    let enabled = enabled.into_var();
    with_context_var(
        EnabledNode {
            child,
            local_enabled: enabled.clone(),
        },
        &IS_ENABLED_VAR,
        merge_var!(&IS_ENABLED_VAR, enabled, |&a, &b| a && b),
    )
}

/// If any interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`BLOCKED`] when `false`, widgets with blocked interactivity do not
/// receive any interaction event and behave like a background visual. To probe the widget state use [`interactivity`] value.
///
/// This property *enables* and *disables* interaction with the widget and its descendants without causing
/// a visual change like [`enabled`], it also blocks "disabled" interactions such as a different cursor or tool-tip for disabled buttons,
/// its use cases are more advanced then [`enabled`], it is mostly used when large parts of the screen are "not ready", hopefully with a message
/// explaining things to the user.
///
/// Note that this affects the widget where it is set and descendants, to disable interaction only in the widgets
/// inside `child` use the [`interactive_node`].
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
            if !self.interactive.copy(ctx) {
                info.push_interactivity(Interactivity::BLOCKED);
            }
            self.child.info(ctx, info);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.interactive);
            self.child.subscriptions(ctx, subs);
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

/// Create a node that disables interaction for all widget inside `node` using [`BLOCKED`].
///
/// Unlike the [`interactive`] property this does not apply to the contextual widget, only `child` and descendants.
///
/// The node works for both if the `child` is a widget or if it contains widgets, the performance
/// is slightly better if the `child` is a widget directly.
///
/// [`BLOCKED`]: Interactivity::BLOCKED
/// [`interactive`]: fn@interactive
pub fn interactive_node(child: impl UiNode, interactive: impl IntoVar<bool>) -> impl UiNode {
    struct BlockInteractionNode<C, I> {
        child: C,
        interactive: I,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, I: Var<bool>> UiNode for BlockInteractionNode<C, I> {
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if self.interactive.copy(ctx) {
                self.child.info(ctx, info);
            } else if let Some(id) = self.child.try_id() {
                // child is a widget.
                info.push_interactivity_filter(move |args| {
                    if args.info.widget_id() == id {
                        Interactivity::BLOCKED
                    } else {
                        Interactivity::ENABLED
                    }
                });
                self.child.info(ctx, info);
            } else {
                let block_range = info.with_children_range(|info| self.child.info(ctx, info));
                if !block_range.is_empty() {
                    // has child widgets.

                    let id = ctx.path.widget_id();
                    info.push_interactivity_filter(move |args| {
                        if let Some(parent) = args.info.parent() {
                            if parent.widget_id() == id {
                                // check child range
                                for (i, item) in parent.children().enumerate() {
                                    if item == args.info {
                                        return if !block_range.contains(&i) {
                                            Interactivity::ENABLED
                                        } else {
                                            Interactivity::BLOCKED
                                        };
                                    } else if i >= block_range.end {
                                        break;
                                    }
                                }
                            }
                        }
                        Interactivity::ENABLED
                    });
                }
            }
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.interactive);
            self.child.subscriptions(ctx, subs);
        }
    }
    BlockInteractionNode {
        child: child.cfg_boxed(),
        interactive: interactive.into_var(),
    }
    .cfg_boxed()
}

fn vis_enabled_eq_state(child: impl UiNode, state: StateVar, expected: bool) -> impl UiNode {
    event_state(child, state, true, WidgetInfoChangedEvent, move |ctx, _| {
        let is_enabled = ctx
            .info_tree
            .get(ctx.path.widget_id())
            .unwrap()
            .interactivity()
            .is_visually_enabled();

        Some(is_enabled == expected)
    })
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
    vis_enabled_eq_state(child, state, true)
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
    vis_enabled_eq_state(child, state, false)
}

/// Sets the widget visibility.
///
/// This property causes the widget to have the `visibility`, the widget actual visibility is computed, for example,
/// widgets that don't render anything are considered `Hidden` even if the visibility property is not set, this property
/// only forces the widget to layout and render according to the specified visibility.
///
/// To probe the visibility state of a widget in `when` clauses use [`is_visible`], [`is_hidden`] or [`is_collapsed`] in `when` clauses,
/// to probe a widget state use [`Widget::render_info`] or [`WidgetInfo::visibility`].
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

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.visibility);
            self.child.subscriptions(ctx, subs);
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

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            if Visibility::Collapsed != self.visibility.copy(ctx) {
                self.child.measure(ctx)
            } else {
                PxSize::zero()
            }
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            if Visibility::Collapsed != self.visibility.copy(ctx) {
                self.child.layout(ctx, wl)
            } else {
                wl.collapse(ctx);
                PxSize::zero()
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            match self.visibility.copy(ctx) {
                Visibility::Visible => self.child.render(ctx, frame),
                Visibility::Hidden => frame.hide(|frame| self.child.render(ctx, frame)),
                Visibility::Collapsed => frame.collapse(ctx.info_tree),
            }
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            match self.visibility.copy(ctx) {
                Visibility::Visible => self.child.render_update(ctx, update),
                Visibility::Hidden => update.hidden(|update| self.child.render_update(ctx, update)),
                Visibility::Collapsed => {}
            }
        }
    }
    VisibilityNode {
        child,
        prev_vis: Visibility::Visible,
        visibility: visibility.into_var(),
    }
}

fn visibility_eq_state(child: impl UiNode, state: StateVar, expected: Visibility) -> impl UiNode {
    event_state(
        child,
        state,
        expected == Visibility::Visible,
        crate::window::FrameImageReadyEvent,
        move |ctx, _| {
            let vis = ctx
                .info_tree
                .get(ctx.path.widget_id())
                .map(|w| w.visibility())
                .unwrap_or(Visibility::Visible);

            Some(vis == expected)
        },
    )
}
/// If the widget is [`Visible`](Visibility::Visible).
#[property(context)]
pub fn is_visible(child: impl UiNode, state: StateVar) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Visible)
}
/// If the widget is [`Hidden`](Visibility::Hidden).
#[property(context)]
pub fn is_hidden(child: impl UiNode, state: StateVar) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Hidden)
}
/// If the widget is [`Collapsed`](Visibility::Collapsed).
#[property(context)]
pub fn is_collapsed(child: impl UiNode, state: StateVar) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Collapsed)
}

/// Defines if and how a widget is hit-tested.
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
    pub fn get<Vr: WithVarsRead>(vars: &Vr) -> HitTestMode {
        HIT_TEST_MODE_VAR.as_ref().get_clone(vars)
    }

    /// Gets the new hit-test mode of the current widget context.
    pub fn get_new<Vw: WithVars>(vars: &Vw) -> Option<HitTestMode> {
        HIT_TEST_MODE_VAR.as_ref().clone_new(vars)
    }

    /// Gets if the hit-test mode has changed.
    pub fn is_new<Vw: WithVars>(vars: &Vw) -> bool {
        HIT_TEST_MODE_VAR.as_ref().is_new(vars)
    }

    /// Gets the update mask for [`WidgetSubscriptions`].
    ///
    /// [`WidgetSubscriptions`]: crate::widget_info::WidgetSubscriptions
    pub fn update_mask<Vr: WithVarsRead>(vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|vars| HIT_TEST_MODE_VAR.as_ref().update_mask(vars))
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
    static HIT_TEST_MODE_VAR: HitTestMode = HitTestMode::default();
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
#[property(context, default(&HIT_TEST_MODE_VAR))]
pub fn hit_test_mode(child: impl UiNode, mode: impl IntoVar<HitTestMode>) -> impl UiNode {
    struct HitTestModeNode<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for HitTestModeNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.updates(&HitTestMode::update_mask(ctx));
            self.child.subscriptions(ctx, subs);
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

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            update.with_auto_hit_test(matches!(HitTestMode::get(ctx.vars), HitTestMode::Visual), |update| {
                self.child.render_update(ctx, update)
            });
        }
    }

    with_context_var(
        HitTestModeNode { child },
        &HIT_TEST_MODE_VAR,
        merge_var!(&HIT_TEST_MODE_VAR, mode.into_var(), |&a, &b| match (a, b) {
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
/// [`hit_test_mode`]: fn@hit_test_mode
#[property(event)]
pub fn is_hit_testable(child: impl UiNode, state: StateVar) -> impl UiNode {
    bind_state(child, HIT_TEST_MODE_VAR.as_ref().map(|m| m.is_hit_testable()), state)
}
