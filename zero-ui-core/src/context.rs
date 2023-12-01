//! New static contexts.

use std::{fmt, mem, sync::Arc, task::Waker};

use atomic::{Atomic, Ordering::Relaxed};
use parking_lot::{Mutex, RwLock};

mod state;
pub use state::*;

mod trace;
pub use trace::*;

mod local;
pub use local::*;

use zero_ui_txt::formatx;

use crate::{
    app::{AppDisconnected, AppEventSender, LoopTimer},
    crate_util::{Handle, HandleOwner, IdSet, WeakHandle},
    event::{Event, EventArgs, EventHandle, EventHandles, EventUpdate, EVENTS, EVENTS_SV},
    handler::{AppHandler, AppHandlerArgs, AppWeakHandle},
    render::ReuseRange,
    text::Txt,
    timer::TIMERS_SV,
    units::*,
    var::context_var,
    var::{AnyVar, AnyVarSubscribe, Var, VarHandle, VarHandles, VarSubscribe, VarValue, VARS},
    widget_info::{InlineSegmentPos, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoTree, WidgetPath},
    widget_instance::WidgetId,
    window::{WindowId, WindowMode},
};

bitflags! {
    #[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
    #[repr(transparent)]
    pub(crate) struct UpdateFlags: u8 {
        const REINIT =        0b1000_0000;
        const INFO =          0b0001_0000;
        const UPDATE =        0b0000_0001;
        const LAYOUT =        0b0000_0010;
        const RENDER =        0b0000_0100;
        const RENDER_UPDATE = 0b0000_1000;
    }
}

struct WindowCtxData {
    id: WindowId,
    mode: WindowMode,
    state: RwLock<OwnedStateMap<WINDOW>>,
    widget_tree: RwLock<Option<WidgetInfoTree>>,

    #[cfg(any(test, doc, feature = "test_util"))]
    frame_id: Atomic<crate::render::FrameId>,
}
impl WindowCtxData {
    #[track_caller]
    fn no_context() -> Self {
        panic!("no window in context")
    }
}

/// Defines the backing data of [`WINDOW`].
///
/// Each window owns this data and calls [`WINDOW.with_context`](WINDOW::with_context) to delegate to it's child node.
pub struct WindowCtx(Option<Arc<WindowCtxData>>);
impl WindowCtx {
    /// New window context.
    pub fn new(id: WindowId, mode: WindowMode) -> Self {
        Self(Some(Arc::new(WindowCtxData {
            id,
            mode,
            state: RwLock::new(OwnedStateMap::default()),
            widget_tree: RwLock::new(None),

            #[cfg(any(test, doc, feature = "test_util"))]
            frame_id: Atomic::new(crate::render::FrameId::first()),
        })))
    }

    /// Sets the widget tree, must be called after every info update.
    ///
    /// Window contexts are partially available in the window new closure, but values like the `widget_tree` is
    /// available on init, so a [`WidgetInfoTree::wgt`] must be set as soon as the window and widget ID are available.
    pub fn set_widget_tree(&mut self, widget_tree: WidgetInfoTree) {
        *self.0.as_mut().unwrap().widget_tree.write() = Some(widget_tree);
    }

    /// Gets the window ID.
    pub fn id(&self) -> WindowId {
        self.0.as_ref().unwrap().id
    }

    /// Gets the window mode.
    pub fn mode(&self) -> WindowMode {
        self.0.as_ref().unwrap().mode
    }

    /// Gets the window tree.
    pub fn widget_tree(&self) -> WidgetInfoTree {
        self.0.as_ref().unwrap().widget_tree.read().as_ref().unwrap().clone()
    }

    /// Call `f` with an exclusive lock to the window state.
    pub fn with_state<R>(&mut self, f: impl FnOnce(&mut OwnedStateMap<WINDOW>) -> R) -> R {
        f(&mut self.0.as_mut().unwrap().state.write())
    }
}

struct WidgetCtxData {
    parent_id: Atomic<Option<WidgetId>>,
    id: WidgetId,
    flags: Atomic<UpdateFlags>,
    state: RwLock<OwnedStateMap<WIDGET>>,
    handles: WidgetHandlesCtxData,
    bounds: Mutex<WidgetBoundsInfo>,
    border: Mutex<WidgetBorderInfo>,
    render_reuse: Mutex<Option<ReuseRange>>,
}
impl WidgetCtxData {
    #[track_caller]
    fn no_context() -> Self {
        panic!("no widget in context")
    }
}

struct WidgetHandlesCtxData {
    var_handles: Mutex<VarHandles>,
    event_handles: Mutex<EventHandles>,
}

impl WidgetHandlesCtxData {
    const fn dummy() -> Self {
        Self {
            var_handles: Mutex::new(VarHandles::dummy()),
            event_handles: Mutex::new(EventHandles::dummy()),
        }
    }
}

/// Defines the backing data for [`WIDGET.with_handles`].
///
/// [`WIDGET.with_handles`]: WIDGET::with_handles
pub struct WidgetHandlesCtx(Option<Arc<WidgetHandlesCtxData>>);
impl WidgetHandlesCtx {
    /// New empty.
    pub fn new() -> Self {
        Self(Some(Arc::new(WidgetHandlesCtxData::dummy())))
    }

    /// Drop all handles.
    pub fn clear(&mut self) {
        let h = self.0.as_ref().unwrap();
        h.var_handles.lock().clear();
        h.event_handles.lock().clear();
    }
}
impl Default for WidgetHandlesCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// Defines the backing data of [`WIDGET`].
///
/// Each widget owns this data and calls [`WIDGET.with_context`] to delegate to it's child node.
///
/// [`WIDGET.with_context`]: WIDGET::with_context
pub struct WidgetCtx(Option<Arc<WidgetCtxData>>);
impl WidgetCtx {
    /// New widget context.
    pub fn new(id: WidgetId) -> Self {
        Self(Some(Arc::new(WidgetCtxData {
            parent_id: Atomic::new(None),
            id,
            flags: Atomic::new(UpdateFlags::empty()),
            state: RwLock::new(OwnedStateMap::default()),
            handles: WidgetHandlesCtxData::dummy(),
            bounds: Mutex::new(WidgetBoundsInfo::default()),
            border: Mutex::new(WidgetBorderInfo::default()),
            render_reuse: Mutex::new(None),
        })))
    }

    #[cfg(test)]
    pub(crate) fn new_test(id: WidgetId, bounds: WidgetBoundsInfo, border: WidgetBorderInfo) -> Self {
        let ctx = Self::new(id);
        let c = ctx.0.as_ref().unwrap();
        *c.bounds.lock() = bounds;
        *c.border.lock() = border;
        ctx
    }

    /// Drops all var and event handles, clears all state.
    ///
    /// If `retain_state` is enabled the state will not be cleared and can still read.
    pub fn deinit(&mut self, retain_state: bool) {
        let ctx = self.0.as_mut().unwrap();
        ctx.handles.var_handles.lock().clear();
        ctx.handles.event_handles.lock().clear();
        ctx.flags.store(UpdateFlags::empty(), Relaxed);
        *ctx.render_reuse.lock() = None;

        if !retain_state {
            ctx.state.write().clear();
        }
    }

    /// Returns `true` if reinit was requested for the widget.
    ///
    /// Note that widget implementers must use [`take_reinit`] to fulfill the request.
    ///
    /// [`take_reinit`]: Self::take_reinit
    pub fn is_pending_reinit(&self) -> bool {
        self.0.as_ref().unwrap().flags.load(Relaxed).contains(UpdateFlags::REINIT)
    }

    /// Returns `true` if an [`WIDGET.reinit`] request was made.
    ///
    /// Unlike other requests, the widget implement must re-init immediately.
    ///
    /// [`WIDGET.reinit`]: WIDGET::reinit
    pub fn take_reinit(&mut self) -> bool {
        let ctx = self.0.as_mut().unwrap();

        let mut flags = ctx.flags.load(Relaxed);
        let r = flags.contains(UpdateFlags::REINIT);
        if r {
            flags.remove(UpdateFlags::REINIT);
            ctx.flags.store(flags, Relaxed);
        }

        r
    }

    /// Gets the widget id.
    pub fn id(&self) -> WidgetId {
        self.0.as_ref().unwrap().id
    }
    /// Gets the widget bounds.
    pub fn bounds(&self) -> WidgetBoundsInfo {
        self.0.as_ref().unwrap().bounds.lock().clone()
    }

    /// Gets the widget borders.
    pub fn border(&self) -> WidgetBorderInfo {
        self.0.as_ref().unwrap().border.lock().clone()
    }

    /// Call `f` with an exclusive lock to the widget state.
    pub fn with_state<R>(&mut self, f: impl FnOnce(&mut OwnedStateMap<WIDGET>) -> R) -> R {
        f(&mut self.0.as_mut().unwrap().state.write())
    }
}

context_local! {
    static WINDOW_CTX: WindowCtxData = WindowCtxData::no_context();
    static WIDGET_CTX: WidgetCtxData = WidgetCtxData::no_context();
    static WIDGET_HANDLES_CTX: WidgetHandlesCtxData = WidgetHandlesCtxData::dummy();
}

/// Current context window.
///
/// This represents the minimum features required for a window context, see [`WINDOW_Ext`] for more
/// features provided by the core window implementation.
///
/// [`WINDOW_Ext`]: crate::window::WINDOW_Ext
pub struct WINDOW;
impl WINDOW {
    /// Calls `f` while the window is set to `ctx`.
    ///
    /// The `ctx` must be `Some(_)`, it will be moved to the [`WINDOW`] storage and back to `ctx` after `f` returns.
    pub fn with_context<R>(&self, ctx: &mut WindowCtx, f: impl FnOnce() -> R) -> R {
        let _span = match ctx.0.as_mut() {
            Some(c) => UpdatesTrace::window_span(c.id),
            None => panic!("window is required"),
        };
        WINDOW_CTX.with_context(&mut ctx.0, f)
    }

    /// Calls `f` while no window is available in the context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        WINDOW_CTX.with_default(f)
    }

    /// Returns `true` if called inside a window.
    pub fn is_in_window(&self) -> bool {
        !WINDOW_CTX.is_default()
    }

    /// Get the widget ID, if called inside a window.
    pub fn try_id(&self) -> Option<WindowId> {
        if WINDOW_CTX.is_default() {
            None
        } else {
            Some(WINDOW_CTX.get().id)
        }
    }

    /// Get the widget ID if called inside a widget, or panic.
    pub fn id(&self) -> WindowId {
        WINDOW_CTX.get().id
    }

    /// Get the window mode.
    pub fn mode(&self) -> WindowMode {
        WINDOW_CTX.get().mode
    }

    /// Gets the window info tree.
    ///
    /// Returns `None` if the window is not inited, panics if called outside of a window or window init closure.
    pub fn info(&self) -> WidgetInfoTree {
        WINDOW_CTX.get().widget_tree.read().clone().expect("window not init")
    }

    /// Calls `f` with a read lock on the current window state map.
    ///
    /// Note that this locks the entire [`WINDOW`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state<R>(&self, f: impl FnOnce(StateMapRef<WINDOW>) -> R) -> R {
        f(WINDOW_CTX.get().state.read().borrow())
    }

    /// Calls `f` with a write lock on the current window state map.
    ///
    /// Note that this locks the entire [`WINDOW`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state_mut<R>(&self, f: impl FnOnce(StateMapMut<WINDOW>) -> R) -> R {
        f(WINDOW_CTX.get().state.write().borrow_mut())
    }

    /// Get the window state `id`, if it is set.
    ///
    /// Panics if not called inside a window.
    pub fn get_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> Option<T> {
        let id = id.into();
        self.with_state(|s| s.get_clone(id))
    }

    /// Require the window state `id`.
    ///
    /// Panics if the `id` is not set or is not called inside a window.
    pub fn req_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> T {
        let id = id.into();
        self.with_state(|s| s.req(id).clone())
    }

    /// Set the window state `id` to `value`.
    ///
    /// Returns the previous set value.
    pub fn set_state<T: StateValue>(&self, id: impl Into<StateId<T>>, value: impl Into<T>) -> Option<T> {
        let id = id.into();
        let value = value.into();
        self.with_state_mut(|mut s| s.set(id, value))
    }

    /// Sets the window state `id` without value.
    ///
    /// Returns if the state `id` was already flagged.
    pub fn flag_state(&self, id: impl Into<StateId<()>>) -> bool {
        let id = id.into();
        self.with_state_mut(|mut s| s.flag(id))
    }

    /// Calls `init` and sets `id` if the `id` is not already set in the widget.
    pub fn init_state<T: StateValue>(&self, id: impl Into<StateId<T>>, init: impl FnOnce() -> T) {
        let id = id.into();
        self.with_state_mut(|mut s| {
            s.entry(id).or_insert_with(init);
        });
    }

    /// Sets the `id` to the default value if it is not already set.
    pub fn init_state_default<T: StateValue + Default>(&self, id: impl Into<StateId<T>>) {
        self.init_state(id.into(), Default::default)
    }

    /// Returns `true` if the `id` is set or flagged in the window.
    pub fn contains_state<T: StateValue>(&self, id: impl Into<StateId<T>>) -> bool {
        let id = id.into();
        self.with_state(|s| s.contains(id))
    }
}

#[cfg(any(test, doc, feature = "test_util"))]
static TEST_WINDOW_CFG: StaticStateId<TestWindowCfg> = StaticStateId::new_unique();

#[cfg(any(test, doc, feature = "test_util"))]
struct TestWindowCfg {
    size: PxSize,
}

/// Test only methods.
#[cfg(any(test, doc, feature = "test_util"))]
impl WINDOW {
    /// Calls `f` inside a new headless window and root widget.
    pub fn with_test_context<R>(&self, update_mode: WidgetUpdateMode, f: impl FnOnce() -> R) -> R {
        let window_id = WindowId::new_unique();
        let root_id = WidgetId::new_unique();
        let mut ctx = WindowCtx::new(window_id, WindowMode::Headless);
        ctx.set_widget_tree(WidgetInfoTree::wgt(window_id, root_id));
        WINDOW.with_context(&mut ctx, || {
            WINDOW.set_state(
                &TEST_WINDOW_CFG,
                TestWindowCfg {
                    size: PxSize::new(Px(1132), Px(956)),
                },
            );

            let mut ctx = WidgetCtx::new(root_id);
            WIDGET.with_context(&mut ctx, update_mode, f)
        })
    }

    /// Get the test window size.
    pub fn test_window_size(&self) -> PxSize {
        WINDOW.with_state_mut(|mut s| s.get_mut(&TEST_WINDOW_CFG).expect("not in test window").size)
    }

    /// Set test window `size`.
    pub fn set_test_window_size(&self, size: PxSize) {
        WINDOW.with_state_mut(|mut s| {
            s.get_mut(&TEST_WINDOW_CFG).expect("not in test window").size = size;
        })
    }

    /// Call inside [`with_test_context`] to init the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_init(&self, content: &mut impl crate::widget_instance::UiNode) -> ContextUpdates {
        content.init();
        WIDGET.test_root_updates();
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to deinit the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_deinit(&self, content: &mut impl crate::widget_instance::UiNode) -> ContextUpdates {
        content.deinit();
        WIDGET.test_root_updates();
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to rebuild info the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_info(&self, content: &mut impl crate::widget_instance::UiNode) -> ContextUpdates {
        let l_size = self.test_window_size();
        let mut info = crate::widget_info::WidgetInfoBuilder::new(
            Arc::default(),
            WINDOW.id(),
            crate::widget_info::access::AccessEnabled::APP,
            WIDGET.id(),
            WidgetBoundsInfo::new_size(l_size, l_size),
            WidgetBorderInfo::new(),
            1.fct(),
        );
        content.info(&mut info);
        let tree = info.finalize(Some(self.info()));
        *WINDOW_CTX.get().widget_tree.write() = Some(tree);
        WIDGET.test_root_updates();
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to delivery an event to the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_event(&self, content: &mut impl crate::widget_instance::UiNode, update: &mut EventUpdate) -> ContextUpdates {
        update.delivery_list_mut().fulfill_search([&WINDOW.info()].into_iter());
        content.event(update);
        WIDGET.test_root_updates();
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to update the `content` as a child of the test window root.
    ///
    /// The `updates` can be set to a custom delivery list, otherwise window root and `content` widget are flagged for update.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_update(&self, content: &mut impl crate::widget_instance::UiNode, updates: Option<&mut WidgetUpdates>) -> ContextUpdates {
        if let Some(updates) = updates {
            updates.delivery_list_mut().fulfill_search([&WINDOW.info()].into_iter());
            content.update(updates)
        } else {
            let target = if let Some(content_id) = content.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()) {
                WidgetPath::new(WINDOW.id(), vec![WIDGET.id(), content_id].into())
            } else {
                WidgetPath::new(WINDOW.id(), vec![WIDGET.id()].into())
            };

            let mut updates = WidgetUpdates::new(UpdateDeliveryList::new_any());
            updates.delivery_list.insert_path(&target);

            content.update(&updates);
        }
        WIDGET.test_root_updates();
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to layout the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_layout(
        &self,
        content: &mut impl crate::widget_instance::UiNode,
        constraints: Option<PxConstraints2d>,
    ) -> (PxSize, ContextUpdates) {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());
        let viewport = self.test_window_size();
        let mut metrics = LayoutMetrics::new(1.fct(), viewport, font_size);
        if let Some(c) = constraints {
            metrics = metrics.with_constraints(c);
        }
        let mut updates = LayoutUpdates::new(UpdateDeliveryList::new_any());
        updates.delivery_list.insert_updates_root(WINDOW.id(), WIDGET.id());
        let size = LAYOUT.with_context(metrics, || {
            crate::widget_info::WidgetLayout::with_root_widget(Arc::new(updates), |wl| content.layout(wl))
        });
        WIDGET.test_root_updates();
        (size, UPDATES.apply())
    }

    /// Call inside [`with_test_context`] to layout the `content` as a child of the test window root.
    ///
    /// Returns the measure and layout size, and the requested updates.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_layout_inline(
        &self,
        content: &mut impl crate::widget_instance::UiNode,
        measure_constraints: (PxConstraints2d, InlineConstraintsMeasure),
        layout_constraints: (PxConstraints2d, InlineConstraintsLayout),
    ) -> ((PxSize, PxSize), ContextUpdates) {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());
        let viewport = self.test_window_size();

        let metrics = LayoutMetrics::new(1.fct(), viewport, font_size)
            .with_constraints(measure_constraints.0)
            .with_inline_constraints(Some(InlineConstraints::Measure(measure_constraints.1)));
        let measure_size = LAYOUT.with_context(metrics, || {
            content.measure(&mut crate::widget_info::WidgetMeasure::new(Arc::default()))
        });

        let metrics = LayoutMetrics::new(1.fct(), viewport, font_size)
            .with_constraints(layout_constraints.0)
            .with_inline_constraints(Some(InlineConstraints::Layout(layout_constraints.1)));

        let mut updates = LayoutUpdates::new(UpdateDeliveryList::new_any());
        updates.delivery_list.insert_updates_root(WINDOW.id(), WIDGET.id());

        let layout_size = LAYOUT.with_context(metrics, || {
            crate::widget_info::WidgetLayout::with_root_widget(Arc::new(updates), |wl| content.layout(wl))
        });
        WIDGET.test_root_updates();
        ((measure_size, layout_size), UPDATES.apply())
    }

    /// Call inside [`with_test_context`] to render the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_render(&self, content: &mut impl crate::widget_instance::UiNode) -> (crate::render::BuiltFrame, ContextUpdates) {
        use crate::render::*;

        let mut frame = {
            let win = WINDOW_CTX.get();
            let wgt = WIDGET_CTX.get();

            let frame_id = win.frame_id.load(Relaxed);
            win.frame_id.store(frame_id.next(), Relaxed);

            let f = FrameBuilder::new_renderless(
                Arc::default(),
                Arc::default(),
                frame_id,
                wgt.id,
                &wgt.bounds.lock(),
                win.widget_tree.read().as_ref().unwrap(),
                1.fct(),
                crate::text::FontAntiAliasing::Default,
            );
            f
        };

        frame.push_inner(self.test_root_translation_key(), false, |frame| {
            content.render(frame);
        });

        let tree = WINDOW_CTX.get().widget_tree.read().as_ref().unwrap().clone();
        let f = frame.finalize(&tree);
        WIDGET.test_root_updates();
        (f, UPDATES.apply())
    }

    /// Call inside [`with_test_context`] to render_update the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_render_update(
        &self,
        content: &mut impl crate::widget_instance::UiNode,
    ) -> (crate::render::BuiltFrameUpdate, ContextUpdates) {
        use crate::render::*;

        let mut update = {
            let win = WINDOW_CTX.get();
            let wgt = WIDGET_CTX.get();

            let frame_id = win.frame_id.load(Relaxed);
            win.frame_id.store(frame_id.next_update(), Relaxed);

            let f = FrameUpdate::new(
                Arc::default(),
                frame_id,
                wgt.id,
                wgt.bounds.lock().clone(),
                None,
                crate::color::RenderColor::BLACK,
            );
            f
        };

        update.update_inner(self.test_root_translation_key(), false, |update| {
            content.render_update(update);
        });
        let tree = WINDOW_CTX.get().widget_tree.read().as_ref().unwrap().clone();
        let f = update.finalize(&tree);
        WIDGET.test_root_updates();
        (f, UPDATES.apply())
    }

    fn test_root_translation_key(&self) -> crate::render::FrameValueKey<PxTransform> {
        static ID: StaticStateId<crate::render::FrameValueKey<PxTransform>> = StaticStateId::new_unique();
        WINDOW.with_state_mut(|mut s| *s.entry(&ID).or_insert_with(crate::render::FrameValueKey::new_unique))
    }
}

/// Defines how widget update requests inside [`WIDGET::with_context`] are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetUpdateMode {
    /// All updates flagged during the closure call are discarded, previous pending
    /// requests are retained.
    ///
    /// This mode is used by [`UiNode::with_context`] and [`UiNodeOp::Measure`].
    ///
    /// [`UiNodeOp::Measure`]: crate::widget_instance::UiNodeOp::Measure
    Ignore,
    /// All updates flagged after the closure call are retained and propagate to the parent widget flags.
    ///
    /// This is the mode is used for all [`UiNodeOp`] delegation, except measure.
    ///
    /// [`UiNodeOp`]: crate::widget_instance::UiNodeOp
    Bubble,
}

/// Current context widget.
pub struct WIDGET;
impl WIDGET {
    /// Calls `f` while the widget is set to `ctx`.
    ///
    /// The `ctx` must be `Some(_)`, it will be moved to the [`WIDGET`] storage and back to `ctx` after `f` returns.
    ///
    /// If `update_mode` is [`WidgetUpdateMode::Bubble`] the update flags requested for the `ctx` after `f` will be copied to the
    /// caller widget context, otherwise they are ignored.
    pub fn with_context<R>(&self, ctx: &mut WidgetCtx, update_mode: WidgetUpdateMode, f: impl FnOnce() -> R) -> R {
        let parent_id = WIDGET.try_id();

        if let Some(ctx) = ctx.0.as_mut() {
            ctx.parent_id.store(parent_id, Relaxed);
        } else {
            unreachable!()
        }

        let prev_flags = match update_mode {
            WidgetUpdateMode::Ignore => ctx.0.as_mut().unwrap().flags.load(Relaxed),
            WidgetUpdateMode::Bubble => UpdateFlags::empty(),
        };

        // call `f` in context.
        let r = WIDGET_CTX.with_context(&mut ctx.0, f);

        let ctx = ctx.0.as_mut().unwrap();

        match update_mode {
            WidgetUpdateMode::Ignore => {
                ctx.flags.store(prev_flags, Relaxed);
            }
            WidgetUpdateMode::Bubble => {
                let wgt_flags = ctx.flags.load(Relaxed);

                if let Some(parent) = parent_id.map(|_| WIDGET_CTX.get()) {
                    let propagate = wgt_flags
                        & (UpdateFlags::UPDATE
                            | UpdateFlags::INFO
                            | UpdateFlags::LAYOUT
                            | UpdateFlags::RENDER
                            | UpdateFlags::RENDER_UPDATE);

                    let _ = parent.flags.fetch_update(Relaxed, Relaxed, |mut u| {
                        if !u.contains(propagate) {
                            u.insert(propagate);
                            Some(u)
                        } else {
                            None
                        }
                    });
                    ctx.parent_id.store(None, Relaxed);
                } else if let Some(window_id) = WINDOW.try_id() {
                    // is at root, register `UPDATES`
                    UPDATES.update_flags_root(wgt_flags, window_id, ctx.id);
                    // some builders don't clear the root widget flags like they do for other widgets.
                    ctx.flags.store(UpdateFlags::empty(), Relaxed);
                } else {
                    // used outside window
                    UPDATES.update_flags(wgt_flags, ctx.id);
                    ctx.flags.store(UpdateFlags::empty(), Relaxed);
                }
            }
        }

        r
    }
    #[cfg(any(test, doc, feature = "test_util"))]
    pub(crate) fn test_root_updates(&self) {
        let ctx = WIDGET_CTX.get();
        // is at root, register `UPDATES`
        UPDATES.update_flags_root(ctx.flags.load(Relaxed), WINDOW.id(), ctx.id);
        // some builders don't clear the root widget flags like they do for other widgets.
        ctx.flags.store(UpdateFlags::empty(), Relaxed);
    }

    /// Calls `f` while no widget is available in the context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        WIDGET_CTX.with_default(f)
    }

    /// Calls `f` with an override target for var and event subscription handles.
    pub fn with_handles<R>(&self, handles: &mut WidgetHandlesCtx, f: impl FnOnce() -> R) -> R {
        WIDGET_HANDLES_CTX.with_context(&mut handles.0, f)
    }

    /// Returns `true` if called inside a widget.
    pub fn is_in_widget(&self) -> bool {
        !WIDGET_CTX.is_default()
    }

    /// Get the widget ID, if called inside a widget.
    pub fn try_id(&self) -> Option<WidgetId> {
        if self.is_in_widget() {
            Some(WIDGET_CTX.get().id)
        } else {
            None
        }
    }

    /// Gets a text with detailed path to the current widget.
    ///
    /// This can be used to quickly identify the current widget during debug, the path printout will contain
    /// the widget types if the inspector metadata is found for the widget.
    ///
    /// This method does not panic if called outside of an widget.
    pub fn trace_path(&self) -> Txt {
        if let Some(w_id) = WINDOW.try_id() {
            if let Some(id) = self.try_id() {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(id) {
                    wgt.trace_path()
                } else {
                    formatx!("{w_id:?}//<no-info>/{id:?}")
                }
            } else {
                formatx!("{w_id:?}//<no-widget>")
            }
        } else if let Some(id) = self.try_id() {
            formatx!("<no-window>//{id:?}")
        } else {
            Txt::from_str("<no-widget>")
        }
    }

    /// Gets a text with a detailed widget id.
    ///
    /// This can be used to quickly identify the current widget during debug, the printout will contain the widget
    /// type if the inspector metadata is found for the widget.
    ///
    /// This method does not panic if called outside of an widget.
    pub fn trace_id(&self) -> Txt {
        if let Some(id) = self.try_id() {
            if WINDOW.try_id().is_some() {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(id) {
                    wgt.trace_id()
                } else {
                    formatx!("{id:?}")
                }
            } else {
                formatx!("{id:?}")
            }
        } else {
            Txt::from("<no-widget>")
        }
    }

    /// Get the widget ID if called inside a widget, or panic.
    pub fn id(&self) -> WidgetId {
        WIDGET_CTX.get().id
    }

    /// Gets the widget info.
    ///
    /// # Panics
    ///
    /// If called before the widget info is inited in the parent window.
    pub fn info(&self) -> WidgetInfo {
        WINDOW.info().get(WIDGET.id()).expect("widget info not init")
    }

    /// Schedule an [`UpdateOp`] for the current widget.
    pub fn update_op(&self, op: UpdateOp) -> &Self {
        match op {
            UpdateOp::Update => self.update(),
            UpdateOp::Info => self.update_info(),
            UpdateOp::Layout => self.layout(),
            UpdateOp::Render => self.render(),
            UpdateOp::RenderUpdate => self.render_update(),
        }
    }

    fn update_impl(&self, flag: UpdateFlags) -> &Self {
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if !f.contains(flag) {
                f.insert(flag);
                Some(f)
            } else {
                None
            }
        });
        self
    }

    /// Schedule an update for the current widget.
    ///
    /// After the current update cycle the app-extensions, parent window and widgets will update again.
    pub fn update(&self) -> &Self {
        self.update_impl(UpdateFlags::UPDATE)
    }

    /// Schedule an info rebuild for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-build the info tree.
    pub fn update_info(&self) -> &Self {
        self.update_impl(UpdateFlags::INFO)
    }

    /// Schedule a re-layout for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-layout.
    pub fn layout(&self) -> &Self {
        self.update_impl(UpdateFlags::LAYOUT)
    }

    /// Schedule a re-render for the current widget.
    ///
    /// After all requested updates and layouts apply the parent window and widgets will re-render.
    ///
    /// This also overrides any pending [`render_update`] request.
    ///
    /// [`render_update`]: Self::render_update
    pub fn render(&self) -> &Self {
        self.update_impl(UpdateFlags::RENDER)
    }

    /// Schedule a frame update for the current widget.
    ///
    /// After all requested updates and layouts apply the parent window and widgets will update the frame.
    ///
    /// This request is supplanted by any [`render`] request.
    ///
    /// [`render`]: Self::render
    pub fn render_update(&self) -> &Self {
        self.update_impl(UpdateFlags::RENDER_UPDATE)
    }

    /// Flags the widget to re-init after the current update returns.
    ///
    /// The widget responds to this request differently depending on the node method that calls it:
    ///
    /// * [`UiNode::init`] and [`UiNode::deinit`]: Request is ignored, removed.
    /// * [`UiNode::event`]: If the widget is pending a reinit, it is reinited first, then the event is propagated to child nodes.
    ///                      If a reinit is requested during event handling the widget is reinited immediately after the event handler.
    /// * [`UiNode::update`]: If the widget is pending a reinit, it is reinited and the update ignored.
    ///                       If a reinit is requested during update the widget is reinited immediately after the update.
    /// * Other methods: Reinit request is flagged and an [`UiNode::update`] is requested for the widget.
    pub fn reinit(&self) {
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if !f.contains(UpdateFlags::REINIT) {
                f.insert(UpdateFlags::REINIT);
                Some(f)
            } else {
                None
            }
        });
    }

    /// Calls `f` with a read lock on the current widget state map.
    ///
    /// Note that this locks the entire [`WIDGET`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state<R>(&self, f: impl FnOnce(StateMapRef<WIDGET>) -> R) -> R {
        f(WIDGET_CTX.get().state.read().borrow())
    }

    /// Calls `f` with a write lock on the current widget state map.
    ///
    /// Note that this locks the entire [`WIDGET`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state_mut<R>(&self, f: impl FnOnce(StateMapMut<WIDGET>) -> R) -> R {
        f(WIDGET_CTX.get().state.write().borrow_mut())
    }

    /// Get the widget state `id`, if it is set.
    ///
    /// Panics if not called inside a widget.
    pub fn get_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> Option<T> {
        let id = id.into();
        self.with_state(|s| s.get_clone(id))
    }

    /// Require the widget state `id`.
    ///
    /// Panics if the `id` is not set or is not called inside a widget.
    pub fn req_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> T {
        let id = id.into();
        self.with_state(|s| s.req(id).clone())
    }

    /// Set the widget state `id` to `value`.
    ///
    /// Returns the previous set value.
    pub fn set_state<T: StateValue>(&self, id: impl Into<StateId<T>>, value: impl Into<T>) -> Option<T> {
        let id = id.into();
        let value = value.into();
        self.with_state_mut(|mut s| s.set(id, value))
    }

    /// Sets the widget state `id` without value.
    ///
    /// Returns if the state `id` was already flagged.
    pub fn flag_state(&self, id: impl Into<StateId<()>>) -> bool {
        let id = id.into();
        self.with_state_mut(|mut s| s.flag(id))
    }

    /// Calls `init` and sets `id` if the `id` is not already set in the widget.
    pub fn init_state<T: StateValue>(&self, id: impl Into<StateId<T>>, init: impl FnOnce() -> T) {
        let id = id.into();
        self.with_state_mut(|mut s| {
            s.entry(id).or_insert_with(init);
        });
    }

    /// Sets the `id` to the default value if it is not already set.
    pub fn init_state_default<T: StateValue + Default>(&self, id: impl Into<StateId<T>>) {
        self.init_state(id.into(), Default::default)
    }

    /// Returns `true` if the `id` is set or flagged in the widget.
    pub fn contains_state<T: StateValue>(&self, id: impl Into<StateId<T>>) -> bool {
        let id = id.into();
        self.with_state(|s| s.contains(id))
    }

    /// Subscribe to receive [`UpdateOp`] when the `var` changes.
    pub fn sub_var_op(&self, op: UpdateOp, var: &impl AnyVar) -> &Self {
        let w = WIDGET_CTX.get();
        let s = var.subscribe(op, w.id);

        if WIDGET_HANDLES_CTX.is_default() {
            w.handles.var_handles.lock().push(s);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().push(s);
        }

        self
    }

    /// Subscribe to receive [`UpdateOp`] when the `var` changes.
    pub fn sub_var_op_when<T: VarValue>(
        &self,
        op: UpdateOp,
        var: &impl Var<T>,
        predicate: impl Fn(&T) -> bool + Send + Sync + 'static,
    ) -> &Self {
        let w = WIDGET_CTX.get();
        let s = var.subscribe_when(op, w.id, predicate);

        if WIDGET_HANDLES_CTX.is_default() {
            w.handles.var_handles.lock().push(s);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().push(s);
        }

        self
    }

    /// Subscribe to receive updates when the `var` changes.
    pub fn sub_var(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Update, var)
    }
    /// Subscribe to receive updates when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_when<T: VarValue>(&self, var: &impl Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Update, var, predicate)
    }

    /// Subscribe to receive info rebuild requests when the `var` changes.
    pub fn sub_var_info(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Info, var)
    }
    /// Subscribe to receive info rebuild requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_info_when<T: VarValue>(&self, var: &impl Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Info, var, predicate)
    }

    /// Subscribe to receive layout requests when the `var` changes.
    pub fn sub_var_layout(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Layout, var)
    }
    /// Subscribe to receive layout requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_layout_when<T: VarValue>(&self, var: &impl Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Layout, var, predicate)
    }

    /// Subscribe to receive render requests when the `var` changes.
    pub fn sub_var_render(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Render, var)
    }
    /// Subscribe to receive render requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_render_when<T: VarValue>(&self, var: &impl Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Render, var, predicate)
    }

    /// Subscribe to receive render update requests when the `var` changes.
    pub fn sub_var_render_update(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::RenderUpdate, var)
    }
    /// Subscribe to receive render update requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_render_update_when<T: VarValue>(
        &self,
        var: &impl Var<T>,
        predicate: impl Fn(&T) -> bool + Send + Sync + 'static,
    ) -> &Self {
        self.sub_var_op_when(UpdateOp::RenderUpdate, var, predicate)
    }

    /// Subscribe to receive events from `event` when the event targets this widget.
    pub fn sub_event<A: EventArgs>(&self, event: &Event<A>) -> &Self {
        let w = WIDGET_CTX.get();
        let s = event.subscribe(w.id);

        if WIDGET_HANDLES_CTX.is_default() {
            w.handles.event_handles.lock().push(s);
        } else {
            WIDGET_HANDLES_CTX.get().event_handles.lock().push(s);
        }

        self
    }

    /// Hold the `handle` until the widget is deinited.
    pub fn push_event_handle(&self, handle: EventHandle) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.event_handles.lock().push(handle);
        } else {
            WIDGET_HANDLES_CTX.get().event_handles.lock().push(handle);
        }
    }

    /// Hold the `handles` until the widget is deinited.
    pub fn push_event_handles(&self, handles: EventHandles) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.event_handles.lock().extend(handles);
        } else {
            WIDGET_HANDLES_CTX.get().event_handles.lock().extend(handles);
        }
    }

    /// Hold the `handle` until the widget is deinited.
    pub fn push_var_handle(&self, handle: VarHandle) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.var_handles.lock().push(handle);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().push(handle);
        }
    }

    /// Hold the `handles` until the widget is deinited.
    pub fn push_var_handles(&self, handles: VarHandles) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.var_handles.lock().extend(handles);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().extend(handles);
        }
    }

    /// Widget bounds, updated every layout.
    pub fn bounds(&self) -> WidgetBoundsInfo {
        WIDGET_CTX.get().bounds.lock().clone()
    }

    /// Widget border, updated every layout.
    pub fn border(&self) -> WidgetBorderInfo {
        WIDGET_CTX.get().border.lock().clone()
    }

    /// Gets the parent widget or `None` if is root.
    ///
    /// Panics if not called inside an widget.
    pub fn parent_id(&self) -> Option<WidgetId> {
        WIDGET_CTX.get().parent_id.load(Relaxed)
    }

    pub(crate) fn layout_is_pending(&self, layout_widgets: &LayoutUpdates) -> bool {
        let ctx = WIDGET_CTX.get();
        ctx.flags.load(Relaxed).contains(UpdateFlags::LAYOUT) || layout_widgets.delivery_list.enter_widget(ctx.id)
    }

    /// Remove update flag and returns if it intersected.
    pub(crate) fn take_update(&self, flag: UpdateFlags) -> bool {
        let mut r = false;
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if f.intersects(flag) {
                r = true;
                f.remove(flag);
                Some(f)
            } else {
                None
            }
        });
        r
    }

    /// Current pending updates.
    #[cfg(debug_assertions)]
    pub(crate) fn pending_update(&self) -> UpdateFlags {
        WIDGET_CTX.get().flags.load(Relaxed)
    }

    /// Remove the render reuse range if render was not invalidated on this widget.
    pub(crate) fn take_render_reuse(&self, render_widgets: &RenderUpdates, render_update_widgets: &RenderUpdates) -> Option<ReuseRange> {
        let ctx = WIDGET_CTX.get();
        let mut try_reuse = true;

        // take RENDER, RENDER_UPDATE
        let _ = ctx.flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if f.intersects(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE) {
                try_reuse = false;
                f.remove(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE);
                Some(f)
            } else {
                None
            }
        });

        if try_reuse && !render_widgets.delivery_list.enter_widget(ctx.id) && !render_update_widgets.delivery_list.enter_widget(ctx.id) {
            ctx.render_reuse.lock().take()
        } else {
            None
        }
    }

    pub(crate) fn set_render_reuse(&self, range: Option<ReuseRange>) {
        *WIDGET_CTX.get().render_reuse.lock() = range;
    }
}

context_local! {
    static LAYOUT_CTX: LayoutCtx = LayoutCtx::no_context();
    static LAYOUT_PASS_CTX: LayoutPassId = LayoutPassId::new();
    static METRICS_USED_CTX: Atomic<LayoutMask> = Atomic::new(LayoutMask::empty());
}

struct LayoutCtx {
    metrics: LayoutMetrics,
}
impl LayoutCtx {
    fn no_context() -> Self {
        panic!("no layout context")
    }
}

/// Identifies the layout pass of a window.
///
/// This value is different for each window layout, but the same for children of panels that do more then one layout pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LayoutPassId(u32);
impl LayoutPassId {
    /// New default.
    pub const fn new() -> Self {
        LayoutPassId(0)
    }

    /// Gets the next layout pass ID.
    pub const fn next(self) -> LayoutPassId {
        LayoutPassId(self.0.wrapping_add(1))
    }
}

/// Current layout context.
///
/// Only available in measure and layout methods.
pub struct LAYOUT;
impl LAYOUT {
    /// Gets the current window layout pass.
    ///
    /// Widgets can be layout more then once per window layout pass, you can use this ID to identify such cases.
    pub fn pass_id(&self) -> LayoutPassId {
        LAYOUT_PASS_CTX.get_clone()
    }

    /// Calls `f` in a new layout pass.
    pub fn with_root_context<R>(&self, pass_id: LayoutPassId, metrics: LayoutMetrics, f: impl FnOnce() -> R) -> R {
        let mut pass = Some(Arc::new(pass_id));
        LAYOUT_PASS_CTX.with_context(&mut pass, || self.with_context(metrics, f))
    }

    /// Calls `f` in a new layout context.
    pub fn with_context<R>(&self, metrics: LayoutMetrics, f: impl FnOnce() -> R) -> R {
        let mut ctx = Some(Arc::new(LayoutCtx { metrics }));
        LAYOUT_CTX.with_context(&mut ctx, f)
    }

    /// Calls `f` without a layout context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        LAYOUT_CTX.with_default(f)
    }

    /// Gets the context metrics.
    pub fn metrics(&self) -> LayoutMetrics {
        LAYOUT_CTX.get().metrics.clone()
    }

    /// Capture all layout metrics used in `f`.
    ///
    /// Note that the captured mask is not propagated to the current context, you can use [`register_metrics_use`] to propagate
    /// the returned mask.
    ///
    /// [`register_metrics_use`]: Self::register_metrics_use
    pub fn capture_metrics_use<R>(&self, f: impl FnOnce() -> R) -> (LayoutMask, R) {
        METRICS_USED_CTX.with_context_value(Atomic::new(LayoutMask::empty()), || {
            let r = f();
            let uses = METRICS_USED_CTX.get().load(Relaxed);
            (uses, r)
        })
    }

    /// Register that the node layout depends on these contextual values.
    ///
    /// Note that the value methods already register by the [`LayoutMetrics`] getter methods.
    pub fn register_metrics_use(&self, uses: LayoutMask) {
        let ctx = METRICS_USED_CTX.get();
        let m = ctx.load(Relaxed);
        ctx.store(m | uses, Relaxed);
    }

    /// Current size constraints.
    pub fn constraints(&self) -> PxConstraints2d {
        LAYOUT_CTX.get().metrics.constraints()
    }

    /// Current perspective constraints.
    pub fn z_constraints(&self) -> PxConstraints {
        LAYOUT_CTX.get().metrics.z_constraints()
    }

    /// Current length constraints for the given axis.
    pub fn constraints_for(&self, axis: LayoutAxis) -> PxConstraints {
        match axis {
            LayoutAxis::X => self.constraints().x,
            LayoutAxis::Y => self.constraints().y,
            LayoutAxis::Z => self.z_constraints(),
        }
    }

    /// Calls `f` with the `constraints` in context.
    pub fn with_constraints<R>(&self, constraints: PxConstraints2d, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_constraints(constraints), f)
    }

    /// Calls `f` with the `constraints` for perspective in context.
    pub fn with_z_constraints<R>(&self, constraints: PxConstraints, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_z_constraints(constraints), f)
    }

    /// Calls `f` with the `constraints` in context.
    pub fn with_constraints_for<R>(&self, axis: LayoutAxis, constraints: PxConstraints, f: impl FnOnce() -> R) -> R {
        match axis {
            LayoutAxis::X => {
                let mut c = self.constraints();
                c.x = constraints;
                self.with_constraints(c, f)
            }
            LayoutAxis::Y => {
                let mut c = self.constraints();
                c.y = constraints;
                self.with_constraints(c, f)
            }
            LayoutAxis::Z => self.with_z_constraints(constraints, f),
        }
    }

    /// Runs a function `f` in a context that has its max size subtracted by `removed` and its final size added by `removed`.
    pub fn with_sub_size(&self, removed: PxSize, f: impl FnOnce() -> PxSize) -> PxSize {
        self.with_constraints(self.constraints().with_less_size(removed), f) + removed
    }

    /// Runs a function `f` in a layout context that has its max size added by `added` and its final size subtracted by `added`.
    pub fn with_add_size(&self, added: PxSize, f: impl FnOnce() -> PxSize) -> PxSize {
        self.with_constraints(self.constraints().with_more_size(added), f) - added
    }

    /// Current inline constraints.
    pub fn inline_constraints(&self) -> Option<InlineConstraints> {
        LAYOUT_CTX.get().metrics.inline_constraints()
    }

    /// Calls `f` with no inline constraints.
    pub fn with_no_inline(&self, f: impl FnOnce() -> PxSize) -> PxSize {
        let metrics = self.metrics();
        if metrics.inline_constraints().is_none() {
            f()
        } else {
            self.with_context(metrics.with_inline_constraints(None), f)
        }
    }

    /// Root font size.
    pub fn root_font_size(&self) -> Px {
        LAYOUT_CTX.get().metrics.root_font_size()
    }

    /// Current font size.
    pub fn font_size(&self) -> Px {
        LAYOUT_CTX.get().metrics.font_size()
    }

    /// Calls `f` with `font_size` in the context.
    pub fn with_font_size<R>(&self, font_size: Px, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_font_size(font_size), f)
    }

    /// Current viewport size.
    pub fn viewport(&self) -> PxSize {
        LAYOUT_CTX.get().metrics.viewport()
    }

    /// Current smallest dimension of the viewport.
    pub fn viewport_min(&self) -> Px {
        LAYOUT_CTX.get().metrics.viewport_min()
    }

    /// Current largest dimension of the viewport.
    pub fn viewport_max(&self) -> Px {
        LAYOUT_CTX.get().metrics.viewport_max()
    }

    /// Current viewport length for the given axis.
    pub fn viewport_for(&self, axis: LayoutAxis) -> Px {
        let vp = self.viewport();
        match axis {
            LayoutAxis::X => vp.width,
            LayoutAxis::Y => vp.height,
            LayoutAxis::Z => Px::MAX,
        }
    }

    /// Calls `f` with `viewport` in the context.
    pub fn with_viewport<R>(&self, viewport: PxSize, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_viewport(viewport), f)
    }

    /// Current scale factor.
    pub fn scale_factor(&self) -> Factor {
        LAYOUT_CTX.get().metrics.scale_factor()
    }

    /// Calls `f` with `scale_factor` in the context.
    pub fn with_scale_factor<R>(&self, scale_factor: Factor, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_scale_factor(scale_factor), f)
    }

    /// Current screen PPI.
    pub fn screen_ppi(&self) -> Ppi {
        LAYOUT_CTX.get().metrics.screen_ppi()
    }

    /// Calls `f` with `screen_ppi` in the context.
    pub fn with_screen_ppi<R>(&self, screen_ppi: Ppi, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_screen_ppi(screen_ppi), f)
    }

    /// Current layout direction.
    pub fn direction(&self) -> LayoutDirection {
        LAYOUT_CTX.get().metrics.direction()
    }

    /// Calls `f` with `direction` in the context.
    pub fn with_direction<R>(&self, direction: LayoutDirection, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_direction(direction), f)
    }

    /// Context leftover length for the widget, given the [`Length::Leftover`] value it communicated to the parent.
    ///
    /// [`leftover_count`]: Self::leftover_count
    pub fn leftover(&self) -> euclid::Size2D<Option<Px>, ()> {
        LAYOUT_CTX.get().metrics.leftover()
    }

    /// Context leftover length for the given axis.
    pub fn leftover_for(&self, axis: LayoutAxis) -> Option<Px> {
        let l = self.leftover();

        match axis {
            LayoutAxis::X => l.width,
            LayoutAxis::Y => l.height,
            LayoutAxis::Z => None,
        }
    }

    /// Calls `f` with [`leftover`] set to `with` and `height`.
    ///
    /// [`leftover`]: Self::leftover
    pub fn with_leftover<R>(&self, width: Option<Px>, height: Option<Px>, f: impl FnOnce() -> R) -> R {
        self.with_context(self.metrics().with_leftover(width, height), f)
    }
}

app_local! {
    static UPDATES_SV: UpdatesService = UpdatesService::new();
}
struct UpdatesService {
    event_sender: Option<AppEventSender>,

    update_ext: UpdateFlags,
    update_widgets: UpdateDeliveryList,
    info_widgets: UpdateDeliveryList,
    layout_widgets: UpdateDeliveryList,
    render_widgets: UpdateDeliveryList,
    render_update_widgets: UpdateDeliveryList,

    pre_handlers: Mutex<Vec<UpdateHandler>>,
    pos_handlers: Mutex<Vec<UpdateHandler>>,

    app_is_awake: bool,
    awake_pending: bool,
}
impl UpdatesService {
    fn new() -> Self {
        Self {
            event_sender: None,
            update_ext: UpdateFlags::empty(),
            update_widgets: UpdateDeliveryList::new_any(),
            info_widgets: UpdateDeliveryList::new_any(),
            layout_widgets: UpdateDeliveryList::new_any(),
            render_widgets: UpdateDeliveryList::new_any(),
            render_update_widgets: UpdateDeliveryList::new_any(),

            pre_handlers: Mutex::new(vec![]),
            pos_handlers: Mutex::new(vec![]),

            app_is_awake: false,
            awake_pending: false,
        }
    }

    fn send_awake(&mut self) {
        if !self.app_is_awake && !self.awake_pending {
            self.awake_pending = true;
            match self.event_sender.as_ref() {
                Some(s) => {
                    if let Err(AppDisconnected(())) = s.send_check_update() {
                        tracing::error!("no app connected to update");
                    }
                }
                None => {
                    tracing::error!("no app connected yet to update");
                }
            }
        }
    }

    fn app_awake(&mut self, wake: bool) {
        self.awake_pending = false;
        self.app_is_awake = wake;
    }
}

/// Specify what app extension and widget operation must be run to satisfy an update request targeting an widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateOp {
    /// The [`AppExtension::update_preview`], [`AppExtension::update_ui`] and [`AppExtension::update`] are called in order,
    /// this is a normal update cycle.
    ///
    /// The [`UiNode::update`] is called for the target widget, parent widgets and any other widget that requested update
    /// in the same cycle. This call happens inside [`AppExtension::update_ui`].
    ///
    /// [`AppExtension::update_preview`]: crate::app::AppExtension::update_preview
    /// [`AppExtension::update_ui`]: crate::app::AppExtension::update_ui
    /// [`AppExtension::update`]: crate::app::AppExtension::update
    Update,
    /// The normal [`Update`] cycle runs, and after the info tree of windows that inited or deinited widgets are rebuild
    /// by calling [`UiNode::info`].  The target widget is also flagged for rebuild.
    ///
    /// [`Update`]: UpdateOp::Render
    Info,
    /// The [`AppExtension::layout`] is called the an update cycle happens without generating anymore update requests.
    ///
    /// The [`UiNode::layout`] is called for the widget target, parent widgets and any other widget that depends on
    /// layout metrics that have changed or that also requested layout update.
    ///
    /// [`AppExtension::layout`]: crate::app::AppExtension::layout
    Layout,
    /// The [`AppExtension::render`] is called after an update and layout cycle happens generating anymore requests for update or layout.
    ///
    /// The [`UiNode::render`] is called for the target widget, parent widgets and all other widgets that also requested render
    /// or that requested [`RenderUpdate`] in the same window.
    ///
    /// [`RenderUpdate`]: UpdateOp::RenderUpdate
    /// [`AppExtension::render`]: crate::app::AppExtension::render
    Render,
    /// Same behavior as [`Render`], except that windows where all widgets only requested render update are rendered
    /// using [`UiNode::render_update`] instead of the full render.
    ///
    /// This OP is upgraded to [`Render`] if any other widget requests a full render in the same window.
    ///
    /// [`Render`]: UpdateOp::Render
    RenderUpdate,
}

/// Update pump and schedule service.
pub struct UPDATES;
impl UPDATES {
    pub(crate) fn init(&self, event_sender: AppEventSender) {
        UPDATES_SV.write().event_sender = Some(event_sender);
    }

    #[must_use]
    #[cfg(any(test, doc, feature = "test_util"))]
    pub(crate) fn apply(&self) -> ContextUpdates {
        self.apply_updates() | self.apply_info() | self.apply_layout_render()
    }

    #[must_use]
    pub(crate) fn apply_updates(&self) -> ContextUpdates {
        let events = EVENTS.apply_updates();
        VARS.apply_updates();

        let (update, update_widgets) = UPDATES.take_update();

        ContextUpdates {
            events,
            update,
            update_widgets,
            info: false,
            info_widgets: InfoUpdates::default(),
            layout: false,
            layout_widgets: LayoutUpdates::default(),
            render: false,
            render_widgets: RenderUpdates::default(),
            render_update_widgets: RenderUpdates::default(),
        }
    }
    #[must_use]
    pub(crate) fn apply_info(&self) -> ContextUpdates {
        let (info, info_widgets) = UPDATES.take_info();

        ContextUpdates {
            events: vec![],
            update: false,
            update_widgets: WidgetUpdates::default(),
            info,
            info_widgets,
            layout: false,
            layout_widgets: LayoutUpdates::default(),
            render: false,
            render_widgets: RenderUpdates::default(),
            render_update_widgets: RenderUpdates::default(),
        }
    }
    #[must_use]
    pub(crate) fn apply_layout_render(&self) -> ContextUpdates {
        let (layout, layout_widgets) = UPDATES.take_layout();
        let (render, render_widgets, render_update_widgets) = UPDATES.take_render();

        ContextUpdates {
            events: vec![],
            update: false,
            update_widgets: WidgetUpdates::default(),
            info: false,
            info_widgets: InfoUpdates::default(),
            layout,
            layout_widgets,
            render,
            render_widgets,
            render_update_widgets,
        }
    }

    pub(crate) fn on_app_awake(&self) {
        UPDATES_SV.write().app_awake(true);
    }

    pub(crate) fn on_app_sleep(&self) {
        UPDATES_SV.write().app_awake(false);
    }

    /// Returns next timer or animation tick time.
    pub(crate) fn next_deadline(&self, timer: &mut LoopTimer) {
        TIMERS_SV.write().next_deadline(timer);
        VARS.next_deadline(timer);
    }

    /// Update timers and animations, returns next wake time.
    pub(crate) fn update_timers(&self, timer: &mut LoopTimer) {
        TIMERS_SV.write().apply_updates(timer);
        VARS.update_animations(timer);
    }

    /// If a call to `apply_updates` will generate updates (ignoring timers).
    #[must_use]
    pub(crate) fn has_pending_updates(&self) -> bool {
        UPDATES_SV.read().update_ext.intersects(UpdateFlags::UPDATE | UpdateFlags::INFO)
            || VARS.has_pending_updates()
            || EVENTS_SV.write().has_pending_updates()
            || TIMERS_SV.read().has_pending_updates()
    }

    #[must_use]
    pub(crate) fn has_pending_layout_or_render(&self) -> bool {
        UPDATES_SV
            .read()
            .update_ext
            .intersects(UpdateFlags::LAYOUT | UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE)
    }

    /// Create an [`AppEventSender`] that can be used to awake the app and send app events from threads outside of the app.
    pub fn sender(&self) -> AppEventSender {
        UPDATES_SV.read().event_sender.as_ref().unwrap().clone()
    }

    /// Create an std task waker that wakes the event loop and updates.
    pub fn waker(&self, target: impl Into<Option<WidgetId>>) -> Waker {
        UPDATES_SV.read().event_sender.as_ref().unwrap().waker(target)
    }

    pub(crate) fn update_flags_root(&self, flags: UpdateFlags, window_id: WindowId, root_id: WidgetId) {
        if flags.is_empty() {
            return;
        }

        let mut u = UPDATES_SV.write();
        if flags.contains(UpdateFlags::UPDATE) {
            u.update_widgets.insert_updates_root(window_id, root_id);
        }
        if flags.contains(UpdateFlags::INFO) {
            u.info_widgets.insert_updates_root(window_id, root_id);
        }
        if flags.contains(UpdateFlags::LAYOUT) {
            u.layout_widgets.insert_updates_root(window_id, root_id);
        }

        if flags.contains(UpdateFlags::RENDER) {
            u.render_widgets.insert_updates_root(window_id, root_id);
        } else if flags.contains(UpdateFlags::RENDER_UPDATE) {
            u.render_update_widgets.insert_updates_root(window_id, root_id);
        }

        u.update_ext |= flags;
    }

    pub(crate) fn update_flags(&self, flags: UpdateFlags, target: impl Into<Option<WidgetId>>) {
        if flags.is_empty() {
            return;
        }

        let mut u = UPDATES_SV.write();

        if let Some(id) = target.into() {
            if flags.contains(UpdateFlags::UPDATE) {
                u.update_widgets.search_widget(id);
            }
            if flags.contains(UpdateFlags::INFO) {
                u.info_widgets.search_widget(id);
            }
            if flags.contains(UpdateFlags::LAYOUT) {
                u.layout_widgets.search_widget(id);
            }

            if flags.contains(UpdateFlags::RENDER) {
                u.render_widgets.search_widget(id);
            } else if flags.contains(UpdateFlags::RENDER_UPDATE) {
                u.render_update_widgets.search_widget(id);
            }
        }

        u.update_ext |= flags;
    }

    /// Schedules an [`UpdateOp`] that optionally affects the `target` widget.
    pub fn update_op(&self, op: UpdateOp, target: impl Into<Option<WidgetId>>) -> &Self {
        let target = target.into();
        match op {
            UpdateOp::Update => self.update(target),
            UpdateOp::Info => self.update_info(target),
            UpdateOp::Layout => self.layout(target),
            UpdateOp::Render => self.render(target),
            UpdateOp::RenderUpdate => self.render_update(target),
        }
    }

    /// Schedules an [`UpdateOp`] for the window only.
    pub fn update_op_window(&self, op: UpdateOp, target: WindowId) -> &Self {
        match op {
            UpdateOp::Update => self.update_window(target),
            UpdateOp::Info => self.update_info_window(target),
            UpdateOp::Layout => self.layout_window(target),
            UpdateOp::Render => self.render_window(target),
            UpdateOp::RenderUpdate => self.render_update_window(target),
        }
    }

    /// Schedules an update that affects the `target`.
    ///
    /// After the current update cycle ends a new update will happen that includes the `target` widget.
    pub fn update(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        UpdatesTrace::log_update();
        self.update_internal(target.into())
    }
    /// Implements `update` without `log_update`.
    pub(crate) fn update_internal(&self, target: Option<WidgetId>) -> &UPDATES {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::UPDATE);
        u.send_awake();
        if let Some(id) = target {
            u.update_widgets.search_widget(id);
        }
        self
    }

    /// Schedules an update for the window only.
    pub fn update_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::UPDATE);
        u.send_awake();
        u.update_widgets.insert_window(target);
        self
    }

    pub(crate) fn send_awake(&self) {
        UPDATES_SV.write().send_awake();
    }

    /// Schedules an info rebuild that affects the `target`.
    ///
    /// After the current update cycle ends a new update will happen that requests an info rebuild that includes the `target` widget.
    pub fn update_info(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::INFO);
        u.send_awake();
        if let Some(id) = target.into() {
            u.info_widgets.search_widget(id);
        }
        self
    }

    /// Schedules an info rebuild for the window only.
    pub fn update_info_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::INFO);
        u.send_awake();
        u.info_widgets.insert_window(target);
        self
    }

    /// Schedules a layout update that affects the `target`.
    ///
    /// After the current update cycle ends and there are no more updates requested a layout pass is issued that includes the `target` widget.
    pub fn layout(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        UpdatesTrace::log_layout();
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::LAYOUT);
        u.send_awake();
        if let Some(id) = target.into() {
            u.layout_widgets.search_widget(id);
        }
        self
    }

    /// Schedules a layout update for the window only.
    pub fn layout_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::LAYOUT);
        u.send_awake();
        u.layout_widgets.insert_window(target);
        self
    }

    /// Schedules a full render that affects the `target`.
    ///
    /// After the current update cycle ends and there are no more updates or layouts requested a render pass is issued that
    /// includes the `target` widget.
    ///
    /// If no `target` is provided only the app extensions receive a render request.
    pub fn render(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::RENDER);
        u.send_awake();
        if let Some(id) = target.into() {
            u.render_widgets.search_widget(id);
        }
        self
    }

    /// Schedules a new frame for the window only.
    pub fn render_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::RENDER);
        u.send_awake();
        u.render_widgets.insert_window(target);
        self
    }

    /// Schedules a render update that affects the `target`.
    ///
    /// After the current update cycle ends and there are no more updates or layouts requested a render pass is issued that
    /// includes the `target` widget marked for render update only. Note that if a full render was requested for another widget
    /// on the same window this request is upgraded to a full frame render.
    pub fn render_update(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::RENDER_UPDATE);
        u.send_awake();
        if let Some(id) = target.into() {
            u.render_update_widgets.search_widget(id);
        }
        self
    }

    /// Schedules a render update for the window only.
    pub fn render_update_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::RENDER_UPDATE);
        u.send_awake();
        u.render_update_widgets.insert_window(target);
        self
    }

    /// Returns `true` is render or render update is requested for the window.
    pub fn is_pending_render(&self, window_id: WindowId) -> bool {
        let u = UPDATES_SV.read();
        u.render_widgets.enter_window(window_id) || u.render_update_widgets.enter_window(window_id)
    }

    /// Schedule the `future` to run in the app context, each future awake work runs as a *preview* update.
    ///
    /// Returns a handle that can be dropped to cancel execution.
    pub fn run<F: std::future::Future<Output = ()> + Send + 'static>(&self, future: F) -> OnUpdateHandle {
        self.run_hn_once(async_app_hn_once!(|_| future.await))
    }

    /// Schedule an *once* handler to run when these updates are applied.
    ///
    /// The callback is any of the *once* [`AppHandler`], including async handlers. If the handler is async and does not finish in
    /// one call it is scheduled to update in *preview* updates.
    pub fn run_hn_once<H: AppHandler<UpdateArgs>>(&self, handler: H) -> OnUpdateHandle {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::UPDATE);
        u.send_awake();
        Self::push_handler(u.pos_handlers.get_mut(), true, handler, true)
    }

    /// Create a preview update handler.
    ///
    /// The `handler` is called every time the app updates, just before the UI updates. It can be any of the non-async [`AppHandler`],
    /// use the [`app_hn!`] or [`app_hn_once!`] macros to declare the closure. You must avoid using async handlers because UI bound async
    /// tasks cause app updates to awake, so it is very easy to lock the app in a constant sequence of updates. You can use [`run`](Self::run)
    /// to start an async app context task.
    ///
    /// Returns an [`OnUpdateHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// [`app_hn_once!`]: macro@crate::handler::app_hn_once
    /// [`app_hn!`]: macro@crate::handler::app_hn
    /// [`async_app_hn!`]: macro@crate::handler::async_app_hn
    pub fn on_pre_update<H>(&self, handler: H) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs>,
    {
        let u = UPDATES_SV.read();
        let r = Self::push_handler(&mut u.pre_handlers.lock(), true, handler, false);
        r
    }

    /// Create an update handler.
    ///
    /// The `handler` is called every time the app updates, just after the UI updates. It can be any of the non-async [`AppHandler`],
    /// use the [`app_hn!`] or [`app_hn_once!`] macros to declare the closure. You must avoid using async handlers because UI bound async
    /// tasks cause app updates to awake, so it is very easy to lock the app in a constant sequence of updates. You can use [`run`](Self::run)
    /// to start an async app context task.
    ///
    /// Returns an [`OnUpdateHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// [`app_hn!`]: macro@crate::handler::app_hn
    /// [`async_app_hn!`]: macro@crate::handler::async_app_hn
    pub fn on_update<H>(&self, handler: H) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs>,
    {
        let u = UPDATES_SV.read();
        let r = Self::push_handler(&mut u.pos_handlers.lock(), false, handler, false);
        r
    }

    fn push_handler<H>(entries: &mut Vec<UpdateHandler>, is_preview: bool, mut handler: H, force_once: bool) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs>,
    {
        let (handle_owner, handle) = OnUpdateHandle::new();
        entries.push(UpdateHandler {
            handle: handle_owner,
            count: 0,
            handler: Box::new(move |args, handle| {
                let handler_args = AppHandlerArgs { handle, is_preview };
                handler.event(args, &handler_args);
                if force_once {
                    handler_args.handle.unsubscribe();
                }
            }),
        });
        handle
    }

    pub(crate) fn on_pre_updates(&self) {
        let _s = tracing::trace_span!("UPDATES.on_pre_updates");
        let mut handlers = mem::take(UPDATES_SV.write().pre_handlers.get_mut());
        Self::retain_updates(&mut handlers);

        let mut u = UPDATES_SV.write();
        handlers.append(u.pre_handlers.get_mut());
        *u.pre_handlers.get_mut() = handlers;
    }

    pub(crate) fn on_updates(&self) {
        let _s = tracing::trace_span!("UPDATES.on_updates");
        let mut handlers = mem::take(UPDATES_SV.write().pos_handlers.get_mut());
        Self::retain_updates(&mut handlers);

        let mut u = UPDATES_SV.write();
        handlers.append(u.pos_handlers.get_mut());
        *u.pos_handlers.get_mut() = handlers;
    }

    fn retain_updates(handlers: &mut Vec<UpdateHandler>) {
        handlers.retain_mut(|e| {
            !e.handle.is_dropped() && {
                e.count = e.count.wrapping_add(1);
                (e.handler)(&UpdateArgs { count: e.count }, &e.handle.weak_handle());
                !e.handle.is_dropped()
            }
        });
    }

    /// Returns (update_ext, update_widgets)
    pub(super) fn take_update(&self) -> (bool, WidgetUpdates) {
        let mut u = UPDATES_SV.write();

        let ext = u.update_ext.contains(UpdateFlags::UPDATE);
        u.update_ext.remove(UpdateFlags::UPDATE);

        (
            ext,
            WidgetUpdates {
                delivery_list: mem::take(&mut u.update_widgets),
            },
        )
    }

    /// Returns (info_ext, info_widgets)
    pub(super) fn take_info(&self) -> (bool, InfoUpdates) {
        let mut u = UPDATES_SV.write();

        let ext = u.update_ext.contains(UpdateFlags::INFO);
        u.update_ext.remove(UpdateFlags::INFO);

        (
            ext,
            InfoUpdates {
                delivery_list: mem::take(&mut u.info_widgets),
            },
        )
    }

    /// Returns (layout_ext, layout_widgets)
    pub(super) fn take_layout(&self) -> (bool, LayoutUpdates) {
        let mut u = UPDATES_SV.write();

        let ext = u.update_ext.contains(UpdateFlags::LAYOUT);
        u.update_ext.remove(UpdateFlags::LAYOUT);

        (
            ext,
            LayoutUpdates {
                delivery_list: mem::take(&mut u.layout_widgets),
            },
        )
    }

    /// Returns (render_ext, render_widgets, render_update_widgets)
    pub(super) fn take_render(&self) -> (bool, RenderUpdates, RenderUpdates) {
        let mut u = UPDATES_SV.write();

        let ext = u.update_ext.intersects(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE);
        u.update_ext.remove(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE);

        (
            ext,
            RenderUpdates {
                delivery_list: mem::take(&mut u.render_widgets),
            },
            RenderUpdates {
                delivery_list: mem::take(&mut u.render_update_widgets),
            },
        )
    }

    pub(crate) fn handler_lens(&self) -> (usize, usize) {
        let u = UPDATES_SV.read();
        let r = (u.pre_handlers.lock().len(), u.pos_handlers.lock().len());
        r
    }
    pub(crate) fn new_update_handlers(&self, pre_from: usize, pos_from: usize) -> Vec<Box<dyn Fn() -> bool>> {
        let u = UPDATES_SV.read();
        let r = u
            .pre_handlers
            .lock()
            .iter()
            .skip(pre_from)
            .chain(u.pos_handlers.lock().iter().skip(pos_from))
            .map(|h| h.handle.weak_handle())
            .map(|h| {
                let r: Box<dyn Fn() -> bool> = Box::new(move || h.upgrade().is_some());
                r
            })
            .collect();
        r
    }
}

/// Represents an [`on_pre_update`](UPDATES::on_pre_update) or [`on_update`](UPDATES::on_update) handler.
///
/// Drop all clones of this handle to drop the binding, or call [`perm`](Self::perm) to drop the handle
/// but keep the handler alive for the duration of the app.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
#[must_use = "dropping the handle unsubscribes update handler"]
pub struct OnUpdateHandle(Handle<()>);
impl OnUpdateHandle {
    fn new() -> (HandleOwner<()>, OnUpdateHandle) {
        let (owner, handle) = Handle::new(());
        (owner, OnUpdateHandle(handle))
    }

    /// Create a handle to nothing, the handle always in the *unsubscribed* state.
    ///
    /// Note that `Option<OnUpdateHandle>` takes up the same space as `OnUpdateHandle` and avoids an allocation.
    pub fn dummy() -> Self {
        assert_non_null!(OnUpdateHandle);
        OnUpdateHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unsubscribe.
    ///
    /// The handler stays in memory for the duration of the app or until another handle calls [`unsubscribe`](Self::unsubscribe.)
    pub fn perm(self) {
        self.0.perm();
    }

    /// If another handle has called [`perm`](Self::perm).
    /// If `true` the var binding will stay active until the app exits, unless [`unsubscribe`](Self::unsubscribe) is called.
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the handler to drop.
    pub fn unsubscribe(self) {
        self.0.force_drop()
    }

    /// If another handle has called [`unsubscribe`](Self::unsubscribe).
    ///
    /// The handler is already dropped or will be dropped in the next app update, this is irreversible.
    pub fn is_unsubscribed(&self) -> bool {
        self.0.is_dropped()
    }

    /// Create a weak handle.
    pub fn downgrade(&self) -> WeakOnUpdateHandle {
        WeakOnUpdateHandle(self.0.downgrade())
    }
}

/// Weak [`OnUpdateHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakOnUpdateHandle(WeakHandle<()>);
impl WeakOnUpdateHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Gets the strong handle if it is still subscribed.
    pub fn upgrade(&self) -> Option<OnUpdateHandle> {
        self.0.upgrade().map(OnUpdateHandle)
    }
}

struct UpdateHandler {
    handle: HandleOwner<()>,
    count: usize,
    handler: Box<dyn FnMut(&UpdateArgs, &dyn AppWeakHandle) + Send>,
}

/// Arguments for an [`on_pre_update`](UPDATES::on_pre_update), [`on_update`](UPDATES::on_update) or [`run`](UPDATES::run) handler.
#[derive(Debug, Clone, Copy)]
pub struct UpdateArgs {
    /// Number of times the handler was called.
    pub count: usize,
}

/// Widget updates of the current cycle.
#[derive(Debug, Default)]
pub struct WidgetUpdates {
    delivery_list: UpdateDeliveryList,
}
impl WidgetUpdates {
    /// New with list.
    pub fn new(delivery_list: UpdateDeliveryList) -> Self {
        Self { delivery_list }
    }

    /// Updates delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// Updates delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// Calls `handle` if update was requested for the [`WINDOW`].
    pub fn with_window<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(WINDOW.id()) {
            Some(handle())
        } else {
            None
        }
    }

    /// Calls `handle` if update was requested for the [`WIDGET`].
    pub fn with_widget<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if WIDGET.take_update(UpdateFlags::UPDATE) || self.delivery_list.enter_widget(WIDGET.id()) {
            Some(handle())
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: WidgetUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Widget info updates of the current cycle.
#[derive(Debug, Default)]
pub struct InfoUpdates {
    delivery_list: UpdateDeliveryList,
}
impl InfoUpdates {
    /// New with list.
    pub fn new(delivery_list: UpdateDeliveryList) -> Self {
        Self { delivery_list }
    }

    /// Request delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// Request delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// Calls `handle` if info rebuild was requested for the [`WINDOW`].
    pub fn with_window<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(WINDOW.id()) {
            Some(handle())
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: InfoUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Widget layout updates of the current cycle.
#[derive(Debug, Default)]
pub struct LayoutUpdates {
    delivery_list: UpdateDeliveryList,
}
impl LayoutUpdates {
    /// New with list.
    pub fn new(delivery_list: UpdateDeliveryList) -> Self {
        Self { delivery_list }
    }

    /// Request delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// Request delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// Calls `handle` if layout rebuild was requested for the [`WINDOW`].
    pub fn with_window<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(WINDOW.id()) {
            Some(handle())
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: LayoutUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Widget render updates of the current cycle.
#[derive(Debug, Default)]
pub struct RenderUpdates {
    delivery_list: UpdateDeliveryList,
}
impl RenderUpdates {
    /// New with list.
    pub fn new(delivery_list: UpdateDeliveryList) -> Self {
        Self { delivery_list }
    }

    /// Request delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// Request delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// Calls `handle` if render frame rebuild or update was requested for the [`WINDOW`].
    pub fn with_window<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(WINDOW.id()) {
            Some(handle())
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: RenderUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Represents all the widgets and windows on route to an update target.
pub struct UpdateDeliveryList {
    subscribers: Box<dyn UpdateSubscribers>,

    windows: IdSet<WindowId>,
    widgets: IdSet<WidgetId>,
    search: IdSet<WidgetId>,
}
impl fmt::Debug for UpdateDeliveryList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateDeliveryList")
            .field("windows", &self.windows)
            .field("widgets", &self.widgets)
            .field("search", &self.search)
            .finish_non_exhaustive()
    }
}
impl Default for UpdateDeliveryList {
    fn default() -> Self {
        Self::new_any()
    }
}
impl UpdateDeliveryList {
    /// New list that only allows `subscribers`.
    pub fn new(subscribers: Box<dyn UpdateSubscribers>) -> Self {
        Self {
            subscribers,
            windows: IdSet::default(),
            widgets: IdSet::default(),
            search: IdSet::default(),
        }
    }

    /// New list that does not allow any entry.
    pub fn new_none() -> Self {
        struct UpdateDeliveryListNone;
        impl UpdateSubscribers for UpdateDeliveryListNone {
            fn contains(&self, _: WidgetId) -> bool {
                false
            }
            fn to_set(&self) -> IdSet<WidgetId> {
                IdSet::default()
            }
        }
        Self::new(Box::new(UpdateDeliveryListNone))
    }

    /// New list that allows all entries.
    ///
    /// This is the default value.
    pub fn new_any() -> Self {
        struct UpdateDeliveryListAny;
        impl UpdateSubscribers for UpdateDeliveryListAny {
            fn contains(&self, _: WidgetId) -> bool {
                true
            }
            fn to_set(&self) -> IdSet<WidgetId> {
                IdSet::default()
            }
        }
        Self::new(Box::new(UpdateDeliveryListAny))
    }

    /// Insert the widgets in the `path` up-to the inner most that is included in the subscribers.
    pub fn insert_path(&mut self, path: &WidgetPath) {
        if let Some(i) = path.widgets_path().iter().rposition(|w| self.subscribers.contains(*w)) {
            self.windows.insert(path.window_id());
            for w in &path.widgets_path()[..=i] {
                self.widgets.insert(*w);
            }
        }
    }
    pub(crate) fn insert_updates_root(&mut self, window_id: WindowId, root_id: WidgetId) {
        self.windows.insert(window_id);
        self.widgets.insert(root_id);
    }

    /// Insert the ancestors of `wgt` and `wgt` up-to the inner most that is included in the subscribers.
    pub fn insert_wgt(&mut self, wgt: &WidgetInfo) {
        let mut any = false;
        for w in wgt.self_and_ancestors() {
            if any || self.subscribers.contains(w.id()) {
                any = true;
                self.widgets.insert(w.id());
            }
        }
        if any {
            self.windows.insert(wgt.tree().window_id());
        }
    }

    /// Insert the window by itself.
    pub fn insert_window(&mut self, id: WindowId) {
        self.windows.insert(id);
    }

    /// Register all subscribers for search and delivery.
    pub fn search_all(&mut self) {
        self.search = self.subscribers.to_set();
    }

    /// Register the widget of unknown location for search before delivery routing starts.
    pub fn search_widget(&mut self, widget_id: WidgetId) {
        if self.subscribers.contains(widget_id) {
            self.search.insert(widget_id);
        }
    }

    /// If the the list has pending widgets that must be found before delivery can start.
    pub fn has_pending_search(&mut self) -> bool {
        !self.search.is_empty()
    }

    /// Search all pending widgets in all `windows`, all search items are cleared, even if not found.
    pub fn fulfill_search<'a, 'b>(&'a mut self, windows: impl Iterator<Item = &'b WidgetInfoTree>) {
        for window in windows {
            self.search.retain(|w| {
                if let Some(w) = window.get(*w) {
                    for w in w.self_and_ancestors() {
                        self.widgets.insert(w.id());
                    }
                    self.windows.insert(window.window_id());
                    false
                } else {
                    true
                }
            });
        }
        self.search.clear();
    }

    /// Copy windows, widgets and search from `other`, trusting that all values are allowed.
    fn extend_unchecked(&mut self, other: UpdateDeliveryList) {
        if self.windows.is_empty() {
            self.windows = other.windows;
        } else {
            self.windows.extend(other.windows);
        }

        if self.widgets.is_empty() {
            self.widgets = other.widgets;
        } else {
            self.widgets.extend(other.widgets);
        }

        if self.search.is_empty() {
            self.search = other.search;
        } else {
            self.search.extend(other.search);
        }
    }

    /// Returns `true` if the window is on the list.
    pub fn enter_window(&self, window_id: WindowId) -> bool {
        self.windows.contains(&window_id)
    }

    /// Returns `true` if the widget is on the list.
    pub fn enter_widget(&self, widget_id: WidgetId) -> bool {
        self.widgets.contains(&widget_id)
    }

    /// Windows in the delivery list.
    pub fn windows(&self) -> &IdSet<WindowId> {
        &self.windows
    }

    /// Found widgets in the delivery list, can be targets or ancestors of targets.
    pub fn widgets(&self) -> &IdSet<WidgetId> {
        &self.widgets
    }

    /// Not found target widgets.
    ///
    /// Each window searches for these widgets and adds then to the [`widgets`] list.
    ///
    /// [`widgets`]: Self::widgets
    pub fn search_widgets(&mut self) -> &IdSet<WidgetId> {
        &self.search
    }
}

/// Represents a set of widgets that subscribe to an event source.
pub trait UpdateSubscribers: Send + Sync + 'static {
    /// Returns `true` if the widget is one of the subscribers.
    fn contains(&self, widget_id: WidgetId) -> bool;

    /// Gets all subscribers as a set.
    fn to_set(&self) -> IdSet<WidgetId>;
}

/// Updates that must be reacted by an app owner.
#[derive(Debug, Default)]
pub struct ContextUpdates {
    /// Events to notify.
    ///
    /// When this is not empty [`update`](Self::update) is `true`.
    pub events: Vec<EventUpdate>,

    /// Update requested.
    ///
    /// When this is `true`, [`update_widgets`](Self::update_widgets)
    /// may contain widgets, if not then only app extensions must update.
    pub update: bool,

    /// Info rebuild requested.
    ///
    /// When this is `true`, [`info_widgets`](Self::info_widgets)
    /// may contain widgets, if not then only app extensions must update.
    pub info: bool,

    /// Layout requested.
    ///
    /// When this is `true`, [`layout_widgets`](Self::layout_widgets)
    /// may contain widgets, if not then only app extensions must update.
    pub layout: bool,

    /// Render requested.
    ///
    /// When this is `true`, [`render_widgets`](Self::render_widgets) or [`render_update_widgets`](Self::render_update_widgets)
    /// may contain widgets, if not then only app extensions must update.
    pub render: bool,

    /// Update targets.
    ///
    /// When this is not empty [`update`](Self::update) is `true`.
    pub update_widgets: WidgetUpdates,

    /// Info rebuild targets.
    ///
    /// When this is not empty [`info`](Self::info) is `true`.
    pub info_widgets: InfoUpdates,

    /// Layout targets.
    ///
    /// When this is not empty [`layout`](Self::layout) is `true`.
    pub layout_widgets: LayoutUpdates,

    /// Full render targets.
    ///
    /// When this is not empty [`render`](Self::render) is `true`.
    pub render_widgets: RenderUpdates,

    /// Render update targets.
    ///
    /// When this is not empty [`render`](Self::render) is `true`.
    pub render_update_widgets: RenderUpdates,
}
impl ContextUpdates {
    /// If has events, update, layout or render was requested.
    pub fn has_updates(&self) -> bool {
        !self.events.is_empty() || self.update || self.info || self.layout || self.render
    }
}
impl std::ops::BitOrAssign for ContextUpdates {
    fn bitor_assign(&mut self, rhs: Self) {
        self.events.extend(rhs.events);
        self.update |= rhs.update;
        self.update_widgets.extend(rhs.update_widgets);
        self.info |= rhs.info;
        self.info_widgets.extend(rhs.info_widgets);
        self.layout |= rhs.layout;
        self.layout_widgets.extend(rhs.layout_widgets);
        self.render |= rhs.render;
        self.render_widgets.extend(rhs.render_widgets);
        self.render_update_widgets.extend(rhs.render_update_widgets);
    }
}
impl std::ops::BitOr for ContextUpdates {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self {
        self |= rhs;
        self
    }
}

/// Constraints for inline measure.
///
/// See [`InlineConstraints`] for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct InlineConstraintsMeasure {
    /// Available space on the first row.
    pub first_max: Px,
    /// Current height of the row in the parent. If the widget wraps and defines the first
    /// row in *this* parent's row, the `mid_clear` value will be the extra space needed to clear
    /// this minimum or zero if the first how is taller. The widget must use this value to estimate the `mid_clear`
    /// value and include it in the overall measured height of the widget.
    pub mid_clear_min: Px,
}

/// Constraints for inline layout.
///
/// See [`InlineConstraints`] for more details.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct InlineConstraintsLayout {
    /// First row rect, defined by the parent.
    pub first: PxRect,
    /// Extra space in-between the first row and the mid-rows that must be offset to clear the other segments in the row.
    pub mid_clear: Px,
    /// Last row rect, defined by the parent.
    pub last: PxRect,

    /// Position of inline segments of the first row.
    pub first_segs: Arc<Vec<InlineSegmentPos>>,
    /// Position of inline segments of the last row.
    pub last_segs: Arc<Vec<InlineSegmentPos>>,
}

/// Constraints for inline measure or layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum InlineConstraints {
    /// Constraints for the measure pass.
    Measure(InlineConstraintsMeasure),
    /// Constraints the layout pass.
    Layout(InlineConstraintsLayout),
}
impl InlineConstraints {
    /// Get the `Measure` data or default.
    pub fn measure(self) -> InlineConstraintsMeasure {
        match self {
            InlineConstraints::Measure(m) => m,
            InlineConstraints::Layout(l) => InlineConstraintsMeasure {
                first_max: l.first.width(),
                mid_clear_min: l.mid_clear,
            },
        }
    }

    /// Get the `Layout` data or default.
    pub fn layout(self) -> InlineConstraintsLayout {
        match self {
            InlineConstraints::Layout(m) => m,
            InlineConstraints::Measure(_) => Default::default(),
        }
    }
}

/// Layout metrics snapshot.
///
/// A snapshot can be taken using the [`LayoutMetrics::snapshot`], you can also
/// get the metrics used during the last layout of a widget using the [`WidgetBoundsInfo::metrics`] method.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LayoutMetricsSnapshot {
    /// The [`constraints`].
    ///
    /// [`constraints`]: LayoutMetrics::constraints
    pub constraints: PxConstraints2d,

    /// The [`inline_constraints`].
    ///
    /// [`inline_constraints`]: LayoutMetrics::inline_constraints
    pub inline_constraints: Option<InlineConstraints>,

    /// The [`z_constraints`].
    ///
    /// [`z_constraints`]: LayoutMetrics::z_constraints
    pub z_constraints: PxConstraints,

    /// The [`font_size`].
    ///
    /// [`font_size`]: LayoutMetrics::font_size
    pub font_size: Px,
    /// The [`root_font_size`].
    ///
    /// [`root_font_size`]: LayoutMetrics::root_font_size
    pub root_font_size: Px,
    /// The [`scale_factor`].
    ///
    /// [`scale_factor`]: LayoutMetrics::scale_factor
    pub scale_factor: Factor,
    /// The [`viewport`].
    ///
    /// [`viewport`]: LayoutMetrics::viewport
    pub viewport: PxSize,
    /// The [`screen_ppi`].
    ///
    /// [`screen_ppi`]: LayoutMetrics::screen_ppi
    pub screen_ppi: Ppi,

    /// The [`direction`].
    ///
    /// [`direction`]: LayoutMetrics::direction
    pub direction: LayoutDirection,

    /// The [`leftover`].
    ///
    /// [`leftover`]: LayoutMetrics::leftover
    pub leftover: euclid::Size2D<Option<Px>, ()>,
}
impl LayoutMetricsSnapshot {
    /// Gets if all of the fields in `mask` are equal between `self` and `other`.
    pub fn masked_eq(&self, other: &Self, mask: LayoutMask) -> bool {
        (!mask.contains(LayoutMask::CONSTRAINTS)
            || (self.constraints == other.constraints
                && self.z_constraints == other.z_constraints
                && self.inline_constraints == other.inline_constraints))
            && (!mask.contains(LayoutMask::FONT_SIZE) || self.font_size == other.font_size)
            && (!mask.contains(LayoutMask::ROOT_FONT_SIZE) || self.root_font_size == other.root_font_size)
            && (!mask.contains(LayoutMask::SCALE_FACTOR) || self.scale_factor == other.scale_factor)
            && (!mask.contains(LayoutMask::VIEWPORT) || self.viewport == other.viewport)
            && (!mask.contains(LayoutMask::SCREEN_PPI) || self.screen_ppi == other.screen_ppi)
            && (!mask.contains(LayoutMask::DIRECTION) || self.direction == other.direction)
            && (!mask.contains(LayoutMask::LEFTOVER) || self.leftover == other.leftover)
    }
}
impl PartialEq for LayoutMetricsSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.constraints == other.constraints
            && self.z_constraints == other.z_constraints
            && self.inline_constraints == other.inline_constraints
            && self.font_size == other.font_size
            && self.root_font_size == other.root_font_size
            && self.scale_factor == other.scale_factor
            && self.viewport == other.viewport
            && self.screen_ppi == other.screen_ppi
    }
}
impl std::hash::Hash for LayoutMetricsSnapshot {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.constraints.hash(state);
        self.inline_constraints.hash(state);
        self.font_size.hash(state);
        self.root_font_size.hash(state);
        self.scale_factor.hash(state);
        self.viewport.hash(state);
        self.screen_ppi.hash(state);
    }
}

/// Layout metrics in a [`LAYOUT`] context.
#[derive(Debug, Clone)]
pub struct LayoutMetrics {
    s: LayoutMetricsSnapshot,
}
impl LayoutMetrics {
    /// New root [`LayoutMetrics`].
    ///
    /// The `font_size` sets both font sizes, the initial PPI is `96.0`, you can use the builder style method and
    /// [`with_screen_ppi`] to set a different value.
    ///
    /// [`with_screen_ppi`]: LayoutMetrics::with_screen_ppi
    pub fn new(scale_factor: Factor, viewport: PxSize, font_size: Px) -> Self {
        LayoutMetrics {
            s: LayoutMetricsSnapshot {
                constraints: PxConstraints2d::new_fill_size(viewport),
                z_constraints: PxConstraints::new_unbounded().with_min(Px(1)),
                inline_constraints: None,
                font_size,
                root_font_size: font_size,
                scale_factor,
                viewport,
                screen_ppi: Ppi::default(),
                direction: LayoutDirection::default(),
                leftover: euclid::size2(None, None),
            },
        }
    }

    /// Current size constraints.
    pub fn constraints(&self) -> PxConstraints2d {
        LAYOUT.register_metrics_use(LayoutMask::CONSTRAINTS);
        self.s.constraints
    }

    /// Current perspective constraints.
    pub fn z_constraints(&self) -> PxConstraints {
        LAYOUT.register_metrics_use(LayoutMask::CONSTRAINTS);
        self.s.z_constraints
    }

    /// Current inline constraints.
    ///
    /// Only present if the parent widget supports inline.
    pub fn inline_constraints(&self) -> Option<InlineConstraints> {
        LAYOUT.register_metrics_use(LayoutMask::CONSTRAINTS);
        self.s.inline_constraints.clone()
    }

    /// Gets the inline or text flow direction.
    pub fn direction(&self) -> LayoutDirection {
        LAYOUT.register_metrics_use(LayoutMask::DIRECTION);
        self.s.direction
    }

    /// Current computed font size.
    pub fn font_size(&self) -> Px {
        LAYOUT.register_metrics_use(LayoutMask::FONT_SIZE);
        self.s.font_size
    }

    /// Computed font size at the root widget.
    pub fn root_font_size(&self) -> Px {
        LAYOUT.register_metrics_use(LayoutMask::ROOT_FONT_SIZE);
        self.s.root_font_size
    }

    /// Pixel scale factor.
    pub fn scale_factor(&self) -> Factor {
        LAYOUT.register_metrics_use(LayoutMask::SCALE_FACTOR);
        self.s.scale_factor
    }

    /// Computed size of the nearest viewport ancestor.
    ///
    /// This is usually the window content area size, but can be the scroll viewport size or any other
    /// value depending on the implementation of the context widgets.
    pub fn viewport(&self) -> PxSize {
        LAYOUT.register_metrics_use(LayoutMask::VIEWPORT);
        self.s.viewport
    }

    /// Smallest dimension of the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn viewport_min(&self) -> Px {
        self.s.viewport.width.min(self.s.viewport.height)
    }

    /// Largest dimension of the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn viewport_max(&self) -> Px {
        self.s.viewport.width.max(self.s.viewport.height)
    }

    /// The current screen "pixels-per-inch" resolution.
    ///
    /// This value is dependent in the actual physical size of the screen that the user must manually measure.
    /// For most of the UI you only need the [`scale_factor`].
    ///
    /// If you are implementing some feature like a "print size preview", you need to use this value, and you
    /// can configure a PPI per screen in the [`MONITORS`] service.
    ///
    /// Default is `96.0`.
    ///
    /// [`MONITORS`]: crate::window::MONITORS
    /// [`scale_factor`]: LayoutMetrics::scale_factor
    pub fn screen_ppi(&self) -> Ppi {
        self.s.screen_ppi
    }

    /// Computed leftover length for the widget, given the [`Length::Leftover`] value it communicated to the parent.
    pub fn leftover(&self) -> euclid::Size2D<Option<Px>, ()> {
        LAYOUT.register_metrics_use(LayoutMask::LEFTOVER);
        self.s.leftover
    }

    /// Sets the [`constraints`] to `constraints`.
    ///
    /// [`constraints`]: Self::constraints
    pub fn with_constraints(mut self, constraints: PxConstraints2d) -> Self {
        self.s.constraints = constraints;
        self
    }

    /// Sets the [`z_constraints`] to `constraints`.
    ///
    /// [`z_constraints`]: Self::z_constraints
    pub fn with_z_constraints(mut self, constraints: PxConstraints) -> Self {
        self.s.z_constraints = constraints;
        self
    }

    /// Set the [`inline_constraints`].
    ///
    /// [`inline_constraints`]: Self::inline_constraints
    pub fn with_inline_constraints(mut self, inline_constraints: Option<InlineConstraints>) -> Self {
        self.s.inline_constraints = inline_constraints;
        self
    }

    /// Sets the [`font_size`].
    ///
    /// [`font_size`]: Self::font_size
    pub fn with_font_size(mut self, font_size: Px) -> Self {
        self.s.font_size = font_size;
        self
    }

    /// Sets the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn with_viewport(mut self, viewport: PxSize) -> Self {
        self.s.viewport = viewport;
        self
    }

    /// Sets the [`scale_factor`].
    ///
    /// [`scale_factor`]: Self::scale_factor
    pub fn with_scale_factor(mut self, scale_factor: Factor) -> Self {
        self.s.scale_factor = scale_factor;
        self
    }

    /// Sets the [`screen_ppi`].
    ///
    /// [`screen_ppi`]: Self::screen_ppi
    pub fn with_screen_ppi(mut self, screen_ppi: Ppi) -> Self {
        self.s.screen_ppi = screen_ppi;
        self
    }

    /// Sets the [`direction`].
    ///
    /// [`direction`]: Self::direction
    pub fn with_direction(mut self, direction: LayoutDirection) -> Self {
        self.s.direction = direction;
        self
    }

    /// Sets the [`leftover`].
    ///
    /// [`leftover`]: Self::leftover
    pub fn with_leftover(mut self, width: Option<Px>, height: Option<Px>) -> Self {
        self.s.leftover = euclid::size2(width, height);
        self
    }

    /// Clones all current metrics into a [snapshot].
    ///
    /// [snapshot]: LayoutMetricsSnapshot
    pub fn snapshot(&self) -> LayoutMetricsSnapshot {
        self.s.clone()
    }
}

context_var! {
    /// Wrap direction of text in a widget context.
    pub static DIRECTION_VAR: LayoutDirection = crate::l10n::LANG_VAR.map(|l| from_unic_char_direction(l.best().character_direction()));
}

/// Defines the layout flow direction.
///
/// This affects inline layout, some [`Align`] options and the base text shaping direction.
///
/// The contextual value can be read during layout in [`LayoutMetrics::direction`], and it can be set using [`LayoutMetrics::with_direction`].
/// Properties that define a more specific *direction* value also set this value, for example, a *TextDirection* property will also set the
/// layout direction.
///
/// Note that this does not affect the layout origin, all points are offsets from the top-left corner independent of this value.
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LayoutDirection {
    /// left-to-right.
    LTR,
    /// Right-to-left.
    RTL,
}
impl LayoutDirection {
    /// Matches `LTR`.
    pub fn is_ltr(self) -> bool {
        matches!(self, Self::LTR)
    }

    /// Matches `RTL`.
    pub fn is_rtl(self) -> bool {
        matches!(self, Self::RTL)
    }
}
impl Default for LayoutDirection {
    /// Default is `LTR`.
    fn default() -> Self {
        Self::LTR
    }
}
impl fmt::Debug for LayoutDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LayoutDirection::")?;
        }
        match self {
            Self::LTR => write!(f, "LTR"),
            Self::RTL => write!(f, "RTL"),
        }
    }
}

pub(crate) fn from_unic_char_direction(d: unic_langid::CharacterDirection) -> LayoutDirection {
    match d {
        unic_langid::CharacterDirection::LTR => LayoutDirection::LTR,
        unic_langid::CharacterDirection::RTL => LayoutDirection::RTL,
    }
}

pub(crate) fn from_unic_level(d: unicode_bidi::Level) -> LayoutDirection {
    if d.is_ltr() {
        LayoutDirection::LTR
    } else {
        LayoutDirection::RTL
    }
}

pub(crate) fn into_unic_level(d: LayoutDirection) -> unicode_bidi::Level {
    match d {
        LayoutDirection::LTR => unicode_bidi::Level::ltr(),
        LayoutDirection::RTL => unicode_bidi::Level::rtl(),
    }
}

pub(crate) fn into_harf_direction(d: LayoutDirection) -> harfbuzz_rs::Direction {
    match d {
        LayoutDirection::LTR => harfbuzz_rs::Direction::Ltr,
        LayoutDirection::RTL => harfbuzz_rs::Direction::Rtl,
    }
}
