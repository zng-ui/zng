//! Context information for app extensions, windows and widgets.

use crate::{app::view_process::ViewRenderer, event::Events, service::Services, units::*, var::Vars, window::WindowId, WidgetId};
use retain_mut::RetainMut;
use std::{cell::Cell, fmt, mem, ops::Deref, ptr, rc::Rc, time::Instant};

#[doc(inline)]
pub use crate::state::*;

/// Required updates for a window layout and frame.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UpdateDisplayRequest {
    /// No update required.
    None = 0b0000_0000,
    /// Windows that requested this must update their existing frame and re-render.
    RenderUpdate = 0b0000_0001,
    /// Windows that requested this must generate a new frame and render.
    Render = 0b0000_0011,
    /// All open windows must generate a new frame and render.
    ForceRender = 0b1000_0011,
    /// Windows that requested this must re-compute layout, generate a new frame and render.
    Layout = 0b0000_0111,
    /// All open windows must re-compute layout, generate a new frame and render.
    ForceLayout = 0b1000_0111,
}
impl UpdateDisplayRequest {
    /// If the request must be applied to all open windows.
    #[inline]
    pub fn is_force(self) -> bool {
        (self as u8 & 0b1000_0000) == 0b1000_0000
    }

    /// If the request includes a re-compute of the window layout.
    #[inline]
    pub fn is_layout(self) -> bool {
        (self as u8 & 0b0000_0100) == 0b0000_0100
    }

    /// If the request includes the generation of a new frame.
    ///
    /// Returns `true` if it is layout too.
    #[inline]
    pub fn is_render(self) -> bool {
        (self as u8 & 0b0000_0010) == 0b0000_0010
    }

    /// If the request is only a render update. If `true` the window must update
    /// the current frame and re-render.
    ///
    /// Returns `false` if it is a full layout or render.
    #[inline]
    pub fn is_render_update(self) -> bool {
        self == UpdateDisplayRequest::RenderUpdate
    }

    /// If contains any update.
    #[inline]
    pub fn is_some(self) -> bool {
        !self.is_none()
    }

    /// If does not contain any update.
    #[inline]
    pub fn is_none(self) -> bool {
        self == UpdateDisplayRequest::None
    }

    /// Combine `self` as the app level request with an specific window request.
    ///
    /// Returns what update the window must apply.
    pub fn in_window(self, window_request: UpdateDisplayRequest) -> UpdateDisplayRequest {
        if self.is_force() {
            self
        } else {
            window_request
        }
    }
}
impl Default for UpdateDisplayRequest {
    #[inline]
    fn default() -> Self {
        UpdateDisplayRequest::None
    }
}
impl std::ops::BitOrAssign for UpdateDisplayRequest {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        let a = (*self) as u8;
        let b = rhs as u8;
        // SAFETY: flag OR
        *self = unsafe { mem::transmute(a | b) }
    }
}
impl std::ops::BitOr for UpdateDisplayRequest {
    type Output = Self;

    #[inline]
    fn bitor(mut self, rhs: Self) -> Self {
        self |= rhs;
        self
    }
}

use crate::app::AppEventSender;
use crate::crate_util::{Handle, HandleOwner, RunOnDrop};
use crate::event::BoxedEventUpdate;
use crate::handler::{self, AppHandler, AppHandlerArgs, AppWeakHandle};
#[doc(inline)]
pub use crate::state_key;
use crate::task;
use crate::timer::Timers;
use crate::{var::VarsRead, window::WindowMode};

/// Represents an [`on_pre_update`](Updates::on_pre_update) or [`on_update`](Updates::on_update) handler.
///
/// Drop all clones of this handle to drop the binding, or call [`permanent`](Self::permanent) to drop the handle
/// but keep the handler alive for the duration of the app.
#[derive(Clone)]
#[must_use = "dropping the handle unsubscribes update handler"]
pub struct OnUpdateHandle(Handle<()>);
impl OnUpdateHandle {
    fn new() -> (HandleOwner<()>, OnUpdateHandle) {
        let (owner, handle) = Handle::new(());
        (owner, OnUpdateHandle(handle))
    }

    /// Create a handle to nothing, the handle always in the *unsubscribed* state.
    #[inline]
    pub fn dummy() -> Self {
        OnUpdateHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unsubscribe.
    ///
    /// The handler stays in memory for the duration of the app or until another handle calls [`unsubscribe`](Self::unsubscribe.)
    #[inline]
    pub fn permanent(self) {
        self.0.permanent();
    }

    /// If another handle has called [`permanent`](Self::permanent).
    /// If `true` the var binding will stay active until the app shutdown, unless [`unsubscribe`](Self::unsubscribe) is called.
    #[inline]
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the handler to drop.
    #[inline]
    pub fn unsubscribe(self) {
        self.0.force_drop()
    }

    /// If another handle has called [`unsubscribe`](Self::unsubscribe).
    ///
    /// The handler is already dropped or will be dropped in the next app update, this is irreversible.
    #[inline]
    pub fn is_unsubscribed(&self) -> bool {
        self.0.is_dropped()
    }
}

struct UpdateHandler {
    handle: HandleOwner<()>,
    count: usize,
    handler: Box<dyn FnMut(&mut AppContext, &UpdateArgs, &dyn AppWeakHandle)>,
}

/// Arguments for an [`on_pre_update`](Updates::on_pre_update), [`on_update`](Updates::on_update) or [`run`](Updates::run) handler.
#[derive(Debug, Clone, Copy)]
pub struct UpdateArgs {
    /// Number of times the handler was called.
    pub count: usize,
}

/// Schedule of actions to apply after an update.
///
/// An instance of this struct is available in [`AppContext`] and derived contexts.
pub struct Updates {
    event_sender: AppEventSender,
    update: bool,

    // `win_display_update` tracks the requests made inside the window
    // in the end `display_update` is applied but windows ignore the request
    // if it was not made inside the window and is not `force`.
    display_update: UpdateDisplayRequest,
    win_display_update: UpdateDisplayRequest,

    pre_handlers: Vec<UpdateHandler>,
    pos_handlers: Vec<UpdateHandler>,
}
impl Updates {
    fn new(event_sender: AppEventSender) -> Self {
        Updates {
            event_sender,
            update: false,
            display_update: UpdateDisplayRequest::None,
            win_display_update: UpdateDisplayRequest::None,

            pre_handlers: vec![],
            pos_handlers: vec![],
        }
    }

    /// Create an [`AppEventSender`] that can be used to awake the app and send app events.
    #[inline]
    pub fn sender(&self) -> AppEventSender {
        self.event_sender.clone()
    }

    /// Schedules a low-pressure update.
    #[inline]
    pub fn update(&mut self) {
        self.update = true;
    }

    /// Gets `true` if a low-pressure update was requested.
    #[inline]
    pub fn update_requested(&self) -> bool {
        self.update
    }

    /// Schedules a layout update.
    ///
    /// In a [`WindowContext`] and [`WidgetContext`] this only requests a layout for the parent window,
    /// use [`layout_all`] to re-layout all open windows.
    ///
    /// [`layout_all`]: Self::layout_all
    #[inline]
    pub fn layout(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::Layout;
        self.display_update |= UpdateDisplayRequest::Layout;
    }

    /// Schedules a layout update for all open windows.
    #[inline]
    pub fn layout_all(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::ForceLayout;
        self.display_update |= UpdateDisplayRequest::ForceLayout;
    }

    /// Gets `true` if a layout update is scheduled.
    #[inline]
    pub fn layout_requested(&self) -> bool {
        self.win_display_update == UpdateDisplayRequest::Layout
    }

    /// Schedules a new frame.
    ///
    /// In a [`WindowContext`] and [`WidgetContext`] this only requests a new frame for the parent window,
    /// use [`render_all`] to re-render all open windows.
    ///
    /// [`render_all`]: Self::render_all
    #[inline]
    pub fn render(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::Render;
        self.display_update |= UpdateDisplayRequest::Render;
    }

    /// Schedules a new frame for all open windows.
    #[inline]
    pub fn render_all(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::ForceRender;
        self.display_update |= UpdateDisplayRequest::ForceRender;
    }

    /// Returns `true` if a new frame is scheduled, including layout.
    #[inline]
    pub fn render_requested(&self) -> bool {
        self.win_display_update.is_render()
    }

    /// Schedule a frame update.
    #[inline]
    pub fn render_update(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::RenderUpdate;
        self.display_update |= UpdateDisplayRequest::RenderUpdate;
    }

    /// Returns `true` if only a frame update is scheduled.
    #[inline]
    pub fn render_update_requested(&self) -> bool {
        self.win_display_update.is_render_update()
    }

    /// Schedule the `updates`.
    #[inline]
    pub fn schedule_display_updates(&mut self, updates: UpdateDisplayRequest) {
        self.win_display_update |= updates;
        self.display_update |= updates;
    }

    /// Schedule an *once* handler to run when these updates are applied.
    ///
    /// The callback is any of the *once* [`AppHandler`], including async handlers. You can use [`app_hn_once!`](handler::app_hn_once!)
    /// or [`async_app_hn_once!`](handler::async_app_hn_once!) to declare the closure. If the handler is async and does not finish in
    /// one call it is scheduled to update in *preview* updates.
    pub fn run<H: AppHandler<UpdateArgs> + handler::marker::OnceHn>(&mut self, handler: H) -> OnUpdateHandle {
        self.update(); // in case of this was called outside of an update.
        Self::push_handler(&mut self.pos_handlers, true, handler)
    }

    /// Create a preview update handler.
    ///
    /// The `handler` is called every time the app updates, just before the UI updates. It can be any of the non-async [`AppHandler`],
    /// use the [`app_hn!`] or [`app_hn_once!`] macros to declare the closure. Async handlers are not allowed because UI bound async
    /// tasks cause app updates to awake, so it is very easy to lock the app in a constant sequence of updates. You can use [`run`](Self::run)
    /// to start an async app context task.
    ///
    /// Returns an [`OnUpdateHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    pub fn on_pre_update<H>(&mut self, handler: H) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs> + handler::marker::NotAsyncHn,
    {
        Self::push_handler(&mut self.pre_handlers, true, handler)
    }

    /// Create an update handler.
    ///
    /// The `handler` is called every time the app updates, just after the UI updates.
    ///
    /// Returns an [`OnUpdateHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    pub fn on_update<H>(&mut self, handler: H) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs> + handler::marker::NotAsyncHn,
    {
        Self::push_handler(&mut self.pos_handlers, false, handler)
    }

    fn push_handler<H>(entries: &mut Vec<UpdateHandler>, is_preview: bool, mut handler: H) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs>,
    {
        let (handle_owner, handle) = OnUpdateHandle::new();
        entries.push(UpdateHandler {
            handle: handle_owner,
            count: 0,
            handler: Box::new(move |ctx, args, handle| {
                handler.event(ctx, args, &AppHandlerArgs { handle, is_preview });
            }),
        });
        handle
    }

    pub(crate) fn on_pre_updates(ctx: &mut AppContext) {
        let mut handlers = mem::take(&mut ctx.updates.pre_handlers);
        Self::retain_updates(ctx, &mut handlers);
        handlers.append(&mut ctx.updates.pre_handlers);
        ctx.updates.pre_handlers = handlers;
    }

    pub(crate) fn on_updates(ctx: &mut AppContext) {
        let mut handlers = mem::take(&mut ctx.updates.pos_handlers);
        Self::retain_updates(ctx, &mut handlers);
        handlers.append(&mut ctx.updates.pos_handlers);
        ctx.updates.pos_handlers = handlers;
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

    fn take_updates(&mut self) -> (bool, UpdateDisplayRequest) {
        (mem::take(&mut self.update), mem::take(&mut self.display_update))
    }
}
/// crate::app::HeadlessApp::block_on
impl Updates {
    pub(crate) fn handler_lens(&self) -> (usize, usize) {
        (self.pre_handlers.len(), self.pos_handlers.len())
    }
    pub(crate) fn new_update_handlers(&self, pre_from: usize, pos_from: usize) -> Vec<Box<dyn Fn() -> bool>> {
        self.pre_handlers
            .iter()
            .skip(pre_from)
            .chain(self.pos_handlers.iter().skip(pos_from))
            .map(|h| h.handle.weak_handle())
            .map(|h| {
                let r: Box<dyn Fn() -> bool> = Box::new(move || h.upgrade().is_some());
                r
            })
            .collect()
    }
}

/// Represents a type that can provide access to [`Updates`] inside the window of function call.
///
/// This is implemented to all sync and async context types and [`Updates`] it-self.
pub trait WithUpdates {
    /// Calls `action` with the [`Updates`] reference.
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R;
}
impl WithUpdates for Updates {
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R {
        action(self)
    }
}
impl<'a> WithUpdates for crate::context::AppContext<'a> {
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R {
        action(self.updates)
    }
}
impl<'a> WithUpdates for crate::context::WindowContext<'a> {
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R {
        action(self.updates)
    }
}
impl<'a> WithUpdates for crate::context::WidgetContext<'a> {
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R {
        action(self.updates)
    }
}
impl WithUpdates for crate::context::AppContextMut {
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R {
        self.with(move |ctx| action(ctx.updates))
    }
}
impl WithUpdates for crate::context::WidgetContextMut {
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R {
        self.with(move |ctx| action(ctx.updates))
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl WithUpdates for crate::context::TestWidgetContext {
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R {
        action(&mut self.updates)
    }
}
impl WithUpdates for crate::app::HeadlessApp {
    fn with_updates<R, A: FnOnce(&mut Updates) -> R>(&mut self, action: A) -> R {
        action(self.ctx().updates)
    }
}

/// Owner of [`AppContext`] objects.
///
/// You can only have one instance of this at a time per-thread at a time.
pub(crate) struct OwnedAppContext {
    app_state: StateMap,
    vars: Vars,
    events: Events,
    services: Services,
    timers: Timers,
    updates: Updates,
}
impl OwnedAppContext {
    /// Produces the single instance of `AppContext` for a normal app run.
    pub fn instance(app_event_sender: AppEventSender) -> Self {
        let updates = Updates::new(app_event_sender.clone());
        OwnedAppContext {
            app_state: StateMap::new(),
            vars: Vars::instance(app_event_sender.clone()),
            events: Events::instance(app_event_sender),
            services: Services::default(),
            timers: Timers::new(),
            updates,
        }
    }

    /// State that lives for the duration of an application, including a headless application.
    pub fn app_state(&self) -> &StateMap {
        &self.app_state
    }

    /// State that lives for the duration of an application, including a headless application.
    pub fn app_state_mut(&mut self) -> &mut StateMap {
        &mut self.app_state
    }

    /// Borrow the app context as an [`AppContext`].
    pub fn borrow(&mut self) -> AppContext {
        AppContext {
            app_state: &mut self.app_state,
            vars: &self.vars,
            events: &mut self.events,
            services: &mut self.services,
            timers: &mut self.timers,
            updates: &mut self.updates,
        }
    }

    /// Borrow the [`Vars`] only.
    pub fn vars(&self) -> &Vars {
        &self.vars
    }

    /// Applies pending `timers`, `sync`, `vars` and `events` and returns the update
    /// requests and a time for the loop to awake and update.
    #[must_use]
    pub fn apply_updates(&mut self) -> ContextUpdates {
        let wake_time = self.timers.apply_updates(&self.vars);
        let events = self.events.apply_updates(&self.vars, &mut self.updates);
        self.vars.apply_updates(&mut self.updates);

        let (update, display_update) = self.updates.take_updates();

        ContextUpdates {
            events,
            update,
            display_update,
            wake_time,
        }
    }
}

/// Full application context.
pub struct AppContext<'a> {
    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// Access to variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a mut Events,
    /// Access to application services.
    pub services: &'a mut Services,

    /// Event loop based timers.
    pub timers: &'a mut Timers,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}
impl<'a> AppContext<'a> {
    /// Returns a [`WindowMode`] value that indicates if the app is headless, headless with renderer or headed.
    ///
    /// Note that specific windows can be in headless modes even if the app is headed.
    pub fn mode(&mut self) -> WindowMode {
        self.services
            .get::<crate::app::view_process::ViewProcess>()
            .map(|p| {
                if p.headless() {
                    WindowMode::HeadlessWithRenderer
                } else {
                    WindowMode::Headed
                }
            })
            .unwrap_or(WindowMode::Headless)
    }

    /// Runs a function `f` in the context of a window.
    #[inline(always)]
    pub fn window_context<R>(
        &mut self,
        window_id: WindowId,
        mode: WindowMode,
        window_state: &mut OwnedStateMap,
        renderer: &Option<ViewRenderer>,
        f: impl FnOnce(&mut WindowContext) -> R,
    ) -> (R, UpdateDisplayRequest) {
        if !self.updates.display_update.is_force() {
            self.updates.win_display_update = UpdateDisplayRequest::None;
        }

        let mut update_state = StateMap::new();

        let r = f(&mut WindowContext {
            window_id: &window_id,
            mode: &mode,
            app_state: self.app_state,
            window_state: &mut window_state.0,
            renderer,
            update_state: &mut update_state,
            vars: self.vars,
            events: self.events,
            services: self.services,
            timers: self.timers,
            updates: self.updates,
        });

        (r, mem::take(&mut self.updates.win_display_update))
    }

    /// Run a function `f` in the layout context of the monitor that contains a window.
    #[inline(always)]
    pub fn outer_layout_context<R>(
        &mut self,
        screen_size: PxSize,
        scale_factor: f32,
        screen_ppi: f32,
        window_id: WindowId,
        root_id: WidgetId,
        f: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        f(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, screen_size, Length::pt_to_px(14.0, scale_factor)).with_screen_ppi(screen_ppi),
            path: &mut WidgetContextPath::new(window_id, root_id),
            app_state: self.app_state,
            window_state: &mut StateMap::new(),
            widget_state: &mut StateMap::new(),
            update_state: &mut StateMap::new(),
            vars: self.vars,
        })
    }
}

/// A window context.
pub struct WindowContext<'a> {
    /// Id of the context window.
    pub window_id: &'a WindowId,

    /// Window mode, headed or not, renderer or not.
    pub mode: &'a WindowMode,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// Connection to the window renderer.
    ///
    /// This is only available after the first render call and if the [`mode`] is not headless. TODO rethink this
    ///
    /// [`mode`]: WindowContext::mode
    pub renderer: &'a Option<ViewRenderer>,

    /// State that lives for the duration of the node tree method call in the window.
    ///
    /// This state lives only for the duration of the function `f` call in [`AppContext::window_context`].
    /// Usually `f` calls one of the [`UiNode`](crate::UiNode) methods and [`WidgetContext`] shares this
    /// state so properties and event handlers can use this state to communicate to further nodes along the
    /// update sequence.
    pub update_state: &'a mut StateMap,

    /// Access to variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a mut Events,
    /// Access to application services.
    pub services: &'a mut Services,

    /// Event loop based timers.
    pub timers: &'a mut Timers,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}
impl<'a> WindowContext<'a> {
    /// Runs a function `f` in the context of a widget.
    #[inline(always)]
    pub fn widget_context<R>(
        &mut self,
        widget_id: WidgetId,
        widget_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut WidgetContext) -> R,
    ) -> R {
        f(&mut WidgetContext {
            path: &mut WidgetContextPath::new(*self.window_id, widget_id),

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &mut widget_state.0,
            update_state: self.update_state,

            vars: self.vars,
            events: self.events,
            services: self.services,

            timers: self.timers,

            updates: self.updates,
        })
    }

    /// Runs a function `f` in the layout context of a widget.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn layout_context<R>(
        &mut self,
        font_size: Px,
        scale_factor: f32,
        screen_ppi: f32,
        viewport_size: PxSize,
        widget_id: WidgetId,
        widget_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        f(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, viewport_size, font_size).with_screen_ppi(screen_ppi),

            path: &mut WidgetContextPath::new(*self.window_id, widget_id),

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &mut widget_state.0,
            update_state: self.update_state,

            vars: self.vars,
        })
    }

    /// Runs a function `f` in the render context of a widget.
    #[inline(always)]
    pub fn render_context<R>(&mut self, widget_id: WidgetId, widget_state: &OwnedStateMap, f: impl FnOnce(&mut RenderContext) -> R) -> R {
        f(&mut RenderContext {
            path: &mut WidgetContextPath::new(*self.window_id, widget_id),
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &widget_state.0,
            update_state: self.update_state,
            vars: self.vars,
        })
    }
}

/// A mock [`WidgetContext`] for testing widgets.
///
/// Only a single instance of this type can exist per-thread at a time, see [`new`] for details.
///
/// This is less cumbersome to use then a full headless app, but also more limited. Use a [`HeadlessApp`]
/// for more complex integration tests.
///
/// [`new`]: TestWidgetContext::new
/// [`HeadlessApp`]: crate::app::HeadlessApp
#[cfg(any(test, doc, feature = "test_util"))]
#[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
pub struct TestWidgetContext {
    /// Id of the pretend window that owns the pretend root widget.
    ///
    /// This is a new unique id.
    pub window_id: WindowId,
    /// Id of the pretend root widget that is the context widget.
    pub root_id: WidgetId,

    /// The [`app_state`] value. Empty by default.
    ///
    /// [`app_state`]: WidgetContext::app_state
    pub app_state: OwnedStateMap,
    /// The [`window_state`] value. Empty by default.
    ///
    /// [`window_state`]: WidgetContext::window_state
    pub window_state: OwnedStateMap,

    /// The [`widget_state`] value. Empty by default.
    ///
    /// [`widget_state`]: WidgetContext::widget_state
    pub widget_state: OwnedStateMap,

    /// The [`update_state`] value. Empty by default.
    ///
    /// WARNING: In a real context this is reset after each update, in this test context the same map is reused
    /// unless you call [`clear`].
    ///
    /// [`update_state`]: WidgetContext::update_state
    /// [`clear`]: OwnedStateMap::clear
    pub update_state: OwnedStateMap,

    /// The [`services`] repository. Empty by default.
    ///
    /// [`services`]: WidgetContext::services
    pub services: Services,

    /// The [`updates`] repository. No request by default.
    ///
    /// WARNING: This is drained of requests after each update, you can do this manually by calling
    /// [`apply_updates`].
    ///
    /// [`updates`]: WidgetContext::updates
    /// [`apply_updates`]: TestWidgetContext::apply_updates
    pub updates: Updates,

    /// The [`vars`] instance.
    ///
    /// [`vars`]: WidgetContext::vars
    pub vars: Vars,

    /// The [`events`] instance. No events registered by default.
    ///
    /// [`events`]: WidgetContext::events
    pub events: Events,

    /// Event loop bases timers.
    ///
    /// TODO testable timers.
    pub timers: Timers,

    receiver: flume::Receiver<crate::app::AppEvent>,
}
#[cfg(any(test, doc, feature = "test_util"))]
impl Default for TestWidgetContext {
    /// [`TestWidgetContext::new`]
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl TestWidgetContext {
    /// Gets a new [`TestWidgetContext`] instance. Panics is another instance is alive in the current thread
    /// or if an app is running in the current thread.
    pub fn new() -> Self {
        if crate::app::App::is_running() {
            panic!("only one `TestWidgetContext` or app is allowed per thread")
        }

        let (sender, receiver) = AppEventSender::new();
        Self {
            window_id: WindowId::new_unique(),
            root_id: WidgetId::new_unique(),
            app_state: OwnedStateMap::new(),
            window_state: OwnedStateMap::new(),
            widget_state: OwnedStateMap::new(),
            update_state: OwnedStateMap::new(),
            services: Services::default(),
            events: Events::instance(sender.clone()),
            vars: Vars::instance(sender.clone()),
            updates: Updates::new(sender),
            timers: Timers::new(),
            receiver,
        }
    }

    /// Calls `action` in a fake widget context.
    pub fn widget_context<R>(&mut self, action: impl FnOnce(&mut WidgetContext) -> R) -> R {
        action(&mut WidgetContext {
            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            app_state: &mut self.app_state.0,
            window_state: &mut self.window_state.0,
            widget_state: &mut self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
            events: &mut self.events,
            services: &mut self.services,
            timers: &mut self.timers,
            updates: &mut self.updates,
        })
    }

    /// Calls `action` in a fake layout context.
    pub fn layout_context<R>(
        &mut self,
        root_font_size: Px,
        font_size: Px,
        viewport_size: PxSize,
        scale_factor: f32,
        screen_ppi: f32,
        action: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        action(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, viewport_size, root_font_size)
                .with_font_size(font_size)
                .with_screen_ppi(screen_ppi),

            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            app_state: &mut self.app_state.0,
            window_state: &mut self.window_state.0,
            widget_state: &mut self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
        })
    }

    /// Calls `action` in a fake render context.
    pub fn render_context<R>(&mut self, action: impl FnOnce(&mut RenderContext) -> R) -> R {
        action(&mut RenderContext {
            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            app_state: &self.app_state.0,
            window_state: &self.window_state.0,
            widget_state: &self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
        })
    }

    /// Applies pending, `sync`, `vars`, `events` and takes all the update requests.
    ///
    /// Returns the [`ContextUpdates`] a full app would use to update the application.
    pub fn apply_updates(&mut self) -> ContextUpdates {
        for ev in self.receiver.try_iter() {
            match ev {
                crate::app::AppEvent::ViewEvent(_) => unimplemented!(),
                crate::app::AppEvent::Event(ev) => self.events.notify_app_event(ev),
                crate::app::AppEvent::Var => self.vars.receive_sended_modify(),
                crate::app::AppEvent::Update => self.updates.update(),
                crate::app::AppEvent::ResumeUnwind(p) => std::panic::resume_unwind(p),
            }
        }
        let wake_time = self.timers.apply_updates(&self.vars);
        let events = self.events.apply_updates(&self.vars, &mut self.updates);
        self.vars.apply_updates(&mut self.updates);
        let (update, display_update) = self.updates.take_updates();
        ContextUpdates {
            events,
            update,
            display_update,
            wake_time,
        }
    }
}

/// Updates that must be reacted by an app context owner.
#[derive(Debug, Default)]
pub struct ContextUpdates {
    /// Events update to notify.
    ///
    /// When this is not empty [`update`](Self::update) is `true`.
    pub events: Vec<BoxedEventUpdate>,

    /// Update requested.
    pub update: bool,

    /// Display update to notify.
    pub display_update: UpdateDisplayRequest,

    /// Time for the loop to awake and update.
    pub wake_time: Option<Instant>,
}
impl ContextUpdates {
    /// If [`update`](Self::update) or [`display_update`](Self::display_update) where requested.
    #[inline]
    pub fn has_updates(&self) -> bool {
        self.update || self.display_update.is_some()
    }
}
impl std::ops::BitOrAssign for ContextUpdates {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.events.extend(rhs.events);
        self.update |= rhs.update;
        self.display_update = rhs.display_update;
        self.wake_time = match (self.wake_time, rhs.wake_time) {
            (None, None) => None,
            (None, Some(t)) | (Some(t), None) => Some(t),
            (Some(a), Some(b)) => Some(a.min(b)),
        };
    }
}
impl std::ops::BitOr for ContextUpdates {
    type Output = Self;

    #[inline]
    fn bitor(mut self, rhs: Self) -> Self {
        self |= rhs;
        self
    }
}

/// A widget context.
pub struct WidgetContext<'a> {
    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the widget.
    pub widget_state: &'a mut StateMap,

    /// State that lives for the duration of the node tree method call in the window.
    ///
    /// This state lives only for the current [`UiNode`](crate::UiNode) method call in all nodes
    /// of the window. You can use this to signal properties and event handlers from nodes that
    /// will be updated further then the current one.
    pub update_state: &'a mut StateMap,

    /// Access to variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a mut Events,
    /// Access to application services.
    pub services: &'a mut Services,

    /// Event loop based timers.
    pub timers: &'a mut Timers,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}
impl<'a> WidgetContext<'a> {
    /// Runs a function `f` in the context of a widget.
    #[inline(always)]
    pub fn widget_context<R>(
        &mut self,
        widget_id: WidgetId,
        widget_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut WidgetContext) -> R,
    ) -> R {
        self.path.push(widget_id);

        let r = self.vars.with_widget_clear(|| {
            f(&mut WidgetContext {
                path: self.path,

                app_state: self.app_state,
                window_state: self.window_state,
                widget_state: &mut widget_state.0,
                update_state: self.update_state,

                vars: self.vars,
                events: self.events,
                services: self.services,

                timers: self.timers,

                updates: self.updates,
            })
        });

        self.path.pop();

        r
    }
}

/// Current widget context path.
pub struct WidgetContextPath {
    window_id: WindowId,
    widget_ids: Vec<WidgetId>,
}
impl WidgetContextPath {
    fn new(window_id: WindowId, root_id: WidgetId) -> Self {
        let mut widget_ids = Vec::with_capacity(50);
        widget_ids.push(root_id);
        WidgetContextPath { window_id, widget_ids }
    }

    fn push(&mut self, widget_id: WidgetId) {
        self.widget_ids.push(widget_id);
    }

    fn pop(&mut self) {
        debug_assert!(self.widget_ids.len() > 1, "cannot pop root");
        self.widget_ids.pop();
    }

    /// Parent window id.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Window root widget id.
    #[inline]
    pub fn root_id(&self) -> WidgetId {
        self.widget_ids[0]
    }

    /// Current widget id.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.widget_ids[self.widget_ids.len() - 1]
    }

    /// Ancestor widgets, parent first.
    #[inline]
    #[allow(clippy::needless_lifetimes)] // clippy bug
    pub fn ancestors<'s>(&'s self) -> impl Iterator<Item = WidgetId> + 's {
        let max = self.widget_ids.len() - 1;
        self.widget_ids[0..max].iter().copied().rev()
    }

    /// Parent widget id.
    #[inline]
    pub fn parent(&self) -> Option<WidgetId> {
        self.ancestors().next()
    }

    /// If the `widget_id` is part of the path.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.widget_ids.iter().any(move |&w| w == widget_id)
    }
}
impl fmt::Debug for WidgetContextPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("WidgetContextPath")
                .field("window_id", &self.window_id)
                .field("widget_ids", &self.widget_ids)
                .finish()
        } else {
            write!(f, "{}", self)
        }
    }
}
impl fmt::Display for WidgetContextPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // "WinId(1)//Wgt(1)/Wgt(23)"
        write!(f, "{}/", self.window_id)?;
        for w in &self.widget_ids {
            write!(f, "/{}", w)?;
        }
        Ok(())
    }
}

/// Layout metrics in a [`LayoutContext`].
///
/// The [`LayoutContext`] type dereferences to this one.
#[derive(Debug, Clone)]
pub struct LayoutMetrics {
    /// Current computed font size.
    pub font_size: Px,

    /// Computed font size at the root widget.
    pub root_font_size: Px,

    /// Pixel scale factor.
    pub scale_factor: f32,

    /// Size of the window content.
    pub viewport_size: PxSize,

    /// The current screen "pixels-per-inch" resolution.
    ///
    /// This value is dependent in the actual physical size of the screen that the user must manually measure.
    /// For most of the UI you only need the [`scale_factor`].
    ///
    /// If you are implementing some feature like a "print size preview", you need to use this value, and you
    /// can configure a PPI per screen in the [`Monitors`] service.
    ///
    /// Default is `96.0`.
    ///
    /// [`Monitors`]: crate::window::Monitors
    /// [`scale_factor`]: LayoutMetrics::scale_factor
    pub screen_ppi: f32,
}
impl LayoutMetrics {
    /// New root [`LayoutMetrics`].
    ///
    /// The `font_size` sets both font sizes, the initial PPI is `96.0`, you can use the builder style method and
    /// [`with_screen_ppi`] to set a different value.
    ///
    /// [`with_screen_ppi`]: LayoutMetrics::with_screen_ppi
    pub fn new(scale_factor: f32, viewport_size: PxSize, font_size: Px) -> Self {
        LayoutMetrics {
            font_size,
            root_font_size: font_size,
            scale_factor,
            viewport_size,
            screen_ppi: 96.0,
        }
    }

    /// Smallest dimension of the [`viewport_size`].
    ///
    /// [`viewport_size`]: Self::viewport_size
    #[inline]
    pub fn viewport_min(&self) -> Px {
        self.viewport_size.width.min(self.viewport_size.height)
    }

    /// Largest dimension of the [`viewport_size`].
    ///
    /// [`viewport_size`]: Self::viewport_size
    #[inline]
    pub fn viewport_max(&self) -> Px {
        self.viewport_size.width.max(self.viewport_size.height)
    }

    /// Sets the [`font_size`].
    ///
    /// [`font_size`]: Self::font_size
    #[inline]
    pub fn with_font_size(mut self, font_size: Px) -> Self {
        self.font_size = font_size;
        self
    }

    /// Sets the [`scale_factor`].
    ///
    /// [`scale_factor`]: Self::scale_factor
    #[inline]
    pub fn with_scale_factor(mut self, scale_factor: f32) -> Self {
        self.scale_factor = scale_factor;
        self
    }

    /// Sets the [`screen_ppi`].
    ///
    /// [`screen_ppi`]: Self::screen_ppi
    #[inline]
    pub fn with_screen_ppi(mut self, screen_ppi: f32) -> Self {
        self.screen_ppi = screen_ppi;
        self
    }
}

/// A widget layout context.
///
/// This type dereferences to [`LayoutMetrics`].
#[derive(Debug)]
pub struct LayoutContext<'a> {
    /// Contextual layout metrics.
    pub metrics: &'a LayoutMetrics,

    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the widget.
    pub widget_state: &'a mut StateMap,

    /// State that lives for the duration of the node tree layout update call in the window.
    ///
    /// This state lives only for the sequence of two [`UiNode::measure`](crate::UiNode::measure) and [`UiNode::arrange`](crate::UiNode::arrange)
    /// method calls in all nodes of the window. You can use this to signal nodes that have not participated in the current
    /// layout update yet, or from `measure` signal `arrange`.
    pub update_state: &'a mut StateMap,

    /// Access to variables.
    pub vars: &'a Vars,
}
impl<'a> Deref for LayoutContext<'a> {
    type Target = LayoutMetrics;

    fn deref(&self) -> &Self::Target {
        self.metrics
    }
}
impl<'a> LayoutContext<'a> {
    /// Runs a function `f` in a layout context that has the new computed font size.
    #[inline(always)]
    pub fn with_font_size<R>(&mut self, new_font_size: Px, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        f(&mut LayoutContext {
            metrics: &self.metrics.clone().with_font_size(new_font_size),

            path: self.path,

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state,

            vars: self.vars,
        })
    }

    /// Runs a function `f` in the layout context of a widget.
    #[inline(always)]
    pub fn with_widget<R>(&mut self, widget_id: WidgetId, widget_state: &mut OwnedStateMap, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        self.path.push(widget_id);

        let r = self.vars.with_widget_clear(|| {
            f(&mut LayoutContext {
                metrics: self.metrics,

                path: self.path,

                app_state: self.app_state,
                window_state: self.window_state,
                widget_state: &mut widget_state.0,
                update_state: self.update_state,

                vars: self.vars,
            })
        });

        self.path.pop();

        r
    }
}

/// A widget render context.
pub struct RenderContext<'a> {
    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// Read-only access to the state that lives for the duration of the application.
    pub app_state: &'a StateMap,

    /// Read-only access to the state that lives for the duration of the window.
    pub window_state: &'a StateMap,

    /// Read-only access to the state that lives for the duration of the widget.
    pub widget_state: &'a StateMap,

    /// State that lives for the duration of the node tree render or render update call in the window.
    ///
    /// This state lives only for the call to [`UiNode::render`](crate::UiNode::render) or
    /// [`UiNode::render_update`](crate::UiNode::render_update) method call in all nodes of the window.
    /// You can use this to signal nodes that have not rendered yet.
    pub update_state: &'a mut StateMap,

    /// Read-only access to variables.
    pub vars: &'a VarsRead,
}
impl<'a> RenderContext<'a> {
    /// Runs a function `f` in the render context of a widget.
    #[inline(always)]
    pub fn with_widget<R>(&mut self, widget_id: WidgetId, widget_state: &OwnedStateMap, f: impl FnOnce(&mut RenderContext) -> R) -> R {
        self.path.push(widget_id);
        let r = f(&mut RenderContext {
            path: self.path,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &widget_state.0,
            update_state: self.update_state,
            vars: self.vars,
        });
        self.path.pop();
        r
    }
}

macro_rules! contextual_ctx {
    ($($Context:ident),+ $(,)?) => {$(paste::paste! {

#[doc = " Represents a *contextual* reference to [`"$Context "`]."]
///
#[doc = "This type exist to provide access to a [`"$Context "`] inside [Ui bound](crate::task::ui) futures."]
#[doc = "Every time the task updates the executor loads a exclusive reference to the context using the paired [`"$Context "Scope`]"]
/// to provide the context for that update. Inside the future you can then call [`with`](Self::with) to get the exclusive
/// reference to the context.
pub struct [<$Context Mut>] {
    ctx: Rc<[<$Context ScopeData>]>,
}
impl Clone for [<$Context Mut>] {
    fn clone(&self) -> Self {
        Self {
            ctx: Rc::clone(&self.ctx)
        }
    }
}
impl [<$Context Mut>] {
    #[doc = "Runs an action with the *contextual* exclusive borrow to a [`"$Context "`]."]
    ///
    /// ## Panics
    ///
    /// Panics if `with` is called again inside `action`, also panics if not called inside the paired
    #[doc = "[`"$Context "Scope::with`]. You should assume that if you have access to a [`"$Context "Mut`] it is in a valid"]
    /// state, the onus of safety is on the caller.
    #[inline]
    pub fn with<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&mut $Context) -> R,
    {
        if self.ctx.borrowed.get() {
            panic!("already in `{0}Mut::with`, cannot borrow `&mut {0}` twice", stringify!($Context));
        }

        let ptr = self.ctx.ptr.get();
        if ptr.is_null() {
            panic!("no `&mut {0}` loaded for `{0}Mut`", stringify!($Context));
        }

        self.ctx.borrowed.set(true);
        let _r = RunOnDrop::new(|| {
            self.ctx.borrowed.set(false);
        });

        let ctx = unsafe { &mut *(ptr as *mut $Context) };
        action(ctx)
    }
}

#[doc = "Pair of [`"$Context "Mut`] that can setup its reference."]
pub struct [<$Context Scope>] {
    ctx: Rc<[<$Context ScopeData>]>,
}
struct [<$Context ScopeData>] {
    ptr: Cell<*mut ()>,
    borrowed: Cell<bool>,
}
impl [<$Context Scope>] {
    #[doc = "Create a new [`"$Context "Scope`], [`"$Context "Mut`] pair."]
    pub fn new() -> (Self, [<$Context Mut>]) {
        let ctx = Rc::new([<$Context ScopeData>] {
            ptr: Cell::new(ptr::null_mut()),
            borrowed: Cell::new(false)
        });

        (Self { ctx: Rc::clone(&ctx) }, [<$Context Mut>] { ctx })
    }

    #[doc = "Runs `action` while the paired [`"$Context "Mut`] points to `ctx`."]
    pub fn with<R, F>(&self, ctx: &mut $Context, action: F) -> R
    where
        F: FnOnce() -> R,
    {
        let prev = self.ctx.ptr.replace(ctx as *mut $Context as *mut ());
        let _r = RunOnDrop::new(|| {
            self.ctx.ptr.set(prev)
        });
        action()
    }
}

    })+};
}
contextual_ctx!(AppContext, WidgetContext);

impl AppContextMut {
    /// Yield for one update.
    ///
    /// Async event handlers run in app updates, the code each `.await` runs in a different update, but only if
    /// the `.await` does not return immediately. This future always awaits once for each new update, so the
    /// code after awaiting is guaranteed to run in a different update.
    ///
    /// Note that this does not cause an immediate update, if no update was requested it will *wait* until one is.
    /// To force an update and then yield use [`update`](Self::update) instead.
    ///
    /// ```
    /// # use zero_ui_core::{context::*, handler::*, app::*};
    /// # HeadlessApp::doc_test((),
    /// async_app_hn!(|ctx, _, _| {
    ///     println!("First update");
    ///     ctx.yield_one().await;
    ///     println!("Second update");
    /// })
    /// # );
    /// ```
    pub async fn yield_one(&self) {
        task::yield_one().await
    }

    /// Requests one update and returns a future that *yields* one update.
    ///
    /// This is like [`yield_one`](Self::yield_one) but also requests the next update, causing the code after
    /// the `.await` to run immediately after one update is processed.
    ///
    /// ```
    /// # use zero_ui_core::{context::*, handler::*, var::*};
    /// # let mut app = zero_ui_core::app::App::blank().run_headless(false);
    /// let foo_var = var(false);
    /// # app.ctx().updates.run(
    /// async_app_hn_once!(foo_var, |ctx, _| {
    ///     // variable assign will cause an update.
    ///     foo_var.set(&ctx, true);
    ///
    ///     ctx.yield_one().await;// wait next update.
    ///
    ///     // we are in the next update now, the variable value is new.
    ///     assert_eq!(Some(true), foo_var.copy_new(&ctx));
    ///
    ///     ctx.update().await;// force next update and wait.
    ///
    ///     // we are in the requested update, variable value is no longer new.
    ///     assert_eq!(None, foo_var.copy_new(&ctx));
    /// })
    /// # ).permanent();
    /// # app.update(false);
    /// # assert!(foo_var.copy(&app.ctx()));
    /// ```
    ///
    /// In the example above, the variable assign causes an app update so `yield_one` processes it immediately,
    /// but the second `.await` needs to cause an update if we don't want to depend on another part of the app
    /// to awake.
    pub async fn update(&self) {
        self.with(|c| c.updates.update());
        self.yield_one().await
    }
}

impl WidgetContextMut {
    /// Yield for one update.
    ///
    /// Async event handlers run in widget updates, the code each `.await` runs in a different update, but only if
    /// the `.await` does not return immediately. This future always awaits once for each new update, so the
    /// code after awaiting is guaranteed to run in a different update.
    ///
    /// Note that this does not cause an immediate update, if no update was requested it will *wait* until one is.
    /// To force an update and then yield use [`update`](Self::update) instead.
    ///
    /// You can reuse this future but it is very cheap to just make a new one.
    ///
    /// ```
    /// # use zero_ui_core::{context::*, handler::*};
    /// # TestWidgetContext::doc_test((),
    /// async_hn!(|ctx, _| {
    ///     println!("First update");
    ///     ctx.yield_one().await;
    ///     println!("Second update");
    /// })
    /// # );
    /// ```
    #[inline]
    pub async fn yield_one(&self) {
        task::yield_one().await
    }

    /// Requests one update and returns a future that *yields* one update.
    ///
    /// This is like [`yield_one`](Self::yield_one) but also requests the next update, causing the code after
    /// the `.await` to run immediately after one update is processed.
    ///
    /// ```
    /// # use zero_ui_core::context::*;
    /// # use zero_ui_core::handler::*;
    /// # use zero_ui_core::var::*;
    /// # TestWidgetContext::doc_test((),
    /// async_hn!(|ctx, _| {
    ///     let foo_var = var(false);
    ///     // variable assign will cause an update.
    ///     foo_var.set(&ctx, true);
    ///
    ///     ctx.yield_one().await;// wait next update.
    ///
    ///     // we are in the next update now, the variable value is new.
    ///     assert_eq!(Some(true), foo_var.copy_new(&ctx));
    ///
    ///     ctx.update().await;// force next update and wait.
    ///
    ///     // we are in the requested update, variable value is no longer new.
    ///     assert_eq!(None, foo_var.copy_new(&ctx));
    /// })
    /// # );
    /// ```
    ///
    /// In the example above, the variable assign causes an app update so `yield_one` processes it immediately,
    /// but the second `.await` needs to cause an update if we don't want to depend on another part of the app
    /// to awake.
    pub async fn update(&self) {
        self.with(|c| c.updates.update());
        self.yield_one().await
    }

    /// Id of the window that owns the context widget.
    pub fn window_id(&self) -> WindowId {
        self.with(|ctx| ctx.path.window_id())
    }

    /// Id of the context widget.
    pub fn widget_id(&self) -> WidgetId {
        self.with(|ctx| ctx.path.widget_id())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::app::App;

    use super::*;

    #[test]
    #[should_panic(expected = "already in `AppContextMut::with`, cannot borrow `&mut AppContext` twice")]
    fn context_reentry() {
        let mut app = App::default().run_headless(false);

        let (scope, ctx) = AppContextScope::new();
        let ctx_a = Rc::new(ctx);
        let ctx_b = Rc::clone(&ctx_a);

        scope.with(&mut app.ctx(), move || {
            ctx_a.with(move |a| {
                ctx_b.with(move |b| {
                    let _invalid: (&mut AppContext, &mut AppContext) = (a, b);
                })
            })
        });
    }
}
