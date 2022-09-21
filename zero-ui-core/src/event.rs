//! App event and commands API.

use std::{any::Any, cell::RefCell, fmt, marker::PhantomData, ops::Deref, thread::LocalKey, time::Instant};

use crate::{
    context::{UpdateDeliveryList, UpdateSubscribers, WidgetContext, WindowContext},
    crate_util::{IdMap, IdSet},
    widget_info::WidgetInfoTree,
    WidgetId,
};

mod args;
pub use args::*;

mod command;
pub use command::*;

mod events;
pub use events::*;

mod channel;
pub use channel::*;

mod properties;
pub use properties::*;

///<span data-del-macro-root></span> Declares new [`Event<A>`] keys.
///
/// Event keys usually represent external events or [`AppExtension`] events, you can also use [`command!`]
/// to declare events specialized for commanding widgets and services.
///
/// [`AppExtension`]: crate::app::AppExtension
///
/// # Examples
///
/// The example defines two events with the same arguments type.
///
/// ```
/// # use zero_ui_core::event::event;
/// # use zero_ui_core::gesture::ClickArgs;
/// event! {
///     /// Event docs.
///     pub static CLICK_EVENT: ClickArgs;
///
///     /// Other event docs.
///     pub static DOUBLE_CLICK_EVENT: ClickArgs;
/// }
/// ```
///
/// # Properties
///
/// If the event targets widgets you can use [`event_property!`] to declare properties that setup event handlers for the event.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `_VAR` suffix.
#[macro_export]
macro_rules! event_macro {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $EVENT:ident: $Args:path;
    )+) => {
        $(
            paste::paste! {
                std::thread_local! {
                    #[doc(hidden)]
                    static [<$EVENT _LOCAL>]: $crate::event::EventData  = $crate::event::EventData::new(std::stringify!($EVENT));
                }

                $(#[$attr])*
                $vis static $EVENT: $crate::event::Event<$Args> = $crate::event::Event::new(&[<$EVENT _LOCAL>]);
            }
        )+
    }
}
#[doc(inline)]
pub use crate::event_macro as event;

#[doc(hidden)]
pub struct EventData {
    name: &'static str,
    widget_subs: RefCell<IdMap<WidgetId, usize>>,
}
impl EventData {
    #[doc(hidden)]
    pub fn new(name: &'static str) -> Self {
        EventData {
            name,
            widget_subs: RefCell::default(),
        }
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
    _args: PhantomData<fn(E)>,
}
impl<E: EventArgs> fmt::Debug for Event<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Event({})", self.name())
        } else {
            write!(f, "{}", self.name())
        }
    }
}
impl<E: EventArgs> Event<E> {
    #[doc(hidden)]
    pub const fn new(local: &'static LocalKey<EventData>) -> Self {
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

    /// Register the widget to receive targeted events from this event.
    pub fn subscribe_widget(&self, widget_id: WidgetId) -> EventWidgetHandle {
        self.as_any().subscribe_widget(widget_id)
    }

    /// Event name.
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
        let mut delivery_list = UpdateDeliveryList::new(Box::new(self.as_any()));
        args.delivery_list(&mut delivery_list);
        EventUpdate {
            event_id: self.id(),
            event_name: self.name(),
            delivery_list,
            timestamp: args.timestamp(),
            propagation: args.propagation().clone(),

            args: Box::new(args),
        }
    }

    /// Schedule an event update.
    pub fn notify<Ev>(&self, events: &mut Ev, args: E)
    where
        Ev: WithEvents,
    {
        let update = self.new_update(args);
        events.with_events(|ev| {
            ev.notify(update);
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
impl fmt::Debug for AnyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "AnyEvent({})", self.name())
        } else {
            write!(f, "{}", self.name())
        }
    }
}
impl AnyEvent {
    /// Event ID.
    pub fn id(&self) -> EventId {
        EventId(self.local as *const _ as _)
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

    fn unsubscribe_widget(&self, widget_id: WidgetId) {
        self.local.with(|l| match l.widget_subs.borrow_mut().entry(widget_id) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let i = e.get_mut();
                if *i == 1 {
                    e.remove();
                } else {
                    *i -= 1;
                }
            }
            std::collections::hash_map::Entry::Vacant(_) => unreachable!(),
        })
    }

    fn subscribe_widget_raw(&self, widget_id: WidgetId) {
        self.local.with(|l| match l.widget_subs.borrow_mut().entry(widget_id) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                *e.get_mut() += 1;
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(1);
            }
        });
    }

    /// Register the widget to receive targeted events from this event.
    pub fn subscribe_widget(&self, widget_id: WidgetId) -> EventWidgetHandle {
        self.subscribe_widget_raw(widget_id);
        EventWidgetHandle { event: *self, widget_id }
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
    event_name: &'static str,
    delivery_list: UpdateDeliveryList,
    timestamp: Instant,
    propagation: EventPropagationHandle,
    args: Box<dyn Any>,
}
impl EventUpdate {
    /// Event ID.
    pub fn event_id(&self) -> EventId {
        self.event_id
    }

    /// Event name.
    pub fn event_name(&self) -> &'static str {
        self.event_name
    }

    /// Event delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
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

    /// Find all targets.
    ///
    /// This must be called before the first window visit, see [`UpdateDeliveryList::fulfill_search`] for details.
    pub fn fulfill_search<'a, 'b>(&'a mut self, windows: impl Iterator<Item = &'b WidgetInfoTree>) {
        self.delivery_list.fulfill_search(windows)
    }

    /// Calls `handle` if the event targets the window.
    pub fn with_window<H: FnOnce(&mut WindowContext, &mut Self) -> R, R>(&mut self, ctx: &mut WindowContext, handle: H) -> Option<R> {
        if self.delivery_list.enter_window(*ctx.window_id) {
            Some(handle(ctx, self))
        } else {
            None
        }
    }

    /// Calls `handle` if the event targets the widget.
    pub fn with_widget<H: FnOnce(&mut WidgetContext, &mut Self) -> R, R>(&mut self, ctx: &mut WidgetContext, handle: H) -> Option<R> {
        if self.delivery_list.enter_widget(ctx.path.widget_id()) {
            Some(handle(ctx, self))
        } else {
            None
        }
    }
}
impl fmt::Debug for EventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventUpdate")
            .field("event_id", &self.event_id)
            .field("event_name", &self.event_name)
            .field("delivery_list", &self.delivery_list)
            .field("timestamp", &self.timestamp)
            .field("propagation", &self.propagation)
            .finish_non_exhaustive()
    }
}

impl UpdateSubscribers for AnyEvent {
    fn contains(&self, widget_id: WidgetId) -> bool {
        self.local.with(|l| l.widget_subs.borrow().contains_key(&widget_id))
    }

    fn to_set(&self) -> IdSet<WidgetId> {
        self.local.with(|l| l.widget_subs.borrow().keys().copied().collect())
    }
}

/// Handle to an event subscription for a widget.
#[derive(Debug)]
#[must_use = "only widgets with subscription handles receive the event"]
pub struct EventWidgetHandle {
    widget_id: WidgetId,
    event: AnyEvent,
}
impl EventWidgetHandle {
    /// The event.
    pub fn event(&self) -> AnyEvent {
        self.event
    }

    /// The widget.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }
}
impl Clone for EventWidgetHandle {
    fn clone(&self) -> Self {
        self.event.subscribe_widget(self.widget_id)
    }
}
impl Drop for EventWidgetHandle {
    fn drop(&mut self) {
        self.event.unsubscribe_widget(self.widget_id);
    }
}
