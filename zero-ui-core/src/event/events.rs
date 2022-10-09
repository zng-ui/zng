use std::mem;

use crate::{app::AppEventSender, context::AppContext, var::Vars};

use super::*;

thread_singleton!(SingletonEvents);

/// Access to application events.
///
/// An instance of this struct is available in [`AppContext`] and derived contexts.
pub struct Events {
    app_event_sender: AppEventSender,

    updates: Vec<EventUpdate>,

    pre_actions: Vec<Box<dyn FnOnce(&mut AppContext, &EventUpdate)>>,
    pos_actions: Vec<Box<dyn FnOnce(&mut AppContext, &EventUpdate)>>,

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
            pre_actions: vec![],
            pos_actions: vec![],
            commands: vec![],
            _singleton: SingletonEvents::assert_new("Events"),
        }
    }

    /// Schedules the raw event update.
    pub fn notify(&mut self, update: EventUpdate) {
        self.updates.push(update);
    }

    pub(super) fn register_command(&mut self, command: Command) {
        if self.commands.iter().any(|c| c == &command) {
            panic!("command `{command:?}` is already registered")
        }
        self.commands.push(command);
    }

    pub(super) fn sender<A>(&mut self, event: Event<A>) -> EventSender<A>
    where
        A: EventArgs + Send,
    {
        EventSender {
            sender: self.app_event_sender.clone(),
            event,
        }
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
        let updates: Vec<_> = self.updates.drain(..).collect();
        for u in &updates {
            u.event.on_update(self, u);
        }
        updates
    }

    pub(crate) fn on_pre_events(ctx: &mut AppContext, update: &mut EventUpdate) {
        let mut actions = mem::take(&mut ctx.events.pre_actions);

        for action in actions.drain(..) {
            action(ctx, update);
        }

        actions.extend(mem::take(&mut ctx.events.pre_actions));
        ctx.events.pre_actions = actions;
    }

    pub(crate) fn on_events(ctx: &mut AppContext, update: &mut EventUpdate) {
        let mut actions = mem::take(&mut ctx.events.pre_actions);

        for action in actions.drain(..) {
            action(ctx, update);
        }

        actions.extend(mem::take(&mut ctx.events.pos_actions));
        ctx.events.pos_actions = actions;
    }

    /// Commands that had handles generated in this app.
    ///
    /// When [`Command::subscribe`] is called for the first time in an app, the command gets registered here.
    ///
    /// [`Command::subscribe`]: crate::event::Command::subscribe
    pub fn commands(&self) -> impl Iterator<Item = Command> + '_ {
        self.commands.iter().copied()
    }

    pub(crate) fn push_once_action(&mut self, action: Box<dyn FnOnce(&mut AppContext, &EventUpdate)>, is_preview: bool) {
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
