//! New static contexts.

use std::{fmt, future::Future, mem, task::Waker};

use linear_map::set::LinearSet;
use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex};

use crate::{
    app::{AppEventSender, LoopTimer},
    app_local,
    context::{
        state_map, AppContext, InlineConstrains, LayoutDirection, LayoutMetrics, OwnedStateMap, StateMapMut, StateMapRef, UpdatesTrace,
        WidgetContext, WindowContext,
    },
    context_local,
    crate_util::{Handle, HandleOwner, IdSet, WeakHandle},
    event::{Event, EventArgs, EventHandle, EventHandles, EventUpdate, EVENTS, EVENTS_SV},
    handler::{AppHandler, AppHandlerArgs, AppWeakHandle},
    task::ui::WidgetTask,
    timer::TIMERS_SV,
    units::*,
    var::{AnyVar, VarHandle, VarHandles, VARS},
    widget_info::{WidgetBorderInfo, WidgetBoundsInfo, WidgetContextInfo, WidgetInfo, WidgetInfoTree, WidgetPath},
    widget_instance::WidgetId,
    window::WindowId,
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

/// Defines the backing data of [`WINDOW`].
///
/// Each window owns this data and calls [`WINDOW.with_context`] to delegate to it's child node.
pub struct WindowCtx {
    id: WindowId,
    state: OwnedStateMap<state_map::Window>,
    widget_tree: Option<WidgetInfoTree>,
}
impl WindowCtx {
    /// New window context.
    pub fn new(id: WindowId) -> Self {
        Self {
            id,
            state: OwnedStateMap::default(),
            widget_tree: None,
        }
    }

    /// Initializes the context.
    ///
    /// Window contexts are partially available in the window new closure, but values like the `widget_tree` are
    /// only available after init.
    pub fn init(&mut self, widget_tree: WidgetInfoTree) {
        self.widget_tree = Some(widget_tree);
    }
}

/// Defines the backing data of [`WIDGET`].
///
/// Each widget owns this data and calls [`WIDGET.with_context`] to delegate to it's child node.
pub struct WidgetCtx {
    id: WidgetId,
    flags: UpdateFlags,
    state: OwnedStateMap<state_map::Widget>,
    var_handles: VarHandles,
    event_handles: EventHandles,
    info: WidgetContextInfo,
}
impl WidgetCtx {
    /// New widget context.
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            flags: UpdateFlags::empty(),
            state: OwnedStateMap::default(),
            var_handles: VarHandles::dummy(),
            event_handles: EventHandles::dummy(),
            info: WidgetContextInfo::default(),
        }
    }

    /// Clears all state and handles.
    pub fn deinit(&mut self) {
        self.state.clear();
        self.var_handles.clear();
        self.event_handles.clear();
        self.flags = UpdateFlags::empty();
    }

    fn take_flag(&mut self, flag: UpdateFlags) -> bool {
        let c = self.flags.contains(flag);
        self.flags.remove(flag);
        c
    }

    /// Returns `true` once if a info rebuild was requested in a previous [`WIDGET.with_context`] call.
    ///
    /// Child nodes can request updates using [`WIDGET.info`].
    pub fn take_info(&mut self) -> bool {
        self.take_flag(UpdateFlags::INFO)
    }

    /// Returns `true` once if a re-layout was requested in a previous [`WIDGET.with_context`] call.
    ///
    /// Child nodes can request updates using [`WIDGET.layout`].
    pub fn take_layout(&mut self) -> bool {
        self.take_flag(UpdateFlags::LAYOUT)
    }

    /// Returns `true` once if a re-render was requested in a previous [`WIDGET.with_context`] call.
    ///
    /// Child nodes can request updates using [`WIDGET.render`].
    ///
    /// Removes render-update requests and must be called before [`take_render_update`].
    ///
    /// [`take_render_update`]: Self::take_render_update
    pub fn take_render(&mut self) -> bool {
        let c = self.flags.contains(UpdateFlags::RENDER);
        if c {
            self.flags.remove(UpdateFlags::RENDER);
            self.flags.remove(UpdateFlags::RENDER_UPDATE);
        }
        c
    }

    /// Returns `true` once if a re-render was requested in a previous [`WIDGET.with_context`] call.
    ///
    /// Child nodes can request updates using [`WIDGET.render_update`].
    ///
    /// Logs an error if a full render is requested, must be called after [`take_render`].
    ///
    /// [`take_render`]: Self::take_render
    pub fn take_render_update(&mut self) -> bool {
        if self.flags.contains(UpdateFlags::RENDER) {
            tracing::error!("widget `{:?}` called `take_render_update` before `take_render`", self.id);
        }
        self.take_flag(UpdateFlags::RENDER_UPDATE)
    }

    /// Returns `true` if an [`WIDGET.reinit`] request was made.
    ///
    /// Unlike other requests, the widget re-init immediately.
    pub fn take_reinit(&mut self) -> bool {
        self.take_flag(UpdateFlags::REINIT)
    }
}

context_local! {
    static WINDOW_CTX: Option<WindowCtx> = None;
    static WIDGET_CTX: Option<WidgetCtx> = None;
}

/// Current context window.
pub struct WINDOW;
impl WINDOW {
    /// Calls `f` while the window is set to `ctx`.
    ///
    /// The `ctx` must be `Some(_)`, it will be moved to the [`WINDOW`] storage and back to `ctx` after `f` returns.
    pub fn with_context<R>(&self, ctx: &mut Option<WindowCtx>, f: impl FnOnce() -> R) -> R {
        assert!(ctx.is_some());
        WINDOW_CTX.with_context_opt(ctx, f)
    }

    /// Calls `f` while no window is available in the context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        WINDOW_CTX.with_context_opt(&mut None, f)
    }

    /// Returns `true` if called inside a window.
    pub fn is_in_window(&self) -> bool {
        WINDOW_CTX.read().is_some()
    }

    #[track_caller]
    fn req(&self) -> MappedRwLockReadGuard<'static, WindowCtx> {
        MappedRwLockReadGuard::map(WINDOW_CTX.read(), |c| c.as_ref().expect("no window in context"))
    }

    #[track_caller]
    fn req_mut(&self) -> MappedRwLockWriteGuard<'static, WindowCtx> {
        MappedRwLockWriteGuard::map(WINDOW_CTX.write(), |c| c.as_mut().expect("no window in context"))
    }

    /// Get the widget ID, if called inside a window.
    pub fn try_id(&self) -> Option<WindowId> {
        WINDOW_CTX.read().as_ref().map(|c| c.id)
    }

    /// Get the widget ID if called inside a widget, or panic.
    pub fn id(&self) -> WindowId {
        self.req().id
    }

    /// Gets the window info tree.
    ///
    /// Returns `None` if the window is not inited, panics if called outside of a window or window init closure.
    pub fn widget_tree(&self) -> Option<WidgetInfoTree> {
        self.req().widget_tree.clone()
    }

    /// Calls `f` with a read lock on the current window state map.
    ///
    /// Note that this locks the entire [`WINDOW`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state<R>(&self, f: impl FnOnce(StateMapRef<state_map::Window>) -> R) -> R {
        f(self.req().state.borrow())
    }

    /// Calls `f` with a write lock on the current window state map.
    ///
    /// Note that this locks the entire [`WINDOW`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state_mut<R>(&self, f: impl FnOnce(StateMapMut<state_map::Window>) -> R) -> R {
        f(self.req_mut().state.borrow_mut())
    }
}

/// Current context widget.
pub struct WIDGET;
impl WIDGET {
    /// Calls `f` while the widget is set to `ctx`.
    ///
    /// The `ctx` must be `Some(_)`, it will be moved to the [`WIDGET`] storage and back to `ctx` after `f` returns.
    pub fn with_context<R>(&self, ctx: &mut Option<WidgetCtx>, f: impl FnOnce() -> R) -> R {
        assert!(ctx.is_some());

        let r = WIDGET_CTX.with_context_opt(ctx, f);

        let ctx = ctx.as_mut().unwrap();

        if let Some(parent) = &mut *WIDGET_CTX.write() {
            if ctx.take_flag(UpdateFlags::UPDATE) {
                parent.flags.insert(UpdateFlags::UPDATE);
            }
            if ctx.flags.contains(UpdateFlags::INFO) {
                parent.flags.insert(UpdateFlags::INFO);
            }
            if ctx.flags.contains(UpdateFlags::LAYOUT) {
                parent.flags.insert(UpdateFlags::LAYOUT);
            }
            if ctx.flags.contains(UpdateFlags::RENDER) {
                parent.flags.insert(UpdateFlags::RENDER);
            } else if ctx.flags.contains(UpdateFlags::RENDER_UPDATE) {
                parent.flags.insert(UpdateFlags::RENDER_UPDATE);
            }
        }
        r
    }

    /// Calls `f` while no widget is available in the context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        WIDGET_CTX.with_context_opt(&mut None, f)
    }

    /// Calls `f` with an override target for var and event subscription handles.
    pub fn with_handles<R>(&self, var_handles: &mut VarHandles, event_handles: &mut EventHandles, f: impl FnOnce() -> R) -> R {
        {
            let mut w = self.req_mut();
            mem::swap(&mut w.var_handles, var_handles);
            mem::swap(&mut w.event_handles, event_handles);
        }
        let r = f();
        {
            let mut w = self.req_mut();
            mem::swap(&mut w.var_handles, var_handles);
            mem::swap(&mut w.event_handles, event_handles);
        }
        r
    }

    /// Returns `true` if called inside a widget.
    pub fn is_in_widget(&self) -> bool {
        WIDGET_CTX.read().is_some()
    }

    #[track_caller]
    fn req(&self) -> MappedRwLockReadGuard<'static, WidgetCtx> {
        MappedRwLockReadGuard::map(WIDGET_CTX.read(), |c| c.as_ref().expect("no widget in context"))
    }

    #[track_caller]
    fn req_mut(&self) -> MappedRwLockWriteGuard<'static, WidgetCtx> {
        MappedRwLockWriteGuard::map(WIDGET_CTX.write(), |c| c.as_mut().expect("no widget in context"))
    }

    /// Get the widget ID, if called inside a widget.
    pub fn try_id(&self) -> Option<WidgetId> {
        WIDGET_CTX.read().as_ref().map(|c| c.id)
    }

    /// Get the widget ID if called inside a widget, or panic.
    pub fn id(&self) -> WidgetId {
        self.req().id
    }

    /// Schedule an update for the current widget.
    ///
    /// After the current update cycle the app-extensions, parent window and widgets will update again.
    pub fn update(&self) -> &Self {
        let mut ctx = self.req_mut();
        if !ctx.flags.contains(UpdateFlags::UPDATE) {
            ctx.flags.insert(UpdateFlags::UPDATE);
            UPDATES.update(ctx.id);
        }
        self
    }

    /// Schedule an info rebuild for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-build the info tree.
    pub fn info(&self) -> &Self {
        let mut ctx = self.req_mut();
        if !ctx.flags.contains(UpdateFlags::LAYOUT) {
            ctx.flags.insert(UpdateFlags::LAYOUT);
            UPDATES.update_ext_internal();
        }
        self
    }

    /// Schedule a re-layout for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-layout.
    pub fn layout(&self) -> &Self {
        let mut ctx = self.req_mut();
        if !ctx.flags.contains(UpdateFlags::LAYOUT) {
            ctx.flags.insert(UpdateFlags::LAYOUT);
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
        let mut ctx = self.req_mut();
        if !ctx.flags.contains(UpdateFlags::RENDER) {
            ctx.flags.insert(UpdateFlags::RENDER);
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
        let mut ctx = self.req_mut();
        if !ctx.flags.contains(UpdateFlags::RENDER_UPDATE) {
            ctx.flags.insert(UpdateFlags::RENDER_UPDATE);
            UPDATES.render();
        }
        self
    }

    /// Flags the widget to re-init after the current update returns.
    ///
    /// The widget will de-init and init as soon as it sees this request.
    pub fn reinit(&self) {
        self.req_mut().flags.insert(UpdateFlags::REINIT);
    }

    /// Calls `f` with a read lock on the current widget state map.
    ///
    /// Note that this locks the entire [`WIDGET`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state<R>(&self, f: impl FnOnce(StateMapRef<state_map::Widget>) -> R) -> R {
        f(self.req().state.borrow())
    }

    /// Calls `f` with a write lock on the current widget state map.
    ///
    /// Note that this locks the entire [`WIDGET`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state_mut<R>(&self, f: impl FnOnce(StateMapMut<state_map::Widget>) -> R) -> R {
        f(self.req_mut().state.borrow_mut())
    }

    /// Subscribe to receive updates when the `var` changes.
    pub fn sub_var(&self, var: &impl AnyVar) -> &Self {
        let mut w = self.req_mut();
        let s = var.subscribe(w.id);
        w.var_handles.push(s);
        self
    }

    /// Subscribe to receive events from `event`.
    pub fn sub_event<A: EventArgs>(&self, event: &Event<A>) -> &Self {
        let mut w = self.req_mut();
        let s = event.subscribe(w.id);
        w.event_handles.push(s);
        self
    }

    /// Hold the `handle`.
    pub fn push_event_handle(&self, handle: EventHandle) {
        self.req_mut().event_handles.push(handle);
    }

    /// Hold the `handle`.
    pub fn push_var_handle(&self, handle: VarHandle) {
        self.req_mut().var_handles.push(handle);
    }

    /// Widget bounds, updated every layout.
    pub fn bounds(&self) -> WidgetBoundsInfo {
        self.req().info.bounds.clone()
    }

    /// Widget border, updated every layout.
    pub fn border(&self) -> WidgetBorderInfo {
        self.req().info.border.clone()
    }

    /// Create an async task that will update in the full context of the widget.
    pub fn async_task<R, F, T>(&self, task: T) -> WidgetTask<R>
    where
        R: 'static,
        F: Future<Output = R> + Send + 'static,
        T: FnOnce() -> F,
    {
        // WidgetTask::new(ctx, task)
        todo!()
    }
}

context_local! {
    static LAYOUT_CTX: Option<LayoutCtx> = None;
}

struct LayoutCtx {
    metrics: LayoutMetrics,
}

/// Current layout context.
///
/// Only available in measure and layout methods.
pub struct LAYOUT;
impl LAYOUT {
    #[track_caller]
    fn req(&self) -> MappedRwLockReadGuard<'static, LayoutCtx> {
        MappedRwLockReadGuard::map(LAYOUT_CTX.read(), |c| c.as_ref().expect("not in layout context"))
    }

    #[track_caller]
    fn req_mut(&self) -> MappedRwLockWriteGuard<'static, LayoutCtx> {
        MappedRwLockWriteGuard::map(LAYOUT_CTX.write(), |c| c.as_mut().expect("not in layout context"))
    }

    /// Calls `f` in a new layout context.
    pub fn with_context<R>(font_size: Px, scale_factor: Factor, screen_ppi: f32, viewport: PxSize, f: impl FnOnce() -> R) -> R {
        let mut ctx = Some(LayoutCtx {
            metrics: LayoutMetrics::new(scale_factor, viewport, font_size).with_screen_ppi(screen_ppi),
        });
        LAYOUT_CTX.with_context_opt(&mut ctx, f)
    }

    /// Calls `f` without a layout context.
    pub fn with_no_context<R>(f: impl FnOnce() -> R) -> R {
        LAYOUT_CTX.with_context_opt(&mut None, f)
    }

    /// Gets the context metrics.
    pub fn metrics(&self) -> LayoutMetrics {
        self.req().metrics.clone()
    }

    /// Calls `metrics` to generate new metrics that are used during the call to `f`.
    pub fn with_metrics<R>(&self, metrics: impl FnOnce(LayoutMetrics) -> LayoutMetrics, f: impl FnOnce() -> R) -> R {
        let new = metrics(self.metrics());
        let prev = mem::replace(&mut self.req_mut().metrics, new);

        let r = f();

        self.req_mut().metrics = prev;

        r
    }

    /// Current size constrains.
    pub fn constrains(&self) -> PxConstrains2d {
        self.req().metrics.constrains()
    }

    /// Calls `constrains` to generate new constrains that are used during the call to  `f`.
    pub fn with_constrains<R>(&self, constrains: impl FnOnce(PxConstrains2d) -> PxConstrains2d, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_constrains(constrains), f)
    }

    /// Current inline constrains.
    pub fn inline_constrains(&self) -> Option<InlineConstrains> {
        self.req().metrics.inline_constrains()
    }

    /// Calls `f` with `inline_constrains` in the context.
    pub fn with_inline_constrains<R>(&self, inline_constrains: Option<InlineConstrains>, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_inline_constrains(inline_constrains), f)
    }

    /// Current font size.
    pub fn font_size(&self) -> Px {
        self.req().metrics.font_size()
    }

    /// Calls `f` with `font_size` in the context.
    pub fn with_font_size<R>(&self, font_size: Px, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_font_size(font_size), f)
    }

    /// Current viewport size.
    pub fn viewport(&self) -> PxSize {
        self.req().metrics.viewport()
    }

    /// Calls `f` with `viewport` in the context.
    pub fn with_viewport<R>(&self, viewport: PxSize, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_viewport(viewport), f)
    }

    /// Current scale factor.
    pub fn scale_factor(&self) -> Factor {
        self.req().metrics.scale_factor()
    }

    /// Calls `f` with `scale_factor` in the context.
    pub fn with_scale_factor<R>(&self, scale_factor: Factor, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_scale_factor(scale_factor), f)
    }

    /// Current screen PPI.
    pub fn screen_ppi(&self) -> f32 {
        self.req().metrics.screen_ppi()
    }

    /// Calls `f` with `screen_ppi` in the context.
    pub fn with_screen_ppi<R>(&self, screen_ppi: f32, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_screen_ppi(screen_ppi), f)
    }

    /// Current layout direction.
    pub fn direction(&self) -> LayoutDirection {
        self.req().metrics.direction()
    }

    /// Calls `f` with `direction` in the context.
    pub fn with_direction<R>(&self, direction: LayoutDirection, f: impl FnOnce() -> R) -> R {
        self.with_metrics(|m| m.with_direction(direction), f)
    }

    /// Context leftover length for the widget, given the [`Length::Leftover`] value it communicated to the parent.
    ///
    /// [`leftover_count`]: Self::leftover_count
    pub fn leftover(&self) -> euclid::Size2D<Option<Px>, ()> {
        self.req().metrics.leftover()
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
}
impl UpdatesService {
    fn new() -> Self {
        Self {
            event_sender: None,
            flags: UpdateFlags::empty(),
            update_widgets: UpdateDeliveryList::new_any(),

            pre_handlers: Mutex::new(vec![]),
            pos_handlers: Mutex::new(vec![]),
        }
    }
}

/// Update pump and schedule service.
pub struct UPDATES;
impl UPDATES {
    pub(crate) fn init(&self, event_sender: AppEventSender) {
        UPDATES_SV.write().event_sender = Some(event_sender.clone());
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
        u.flags.insert(UpdateFlags::UPDATE);
        if let Some(id) = target {
            u.update_widgets.search_widget(id);
        }
        u.event_sender.as_ref().unwrap().send_ext_update();
    }

    pub(crate) fn recv_update_internal(&mut self, targets: Vec<WidgetId>) {
        let mut u = UPDATES_SV.write();

        if !u.flags.contains(UpdateFlags::UPDATE) {
            u.flags.insert(UpdateFlags::UPDATE);
            u.event_sender.as_ref().unwrap().send_ext_update();
        }

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
        let mut u = UPDATES_SV.write();

        if !u.flags.contains(UpdateFlags::UPDATE) {
            u.flags.insert(UpdateFlags::UPDATE);
            u.event_sender.as_ref().unwrap().send_ext_update();
        }
    }

    /// Schedules a layout update that will affect all app extensions and widgets with invalidated layout.
    pub fn layout(&self) -> &Self {
        UpdatesTrace::log_layout();
        self.layout_internal();
        self
    }
    pub(crate) fn layout_internal(&self) {
        UPDATES_SV.write().flags.insert(UpdateFlags::LAYOUT);
    }

    /// Schedules a render update that will affect all app extensions and widgets with invalidated layout.
    pub fn render(&self) -> &Self {
        self.render_internal();
        self
    }
    pub(crate) fn render_internal(&self) {
        UPDATES_SV.write().flags.insert(UpdateFlags::RENDER);
    }

    /// Schedule an *once* handler to run when these updates are applied.
    ///
    /// The callback is any of the *once* [`AppHandler`], including async handlers. If the handler is async and does not finish in
    /// one call it is scheduled to update in *preview* updates.
    pub fn run<H: AppHandler<UpdateArgs>>(&self, handler: H) -> OnUpdateHandle {
        let mut u = UPDATES_SV.write();
        u.flags.insert(UpdateFlags::UPDATE); // in case this was called outside of an update.
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
        let r = Self::push_handler(&mut *u.pre_handlers.lock(), true, handler, false);
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
        let r = Self::push_handler(&mut *u.pos_handlers.lock(), false, handler, false);
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
            handler: Box::new(move |ctx, args, handle| {
                let handler_args = AppHandlerArgs { handle, is_preview };
                handler.event(ctx, args, &handler_args);
                if force_once {
                    handler_args.handle.unsubscribe();
                }
            }),
        });
        handle
    }

    pub(crate) fn on_pre_updates(&self, ctx: &mut AppContext) {
        let mut handlers = mem::take(UPDATES_SV.write().pre_handlers.get_mut());
        Self::retain_updates(ctx, &mut handlers);

        let mut u = UPDATES_SV.write();
        handlers.append(u.pre_handlers.get_mut());
        *u.pre_handlers.get_mut() = handlers;
    }

    pub(crate) fn on_updates(&self, ctx: &mut AppContext) {
        let mut handlers = mem::take(UPDATES_SV.write().pos_handlers.get_mut());
        Self::retain_updates(ctx, &mut handlers);

        let mut u = UPDATES_SV.write();
        handlers.append(u.pos_handlers.get_mut());
        *u.pos_handlers.get_mut() = handlers;
    }

    fn retain_updates(ctx: &mut AppContext, handlers: &mut Vec<UpdateHandler>) {
        handlers.retain_mut(|e| {
            !e.handle.is_dropped() && {
                e.count = e.count.wrapping_add(1);
                (e.handler)(ctx, &UpdateArgs { count: e.count }, &e.handle.weak_handle());
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
    handler: Box<dyn FnMut(&mut AppContext, &UpdateArgs, &dyn AppWeakHandle) + Send>,
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

    /// Calls `handle` if the event targets the window.
    pub fn with_window<H, R>(&mut self, ctx: &mut WindowContext, handle: H) -> Option<R>
    where
        H: FnOnce(&mut WindowContext, &mut Self) -> R,
    {
        if self.delivery_list.enter_window(*ctx.window_id) {
            Some(handle(ctx, self))
        } else {
            None
        }
    }

    /// Calls `handle` if the event targets the widget.
    pub fn with_widget<H, R>(&mut self, ctx: &mut WidgetContext, handle: H) -> Option<R>
    where
        H: FnOnce(&mut WidgetContext, &mut Self) -> R,
    {
        if self.delivery_list.enter_widget(ctx.path.widget_id()) {
            Some(handle(ctx, self))
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
        self.widgets.is_empty()
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

/// Types to remove.
pub mod temp {
    use super::*;

    /// Info, Layout or render updates that where requested by the content of a window.
    ///
    /// Unlike the general updates, layout and render can be optimized to only apply if
    /// the window content requested it.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct InfoLayoutRenderUpdates {
        /// Info tree rebuild requested.
        ///
        /// Windows should call [`UiNode::info`] to rebuild the info tree as soon as they receive this flag.
        ///
        /// [`UiNode::info`]: crate::widget_instance::UiNode::info
        pub info: bool,

        /// Layout requested.
        pub layout: bool,
        /// Full frame or frame update requested.
        pub render: WindowRenderUpdate,
    }
    impl InfoLayoutRenderUpdates {
        /// No updates, this the default value.
        pub fn none() -> Self {
            Self::default()
        }

        /// Update layout and render frame.
        pub fn all() -> Self {
            InfoLayoutRenderUpdates {
                info: true,
                layout: true,
                render: WindowRenderUpdate::Render,
            }
        }

        /// Info tree rebuild and subscriptions only.
        pub fn info() -> Self {
            InfoLayoutRenderUpdates {
                info: true,
                layout: false,
                render: WindowRenderUpdate::None,
            }
        }

        /// Update layout only.
        pub fn layout() -> Self {
            InfoLayoutRenderUpdates {
                info: false,
                layout: true,
                render: WindowRenderUpdate::None,
            }
        }

        /// Update render only.
        pub fn render() -> Self {
            InfoLayoutRenderUpdates {
                info: false,
                layout: false,
                render: WindowRenderUpdate::Render,
            }
        }

        /// Update render-update only.
        pub fn render_update() -> Self {
            InfoLayoutRenderUpdates {
                info: false,
                layout: false,
                render: WindowRenderUpdate::RenderUpdate,
            }
        }

        /// Returns if `self` is not equal to [`none`].
        ///
        /// [`none`]: Self::none
        pub fn is_any(self) -> bool {
            self != Self::none()
        }

        /// Returns if `self` is equal to [`none`].
        ///
        /// [`none`]: Self::none
        pub fn is_none(self) -> bool {
            self == Self::none()
        }
    }
    impl std::ops::BitOrAssign for InfoLayoutRenderUpdates {
        fn bitor_assign(&mut self, rhs: Self) {
            self.info |= rhs.info;
            self.layout |= rhs.layout;
            self.render |= rhs.render;
        }
    }
    impl std::ops::BitOr for InfoLayoutRenderUpdates {
        type Output = Self;

        fn bitor(mut self, rhs: Self) -> Self {
            self |= rhs;
            self
        }
    }

    /// Kind of render updated requested by the content of a window.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WindowRenderUpdate {
        /// No render update requested.
        None,
        /// Full frame requested.
        Render,
        /// Only frame update requested.
        RenderUpdate,
    }
    impl WindowRenderUpdate {
        /// If full frame was requested.
        pub fn is_render(self) -> bool {
            matches!(self, Self::Render)
        }

        /// If only frame update was requested.
        pub fn is_render_update(self) -> bool {
            matches!(self, Self::RenderUpdate)
        }

        /// If no render was requested.
        pub fn is_none(self) -> bool {
            matches!(self, Self::None)
        }

        /// Returns a copy of `self` and replaces `self` with `None`
        pub fn take(&mut self) -> Self {
            mem::take(self)
        }
    }
    impl Default for WindowRenderUpdate {
        fn default() -> Self {
            WindowRenderUpdate::None
        }
    }
    impl std::ops::BitOrAssign for WindowRenderUpdate {
        fn bitor_assign(&mut self, rhs: Self) {
            use WindowRenderUpdate::*;
            *self = match (*self, rhs) {
                (Render, _) | (_, Render) => Render,
                (RenderUpdate, _) | (_, RenderUpdate) => RenderUpdate,
                _ => None,
            };
        }
    }
    impl std::ops::BitOr for WindowRenderUpdate {
        type Output = Self;

        fn bitor(mut self, rhs: Self) -> Self {
            self |= rhs;
            self
        }
    }
}
pub use temp::*;
