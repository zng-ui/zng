//! Context information for app extensions, windows and widgets.

use crate::{event::Events, service::Services, units::*, var::Vars, window::WindowId, WidgetId};
use retain_mut::RetainMut;
use std::ops::DerefMut;
use std::{cell::Cell, fmt, mem, ops::Deref, ptr, rc::Rc, time::Instant};

#[doc(inline)]
pub use crate::state::*;

use crate::app::AppEventSender;
use crate::crate_util::{Handle, HandleOwner, RunOnDrop};
use crate::event::BoxedEventUpdate;
use crate::handler::{self, AppHandler, AppHandlerArgs, AppWeakHandle};
#[doc(inline)]
pub use crate::state_key;
use crate::task;
use crate::timer::Timers;
use crate::widget_info::UpdateMask;
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
    next_updates: UpdateMask,
    current: UpdateMask,
    update: bool,
    layout: bool,
    l_updates: LayoutUpdates,

    pre_handlers: Vec<UpdateHandler>,
    pos_handlers: Vec<UpdateHandler>,
}
impl Updates {
    fn new(event_sender: AppEventSender) -> Self {
        Updates {
            event_sender,
            next_updates: UpdateMask::none(),
            current: UpdateMask::none(),
            update: false,
            layout: false,
            l_updates: LayoutUpdates {
                render: false,
                window_updates: WindowUpdates::default(),
            },

            pre_handlers: vec![],
            pos_handlers: vec![],
        }
    }

    /// Create an [`AppEventSender`] that can be used to awake the app and send app events.
    #[inline]
    pub fn sender(&self) -> AppEventSender {
        self.event_sender.clone()
    }

    /// Reference a mask that represents all the variable or other update sources
    /// that are updating during this call of [`UiNode::update`].
    ///
    /// Note that this value is only valid in [`UiNode::update`] and is used by widget roots to optimize the call to update.
    ///
    /// [`UiNode::update`]: crate::UiNode::update
    #[inline]
    pub fn current(&self) -> &UpdateMask {
        &self.current
    }

    /// Schedules an update.
    #[inline]
    pub fn update(&mut self, mask: UpdateMask) {
        // tracing::trace!("requested `update`");
        self.next_updates |= mask;
        self.update = true;
    }

    /// Schedules an update that only affects the app extensions.
    ///
    /// This is the equivalent of calling [`update`] with [`UpdateMask::none`].
    ///
    /// [`update`]: Self::update
    #[inline]
    pub fn update_ext(&mut self) {
        self.update(UpdateMask::none());
    }

    /// Gets `true` if an update was requested.
    #[inline]
    pub fn update_requested(&self) -> bool {
        self.update
    }

    /// Schedules a info tree rebuild, layout and render.
    #[inline]
    pub fn info_layout_and_render(&mut self) {
        self.info();
        self.layout();
        self.render();
    }

    /// Schedules subscriptions aggregation, layout and render.
    #[inline]
    pub fn subscriptions_layout_and_render(&mut self) {
        self.subscriptions();
        self.layout();
        self.render();
    }

    /// Schedules a layout and render update.
    #[inline]
    pub fn layout_and_render(&mut self) {
        self.layout();
        self.render();
    }

    /// Schedules a layout update for the parent window.
    #[inline]
    pub fn layout(&mut self) {
        // tracing::trace!("requested `layout`");
        self.layout = true;
        self.l_updates.window_updates.layout = true;
    }

    /// Gets `true` if a layout update is scheduled.
    #[inline]
    pub fn layout_requested(&self) -> bool {
        self.layout
    }

    /// Flags a widget tree info rebuild and subscriptions aggregation for the parent window.
    ///
    /// The window will call [`UiNode::info`] as soon as the current UI node method finishes,
    /// requests outside windows are ignored.
    ///
    /// [`UiNode::info`]: crate::UiNode::info
    #[inline]
    pub fn info(&mut self) {
        // tracing::trace!("requested `info`");
        self.l_updates.window_updates.info = true;
        self.l_updates.window_updates.subscriptions = true;
    }

    /// Flag a subscriptions aggregation for the parent window.
    ///
    /// The window will call [`UiNode::subscriptions`] as soon as the current UI node method finishes,
    /// requests outside windows are ignored, widgets also call and cache subscriptions as soon as they receive this flag.
    ///
    /// [`UiNode::subscriptions`]: crate::UiNode::subscriptions
    #[inline]
    pub fn subscriptions(&mut self) {
        // tracing::trace!("requested `subscriptions`");
        self.l_updates.window_updates.subscriptions = true;
    }

    /// Gets `true` if a widget info rebuild is scheduled.
    #[inline]
    pub fn info_requested(&self) -> bool {
        self.l_updates.window_updates.info
    }

    /// Gets `true` if a widget info rebuild or subscriptions aggregation was requested for the parent window.
    #[inline]
    pub fn subscriptions_requested(&self) -> bool {
        self.l_updates.window_updates.subscriptions
    }

    /// Schedules a new full frame for the parent window.
    #[inline]
    pub fn render(&mut self) {
        // tracing::trace!("requested `render`");
        self.l_updates.render();
    }

    /// Returns `true` if a new frame or frame update is scheduled.
    #[inline]
    pub fn render_requested(&self) -> bool {
        self.l_updates.render_requested()
    }

    /// Schedule a frame update for the parent window.
    ///
    /// Note that if another widget requests a full [`render`] this update will not run.
    ///
    /// [`render`]: Updates::render
    #[inline]
    pub fn render_update(&mut self) {
        // tracing::trace!("requested `render_update`");
        self.l_updates.render_update();
    }

    /// Schedule an *once* handler to run when these updates are applied.
    ///
    /// The callback is any of the *once* [`AppHandler`], including async handlers. You can use [`app_hn_once!`](handler::app_hn_once!)
    /// or [`async_app_hn_once!`](handler::async_app_hn_once!) to declare the closure. If the handler is async and does not finish in
    /// one call it is scheduled to update in *preview* updates.
    pub fn run<H: AppHandler<UpdateArgs> + handler::marker::OnceHn>(&mut self, handler: H) -> OnUpdateHandle {
        self.update = true; // in case of this was called outside of an update.
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

    fn enter_window_ctx(&mut self) {
        self.l_updates.window_updates = WindowUpdates::default();
    }

    fn take_win_updates(&mut self) -> WindowUpdates {
        mem::take(&mut self.l_updates.window_updates)
    }

    fn take_updates(&mut self) -> (bool, bool, bool) {
        self.current = mem::take(&mut self.next_updates);
        (
            mem::take(&mut self.update),
            mem::take(&mut self.layout),
            mem::take(&mut self.l_updates.render),
        )
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
impl Deref for Updates {
    type Target = LayoutUpdates;

    fn deref(&self) -> &Self::Target {
        &self.l_updates
    }
}
impl DerefMut for Updates {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.l_updates
    }
}

/// Subsect of [`Updates`] that is accessible in [`LayoutContext`].
pub struct LayoutUpdates {
    render: bool,
    window_updates: WindowUpdates,
}
impl LayoutUpdates {
    /// Schedules a new frame for the parent window.
    #[inline]
    pub fn render(&mut self) {
        self.render = true;
        self.window_updates.render = WindowRenderUpdate::Render;
    }

    /// Schedule a frame update for the parent window.
    ///
    /// Note that if another widget requests a full [`render`] this update will not run.
    ///
    /// [`render`]: LayoutUpdates::render
    #[inline]
    pub fn render_update(&mut self) {
        self.render = true;
        self.window_updates.render |= WindowRenderUpdate::RenderUpdate;
    }

    /// Returns `true` if a new frame or frame update is scheduled.
    #[inline]
    pub fn render_requested(&self) -> bool {
        self.render
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
        let events = self.events.apply_updates(&self.vars);
        self.vars.apply_updates(&mut self.updates);

        let (update, layout, render) = self.updates.take_updates();

        ContextUpdates {
            events,
            update,
            layout,
            render,
        }
    }

    /// Returns next timer tick time.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.timers.next_deadline(&self.vars)
    }

    /// Update timers, returns next timer tick time.
    #[must_use]
    pub fn update_timers(&mut self) -> Option<Instant> {
        self.timers.apply_updates(&self.vars)
    }

    /// If a call to `apply_updates` will generate updates (ignoring timers).
    #[must_use]
    pub fn has_pending_updates(&mut self) -> bool {
        self.updates.update_requested()
            || self.updates.layout_requested()
            || self.updates.render_requested()
            || self.vars.has_pending_updates()
            || self.events.has_pending_updates()
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
    pub fn window_mode(&mut self) -> WindowMode {
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
    ///
    /// Returns the function result and
    #[inline(always)]
    pub fn window_context<R>(
        &mut self,
        window_id: WindowId,
        window_mode: WindowMode,
        window_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut WindowContext) -> R,
    ) -> (R, WindowUpdates) {
        self.updates.enter_window_ctx();

        let mut update_state = StateMap::new();

        let r = f(&mut WindowContext {
            window_id: &window_id,
            window_mode: &window_mode,
            app_state: self.app_state,
            window_state: &mut window_state.0,
            update_state: &mut update_state,
            vars: self.vars,
            events: self.events,
            services: self.services,
            timers: self.timers,
            updates: self.updates,
        });

        (r, self.updates.take_win_updates())
    }
}

/// A window context.
pub struct WindowContext<'a> {
    /// Id of the context window.
    pub window_id: &'a WindowId,

    /// Window mode, headed or not, renderer or not.
    pub window_mode: &'a WindowMode,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

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

    /// Run a function `f` in the info context of a widget.
    #[inline(always)]
    pub fn info_context<R>(&mut self, widget_id: WidgetId, widget_state: &OwnedStateMap, f: impl FnOnce(&mut InfoContext) -> R) -> R {
        f(&mut InfoContext {
            path: &mut WidgetContextPath::new(*self.window_id, widget_id),
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &widget_state.0,
            update_state: self.update_state,
            vars: self.vars,
        })
    }

    /// Runs a function `f` in the layout context of a widget.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn layout_context<R>(
        &mut self,
        font_size: Px,
        scale_factor: Factor,
        screen_ppi: f32,
        viewport_size: PxSize,
        metrics_diff: LayoutMask,
        widget_id: WidgetId,
        widget_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        f(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, viewport_size, font_size)
                .with_screen_ppi(screen_ppi)
                .with_diff(metrics_diff),

            path: &mut WidgetContextPath::new(*self.window_id, widget_id),

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &mut widget_state.0,
            update_state: self.update_state,

            vars: self.vars,

            updates: self.updates,
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
use crate::widget_info::{BoundsRect, WidgetInfoBuilder, WidgetInfoTree, WidgetRendered, WidgetSubscriptions};
#[cfg(any(test, doc, feature = "test_util"))]
impl TestWidgetContext {
    /// Gets a new [`TestWidgetContext`] instance. Panics is another instance is alive in the current thread
    /// or if an app is running in the current thread.
    pub fn new() -> Self {
        if crate::app::App::is_running() {
            panic!("only one `TestWidgetContext` or app is allowed per thread")
        }

        let (sender, receiver) = AppEventSender::new();
        let window_id = WindowId::new_unique();
        let root_id = WidgetId::new_unique();
        Self {
            window_id,
            root_id,
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

    /// Calls `action` in a fake info context.
    pub fn info_context<R>(&mut self, action: impl FnOnce(&mut InfoContext) -> R) -> R {
        action(&mut InfoContext {
            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            app_state: &self.app_state.0,
            window_state: &self.window_state.0,
            widget_state: &self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
        })
    }

    /// Builds a info tree.
    pub fn info_tree<R>(
        &mut self,
        root_bounds: BoundsRect,
        rendered: WidgetRendered,
        action: impl FnOnce(&mut InfoContext, &mut WidgetInfoBuilder) -> R,
    ) -> (WidgetInfoTree, R) {
        let mut builder = WidgetInfoBuilder::new(self.window_id, self.root_id, root_bounds, rendered, None);
        let r = self.info_context(|ctx| action(ctx, &mut builder));
        let (t, _) = builder.finalize();
        (t, r)
    }

    /// Aggregate subscriptions.
    pub fn subscriptions<R>(&mut self, action: impl FnOnce(&mut InfoContext, &mut WidgetSubscriptions) -> R) -> (WidgetSubscriptions, R) {
        let mut subs = WidgetSubscriptions::new();
        let r = self.info_context(|ctx| action(ctx, &mut subs));
        (subs, r)
    }

    /// Calls `action` in a fake layout context.
    #[allow(clippy::too_many_arguments)]
    pub fn layout_context<R>(
        &mut self,
        root_font_size: Px,
        font_size: Px,
        viewport_size: PxSize,
        scale_factor: Factor,
        screen_ppi: f32,
        metrics_diff: LayoutMask,
        action: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        action(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, viewport_size, root_font_size)
                .with_font_size(font_size)
                .with_screen_ppi(screen_ppi)
                .with_diff(metrics_diff),

            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            app_state: &mut self.app_state.0,
            window_state: &mut self.window_state.0,
            widget_state: &mut self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
            updates: &mut self.updates,
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
    /// Returns the [`WindowUpdates`] and [`ContextUpdates`] a full app and window would
    /// use to update the application.
    pub fn apply_updates(&mut self) -> (WindowUpdates, ContextUpdates) {
        let win_updt = self.updates.take_win_updates();

        for ev in self.receiver.try_iter() {
            match ev {
                crate::app::AppEvent::ViewEvent(_) => unimplemented!(),
                crate::app::AppEvent::Event(ev) => self.events.notify_app_event(ev),
                crate::app::AppEvent::Var => self.vars.receive_sended_modify(),
                crate::app::AppEvent::Update(mask) => self.updates.update(mask),
                crate::app::AppEvent::ResumeUnwind(p) => std::panic::resume_unwind(p),
            }
        }
        let events = self.events.apply_updates(&self.vars);
        self.vars.apply_updates(&mut self.updates);
        let (update, layout, render) = self.updates.take_updates();

        (
            win_updt,
            ContextUpdates {
                events,
                update,
                layout,
                render,
            },
        )
    }

    /// Update timers, returns next timer tick time.
    pub fn update_timers(&mut self) -> Option<Instant> {
        self.timers.apply_updates(&self.vars)
    }

    /// Force set the current update mask.
    pub fn set_current_update(&mut self, current: UpdateMask) {
        self.updates.current = current;
    }
}

/// Updates that must be reacted by an app context owner.
#[derive(Debug, Default)]
pub struct ContextUpdates {
    /// Events to notify.
    ///
    /// When this is not empty [`update`](Self::update) is `true`.
    pub events: Vec<BoxedEventUpdate>,

    /// Update requested.
    pub update: bool,

    /// Layout requested.
    pub layout: bool,

    /// Full frame or frame update requested.
    pub render: bool,
}
impl ContextUpdates {
    /// If has events, update, layout or render was requested.
    #[inline]
    pub fn has_updates(&self) -> bool {
        self.update || self.layout || self.render
    }
}
impl std::ops::BitOrAssign for ContextUpdates {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.events.extend(rhs.events);
        self.update |= rhs.update;
        self.layout |= rhs.layout;
        self.render |= rhs.render;
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

/// Info, Layout or render updates that where requested by the content of a window.
///
/// Unlike the general updates, layout and render can be optimized to only apply if
/// the window content requested it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WindowUpdates {
    /// Info tree rebuild requested.
    ///
    /// Windows should call [`UiNode::info`] to rebuild the info tree as soon as they receive this flag.
    ///
    /// [`UiNode::info`]: crate::UiNode::info
    pub info: bool,
    /// Subscriptions re-count requested.
    ///
    /// Windows should call [`UiNode::subscriptions`] to aggregate the subscriptions masks
    /// as soon as they receive this flag.
    ///
    /// [`UiNode::subscriptions`]: crate::UiNode::subscriptions
    pub subscriptions: bool,

    /// Layout requested.
    pub layout: bool,
    /// Full frame or frame update requested.
    pub render: WindowRenderUpdate,
}
impl WindowUpdates {
    /// No updates, this the default value.
    pub fn none() -> Self {
        Self::default()
    }

    /// Update layout and render frame.
    pub fn all() -> Self {
        WindowUpdates {
            info: true,
            subscriptions: true,
            layout: true,
            render: WindowRenderUpdate::Render,
        }
    }

    /// Info tree rebuild and subscriptions only.
    pub fn info() -> Self {
        WindowUpdates {
            info: true,
            subscriptions: true,
            layout: false,
            render: WindowRenderUpdate::None,
        }
    }

    /// Subscriptions aggregation only.
    pub fn subscriptions() -> Self {
        WindowUpdates {
            info: false,
            subscriptions: true,
            layout: false,
            render: WindowRenderUpdate::None,
        }
    }

    /// Update layout only.
    pub fn layout() -> Self {
        WindowUpdates {
            info: false,
            subscriptions: false,
            layout: true,
            render: WindowRenderUpdate::None,
        }
    }

    /// Update render only.
    pub fn render() -> Self {
        WindowUpdates {
            info: false,
            subscriptions: false,
            layout: false,
            render: WindowRenderUpdate::Render,
        }
    }

    /// Update render-update only.
    pub fn render_update() -> Self {
        WindowUpdates {
            info: false,
            subscriptions: false,
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
impl std::ops::BitOrAssign for WindowUpdates {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.info |= rhs.info;
        self.subscriptions |= rhs.subscriptions;
        self.layout |= rhs.layout;
        self.render |= rhs.render;
    }
}
impl std::ops::BitOr for WindowUpdates {
    type Output = Self;

    #[inline]
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
    #[inline]
    pub fn is_render(self) -> bool {
        matches!(self, Self::Render)
    }

    /// If only frame update was requested.
    #[inline]
    pub fn is_render_update(self) -> bool {
        matches!(self, Self::RenderUpdate)
    }

    /// If no render was requested.
    #[inline]
    pub fn is_none(self) -> bool {
        matches!(self, Self::None)
    }
}
impl Default for WindowRenderUpdate {
    fn default() -> Self {
        WindowRenderUpdate::None
    }
}
impl std::ops::BitOrAssign for WindowRenderUpdate {
    #[inline]
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
    /// This state lives only for the current [`UiNode`] method call in all nodes
    /// of the window. You can use this to signal properties and event handlers from nodes that
    /// will be updated further then the current one.
    ///
    /// [`UiNode`]: crate::UiNode
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

    /// Returns `true` if the current widget is the window.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.widget_ids.len() == 1
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
    pub scale_factor: Factor,

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

    /// What metrics changed from the last layout in the same context.
    pub diff: LayoutMask,
}
impl LayoutMetrics {
    /// New root [`LayoutMetrics`].
    ///
    /// The `font_size` sets both font sizes, the initial PPI is `96.0`, you can use the builder style method and
    /// [`with_screen_ppi`] to set a different value.
    ///
    /// [`with_screen_ppi`]: LayoutMetrics::with_screen_ppi
    pub fn new(scale_factor: Factor, viewport_size: PxSize, font_size: Px) -> Self {
        LayoutMetrics {
            font_size,
            root_font_size: font_size,
            scale_factor,
            viewport_size,
            screen_ppi: 96.0,
            diff: LayoutMask::all(),
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

    /// Computes the full diff mask of changes in a [`UiNode::measure`].
    ///
    /// Note that the node owner must store the previous available size, this
    /// method updates the `prev_available_size` to the new `available_size` after the comparison.
    ///
    /// [`UiNode::measure`]: crate::UiNode::measure
    #[inline]
    pub fn measure_diff(
        &self,
        prev_available_size: &mut Option<AvailableSize>,
        available_size: AvailableSize,
        default_is_new: bool,
    ) -> LayoutMask {
        self.node_diff(prev_available_size, available_size, default_is_new)
    }

    /// Computes the full diff mask of changes in a [`UiNode::arrange`].
    ///
    /// Note that the node owner must store the previous final size, this method
    /// updates the `prev_final_size` to the new `final_size` after the comparison.
    ///
    /// [`UiNode::arrange`]: crate::UiNode::arrange
    #[inline]
    pub fn arrange_diff(&self, prev_final_size: &mut Option<PxSize>, final_size: PxSize, default_is_new: bool) -> LayoutMask {
        self.node_diff(prev_final_size, final_size, default_is_new)
    }

    fn node_diff<A: PartialEq>(&self, prev: &mut Option<A>, new: A, default_is_new: bool) -> LayoutMask {
        let mut diff = self.diff;
        if let Some(p) = prev {
            if *p != new {
                diff |= LayoutMask::AVAILABLE_SIZE;
                *p = new;
            }
        } else {
            diff |= LayoutMask::AVAILABLE_SIZE;
            *prev = Some(new);
        }
        if default_is_new {
            diff |= LayoutMask::DEFAULT_VALUE;
        }
        diff
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
    pub fn with_scale_factor(mut self, scale_factor: Factor) -> Self {
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

    /// Sets the [`diff`].
    ///
    /// [`diff`]: Self::diff
    #[inline]
    pub fn with_diff(mut self, diff: LayoutMask) -> Self {
        self.diff = diff;
        self
    }
}

/// A widget layout context.
///
/// This type dereferences to [`LayoutMetrics`].
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
    ///
    /// Note that if you assign a variable any frame request is deferred and the app loop goes back
    /// to the [`UiNode::update`] cycle.
    ///
    /// [`UiNode::update`]: crate::UiNode::update
    pub vars: &'a Vars,

    /// Updates that can be requested in layout context.
    pub updates: &'a mut LayoutUpdates,
}
impl<'a> Deref for LayoutContext<'a> {
    type Target = LayoutMetrics;

    fn deref(&self) -> &Self::Target {
        self.metrics
    }
}
impl<'a> LayoutContext<'a> {
    /// Runs a function `f` in a layout context that has the new computed font size.
    ///
    /// The `font_size_new` flag indicates if the `font_size` value changed from the previous layout call.
    #[inline(always)]
    pub fn with_font_size<R>(&mut self, font_size: Px, font_size_new: bool, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        let mut diff = self.metrics.diff;
        diff.set(LayoutMask::FONT_SIZE, font_size_new);
        f(&mut LayoutContext {
            metrics: &self.metrics.clone().with_font_size(font_size).with_diff(diff),

            path: self.path,

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state,

            vars: self.vars,
            updates: self.updates,
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
                updates: self.updates,
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

/// A widget info context.
pub struct InfoContext<'a> {
    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// Read-only access to the state that lives for the duration of the application.
    pub app_state: &'a StateMap,

    /// Read-only access to the state that lives for the duration of the window.
    pub window_state: &'a StateMap,

    /// Read-only access to the state that lives for the duration of the widget.
    pub widget_state: &'a StateMap,

    /// State that lives for the duration of the node tree rebuild or subscriptions aggregation call in the window.
    ///
    /// This state lives only for the call to [`UiNode::info`](crate::UiNode::info) or
    /// [`UiNode::subscriptions`](crate::UiNode::subscriptions) method call in all nodes of the window.
    /// You can use this to signal nodes that have not added info yet.
    pub update_state: &'a mut StateMap,

    /// Read-only access to variables.
    pub vars: &'a VarsRead,
}
impl<'a> InfoContext<'a> {
    /// Runs a function `f` in the info context of a widget.
    #[inline(always)]
    pub fn with_widget<R>(&mut self, widget_id: WidgetId, widget_state: &OwnedStateMap, f: impl FnOnce(&mut InfoContext) -> R) -> R {
        self.path.push(widget_id);
        let r = f(&mut InfoContext {
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
        self.with(|c| c.updates.update(UpdateMask::none()));
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
        // TODO handler update mask?
        self.with(|c| c.updates.update(UpdateMask::all()));
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
