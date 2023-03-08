//! New static contexts.

use std::{fmt, mem, sync::Arc, task::Waker};

use linear_map::set::LinearSet;
use parking_lot::{Mutex, RwLock};

mod state;
pub use state::*;

mod trace;
pub use trace::*;

mod local;
pub use local::*;

use crate::{
    app::{AppDisconnected, AppEventSender, LoopTimer},
    crate_util::{Handle, HandleOwner, IdSet, WeakHandle},
    event::{Event, EventArgs, EventHandle, EventHandles, EventUpdate, EVENTS, EVENTS_SV},
    handler::{AppHandler, AppHandlerArgs, AppWeakHandle},
    render::ReuseRange,
    timer::TIMERS_SV,
    units::*,
    var::{AnyVar, VarHandle, VarHandles, VARS},
    widget_info::{
        InlineSegmentPos, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoTree, WidgetInlineMeasure, WidgetMeasure, WidgetPath,
    },
    widget_instance::{UiNode, WidgetId},
    window::{WindowId, WindowMode},
};

bitflags! {
    pub(crate) struct UpdateFlags: u8 {
        const REINIT = 0b1000_0000;
        const INFO =   0b0001_0000;
        const UPDATE = 0b0000_0001;
        const LAYOUT = 0b0000_0010;
        const RENDER = 0b0000_0100;
        const RENDER_UPDATE = 0b0000_1000;
    }
}

struct WindowCtxData {
    id: WindowId,
    mode: WindowMode,
    state: RwLock<OwnedStateMap<WINDOW>>,
    widget_tree: RwLock<Option<WidgetInfoTree>>,

    #[cfg(any(test, doc, feature = "test_util"))]
    frame_id: Mutex<crate::render::FrameId>,
}
impl WindowCtxData {
    fn no_context() -> Self {
        panic!("no window in context")
    }
}

/// Defines the backing data of [`WINDOW`].
///
/// Each window owns this data and calls [`WINDOW.with_context`](WINDOW::with_context) to delegate to it's child node.
pub struct WindowCtx(Mutex<Option<Arc<WindowCtxData>>>);
impl WindowCtx {
    /// New window context.
    pub fn new(id: WindowId, mode: WindowMode) -> Self {
        Self(Mutex::new(Some(Arc::new(WindowCtxData {
            id,
            mode,
            state: RwLock::new(OwnedStateMap::default()),
            widget_tree: RwLock::new(None),

            #[cfg(any(test, doc, feature = "test_util"))]
            frame_id: Mutex::new(crate::render::FrameId::first()),
        }))))
    }

    /// Sets the widget tree, must be called after every info update.
    ///
    /// Window contexts are partially available in the window new closure, but values like the `widget_tree` is
    /// available on init, so a [`WidgetInfoTree::wgt`] must be set as soon as the window and widget ID are available.
    pub fn set_widget_tree(&mut self, widget_tree: WidgetInfoTree) {
        *self.0.get_mut().as_mut().unwrap().widget_tree.write() = Some(widget_tree);
    }

    /// Gets the window ID.
    pub fn id(&self) -> WindowId {
        self.0.lock().as_ref().unwrap().id
    }

    /// Gets the window mode.
    pub fn mode(&self) -> WindowMode {
        self.0.lock().as_ref().unwrap().mode
    }

    /// Gets the window tree.
    pub fn widget_tree(&self) -> WidgetInfoTree {
        self.0.lock().as_ref().unwrap().widget_tree.read().as_ref().unwrap().clone()
    }

    /// Call `f` with an exclusive lock to the window state.
    pub fn with_state<R>(&mut self, f: impl FnOnce(&mut OwnedStateMap<WINDOW>) -> R) -> R {
        f(&mut self.0.get_mut().as_mut().unwrap().state.write())
    }
}

struct WidgetCtxData {
    parent_id: Mutex<Option<WidgetId>>,
    id: WidgetId,
    flags: Mutex<UpdateFlags>,
    state: RwLock<OwnedStateMap<WIDGET>>,
    var_handles: Mutex<VarHandles>,
    event_handles: Mutex<EventHandles>,
    bounds: Mutex<WidgetBoundsInfo>,
    border: Mutex<WidgetBorderInfo>,
    render_reuse: Mutex<Option<ReuseRange>>,
}
impl WidgetCtxData {
    fn no_context() -> Self {
        panic!("no widget in context")
    }

    fn take_flag(&self, flag: UpdateFlags) -> bool {
        let c = self.flags.lock().contains(flag);
        self.flags.lock().remove(flag);
        c
    }
}

/// Defines the backing data of [`WIDGET`].
///
/// Each widget owns this data and calls [`WIDGET.with_context`] to delegate to it's child node.
///
/// [`WIDGET.with_context`]: WIDGET::with_context
pub struct WidgetCtx(Mutex<Option<Arc<WidgetCtxData>>>);
impl WidgetCtx {
    /// New widget context.
    pub fn new(id: WidgetId) -> Self {
        Self(Mutex::new(Some(Arc::new(WidgetCtxData {
            parent_id: Mutex::new(None),
            id,
            flags: Mutex::new(UpdateFlags::empty()),
            state: RwLock::new(OwnedStateMap::default()),
            var_handles: Mutex::new(VarHandles::dummy()),
            event_handles: Mutex::new(EventHandles::dummy()),
            bounds: Mutex::new(WidgetBoundsInfo::default()),
            border: Mutex::new(WidgetBorderInfo::default()),
            render_reuse: Mutex::new(None),
        }))))
    }

    /// Drops all var and event handles.
    pub fn deinit(&mut self) {
        let ctx = self.0.get_mut().as_mut().unwrap();
        ctx.var_handles.lock().clear();
        ctx.event_handles.lock().clear();
        *ctx.flags.lock() = UpdateFlags::empty();
    }

    fn take_flag(&self, flag: UpdateFlags) -> bool {
        let mut ctx = self.0.lock();
        let ctx = ctx.as_mut().unwrap();
        ctx.take_flag(flag)
    }

    fn contains_flag(&self, flag: UpdateFlags) -> bool {
        self.0.lock().as_ref().unwrap().flags.lock().contains(flag)
    }

    /// Returns `true` once if a info rebuild was requested in a previous [`WIDGET.with_context`] call.
    ///
    /// Child nodes can request updates using [`WIDGET.info`].
    ///
    /// [`WIDGET.with_context`]: WIDGET::with_context
    /// [`WIDGET.info`]: WIDGET::info
    pub fn take_info(&self) -> bool {
        self.take_flag(UpdateFlags::INFO)
    }

    /// Returns `true` if re-layout was requested for the widget.
    pub fn is_pending_layout(&self) -> bool {
        self.contains_flag(UpdateFlags::LAYOUT)
    }

    /// Returns `true` once if a re-layout was requested in a previous [`WIDGET.with_context`] call.
    ///
    /// Child nodes can request updates using [`WIDGET.layout`].    
    ///  
    /// [`WIDGET.with_context`]: WIDGET::with_context
    /// [`WIDGET.layout`]: WIDGET::layout
    pub fn take_layout(&mut self) -> bool {
        self.take_flag(UpdateFlags::LAYOUT)
    }

    /// Returns `true` if full re-render was requested for the widget.
    ///
    /// Note that [`take_render`] will upgrade [`is_pending_render_update`] to full render.
    ///
    /// [`take_render`]: Self::take_render
    /// [`is_pending_render_update`]: Self::is_pending_render_update
    pub fn is_pending_render(&self) -> bool {
        self.contains_flag(UpdateFlags::RENDER)
    }

    /// Returns the render reuse range once if a re-render was **not** requested in a previous [`WIDGET.with_context`] call.
    ///
    /// Child nodes can request updates using [`WIDGET.render`]. Upgrades [`WIDGET.render_update`] requests to a full render request,
    /// the widget node will receive a single call to [`UiNode::render`] or [`UiNode::render_update`], if any other widget in the window
    /// requests full render the render updates are converted to full render.
    ///
    /// The updated reuse range must be stored using [`set_render_reuse`] after the widget is pushed onto the frame.
    ///
    /// [`take_render_update`]: Self::take_render_update
    /// [`set_render_reuse`]: Self::set_render_reuse
    /// [`WIDGET.with_context`]: WIDGET::with_context
    /// [`WIDGET.render`]: WIDGET::render
    /// [`WIDGET.render_update`]: WIDGET::render_update
    pub fn take_render(&self) -> Option<ReuseRange> {
        let mut ctx = self.0.lock();
        let ctx = ctx.as_mut().unwrap();

        let mut flags = ctx.flags.lock();
        if flags.contains(UpdateFlags::RENDER) || flags.contains(UpdateFlags::RENDER_UPDATE) {
            flags.remove(UpdateFlags::RENDER);
            flags.remove(UpdateFlags::RENDER_UPDATE);
            *ctx.render_reuse.lock() = None;
            None
        } else {
            ctx.render_reuse.lock().take()
        }
    }

    /// Gets a copy of the stored render reuse range.
    ///
    /// Note that widget render implementers must use [`take_render`], this method is only for inspecting the value.
    ///
    /// [`take_render`]: Self::take_render
    pub fn render_reuse(&self) -> Option<ReuseRange> {
        self.0.lock().as_ref().unwrap().render_reuse.lock().clone()
    }

    /// Store a render reuse range that can be used next render if no render request is made.
    pub fn set_render_reuse(&self, range: Option<ReuseRange>) {
        let mut ctx = self.0.lock();
        *ctx.as_mut().unwrap().render_reuse.lock() = range;
    }

    /// Returns `true` if frame update was requested for the widget.
    pub fn is_pending_render_update(&self) -> bool {
        self.contains_flag(UpdateFlags::RENDER_UPDATE)
    }

    /// Returns `true` once if a re-render was requested in a previous [`WIDGET.with_context`] call.
    ///
    /// Child nodes can request updates using [`WIDGET.render_update`].
    ///
    /// Logs an error if a full render is requested, must be called after [`take_render`].
    ///
    /// [`take_render`]: Self::take_render
    ///
    /// [`WIDGET.with_context`]: WIDGET::with_context
    /// [`WIDGET.render_update`]: WIDGET::render_update
    pub fn take_render_update(&self) -> bool {
        let mut ctx = self.0.lock();
        let ctx = ctx.as_mut().unwrap();

        let mut flags = ctx.flags.lock();
        if flags.contains(UpdateFlags::RENDER) {
            tracing::error!("widget `{:?}` called `take_render_update` before `take_render`", ctx.id);
        }
        let r = flags.contains(UpdateFlags::RENDER_UPDATE);
        flags.remove(UpdateFlags::RENDER_UPDATE);
        r
    }

    /// Returns `true` if an [`WIDGET.reinit`] request was made.
    ///
    /// Unlike other requests, the widget re-init immediately.
    ///
    /// [`WIDGET.reinit`]: WIDGET::reinit
    pub fn take_reinit(&mut self) -> bool {
        self.take_flag(UpdateFlags::REINIT)
    }

    /// Gets the widget id.
    pub fn id(&self) -> WidgetId {
        self.0.lock().as_ref().unwrap().id
    }
    /// Gets the widget bounds.
    pub fn bounds(&self) -> WidgetBoundsInfo {
        self.0.lock().as_ref().unwrap().bounds.lock().clone()
    }

    /// Gets the widget borders.
    pub fn border(&self) -> WidgetBorderInfo {
        self.0.lock().as_ref().unwrap().border.lock().clone()
    }

    /// Call `f` with an exclusive lock to the widget state.
    pub fn with_state<R>(&mut self, f: impl FnOnce(&mut OwnedStateMap<WIDGET>) -> R) -> R {
        f(&mut self.0.get_mut().as_mut().unwrap().state.write())
    }
}

context_local! {
    static WINDOW_CTX: WindowCtxData = WindowCtxData::no_context();
    static WIDGET_CTX: WidgetCtxData = WidgetCtxData::no_context();
}

/// Current context window.
///
/// This represents the minimum features required for a window context, see [`WINDOW_CTRL`] for more
/// features provided by the core window implementation.
///
/// [`WINDOW_CTRL`]: crate::window::WINDOW_CTRL
pub struct WINDOW;
impl WINDOW {
    /// Calls `f` while the window is set to `ctx`.
    ///
    /// The `ctx` must be `Some(_)`, it will be moved to the [`WINDOW`] storage and back to `ctx` after `f` returns.
    pub fn with_context<R>(&self, ctx: &WindowCtx, f: impl FnOnce() -> R) -> R {
        let mut ctx = ctx.0.lock();
        let _span = match &*ctx {
            Some(c) => UpdatesTrace::window_span(c.id),
            None => panic!("window is required"),
        };
        WINDOW_CTX.with_context(&mut ctx, f)
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
    pub fn widget_tree(&self) -> WidgetInfoTree {
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

/// Test only methods.
#[cfg(any(test, doc, feature = "test_util"))]
impl WINDOW {
    /// Calls `f` inside a new headless window and root widget.
    pub fn with_test_context<R>(&self, f: impl FnOnce() -> R) -> R {
        let window_id = WindowId::new_unique();
        let root_id = WidgetId::new_unique();
        let mut ctx = WindowCtx::new(window_id, WindowMode::Headless);
        ctx.set_widget_tree(WidgetInfoTree::wgt(window_id, root_id));
        WINDOW.with_context(&ctx, || {
            let ctx = WidgetCtx::new(root_id);
            WIDGET.with_context(&ctx, f)
        })
    }

    /// Call inside [`with_test_context`] to init the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_init(&self, content: &mut impl UiNode) -> ContextUpdates {
        content.init();
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to deinit the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_deinit(&self, content: &mut impl UiNode) -> ContextUpdates {
        content.deinit();
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to rebuild info the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_info(&self, content: &impl UiNode) -> ContextUpdates {
        let l_size = PxSize::new(1000.into(), 800.into());
        let mut info = crate::widget_info::WidgetInfoBuilder::new(
            WINDOW.id(),
            WIDGET.id(),
            WidgetBoundsInfo::new_size(l_size, l_size),
            WidgetBorderInfo::new(),
            1.fct(),
            None,
        );
        content.info(&mut info);
        let tree = info.finalize().0;
        *WINDOW_CTX.get().widget_tree.write() = Some(tree);
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to delivery an event to the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_event(&self, content: &mut impl UiNode, update: &mut EventUpdate) -> ContextUpdates {
        update.fulfill_search([&WINDOW.widget_tree()].into_iter());
        content.event(update);
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to update the `content` as a child of the test window root.
    ///
    /// The `updates` can be set to a custom delivery list, otherwise window root and `content` widget are flagged for update.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_update(&self, content: &mut impl UiNode, updates: Option<&mut WidgetUpdates>) -> ContextUpdates {
        if let Some(updates) = updates {
            updates.fulfill_search([&WINDOW.widget_tree()].into_iter());
            content.update(updates)
        } else {
            let target = if let Some(content_id) = content.with_context(|| WIDGET.id()) {
                WidgetPath::new(WINDOW.id(), vec![WIDGET.id(), content_id].into())
            } else {
                WidgetPath::new(WINDOW.id(), vec![WIDGET.id()].into())
            };

            let mut updates = WidgetUpdates::new(UpdateDeliveryList::new_any());
            updates.delivery_list.insert_path(&target);

            content.update(&mut updates);
        }
        UPDATES.apply()
    }

    /// Call inside [`with_test_context`] to layout the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_layout(&self, content: &mut impl UiNode, constrains: Option<PxConstrains2d>) -> (PxSize, ContextUpdates) {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());

        let viewport = content
            .with_context(|| WIDGET.bounds().outer_size())
            .unwrap_or_else(|| PxSize::new(Px(1000), Px(800)));

        let metrics = LayoutMetrics::new(1.fct(), viewport, font_size).with_constrains(|c| constrains.unwrap_or(c));
        let size = LAYOUT.with_context(metrics, || {
            crate::widget_info::WidgetLayout::with_root_widget(0, |wl| content.layout(wl))
        });
        (size, UPDATES.apply())
    }

    /// Call inside [`with_test_context`] to render the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_render(&self, content: &impl UiNode) -> (crate::render::BuiltFrame, ContextUpdates) {
        use crate::render::*;

        let mut frame = {
            let win = WINDOW_CTX.get();
            let wgt = WIDGET_CTX.get();

            let mut frame_id = win.frame_id.lock();
            *frame_id = frame_id.next();

            let f = FrameBuilder::new_renderless(
                *frame_id,
                wgt.id,
                &wgt.bounds.lock(),
                win.widget_tree.read().as_ref().unwrap(),
                1.fct(),
                crate::text::FontAntiAliasing::Default,
                None,
            );
            f
        };

        frame.push_inner(self.test_root_translation_key(), false, |frame| {
            content.render(frame);
        });

        let tree = WINDOW_CTX.get().widget_tree.read().as_ref().unwrap().clone();
        let f = frame.finalize(&tree).0;

        (f, UPDATES.apply())
    }

    /// Call inside [`with_test_context`] to render_update the `content` as a child of the test window root.
    ///
    /// [`with_test_context`]: Self::with_test_context
    pub fn test_render_update(&self, content: &impl UiNode) -> (crate::render::BuiltFrameUpdate, ContextUpdates) {
        use crate::render::*;

        let mut update = {
            let win = WINDOW_CTX.get();
            let wgt = WIDGET_CTX.get();

            let mut frame_id = win.frame_id.lock();
            *frame_id = frame_id.next_update();

            let f = FrameUpdate::new(
                *frame_id,
                wgt.id,
                wgt.bounds.lock().clone(),
                None,
                crate::color::RenderColor::BLACK,
                None,
            );
            f
        };

        update.update_inner(self.test_root_translation_key(), false, |update| {
            content.render_update(update);
        });
        let tree = WINDOW_CTX.get().widget_tree.read().as_ref().unwrap().clone();
        let f = update.finalize(&tree).0;

        (f, UPDATES.apply())
    }

    fn test_root_translation_key(&self) -> crate::render::FrameValueKey<PxTransform> {
        static ID: StaticStateId<crate::render::FrameValueKey<PxTransform>> = StaticStateId::new_unique();
        WINDOW.with_state_mut(|mut s| *s.entry(&ID).or_insert_with(crate::render::FrameValueKey::new_unique))
    }
}

/// Current context widget.
pub struct WIDGET;
impl WIDGET {
    /// Calls `f` while the widget is set to `ctx`.
    ///
    /// The `ctx` must be `Some(_)`, it will be moved to the [`WIDGET`] storage and back to `ctx` after `f` returns.
    pub fn with_context<R>(&self, ctx: &WidgetCtx, f: impl FnOnce() -> R) -> R {
        let parent_id = WIDGET.try_id();

        let mut ctx = ctx.0.lock();
        let old_flags = if let Some(ctx) = &mut *ctx {
            *ctx.parent_id.lock() = parent_id;
            mem::replace(&mut *ctx.flags.lock(), UpdateFlags::empty())
        } else {
            unreachable!()
        };

        let r = WIDGET_CTX.with_context(&mut ctx, f);

        let ctx = ctx.as_mut().unwrap();

        if let Some(parent) = parent_id.map(|_| WIDGET_CTX.get()) {
            if ctx.take_flag(UpdateFlags::UPDATE) {
                // only used to avoid making too many `UPDATES.update(wgt_id)` requests.
                parent.flags.lock().insert(UpdateFlags::UPDATE);
            }

            // used by window to immediately rebuild info.
            if ctx.flags.lock().contains(UpdateFlags::INFO) {
                parent.flags.lock().insert(UpdateFlags::INFO);
            }

            // invalidate layout & render
            if ctx.flags.lock().contains(UpdateFlags::LAYOUT) {
                parent.flags.lock().insert(UpdateFlags::LAYOUT);
            }
            if ctx.flags.lock().contains(UpdateFlags::RENDER) {
                parent.flags.lock().insert(UpdateFlags::RENDER);
            } else if ctx.flags.lock().contains(UpdateFlags::RENDER_UPDATE) {
                parent.flags.lock().insert(UpdateFlags::RENDER_UPDATE);
            }
        }
        ctx.parent_id.lock().take();
        ctx.flags.lock().insert(old_flags);

        r
    }

    /// Calls `f` while no widget is available in the context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        WIDGET_CTX.with_default(f)
    }

    /// Calls `f` with an override target for var and event subscription handles.
    pub fn with_handles<R>(&self, var_handles: &mut VarHandles, event_handles: &mut EventHandles, f: impl FnOnce() -> R) -> R {
        let w = WIDGET_CTX.get();
        {
            mem::swap(&mut *w.var_handles.lock(), var_handles);
            mem::swap(&mut *w.event_handles.lock(), event_handles);
        }
        let r = f();
        {
            mem::swap(&mut *w.var_handles.lock(), var_handles);
            mem::swap(&mut *w.event_handles.lock(), event_handles);
        }
        r
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

    /// Get the widget ID if called inside a widget, or panic.
    pub fn id(&self) -> WidgetId {
        WIDGET_CTX.get().id
    }

    /// Schedule an update for the current widget.
    ///
    /// After the current update cycle the app-extensions, parent window and widgets will update again.
    pub fn update(&self) -> &Self {
        let w = WIDGET_CTX.get();
        let mut flags = w.flags.lock();
        if !flags.contains(UpdateFlags::UPDATE) {
            flags.insert(UpdateFlags::UPDATE);
            let id = w.id;
            drop(flags);
            UPDATES.update(id);
        }
        self
    }

    /// Schedule an info rebuild for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-build the info tree.
    pub fn info(&self) -> &Self {
        WIDGET_CTX.get().flags.lock().insert(UpdateFlags::INFO);
        self
    }

    /// Schedule a re-layout for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-layout.
    pub fn layout(&self) -> &Self {
        let w = WIDGET_CTX.get();
        let mut flags = w.flags.lock();
        if !flags.contains(UpdateFlags::LAYOUT) {
            flags.insert(UpdateFlags::LAYOUT);
            drop(flags);
            UPDATES.layout();
        }
        self
    }

    /// Schedule a re-render for the current widget.
    ///
    /// After all requested updates and layouts apply the parent window and widgets will re-render.
    ///
    /// This also overrides any pending [`render_update`] request.
    ///
    /// [`render_update`]: Self::render_update
    pub fn render(&self) -> &Self {
        let w = WIDGET_CTX.get();
        let mut flags = w.flags.lock();
        if !flags.contains(UpdateFlags::RENDER) {
            flags.insert(UpdateFlags::RENDER);
            drop(flags);
            UPDATES.render();
        }
        self
    }

    /// Schedule a frame update for the current widget.
    ///
    /// After all requested updates and layouts apply the parent window and widgets will update the frame.
    ///
    /// This request is supplanted by any [`render`] request.
    ///
    /// [`render`]: Self::render
    pub fn render_update(&self) -> &Self {
        let w = WIDGET_CTX.get();
        let mut flags = w.flags.lock();
        if !flags.contains(UpdateFlags::RENDER_UPDATE) {
            flags.insert(UpdateFlags::RENDER_UPDATE);
            drop(flags);
            UPDATES.render();
        }
        self
    }

    /// Flags the widget to re-init after the current update returns.
    ///
    /// The widget will de-init and init as soon as it sees this request.
    pub fn reinit(&self) {
        WIDGET_CTX.get().flags.lock().insert(UpdateFlags::REINIT);
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

    /// Subscribe to receive updates when the `var` changes.
    pub fn sub_var(&self, var: &impl AnyVar) -> &Self {
        let w = WIDGET_CTX.get();
        let s = var.subscribe(w.id);
        w.var_handles.lock().push(s);
        self
    }

    /// Subscribe to receive events from `event`.
    pub fn sub_event<A: EventArgs>(&self, event: &Event<A>) -> &Self {
        let w = WIDGET_CTX.get();
        let s = event.subscribe(w.id);
        w.event_handles.lock().push(s);
        self
    }

    /// Hold the `handle` until the widget is deinited.
    pub fn push_event_handle(&self, handle: EventHandle) {
        WIDGET_CTX.get().event_handles.lock().push(handle);
    }

    /// Hold the `handles` until the widget is deinited.
    pub fn push_event_handles(&self, handles: EventHandles) {
        WIDGET_CTX.get().event_handles.lock().extend(handles);
    }

    /// Hold the `handle` until the widget is deinited.
    pub fn push_var_handle(&self, handle: VarHandle) {
        WIDGET_CTX.get().var_handles.lock().push(handle);
    }

    /// Hold the `handles` until the widget is deinited.
    pub fn push_var_handles(&self, handles: VarHandles) {
        WIDGET_CTX.get().var_handles.lock().extend(handles);
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
        *WIDGET_CTX.get().parent_id.lock()
    }
}

context_local! {
    static LAYOUT_CTX: LayoutCtx = LayoutCtx::no_context();
}

struct LayoutCtx {
    metrics: LayoutMetrics,
}
impl LayoutCtx {
    fn no_context() -> Self {
        panic!("no layout context")
    }
}

/// Current layout context.
///
/// Only available in measure and layout methods.
pub struct LAYOUT;
impl LAYOUT {
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

    /// Calls `metrics` to generate new metrics that are used during the call to `f`.
    pub fn with_metrics<R>(&self, metrics: impl FnOnce(LayoutMetrics) -> LayoutMetrics, f: impl FnOnce() -> R) -> R {
        let new = metrics(self.metrics());
        self.with_context(new, f)
    }

    /// Current size constrains.
    pub fn constrains(&self) -> PxConstrains2d {
        LAYOUT_CTX.get().metrics.constrains()
    }

    /// Current length constrains for the given axis.
    pub fn constrains_for(&self, x_axis: bool) -> PxConstrains {
        let c = self.constrains();
        if x_axis {
            c.x
        } else {
            c.y
        }
    }

    /// Calls `constrains` to generate new constrains that are used during the call to  `f`.
    pub fn with_constrains<R>(&self, constrains: impl FnOnce(PxConstrains2d) -> PxConstrains2d, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_constrains(constrains), f)
    }

    /// Calls `constrains` to generate new constrains for the given axis that are used during the call to  `f`.
    pub fn with_constrains_for<R>(&self, x_axis: bool, constrains: impl FnOnce(PxConstrains) -> PxConstrains, f: impl FnOnce() -> R) -> R {
        self.with_constrains(
            move |mut c| {
                if x_axis {
                    c.x = constrains(c.x);
                } else {
                    c.y = constrains(c.y);
                }
                c
            },
            f,
        )
    }

    /// Runs a function `f` in a context that has its max size subtracted by `removed` and its final size added by `removed`.
    pub fn with_sub_size(&self, removed: PxSize, f: impl FnOnce() -> PxSize) -> PxSize {
        self.with_constrains(|c| c.with_less_size(removed), f) + removed
    }

    /// Runs a function `f` in a layout context that has its max size added by `added` and its final size subtracted by `added`.
    pub fn with_add_size(&self, added: PxSize, f: impl FnOnce() -> PxSize) -> PxSize {
        self.with_constrains(|c| c.with_more_size(added), f) - added
    }

    /// Current inline constrains.
    pub fn inline_constrains(&self) -> Option<InlineConstrains> {
        LAYOUT_CTX.get().metrics.inline_constrains()
    }

    /// Calls `f` with `inline_constrains` in the context.
    pub fn with_inline_constrains<R>(&self, inline_constrains: Option<InlineConstrains>, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_inline_constrains(inline_constrains), f)
    }

    /// Runs a function `f` in a measure context that has a new or modified inline constrain.
    ///
    /// The `inline_constrains` closure is called to produce the new constrains, the input is the current constrains.
    /// If it returns `None` inline is disabled for the widget.
    ///
    /// Note that panels implementing inline layout should prefer using [`measure_inline`] instead of this method.
    ///
    /// [`measure_inline`]: Self::measure_inline

    pub fn with_inline_measure<R>(
        &self,
        wm: &mut WidgetMeasure,
        inline_constrains: impl FnOnce(Option<InlineConstrainsMeasure>) -> Option<InlineConstrainsMeasure>,
        f: impl FnOnce(&mut WidgetMeasure) -> R,
    ) -> R {
        let inline_constrains = inline_constrains(self.inline_constrains().map(InlineConstrains::measure)).map(InlineConstrains::Measure);
        if inline_constrains.is_none() {
            wm.disable_inline();
        }

        self.with_inline_constrains(inline_constrains, || f(wm))
    }

    /// Runs a function `f` in a measure context that has a new or modified inline constrain.
    ///
    /// The `inline_constrains` closure is called to produce the new constrains, the input is the current constrains.
    /// If it returns `None` inline is disabled for the widget.
    ///
    /// Note that panels implementing inline layout should prefer using [`layout_inline`] instead of this method.
    ///
    /// [`layout_inline`]: Self::layout_inline

    pub fn with_inline_layout<R>(
        &self,
        inline_constrains: impl FnOnce(Option<InlineConstrainsLayout>) -> Option<InlineConstrainsLayout>,
        f: impl FnOnce() -> R,
    ) -> R {
        let inline_constrains = inline_constrains(self.inline_constrains().map(InlineConstrains::layout)).map(InlineConstrains::Layout);
        self.with_inline_constrains(inline_constrains, f)
    }

    /// Measure the child in a new inline context.
    ///
    /// The `first_max` is the available space for the first row. The `mid_clear_min` is the current height of the row.
    ///
    /// Returns the measured inline data and the desired size, or `None` and the desired size if the
    /// widget does not support measure. Note that the measured data is also updated in [`WidgetBoundsInfo::measure_inline`].
    pub fn measure_inline(&self, first_max: Px, mid_clear_min: Px, child: &impl UiNode) -> (Option<WidgetInlineMeasure>, PxSize) {
        let constrains = InlineConstrains::Measure(InlineConstrainsMeasure { first_max, mid_clear_min });
        let size = self.with_inline_constrains(Some(constrains), || child.measure(&mut WidgetMeasure::new()));
        let inline = child.with_context(|| WIDGET.bounds().measure_inline()).flatten();
        (inline, size)
    }

    /// Runs a function `f` in a layout context that has enabled inline.
    pub fn layout_inline<R>(
        &mut self,
        first: PxRect,
        mid_clear: Px,
        last: PxRect,
        first_segs: Arc<Vec<InlineSegmentPos>>,
        last_segs: Arc<Vec<InlineSegmentPos>>,
        f: impl FnOnce() -> R,
    ) -> R {
        let constrains = InlineConstrains::Layout(InlineConstrainsLayout {
            first,
            mid_clear,
            last,
            first_segs,
            last_segs,
        });
        self.with_inline_constrains(Some(constrains), f)
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
        self.with_metrics(|m| m.with_font_size(font_size), f)
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
    pub fn viewport_for(&self, x_axis: bool) -> Px {
        let vp = self.viewport();
        if x_axis {
            vp.width
        } else {
            vp.height
        }
    }

    /// Calls `f` with `viewport` in the context.
    pub fn with_viewport<R>(&self, viewport: PxSize, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_viewport(viewport), f)
    }

    /// Current scale factor.
    pub fn scale_factor(&self) -> Factor {
        LAYOUT_CTX.get().metrics.scale_factor()
    }

    /// Calls `f` with `scale_factor` in the context.
    pub fn with_scale_factor<R>(&self, scale_factor: Factor, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_scale_factor(scale_factor), f)
    }

    /// Current screen PPI.
    pub fn screen_ppi(&self) -> f32 {
        LAYOUT_CTX.get().metrics.screen_ppi()
    }

    /// Calls `f` with `screen_ppi` in the context.
    pub fn with_screen_ppi<R>(&self, screen_ppi: f32, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_screen_ppi(screen_ppi), f)
    }

    /// Current layout direction.
    pub fn direction(&self) -> LayoutDirection {
        LAYOUT_CTX.get().metrics.direction()
    }

    /// Calls `f` with `direction` in the context.
    pub fn with_direction<R>(&self, direction: LayoutDirection, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_direction(direction), f)
    }

    /// Context leftover length for the widget, given the [`Length::Leftover`] value it communicated to the parent.
    ///
    /// [`leftover_count`]: Self::leftover_count
    pub fn leftover(&self) -> euclid::Size2D<Option<Px>, ()> {
        LAYOUT_CTX.get().metrics.leftover()
    }

    /// Context leftover length for the given axis.
    pub fn leftover_for(&self, x_axis: bool) -> Option<Px> {
        let l = self.leftover();
        if x_axis {
            l.width
        } else {
            l.height
        }
    }

    /// Calls `f` with [`leftover`] set to `with` and `height`.
    ///
    /// [`leftover`]: Self::leftover
    pub fn with_leftover<R>(&self, width: Option<Px>, height: Option<Px>, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_leftover(width, height), f)
    }
}

app_local! {
    static UPDATES_SV: UpdatesService = UpdatesService::new();
}
struct UpdatesService {
    event_sender: Option<AppEventSender>,

    flags: UpdateFlags,
    update_widgets: UpdateDeliveryList,

    pre_handlers: Mutex<Vec<UpdateHandler>>,
    pos_handlers: Mutex<Vec<UpdateHandler>>,

    app_is_awake: bool,
    awake_pending: bool,
}
impl UpdatesService {
    fn new() -> Self {
        Self {
            event_sender: None,
            flags: UpdateFlags::empty(),
            update_widgets: UpdateDeliveryList::new_any(),

            pre_handlers: Mutex::new(vec![]),
            pos_handlers: Mutex::new(vec![]),

            app_is_awake: false,
            awake_pending: false,
        }
    }

    fn flag_update(&mut self, flag: UpdateFlags) {
        if !self.flags.contains(flag) {
            self.flags.insert(flag);
            self.send_awake();
        }
    }

    fn send_awake(&mut self) {
        if !self.app_is_awake && !self.awake_pending {
            self.awake_pending = true;
            if let Err(AppDisconnected(())) = self.event_sender.as_ref().unwrap().send_check_update() {
                tracing::error!("no app connected to update");
            }
        }
    }

    fn app_awake(&mut self, wake: bool) {
        self.awake_pending = false;
        self.app_is_awake = wake;
    }
}

/// Update pump and schedule service.
pub struct UPDATES;
impl UPDATES {
    pub(crate) fn init(&self, event_sender: AppEventSender) {
        UPDATES_SV.write().event_sender = Some(event_sender);
    }

    /// Applies pending `timers`, `sync`, `vars` and `events` and returns the update
    /// requests and a time for the loop to awake and update.
    #[must_use]
    pub(crate) fn apply(&self) -> ContextUpdates {
        let events = EVENTS.apply_updates();
        VARS.apply_updates();

        let (update, update_widgets, layout, render) = UPDATES.take_updates();

        ContextUpdates {
            events,
            update,
            update_widgets,
            layout,
            render,
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
    pub fn has_pending_updates(&self) -> bool {
        let us = !UPDATES_SV.read().flags.is_empty();
        us || VARS.has_pending_updates() || EVENTS_SV.write().has_pending_updates() || TIMERS_SV.read().has_pending_updates()
    }

    /// Create an [`AppEventSender`] that can be used to awake the app and send app events from threads outside of the app.
    pub fn sender(&self) -> AppEventSender {
        UPDATES_SV.read().event_sender.as_ref().unwrap().clone()
    }

    /// Create an std task waker that wakes the event loop and updates all `targets`.
    pub fn waker(&self, targets: Vec<WidgetId>) -> Waker {
        UPDATES_SV.read().event_sender.as_ref().unwrap().waker(targets)
    }

    /// Schedules an update that affects the `target`.
    ///
    /// After the current update cycle ends a new update will happen that includes the `target` widget.
    pub fn update(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        UpdatesTrace::log_update();
        self.update_internal(target.into());
        self
    }
    pub(crate) fn update_internal(&self, target: Option<WidgetId>) {
        let mut u = UPDATES_SV.write();
        u.flag_update(UpdateFlags::UPDATE);
        if let Some(id) = target {
            u.update_widgets.search_widget(id);
        }
    }

    pub(crate) fn send_awake(&self) {
        UPDATES_SV.write().send_awake();
    }

    pub(crate) fn recv_update_internal(&mut self, targets: Vec<WidgetId>) {
        let mut u = UPDATES_SV.write();
        u.flags.insert(UpdateFlags::UPDATE);
        for id in targets {
            u.update_widgets.search_widget(id);
        }
    }

    /// Schedules an update that only affects the app extensions.
    ///
    /// This is the equivalent of calling [`update`] with a `None`.
    ///
    /// [`update`]: Self::update
    pub fn update_ext(&self) -> &Self {
        UpdatesTrace::log_update();
        self.update_ext_internal();
        self
    }
    pub(crate) fn update_ext_internal(&self) {
        UPDATES_SV.write().flag_update(UpdateFlags::UPDATE);
    }

    /// Schedules a layout update that will affect all app extensions.
    ///
    /// Note that you must use [`WIDGET.layout`] to request a layout update for an widget.
    ///
    /// [`WIDGET.layout`]: WIDGET::layout
    pub fn layout(&self) -> &Self {
        UpdatesTrace::log_layout();
        self.layout_internal();
        self
    }
    pub(crate) fn layout_internal(&self) {
        UPDATES_SV.write().flag_update(UpdateFlags::LAYOUT);
    }

    /// Schedules a render update that will affect all app extensions.
    ///
    /// Note that you must use [`WIDGET.render`] or [`WIDGET.render_update`] to request a render update for an widget.
    ///
    /// [`WIDGET.render`]: WIDGET::render
    /// [`WIDGET.render_update`]: WIDGET::render_update
    pub fn render(&self) -> &Self {
        self.render_internal();
        self
    }
    pub(crate) fn render_internal(&self) {
        UPDATES_SV.write().flag_update(UpdateFlags::RENDER);
    }

    /// Schedule an *once* handler to run when these updates are applied.
    ///
    /// The callback is any of the *once* [`AppHandler`], including async handlers. If the handler is async and does not finish in
    /// one call it is scheduled to update in *preview* updates.
    pub fn run<H: AppHandler<UpdateArgs>>(&self, handler: H) -> OnUpdateHandle {
        let mut u = UPDATES_SV.write();
        u.flag_update(UpdateFlags::UPDATE);
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
        let mut handlers = mem::take(UPDATES_SV.write().pre_handlers.get_mut());
        Self::retain_updates(&mut handlers);

        let mut u = UPDATES_SV.write();
        handlers.append(u.pre_handlers.get_mut());
        *u.pre_handlers.get_mut() = handlers;
    }

    pub(crate) fn on_updates(&self) {
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

    pub(super) fn take_updates(&self) -> (bool, WidgetUpdates, bool, bool) {
        let mut u = UPDATES_SV.write();
        let update = u.flags.contains(UpdateFlags::UPDATE);
        let layout = u.flags.contains(UpdateFlags::LAYOUT);
        let render = u.flags.contains(UpdateFlags::RENDER);
        u.flags = UpdateFlags::empty();
        (
            update,
            WidgetUpdates {
                delivery_list: mem::take(&mut u.update_widgets),
            },
            layout,
            render,
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

    /// Find all targets.
    ///
    /// This must be called before the first window visit, see [`UpdateDeliveryList::fulfill_search`] for details.
    pub fn fulfill_search<'a, 'b>(&'a mut self, windows: impl Iterator<Item = &'b WidgetInfoTree>) {
        self.delivery_list.fulfill_search(windows)
    }

    /// Calls `handle` if the event targets the current [`WINDOW`].
    pub fn with_window<H, R>(&mut self, handle: H) -> Option<R>
    where
        H: FnOnce(&mut Self) -> R,
    {
        if self.delivery_list.enter_window(WINDOW.id()) {
            Some(handle(self))
        } else {
            None
        }
    }

    /// Calls `handle` if the event targets the current [`WIDGET`].
    pub fn with_widget<H, R>(&mut self, handle: H) -> Option<R>
    where
        H: FnOnce(&mut Self) -> R,
    {
        if self.delivery_list.enter_widget(WIDGET.id()) {
            Some(handle(self))
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: WidgetUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Represents all the widgets and windows on route to an update target.
pub struct UpdateDeliveryList {
    subscribers: Box<dyn UpdateSubscribers>,

    windows: LinearSet<WindowId>,
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
            windows: LinearSet::default(),
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

    /// Insert the ancestors of `wgt` and `wgt` up-to the inner most that is included in the subscribers.
    pub fn insert_wgt(&mut self, wgt: WidgetInfo) {
        let mut any = false;
        for w in wgt.self_and_ancestors() {
            if any || self.subscribers.contains(w.widget_id()) {
                any = true;
                self.widgets.insert(w.widget_id());
            }
        }
        if any {
            self.windows.insert(wgt.tree().window_id());
        }
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
    pub fn has_pending_search(&self) -> bool {
        !self.search.is_empty()
    }

    /// Search all pending widgets in all `windows`, all search items are cleared, even if not found.
    pub fn fulfill_search<'a, 'b>(&'a mut self, windows: impl Iterator<Item = &'b WidgetInfoTree>) {
        for window in windows {
            self.search.retain(|w| {
                if let Some(w) = window.get(*w) {
                    for w in w.self_and_ancestors() {
                        self.widgets.insert(w.widget_id());
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

    /// Returns `true` if the window is on the list.
    ///
    /// The window is removed from the list.
    pub fn enter_window(&mut self, window_id: WindowId) -> bool {
        self.windows.remove(&window_id)
    }

    /// Returns `true` if the widget is on the list.
    ///
    /// The widget is removed from the list.
    pub fn enter_widget(&mut self, widget_id: WidgetId) -> bool {
        self.widgets.remove(&widget_id)
    }

    /// Returns `true` if has entered all widgets on the list.
    pub fn is_done(&self) -> bool {
        self.widgets.is_empty() && self.search.is_empty()
    }

    /// Copy windows, widgets and search from `other`, trusting that all values are allowed.
    fn extend_unchecked(&mut self, other: UpdateDeliveryList) {
        self.windows.extend(other.windows);
        self.widgets.extend(other.widgets);
        self.search.extend(other.search)
    }

    pub(crate) fn clear(&mut self) {
        self.widgets.clear();
        self.windows.clear();
        self.search.clear();
    }

    /// Windows in the delivery list.
    ///
    /// Note that each window that is visited is removed from the list.
    pub fn windows(&self) -> &LinearSet<WindowId> {
        &self.windows
    }

    /// Found widgets in the delivery list, can be targets of ancestors of targets.
    ///
    /// Note that each widget that is visited is removed from the list.
    pub fn widgets(&self) -> &IdSet<WidgetId> {
        &self.widgets
    }

    /// Not found target widgets.
    ///
    /// Each window searches for these widgets and adds then to the [`widgets`] list.
    ///
    /// [`widgets`]: Self::widgets
    pub fn search_widgets(&self) -> &IdSet<WidgetId> {
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
    /// When this is `true`, [`update`](Self::update) may contain widgets, if not then only
    /// app extensions must update.
    pub update: bool,

    /// Update targets.
    ///
    /// When this is not empty [`update`](Self::update) is `true`.
    pub update_widgets: WidgetUpdates,

    /// Layout requested.
    pub layout: bool,

    /// Full frame or frame update requested.
    pub render: bool,
}
impl ContextUpdates {
    /// If has events, update, layout or render was requested.
    pub fn has_updates(&self) -> bool {
        self.update || self.layout || self.render
    }
}
impl std::ops::BitOrAssign for ContextUpdates {
    fn bitor_assign(&mut self, rhs: Self) {
        self.events.extend(rhs.events);
        self.update |= rhs.update;
        self.update_widgets.extend(rhs.update_widgets);
        self.layout |= rhs.layout;
        self.render |= rhs.render;
    }
}
impl std::ops::BitOr for ContextUpdates {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self {
        self |= rhs;
        self
    }
}

/// Constrains for inline measure.
///
/// See [`InlineConstrains`] for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct InlineConstrainsMeasure {
    /// Reserved space on the first row.
    pub first_max: Px,
    /// Current height of the row in the parent. If the widget wraps and defines the first
    /// row in *this* parent's row, the `mid_clear` value will be the extra space needed to clear
    /// this minimum or zero if the first how is taller. The widget must use this value to estimate the `mid_clear`
    /// value and include it in the overall measured height of the widget.
    pub mid_clear_min: Px,
}

/// Constrains for inline layout.
///
/// See [`InlineConstrains`] for more details.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct InlineConstrainsLayout {
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

/// Constrains for inline measure or layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InlineConstrains {
    /// Constrains for the measure pass.
    Measure(InlineConstrainsMeasure),
    /// Constrains the layout pass.
    Layout(InlineConstrainsLayout),
}
impl InlineConstrains {
    /// Get the `Measure` data or default.
    pub fn measure(self) -> InlineConstrainsMeasure {
        match self {
            InlineConstrains::Measure(m) => m,
            InlineConstrains::Layout(_) => Default::default(),
        }
    }

    /// Get the `Layout` data or default.
    pub fn layout(self) -> InlineConstrainsLayout {
        match self {
            InlineConstrains::Layout(m) => m,
            InlineConstrains::Measure(_) => Default::default(),
        }
    }
}

/// Layout metrics snapshot.
///
/// A snapshot can be taken using the [`LayoutMetrics::snapshot`], you can also
/// get the metrics used during the last layout of a widget using the [`WidgetBoundsInfo::metrics`] method.
#[derive(Clone, Debug)]
pub struct LayoutMetricsSnapshot {
    /// The [`constrains`].
    ///
    /// [`constrains`]: LayoutMetrics::constrains
    pub constrains: PxConstrains2d,

    /// The [`inline_constrains`].
    ///
    /// [`inline_constrains`]: LayoutMetrics::inline_constrains
    pub inline_constrains: Option<InlineConstrains>,

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
    pub screen_ppi: f32,

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
        (!mask.contains(LayoutMask::CONSTRAINS)
            || (self.constrains == other.constrains && self.inline_constrains == other.inline_constrains))
            && (!mask.contains(LayoutMask::FONT_SIZE) || self.font_size == other.font_size)
            && (!mask.contains(LayoutMask::ROOT_FONT_SIZE) || self.root_font_size == other.root_font_size)
            && (!mask.contains(LayoutMask::SCALE_FACTOR) || self.scale_factor == other.scale_factor)
            && (!mask.contains(LayoutMask::VIEWPORT) || self.viewport == other.viewport)
            && (!mask.contains(LayoutMask::SCREEN_PPI) || about_eq(self.screen_ppi, other.screen_ppi, 0.0001))
            && (!mask.contains(LayoutMask::DIRECTION) || self.direction == other.direction)
            && (!mask.contains(LayoutMask::LEFTOVER) || self.leftover == other.leftover)
    }
}
impl PartialEq for LayoutMetricsSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.constrains == other.constrains
            && self.inline_constrains == other.inline_constrains
            && self.font_size == other.font_size
            && self.root_font_size == other.root_font_size
            && self.scale_factor == other.scale_factor
            && self.viewport == other.viewport
            && about_eq(self.screen_ppi, other.screen_ppi, 0.0001)
    }
}
impl std::hash::Hash for LayoutMetricsSnapshot {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.constrains.hash(state);
        self.inline_constrains.hash(state);
        self.font_size.hash(state);
        self.root_font_size.hash(state);
        self.scale_factor.hash(state);
        self.viewport.hash(state);
        about_eq_hash(self.screen_ppi, 0.0001, state);
    }
}

/// Layout metrics in a [`LAYOUT`] context.
#[derive(Debug, Clone)]
pub struct LayoutMetrics {
    use_mask: Arc<Mutex<LayoutMask>>,

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
            use_mask: Arc::new(Mutex::new(LayoutMask::NONE)),
            s: LayoutMetricsSnapshot {
                constrains: PxConstrains2d::new_fill_size(viewport),
                inline_constrains: None,
                font_size,
                root_font_size: font_size,
                scale_factor,
                viewport,
                screen_ppi: 96.0,
                direction: LayoutDirection::default(),
                leftover: euclid::size2(None, None),
            },
        }
    }

    /// What metrics where requested so far in the widget or descendants.
    pub fn metrics_used(&self) -> LayoutMask {
        *self.use_mask.lock()
    }

    /// Register that the node layout depends on these contextual values.
    ///
    /// Note that the value methods already register use when they are used.
    pub fn register_use(&self, mask: LayoutMask) {
        let mut m = self.use_mask.lock();
        *m |= mask;
    }

    /// Get metrics without registering use.
    ///
    /// The `req` closure is called to get a value, then the [`metrics_used`] is undone to the previous state.
    ///
    /// [`metrics_used`]: Self::metrics_used
    pub fn peek<R>(&self, req: impl FnOnce(&Self) -> R) -> R {
        let m = *self.use_mask.lock();
        let r = req(self);
        *self.use_mask.lock() = m;
        r
    }

    /// Current size constrains.
    pub fn constrains(&self) -> PxConstrains2d {
        self.register_use(LayoutMask::CONSTRAINS);
        self.s.constrains
    }

    /// Current inline constrains.
    ///
    /// Only present if the parent widget supports inline.
    pub fn inline_constrains(&self) -> Option<InlineConstrains> {
        self.register_use(LayoutMask::CONSTRAINS);
        self.s.inline_constrains.clone()
    }

    /// Gets the inline or text flow direction.
    pub fn direction(&self) -> LayoutDirection {
        self.register_use(LayoutMask::DIRECTION);
        self.s.direction
    }

    /// Current computed font size.
    pub fn font_size(&self) -> Px {
        self.register_use(LayoutMask::FONT_SIZE);
        self.s.font_size
    }

    /// Computed font size at the root widget.
    pub fn root_font_size(&self) -> Px {
        self.register_use(LayoutMask::ROOT_FONT_SIZE);
        self.s.root_font_size
    }

    /// Pixel scale factor.
    pub fn scale_factor(&self) -> Factor {
        self.register_use(LayoutMask::SCALE_FACTOR);
        self.s.scale_factor
    }

    /// Computed size of the nearest viewport ancestor.
    ///
    /// This is usually the window content area size, but can be the scroll viewport size or any other
    /// value depending on the implementation of the context widgets.
    pub fn viewport(&self) -> PxSize {
        self.register_use(LayoutMask::VIEWPORT);
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
    pub fn screen_ppi(&self) -> f32 {
        self.s.screen_ppi
    }

    /// Computed leftover length for the widget, given the [`Length::Leftover`] value it communicated to the parent.
    pub fn leftover(&self) -> euclid::Size2D<Option<Px>, ()> {
        self.register_use(LayoutMask::LEFTOVER);
        self.s.leftover
    }

    /// Sets the [`constrains`] to the value returned by `constrains`. The closure input is the current constrains.
    ///
    /// [`constrains`]: Self::constrains
    pub fn with_constrains(mut self, constrains: impl FnOnce(PxConstrains2d) -> PxConstrains2d) -> Self {
        self.s.constrains = constrains(self.s.constrains);
        self
    }

    /// Set the [`inline_constrains`].
    ///
    /// [`inline_constrains`]: Self::inline_constrains
    pub fn with_inline_constrains(mut self, inline_constrains: Option<InlineConstrains>) -> Self {
        self.s.inline_constrains = inline_constrains;
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
    pub fn with_screen_ppi(mut self, screen_ppi: f32) -> Self {
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

    pub(crate) fn enter_widget_ctx(&self) -> LayoutMask {
        mem::replace(&mut *self.use_mask.lock(), LayoutMask::NONE)
    }

    pub(crate) fn exit_widget_ctx(&self, parent_use: LayoutMask) -> LayoutMask {
        let mut use_mask = self.use_mask.lock();
        let wgt_use = *use_mask;
        *use_mask = parent_use | wgt_use;
        wgt_use
    }
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
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
impl From<unic_langid::CharacterDirection> for LayoutDirection {
    fn from(value: unic_langid::CharacterDirection) -> Self {
        match value {
            unic_langid::CharacterDirection::LTR => Self::LTR,
            unic_langid::CharacterDirection::RTL => Self::RTL,
        }
    }
}
impl From<unicode_bidi::Level> for LayoutDirection {
    fn from(value: unicode_bidi::Level) -> Self {
        if value.is_ltr() {
            Self::LTR
        } else {
            Self::RTL
        }
    }
}
impl TryFrom<harfbuzz_rs::Direction> for LayoutDirection {
    type Error = harfbuzz_rs::Direction;

    fn try_from(value: harfbuzz_rs::Direction) -> Result<Self, Self::Error> {
        match value {
            harfbuzz_rs::Direction::Ltr => Ok(Self::LTR),
            harfbuzz_rs::Direction::Rtl => Ok(Self::RTL),
            other => Err(other),
        }
    }
}
impl From<LayoutDirection> for unic_langid::CharacterDirection {
    fn from(value: LayoutDirection) -> Self {
        match value {
            LayoutDirection::LTR => Self::LTR,
            LayoutDirection::RTL => Self::RTL,
        }
    }
}
impl From<LayoutDirection> for unicode_bidi::Level {
    fn from(value: LayoutDirection) -> Self {
        match value {
            LayoutDirection::LTR => Self::ltr(),
            LayoutDirection::RTL => Self::rtl(),
        }
    }
}
impl From<LayoutDirection> for harfbuzz_rs::Direction {
    fn from(value: LayoutDirection) -> Self {
        match value {
            LayoutDirection::LTR => Self::Ltr,
            LayoutDirection::RTL => Self::Rtl,
        }
    }
}
