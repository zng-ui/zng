use std::{
    any::Any,
    cell::{Cell, RefCell},
    fmt,
    marker::PhantomData,
    ops::Deref,
    thread::LocalKey,
    time::Instant,
};

use crate::{
    command::{CommandArgs, CommandMeta, CommandScope},
    context::{OwnedStateMap, StateMapMut},
    event::{EventArgs, EventDeliveryList, EventPropagationHandle, WithEvents},
    widget_info::EventSlot, crate_util::{FxHashMap},
};

#[macro_export]
macro_rules! event2 {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $EVENT:ident: $Args:path;
    )+) => {
        $(
            paste::paste! {
                #[doc(hidden)]
                std::thread_local! {
                    static [<$EVENT _LOCAL>] = $crate::event::EventData::new(std::stringifly!($EVENT));
                }
    
                $(#[$attr])*
                $vis static $EVENT: $crate::event::Event<$Args> = $crate::event::Event::new([<$EVENT _LOCAL>]);
            }
        )+
    }
}

/// Represents the [`Event::meta`]
pub enum EventMeta {}

#[doc(hidden)]
pub struct EventData {
    slot: EventSlot,
    name: &'static str,
}
impl EventData {
    #[doc(hidden)]
    pub fn new(name: &'static str) -> Self {
        EventData {
            slot: EventSlot::next(),
            name,
        }
    }

    fn slot(&self) -> EventSlot {
        self.slot
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

/// Unique identifier of an [`Event`] instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(usize);

/// Represents an event.
pub struct Event<E: EventArgs> {
    local: &'static LocalKey<EventData>,
    _args: PhantomData<E>,
}
impl<E: EventArgs> fmt::Debug for Event<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Event<{}>", self.name())
        } else {
            write!(f, "{}", self.name())
        }
    }
}
impl<E: EventArgs> Event<E> {
    #[doc(hidden)]
    pub fn new(local: &'static LocalKey<EventData>) -> Self {
        Event { local, _args: PhantomData }
    }

    /// Gets the event without the args type.
    pub fn as_any(&self) -> AnyEvent {
        AnyEvent { local: self.local }
    }

    /// Event ID.
    pub fn id(&self) -> EventId {
        EventId(self.local as *const _ as _)
    }

    /// Event slot in the app of the current thread.
    pub fn slot(&self) -> EventSlot {
        self.local.with(EventData::slot)
    }

    /// Display name.
    pub fn name(&self) -> &'static str {
        self.local.with(EventData::name)
    }

    /// Returns `true` if the update is for this event.
    pub fn has(&self, update: &EventUpdate) -> bool {
        self.id() == update.event_id
    }

    /// Get the event update args if the update is for this event.
    pub fn on<'a>(&self, update: &'a EventUpdate) -> Option<&'a E> {
        if self.id() == update.event_id {
            update.args.downcast_ref()
        } else {
            None
        }
    }

    /// Get the event update args if the update is for this event and propagation is not stopped.
    pub fn on_unhandled<'a>(&self, update: &'a EventUpdate) -> Option<&'a E> {
        self.on(update).filter(|a| a.propagation().is_stopped())
    }

    /// Calls `handler` if the update is for this event and propagation is not stopped, after the handler is called propagation is stopped.
    pub fn handle<R>(&self, update: &EventUpdate, handler: impl FnOnce(&E) -> R) -> Option<R> {
        if let Some(args) = self.on(update) {
            args.handle(handler)
        } else {
            None
        }
    }

    /// Create an event update for this event.
    pub fn new_update(&self, args: E) -> EventUpdate {
        EventUpdate {
            event_id: self.id(),
            event_slot: self.slot(),
            delivery_list: args.delivery_list(),
            timestamp: args.timestamp(),
            propagation: args.propagation().clone(),

            args: Box::new(args),
        }
    }

    /// Schedule an event update.
    pub fn notify<Ev: WithEvents>(&self, events: &mut Ev, args: E) {
        let update = self.new_update(args);
        events.with_events(|ev| {
            // ev.notify(event, args)
        })
    }
}
impl<E: EventArgs> Clone for Event<E> {
    fn clone(&self) -> Self {
        Self {
            local: self.local,
            _args: PhantomData,
        }
    }
}
impl<E: EventArgs> Copy for Event<E> {}
impl<E: EventArgs> PartialEq for Event<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
impl<E: EventArgs> Eq for Event<E> {}

/// Represents an [`Event`] without the args type.
#[derive(Clone, Copy)]
pub struct AnyEvent {
    local: &'static LocalKey<EventData>,
}
impl AnyEvent {
    /// Event ID.
    pub fn id(&self) -> EventId {
        EventId(self.local as *const _ as _)
    }

    /// Event slot in the app of the current thread.
    pub fn slot(&self) -> EventSlot {
        self.local.with(EventData::slot)
    }

    /// Display name.
    pub fn name(&self) -> &'static str {
        self.local.with(EventData::name)
    }

    /// Returns `true` is `self` is the type erased `event`.
    pub fn is<E: EventArgs>(&self, event: &Event<E>) -> bool {
        self == event
    }

    /// Returns `true` if the update is for this event.
    pub fn has(&self, update: &EventUpdate) -> bool {
        self.id() == update.event_id
    }
}
impl PartialEq for AnyEvent {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
impl Eq for AnyEvent {}
impl<E: EventArgs> PartialEq<AnyEvent> for Event<E> {
    fn eq(&self, other: &AnyEvent) -> bool {
        self.id() == other.id()
    }
}
impl<E: EventArgs> PartialEq<Event<E>> for AnyEvent {
    fn eq(&self, other: &Event<E>) -> bool {
        self.id() == other.id()
    }
}

/// Represents a single event update.
pub struct EventUpdate {
    event_id: EventId,
    event_slot: EventSlot,
    delivery_list: EventDeliveryList,
    timestamp: Instant,
    propagation: EventPropagationHandle,
    args: Box<dyn Any>,
}
impl EventUpdate {
    /// Event ID.
    pub fn event_id(&self) -> EventId {
        self.event_id
    }

    /// Event slot.
    pub fn event_slot(&self) -> EventSlot {
        self.event_slot
    }

    /// Event delivery list.
    pub fn delivery_list(&self) -> &EventDeliveryList {
        &self.delivery_list
    }

    /// Gets the instant this event happen.
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    /// Propagation handle associated with this event update.
    ///
    /// Cloned arguments share the same handle, some arguments may also share the handle
    /// of another event if they share the same cause.
    pub fn propagation(&self) -> &EventPropagationHandle {
        &self.propagation
    }
}

struct EventUpdateMsg {
    args: Box<dyn FnOnce() -> EventUpdate + Send>, // EventArgs don't need to be Send in this case
}
impl EventUpdateMsg {
    fn get(self) -> EventUpdate {
        (self.args)()
    }
}

#[macro_export]
macro_rules! command2 {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident $(= { $init:expr })? ;
    )+) => {
        $(
            paste::paste! {
                #[doc(hidden)]
                std::thread_local! {
                    static [<$COMMAND _LOCAL>] = $crate::event::EventData::new(std::stringifly!($EVENT));
                    static [<$COMMAND _DATA>] = $crate::event::CommandData::new();
                }
    
                $(#[$attr])*
                $vis static $COMMAND: $crate::event::Command = $crate::event::Command::new([<$COMMAND _LOCAL>], [<$COMMAND _DATA>]);
            }
        )+
    }
}

#[doc(hidden)]
pub struct CommandData {
    meta_init: Option<Box<dyn Fn(StateMapMut<EventMeta>)>>,
    meta_inited: Cell<bool>,
    meta: RefCell<OwnedStateMap<EventMeta>>,
    scoped_meta: RefCell<FxHashMap<CommandScope, OwnedStateMap<EventMeta>>>,
}
impl CommandData {
    
}

/// Presents a command event.
#[derive(Clone, Copy)]
pub struct Command {
    event: Event<CommandArgs>,
    local: &'static LocalKey<CommandData>,
    scope: CommandScope,
}
impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Command")
                .field("event", &self.event)
                .field("scope", &self.scope)
                .finish_non_exhaustive()
        } else {
            write!(f, "{}", self.event.name())?;
            match self.scope {
                CommandScope::App => Ok(()),
                CommandScope::Window(id) => write!(f, "({id})"),
                CommandScope::Widget(id) => write!(f, "({id})"),
            }
        }
    }
}
impl Command {
    #[doc(hidden)]
    pub fn new(event_local: &'static LocalKey<EventData>, command_local: &'static LocalKey<CommandData>) -> Self {
        Command {
            event: Event::new(event_local),
            local: command_local,
            scope: CommandScope::App,
        }
    }

    /// Raw command event.
    pub fn event(&self) -> Event<CommandArgs> {
        self.event
    }

    /// Command operating scope.
    pub fn scope(&self) -> CommandScope {
        self.scope
    }

    /// Gets the command in a new `scope`.
    pub fn scoped(mut self, scope: CommandScope) -> Command {
        self.scope = scope;
        self
    }

    /// Visit the command custom metadata of the current scope.
    pub fn with_meta<R>(&self, visit: impl FnOnce(&mut CommandMeta) -> R) -> R {
        todo!()
    }

    /// Returns `true` if the update is for this command and scope.
    pub fn has(&self, update: &EventUpdate) -> bool {
        self.on(update).is_some()
    }

    /// Get the command update args if the update is for this command and scope.
    pub fn on<'a>(&self, update: &'a EventUpdate) -> Option<&'a CommandArgs> {
        self.event.on(update).filter(|a| a.scope == self.scope)
    }

    /// Get the event update args if the update is for this event and propagation is not stopped.
    pub fn on_unhandled<'a>(&self, update: &'a EventUpdate) -> Option<&'a CommandArgs> {
        self.event
            .on(update)
            .filter(|a| a.scope == self.scope && a.propagation().is_stopped())
    }

    /// Calls `handler` if the update is for this event and propagation is not stopped, after the handler is called propagation is stopped.
    pub fn handle<R>(&self, update: &EventUpdate, handler: impl FnOnce(&CommandArgs) -> R) -> Option<R> {
        if let Some(args) = self.on(update) {
            args.handle(handler)
        } else {
            None
        }
    }
}
impl Deref for Command {
    type Target = Event<CommandArgs>;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}
impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event && self.scope == other.scope
    }
}
impl Eq for Command { }