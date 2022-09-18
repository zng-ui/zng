use std::{any::Any, marker::PhantomData, time::Instant, thread::LocalKey};

use crate::{
    event::{EventArgs, EventDeliveryList, EventPropagationHandle, WithEvents},
    widget_info::EventSlot,
};

#[macro_export]
macro_rules! event2 {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $EVENT:ident: $Args:path;
    )+) => {paste::paste! {
        $(
            #[doc(hidden)]
            std::thread_local! {
                static [<$EVENT _LOCAL>] = $crate::event::EventData::new_unique();
            }

            $(#[$attr])*
            $vis static $EVENT: $crate::event::Event<$Args> = $crate::event::Event::new([<$EVENT _LOCAL>]);
        )+
    }}
}

#[doc(hidden)]
pub struct EventData {
    slot: EventSlot,
}
impl EventData {
    #[doc(hidden)]
    pub fn new_unique() -> Self {
        EventData {
            slot: EventSlot::next(),
        }
    }    

    fn slot(&self) -> EventSlot {
        self.slot
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
impl<E: EventArgs> Event<E> {
    #[doc(hidden)]
    pub fn new(local: &'static LocalKey<EventData>) -> Self {
        Event {
            local,
            _args: PhantomData,
        }
    }

    /// Event ID.
    pub fn id(&self) -> EventId {
        EventId(self.local as *const _ as _)
    }

    /// Event slot.
    pub fn slot(&self) -> EventSlot {
        self.local.with(EventData::slot)
    }

    /// Returns `true` if the update is for this event.
    pub fn is(&self, update: &EventUpdate) -> bool {
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
impl<E: EventArgs> Copy for Event<E> {
}
impl<E: EventArgs> PartialEq for Event<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
impl<E: EventArgs> Eq for Event<E> {}

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
