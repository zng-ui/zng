use std::{
    mem,
    ops::{Deref, DerefMut},
};

use crate::{
    app::AppEventSender,
    crate_util::{Handle, HandleOwner, WeakHandle},
    event::BoxedEventUpdate,
    handler::{self, AppHandler, AppHandlerArgs, AppWeakHandle},
    widget_info::UpdateMask,
};

#[allow(unused_imports)] // nightly
use retain_mut::RetainMut;

use super::{AppContext, UpdatesTrace};

/// Represents an [`on_pre_update`](Updates::on_pre_update) or [`on_update`](Updates::on_update) handler.
///
/// Drop all clones of this handle to drop the binding, or call [`perm`](Self::perm) to drop the handle
/// but keep the handler alive for the duration of the app.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[must_use = "dropping the handle unsubscribes update handler"]
pub struct OnUpdateHandle(Handle<()>);
impl OnUpdateHandle {
    fn new() -> (HandleOwner<()>, OnUpdateHandle) {
        let (owner, handle) = Handle::new(());
        (owner, OnUpdateHandle(handle))
    }

    /// Create a handle to nothing, the handle always in the *unsubscribed* state.
    pub fn dummy() -> Self {
        OnUpdateHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unsubscribe.
    ///
    /// The handler stays in memory for the duration of the app or until another handle calls [`unsubscribe`](Self::unsubscribe.)
    pub fn perm(self) {
        self.0.perm();
    }

    /// If another handle has called [`perm`](Self::perm).
    /// If `true` the var binding will stay active until the app shutdown, unless [`unsubscribe`](Self::unsubscribe) is called.
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
    pub(super) current: UpdateMask,
    update: bool,
    layout: bool,
    l_updates: LayoutUpdates,

    pre_handlers: Vec<UpdateHandler>,
    pos_handlers: Vec<UpdateHandler>,
}
impl Updates {
    pub(super) fn new(event_sender: AppEventSender) -> Self {
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
    pub fn sender(&self) -> AppEventSender {
        self.event_sender.clone()
    }

    /// Reference a mask that represents all the variable or other update sources
    /// that are updating during this call of [`UiNode::update`].
    ///
    /// Note that this value is only valid in [`UiNode::update`] and is used by widget roots to optimize the call to update.
    ///
    /// [`UiNode::update`]: crate::UiNode::update
    pub fn current(&self) -> &UpdateMask {
        &self.current
    }

    /// Schedules an update.
    pub fn update(&mut self, mask: UpdateMask) {
        UpdatesTrace::log_update();
        self.update_internal(mask);
    }

    pub(crate) fn update_internal(&mut self, mask: UpdateMask) {
        self.next_updates |= mask;
        self.update = true;
    }

    /// Schedules an update that only affects the app extensions.
    ///
    /// This is the equivalent of calling [`update`] with [`UpdateMask::none`].
    ///
    /// [`update`]: Self::update
    pub fn update_ext(&mut self) {
        self.update(UpdateMask::none());
    }

    pub(crate) fn update_ext_internal(&mut self) {
        self.update_internal(UpdateMask::none())
    }

    /// Gets `true` if an update was requested.
    pub fn update_requested(&self) -> bool {
        self.update
    }

    /// Schedules a info tree rebuild, layout and render.
    pub fn info_layout_and_render(&mut self) {
        self.info();
        self.layout();
        self.render();
    }

    /// Schedules subscriptions aggregation, layout and render.
    pub fn subscriptions_layout_and_render(&mut self) {
        self.subscriptions();
        self.layout();
        self.render();
    }

    /// Schedules a layout and render update.
    pub fn layout_and_render(&mut self) {
        self.layout();
        self.render();
    }

    /// Schedules a layout update for the parent window.
    pub fn layout(&mut self) {
        UpdatesTrace::log_layout();
        self.layout = true;
        self.l_updates.window_updates.layout = true;
    }

    /// Gets `true` if a layout update is scheduled.
    pub fn layout_requested(&self) -> bool {
        self.layout
    }

    /// Flags a widget tree info rebuild and subscriptions aggregation for the parent window.
    ///
    /// The window will call [`UiNode::info`] as soon as the current UI node method finishes,
    /// requests outside windows are ignored.
    ///
    /// [`UiNode::info`]: crate::UiNode::info
    pub fn info(&mut self) {
        // tracing::trace!("requested `info`");
        self.l_updates.window_updates.info = true;
        self.l_updates.window_updates.subscriptions = true;
    }

    /// Flag a subscriptions aggregation for the parent window.
    ///
    /// The window will call [`UiNode::subscriptions`] as soon as the current UI node method finishes,
    /// requests outside windows are ignored.
    ///
    /// [`UiNode::subscriptions`]: crate::UiNode::subscriptions
    pub fn subscriptions(&mut self) {
        // tracing::trace!("requested `subscriptions`");
        self.l_updates.window_updates.subscriptions = true;
    }

    /// Gets `true` if a widget info rebuild is scheduled.
    pub fn info_requested(&self) -> bool {
        self.l_updates.window_updates.info
    }

    /// Gets `true` if a widget info rebuild or subscriptions aggregation was requested for the parent window.
    pub fn subscriptions_requested(&self) -> bool {
        self.l_updates.window_updates.subscriptions
    }

    /// Schedules a new full frame for the parent window.
    pub fn render(&mut self) {
        // tracing::trace!("requested `render`");
        self.l_updates.render();
    }

    /// Returns `true` if a new frame or frame update is scheduled.
    pub fn render_requested(&self) -> bool {
        self.l_updates.render_requested()
    }

    /// Schedule a frame update for the parent window.
    ///
    /// Note that if another widget requests a full [`render`] this update will not run.
    ///
    /// [`render`]: Updates::render
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

    pub(super) fn enter_window_ctx(&mut self) {
        self.l_updates.window_updates = WindowUpdates::default();
    }

    pub(super) fn take_win_updates(&mut self) -> WindowUpdates {
        mem::take(&mut self.l_updates.window_updates)
    }

    pub(super) fn take_updates(&mut self) -> (bool, bool, bool) {
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
///
/// [`LayoutContext`]: crate::context::LayoutContext
pub struct LayoutUpdates {
    render: bool,
    window_updates: WindowUpdates,
}
impl LayoutUpdates {
    /// Schedules a new frame for the parent window.
    pub fn render(&mut self) {
        self.render = true;
        self.window_updates.render = WindowRenderUpdate::Render;
    }

    /// Schedule a frame update for the parent window.
    ///
    /// Note that if another widget requests a full [`render`] this update will not run.
    ///
    /// [`render`]: LayoutUpdates::render
    pub fn render_update(&mut self) {
        self.render = true;
        self.window_updates.render |= WindowRenderUpdate::RenderUpdate;
    }

    /// Returns `true` if a new frame or frame update is scheduled.
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
    pub fn has_updates(&self) -> bool {
        self.update || self.layout || self.render
    }
}
impl std::ops::BitOrAssign for ContextUpdates {
    fn bitor_assign(&mut self, rhs: Self) {
        self.events.extend(rhs.events);
        self.update |= rhs.update;
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
    fn bitor_assign(&mut self, rhs: Self) {
        self.info |= rhs.info;
        self.subscriptions |= rhs.subscriptions;
        self.layout |= rhs.layout;
        self.render |= rhs.render;
    }
}
impl std::ops::BitOr for WindowUpdates {
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
