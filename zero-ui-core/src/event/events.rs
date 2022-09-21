use std::{mem, rc::Rc};

use crate::{
    app::AppEventSender,
    context::AppContext,
    crate_util::{Handle, HandleOwner, WeakHandle},
    handler::{AppHandler, AppHandlerArgs, AppWeakHandle},
    var::Vars,
};

use super::*;

struct OnEventHandler {
    handle: HandleOwner<()>,
    handler: Box<dyn FnMut(&mut AppContext, &mut EventUpdate, &dyn AppWeakHandle)>,
}

/// Represents an app context event handler created by [`Events::on_event`] or [`Events::on_pre_event`].
///
/// Drop all clones of this handle to drop the handler, or call [`unsubscribe`](Self::unsubscribe) to drop the handle
/// without dropping the handler.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
#[must_use = "the event handler unsubscribes if the handle is dropped"]
pub struct OnEventHandle(Handle<()>);
impl OnEventHandle {
    fn new() -> (HandleOwner<()>, OnEventHandle) {
        let (owner, handle) = Handle::new(());
        (owner, OnEventHandle(handle))
    }

    /// Create a handle to nothing, the handle always in the *unsubscribed* state.
    ///
    /// Note that `Option<OnEventHandle>` takes up the same space as `OnEventHandle` and avoids an allocation.
    pub fn dummy() -> Self {
        assert_non_null!(OnEventHandle);
        OnEventHandle(Handle::dummy(()))
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
    pub fn downgrade(&self) -> WeakOnEventHandle {
        WeakOnEventHandle(self.0.downgrade())
    }
}

/// Weak [`OnEventHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakOnEventHandle(WeakHandle<()>);
impl WeakOnEventHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Gets the strong handle if it is still subscribed.
    pub fn upgrade(&self) -> Option<OnEventHandle> {
        self.0.upgrade().map(OnEventHandle)
    }
}

thread_singleton!(SingletonEvents);

type BufferEntry = Box<dyn Fn(&EventUpdate) -> Retain>;

/// Access to application events.
///
/// An instance of this struct is available in [`AppContext`] and derived contexts.
pub struct Events {
    app_event_sender: AppEventSender,

    updates: Vec<EventUpdate>,

    pre_buffers: Vec<BufferEntry>,
    buffers: Vec<BufferEntry>,
    pre_handlers: Vec<OnEventHandler>,
    pos_handlers: Vec<OnEventHandler>,

    commands: Vec<Command>,

    _singleton: SingletonEvents,
}
impl Events {
    /// If an instance of `Events` already exists in the  current thread.
    pub(crate) fn instantiated() -> bool {
        SingletonEvents::in_use()
    }

    /// Produces the instance of `Events`. Only a single
    /// instance can exist in a thread at a time, panics if called
    /// again before dropping the previous instance.
    pub(crate) fn instance(app_event_sender: AppEventSender) -> Self {
        Events {
            app_event_sender,
            updates: vec![],
            pre_buffers: vec![],
            buffers: vec![],
            pre_handlers: vec![],
            pos_handlers: vec![],
            commands: vec![],
            _singleton: SingletonEvents::assert_new("Events"),
        }
    }

    /// Schedules the raw event update.
    pub fn notify(&mut self, update: EventUpdate) {
        self.updates.push(update);
    }

    pub(crate) fn register_command(&mut self, command: Command) {
        if self.commands.iter().any(|c| c == &command) {
            panic!("command `{command:?}` is already registered")
        }
        self.commands.push(command);
    }

    /// Creates an event buffer that listens to `E`. The event updates are pushed as soon as possible, before
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the buffer to stop listening.
    pub fn pre_buffer<A: EventArgs>(&mut self, event: Event<A>) -> EventBuffer<A> {
        Self::push_buffer(&mut self.pre_buffers, event)
    }

    /// Creates an event buffer that listens to `E`. The event updates are pushed only after
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the buffer to stop listening.
    pub fn buffer<A: EventArgs>(&mut self, event: Event<A>) -> EventBuffer<A> {
        Self::push_buffer(&mut self.buffers, event)
    }

    fn push_buffer<A: EventArgs>(buffers: &mut Vec<BufferEntry>, event: Event<A>) -> EventBuffer<A> {
        let buf = EventBuffer::never(event);
        let weak = Rc::downgrade(&buf.queue);
        buffers.push(Box::new(move |update| {
            let mut retain = false;
            if let Some(rc) = weak.upgrade() {
                if let Some(args) = event.on(update) {
                    rc.borrow_mut().push_back(args.clone());
                }
                retain = true;
            }
            retain
        }));
        buf
    }

    /// Creates a sender that can raise an event from other threads and without access to [`Events`].
    pub fn sender<A>(&mut self, event: Event<A>) -> EventSender<A>
    where
        A: EventArgs + Send,
    {
        EventSender {
            sender: self.app_event_sender.clone(),
            event,
        }
    }

    /// Creates a channel that can listen to event from another thread. The event updates are sent as soon as possible, before
    /// the UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the receiver to stop listening.
    pub fn pre_receiver<A>(&mut self, event: Event<A>) -> EventReceiver<A>
    where
        A: EventArgs + Send,
    {
        Self::push_receiver(&mut self.pre_buffers, event)
    }

    /// Creates a channel that can listen to event from another thread. The event updates are sent only after the
    /// UI and [`on_event`](Self::on_event) are notified.
    ///
    /// Drop the receiver to stop listening.
    pub fn receiver<A>(&mut self, event: Event<A>) -> EventReceiver<A>
    where
        A: EventArgs + Send,
    {
        Self::push_receiver(&mut self.buffers, event)
    }

    fn push_receiver<A>(buffers: &mut Vec<BufferEntry>, event: Event<A>) -> EventReceiver<A>
    where
        A: EventArgs + Send,
    {
        let (sender, receiver) = flume::unbounded();

        buffers.push(Box::new(move |update| {
            let mut retain = true;
            if let Some(args) = event.on(update) {
                retain = sender.send(args.clone()).is_ok();
            }
            retain
        }));

        EventReceiver { receiver, event }
    }

    /// Creates a preview event handler.
    ///
    /// The event `handler` is called for every update of `E` that has not stopped [`propagation`](EventArgs::propagation).
    /// The handler is called before UI handlers and [`on_event`](Self::on_event) handlers, it is called after all previous registered
    /// preview handlers.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::handler::app_hn;
    /// # use zero_ui_core::focus::{FOCUS_CHANGED_EVENT, FocusChangedArgs};
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handle = ctx.events.on_pre_event(FOCUS_CHANGED_EVENT, app_hn!(|_ctx, args: &FocusChangedArgs, _| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
    /// # }
    /// ```
    /// The example listens to all `FOCUS_CHANGED_EVENT` events, independent of widget context and before all UI handlers.
    ///
    /// # Handlers
    ///
    /// the event handler can be any type that implements [`AppHandler`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`app_hn!`], [`async_app_hn!`],
    /// [`app_hn_once!`] and [`async_app_hn_once!`].
    ///
    /// ## Async
    ///
    /// Note that for async handlers only the code before the first `.await` is called in the *preview* moment, code after runs in
    /// subsequent event updates, after the event has already propagated, so stopping [`propagation`](EventArgs::propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn_once!`]: crate::handler::async_app_hn_once!
    pub fn on_pre_event<A, H>(&mut self, event: Event<A>, handler: H) -> OnEventHandle
    where
        A: EventArgs,
        H: AppHandler<A>,
    {
        Self::push_event_handler(&mut self.pre_handlers, event, true, handler)
    }

    /// Creates an event handler.
    ///
    /// The event `handler` is called for every update of `E` that has not stopped [`propagation`](EventArgs::propagation).
    /// The handler is called after all [`on_pre_event`],(Self::on_pre_event) all UI handlers and all [`on_event`](Self::on_event) handlers
    /// registered before this one.
    ///
    /// Returns a [`OnEventHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::handler::app_hn;
    /// # use zero_ui_core::focus::{FOCUS_CHANGED_EVENT, FocusChangedArgs};
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handle = ctx.events.on_event(FOCUS_CHANGED_EVENT, app_hn!(|_ctx, args: &FocusChangedArgs, _| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
    /// # }
    /// ```
    /// The example listens to all `FOCUS_CHANGED_EVENT` events, independent of widget context, after the UI was notified.
    ///
    /// # Handlers
    ///
    /// the event handler can be any type that implements [`AppHandler`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`app_hn!`], [`async_app_hn!`],
    /// [`app_hn_once!`] and [`async_app_hn_once!`].
    ///
    /// ## Async
    ///
    /// Note that for async handlers only the code before the first `.await` is called in the *preview* moment, code after runs in
    /// subsequent event updates, after the event has already propagated, so stopping [`propagation`](EventArgs::propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn_once!`]: crate::handler::async_app_hn_once!
    pub fn on_event<A, H>(&mut self, event: Event<A>, handler: H) -> OnEventHandle
    where
        A: EventArgs,
        H: AppHandler<A>,
    {
        Self::push_event_handler(&mut self.pos_handlers, event, false, handler)
    }

    fn push_event_handler<A, H>(handlers: &mut Vec<OnEventHandler>, event: Event<A>, is_preview: bool, mut handler: H) -> OnEventHandle
    where
        A: EventArgs,
        H: AppHandler<A>,
    {
        let (handle_owner, handle) = OnEventHandle::new();
        let handler = move |ctx: &mut AppContext, update: &mut EventUpdate, handle: &dyn AppWeakHandle| {
            if let Some(args) = event.on(update) {
                if !args.propagation().is_stopped() {
                    handler.event(ctx, args, &AppHandlerArgs { handle, is_preview });
                }
            }
        };
        handlers.push(OnEventHandler {
            handle: handle_owner,
            handler: Box::new(handler),
        });
        handle
    }

    pub(crate) fn has_pending_updates(&mut self) -> bool {
        !self.updates.is_empty()
    }

    #[must_use]
    pub(crate) fn apply_updates(&mut self, vars: &Vars) -> Vec<EventUpdate> {
        let _s = tracing::trace_span!("Events").entered();
        for command in &self.commands {
            command.update_state(vars);
        }
        self.updates.drain(..).collect()
    }

    pub(crate) fn on_pre_events(ctx: &mut AppContext, update: &mut EventUpdate) {
        ctx.events.pre_buffers.retain(|buf| buf(update));

        let mut handlers = mem::take(&mut ctx.events.pre_handlers);
        Self::notify_retain(&mut handlers, ctx, update);
        handlers.extend(mem::take(&mut ctx.events.pre_handlers));
        ctx.events.pre_handlers = handlers;
    }

    pub(crate) fn on_events(ctx: &mut AppContext, update: &mut EventUpdate) {
        let mut handlers = mem::take(&mut ctx.events.pos_handlers);
        Self::notify_retain(&mut handlers, ctx, update);
        handlers.extend(mem::take(&mut ctx.events.pos_handlers));
        ctx.events.pos_handlers = handlers;

        ctx.events.buffers.retain(|buf| buf(update));
    }

    fn notify_retain(handlers: &mut Vec<OnEventHandler>, ctx: &mut AppContext, update: &mut EventUpdate) {
        handlers.retain_mut(|e| {
            !e.handle.is_dropped() && {
                (e.handler)(ctx, update, &e.handle.weak_handle());
                !e.handle.is_dropped()
            }
        });
    }

    /// Commands that had handles generated in this app.
    ///
    /// When [`Command::new_handle`] is called for the first time in an app, the command gets registered here.
    ///
    /// [`Command::new_handle`]: crate::event::Command::new_handle
    pub fn commands(&self) -> impl Iterator<Item = Command> + '_ {
        self.commands.iter().copied()
    }
}
impl Drop for Events {
    fn drop(&mut self) {
        for cmd in &self.commands {
            cmd.on_exit();
        }
    }
}

/// Represents a type that can provide access to [`Events`] inside the window of function call.
///
/// This is used to make event notification less cumbersome to use, it is implemented to all sync and async context types
/// and [`Events`] it-self.
///
/// # Examples
///
/// The example demonstrate how this `trait` simplifies calls to [`Event::notify`].
///
/// ```
/// # use zero_ui_core::{var::*, event::*, context::*};
/// # event_args! { pub struct BarArgs { pub msg: &'static str, .. fn delivery_list(&self, list: &mut UpdateDeliveryList) { list.search_all() } } }
/// # event! { pub static BAR_EVENT: BarArgs; }
/// # struct Foo { } impl Foo {
/// fn update(&mut self, ctx: &mut WidgetContext) {
///     BAR_EVENT.notify(ctx, BarArgs::now("we are not borrowing `ctx` so can use it directly"));
///
///    // ..
///    let services = &mut ctx.services;
///    BAR_EVENT.notify(ctx, BarArgs::now("we are partially borrowing `ctx` but not `ctx.vars` so we use that"));
/// }
///
/// async fn handler(&mut self, mut ctx: WidgetContextMut) {
///     BAR_EVENT.notify(&mut ctx, BarArgs::now("async contexts can also be used"));
/// }
/// # }
/// ```
pub trait WithEvents {
    /// Calls `action` with the [`Events`] reference.
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R;
}
impl WithEvents for Events {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self)
    }
}
impl<'a> WithEvents for crate::context::AppContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl<'a> WithEvents for crate::context::WindowContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl<'a> WithEvents for crate::context::WidgetContext<'a> {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.events)
    }
}
impl WithEvents for crate::context::AppContextMut {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        self.with(move |ctx| action(ctx.events))
    }
}
impl WithEvents for crate::context::WidgetContextMut {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        self.with(move |ctx| action(ctx.events))
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl WithEvents for crate::context::TestWidgetContext {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(&mut self.events)
    }
}
impl WithEvents for crate::app::HeadlessApp {
    fn with_events<R, A: FnOnce(&mut Events) -> R>(&mut self, action: A) -> R {
        action(self.ctx().events)
    }
}

type Retain = bool;
