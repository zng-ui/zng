//! New static contexts.

use std::{future::Future, mem};

use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard};

use crate::{
    app::AppEventSender,
    app_local,
    context::{
        state_map, InlineConstrains, LayoutDirection, LayoutMetrics, OwnedStateMap, StateMapMut, StateMapRef, UpdateDeliveryList,
        UpdatesTrace,
    },
    context_local,
    event::{Event, EventArgs, EventHandle, EventHandles},
    task::ui::WidgetTask,
    units::*,
    var::{AnyVar, VarHandle, VarHandles},
    widget_info::{WidgetBorderInfo, WidgetBoundsInfo, WidgetContextInfo, WidgetInfoTree},
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
    /// Child nodes can request updates using [`WIDGET.rebuild_info`].
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
    pub fn rebuild_info(&self) -> &Self {
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
}
impl UpdatesService {
    fn new() -> Self {
        Self {
            event_sender: None,
            flags: UpdateFlags::empty(),
            update_widgets: UpdateDeliveryList::new_any(),
        }
    }

    pub(crate) fn init(&mut self, event_sender: AppEventSender) {
        self.event_sender = Some(event_sender);
    }
}

/// Update pump and schedule service.
pub struct UPDATES;
impl UPDATES {
    /// Create an [`AppEventSender`] that can be used to awake the app and send app events from threads outside of the app.
    pub fn sender(&self) -> AppEventSender {
        UPDATES_SV.read().event_sender.as_ref().unwrap().clone()
    }

    /// Schedules an update that affects the `target`.
    ///
    /// After the current update cycle ends a new update will happen that includes the `target` widget.
    pub fn update(&self, target: impl Into<Option<WidgetId>>) {
        UpdatesTrace::log_update();
        self.update_internal(target.into());
    }
    pub(crate) fn update_internal(&self, target: Option<WidgetId>) {
        let mut u = UPDATES_SV.write();
        u.flags.insert(UpdateFlags::UPDATE);
        if let Some(id) = target {
            u.update_widgets.search_widget(id);
        }
    }

    /// Schedules an update that only affects the app extensions.
    ///
    /// This is the equivalent of calling [`update`] with a `None`.
    ///
    /// [`update`]: Self::update
    pub fn update_ext(&self) {
        UpdatesTrace::log_update();
        self.update_ext_internal();
    }
    pub(crate) fn update_ext_internal(&self) {
        UPDATES_SV.write().flags.insert(UpdateFlags::UPDATE);
    }

    /// Schedules a layout update that will affect all app extensions and widgets with invalidated layout.
    pub fn layout(&self) {
        UpdatesTrace::log_layout();
        self.layout_internal();
    }
    pub(crate) fn layout_internal(&self) {
        UPDATES_SV.write().flags.insert(UpdateFlags::LAYOUT);
    }

    /// Schedules a render update that will affect all app extensions and widgets with invalidated layout.
    pub fn render(&self) {
        self.render_internal();
    }
    pub(crate) fn render_internal(&self) {
        UPDATES_SV.write().flags.insert(UpdateFlags::RENDER);
    }
}
