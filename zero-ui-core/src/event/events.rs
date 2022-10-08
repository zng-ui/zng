use std::{mem, rc::Rc};

use crate::{
    app::AppEventSender,
    clone_move,
    context::AppContext,
    crate_util::{Handle, HandleOwner, WeakHandle},
    handler::{AppHandler, AppHandlerArgs, AppWeakHandle},
    var::Vars,
};

use super::*;

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
    /// When [`Command::subscribe`] is called for the first time in an app, the command gets registered here.
    ///
    /// [`Command::subscribe`]: crate::event::Command::subscribe
    pub fn commands(&self) -> impl Iterator<Item = Command> + '_ {
        self.commands.iter().copied()
    }

    pub(crate) fn push_once_action(&mut self, action: Box<dyn FnOnce(&mut AppContext, &EventUpdate)>, is_preview: bool) -> _ {
        if is_preview {
            self.pre_actions.push(action);
        } else {
            self.pos_actions.push(action);
        }
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
