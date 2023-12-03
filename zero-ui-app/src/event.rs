//! App event and commands API.

use std::{
    any::Any,
    fmt,
    marker::PhantomData,
    mem,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

mod args;
pub use args::*;

mod command;
pub use command::*;

mod events;
pub use events::*;

mod channel;
pub use channel::*;

use crate::{
    handler::{AppHandler, AppHandlerArgs, WidgetHandler},
    update::{EventUpdate, UpdateDeliveryList, UpdateSubscribers},
    widget::{
        instance::{match_node, UiNode, UiNodeOp},
        WidgetId, WIDGET,
    },
};
use parking_lot::Mutex;
use zero_ui_app_context::AppLocal;
use zero_ui_clone_move::clmv;
use zero_ui_unique_id::{IdEntry, IdMap, IdSet};

///<span data-del-macro-root></span> Declares new [`Event<A>`] keys.
///
/// Event keys usually represent external events or [`AppExtension`] events, you can also use [`command!`]
/// to declare events specialized for commanding widgets and services.
///
/// [`AppExtension`]: crate::AppExtension
///
/// # Examples
///
/// The example defines two events with the same arguments type.
///
/// ```
/// # use zero_ui_app::event::*;
/// # event_args! { pub struct ClickArgs { .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
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
            $(#[$attr])*
            $vis static $EVENT: $crate::event::Event<$Args> = {
                $crate::event::app_local! {
                    static LOCAL: $crate::event::EventData = const { $crate::event::EventData::new(std::stringify!($EVENT)) };
                }
                $crate::event::Event::new(&LOCAL)
            };
        )+
    }
}
#[doc(inline)]
pub use crate::event_macro as event;

#[doc(hidden)]
#[macro_export]
macro_rules! __event_property {
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
            filter: $filter:expr,
            with: $($with:expr)? $(,)?
        }
    ) => { $crate::event::paste! {
        $(#[$on_event_attrs])*
        ///
        /// # Preview
        ///
        #[doc = "You can preview this event using [`on_pre_"$event "`](fn.on_pre_"$event ".html)."]
        /// Otherwise the handler is only called after the widget content has a chance of handling the event by stopping propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::widget::property(EVENT, default( $crate::handler::hn!(|_|{}) ))]
        $vis fn [<on_ $event>](
            child: impl $crate::widget::instance::UiNode,
            handler: impl $crate::handler::WidgetHandler<$Args>,
        ) -> impl $crate::widget::instance::UiNode {
            $crate::__event_property!(with($crate::event::on_event(child, $EVENT, $filter, handler), false, $($with)?))
        }

        #[doc = "Preview [`on_"$event "`](fn.on_"$event ".html) event."]
        ///
        /// # Preview
        ///
        /// Preview event properties call the handler before the main event property and before the widget content, if you stop
        /// the propagation of a preview event the main event handler is not called.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property, note that only the code before the fist `.await` is *preview*,
        /// subsequent code runs in widget updates.
        #[$crate::widget::property(EVENT, default( $crate::handler::hn!(|_|{}) ))]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::widget::instance::UiNode,
            handler: impl $crate::handler::WidgetHandler<$Args>,
        ) -> impl $crate::widget::instance::UiNode {
            $crate::__event_property!(with($crate::event::on_pre_event(child, $EVENT, $filter, handler), true, $($with)?))
        }
    } };

    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $EVENT,
                args: $Args,
                filter: |_args| true,
                with:
            }
        }
    };

    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
            filter: $filter:expr,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $EVENT,
                args: $Args,
                filter: $filter,
                with:
            }
        }
    };

    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path,
            with: $with:expr,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $EVENT,
                args: $Args,
                filter: |_args| true,
                with: $with,
            }
        }
    };

    (with($child:expr, $preview:expr,)) => { $child };
    (with($child:expr, $preview:expr, $with:expr)) => { ($with)($child, $preview) };
}

///<span data-del-macro-root></span> Declare one or more event properties.
///
/// Each declaration expands to two properties `on_$event`, `on_pre_$event`.
/// The preview properties call [`on_pre_event`], the main event properties call [`on_event`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_app::event::*;
/// # use zero_ui_view_api::keyboard::KeyState;
/// # event_args! { pub struct KeyInputArgs { pub state: KeyState, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # event! { pub static KEY_INPUT_EVENT: KeyInputArgs; }
/// event_property! {
///     /// on_key_input docs.
///     pub fn key_input {
///         event: KEY_INPUT_EVENT,
///         args: KeyInputArgs,
///         // default filter is |args| true,
///     }
///
///     pub(crate) fn key_down {
///         event: KEY_INPUT_EVENT,
///         args: KeyInputArgs,
///         // optional filter:
///         filter: |args| args.state == KeyState::Pressed,
///     }
/// }
/// ```
///
/// # Filter
///
/// App events are delivered to all `UiNode` inside all widgets in the [`UpdateDeliveryList`] and event subscribers list,
/// event properties can specialize further by defining a filter predicate.
///
/// The `filter` predicate is called if [`propagation`] is not stopped. It must return `true` if the event arguments
/// are relevant in the context of the widget and event property. If it returns `true` the `handler` closure is called.
/// See [`on_event`] and [`on_pre_event`] for more information.
///
/// If you don't provide a filter predicate the default always allows, so all app events targeting the widget and not already handled
/// are allowed by default.  Note that events that represent an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`]
/// targets, event properties should probably distinguish if they fire on normal interactions vs on *disabled* interactions.
///
/// # Async
///
/// Async event handlers are supported by properties generated by this macro, but only the code before the first `.await` executes
/// in the event track, subsequent code runs in widget updates.
///
/// # Commands
///
/// You can use [`command_property`] to declare command event properties.
///
/// # With Extra Nodes
///
/// You can wrap the event handler node with extra nodes by setting the optional `with` closure:
///
/// ```
/// # fn main() { }
/// # use zero_ui_app::{event::*, widget::instance::UiNode};
/// # event_args! { pub struct KeyInputArgs { .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) {} } }
/// # event! { pub static KEY_INPUT_EVENT: KeyInputArgs; }
/// # fn some_node(child: impl UiNode) -> impl UiNode { child }
/// event_property! {
///     pub fn key_input {
///         event: KEY_INPUT_EVENT,
///         args: KeyInputArgs,
///         with: |child, _preview| some_node(child),
///     }
/// }
/// ```
///
/// The closure receives two arguments, the handler `UiNode` and a `bool` that is `true` if the closure is called in in the *on_pre_*
/// property or `false` when called in the *on_* property.
///
/// [`on_pre_event`]: crate::event::on_pre_event
/// [`on_event`]: crate::event::on_event
/// [`propagation`]: AnyEventArgs::propagation
/// [`ENABLED`]: crate::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget::info::Interactivity::DISABLED
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $EVENT:path,
            args: $Args:path $(,
            filter: $filter:expr)? $(,
            with: $with:expr)? $(,)?
        }
    )+) => {$(
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $EVENT,
                args: $Args,
                $(filter: $filter,)?
                $(with: $with,)?
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::event_property;

/// Helper for declaring event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`propagation`] was not stopped. It must return `true` if the event arguments are
/// relevant in the context of the widget. If it returns `true` the `handler` closure is called. Note that events that represent
/// an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`] targets, event properties should probably distinguish
/// if they fire on normal interactions vs on *disabled* interactions.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///
/// # Commands
///
/// You can use [`on_command`] to declare command event properties.
///
/// [`propagation`]: AnyEventArgs::propagation
/// [`ENABLED`]: crate::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget::info::Interactivity::DISABLED
pub fn on_event<C, A, F, H>(child: C, event: Event<A>, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    #[cfg(dyn_closure)]
    let filter: Box<dyn FnMut(&A) -> bool + Send> = Box::new(filter);
    on_event_impl(child.cfg_boxed(), event, filter, handler.cfg_boxed()).cfg_boxed()
}
fn on_event_impl<C, A, F, H>(child: C, event: Event<A>, mut filter: F, mut handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&event);
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = event.on(update) {
                if !args.propagation().is_stopped() && filter(args) {
                    handler.event(args);
                }
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            handler.update();
        }
        _ => {}
    })
}

/// Helper for declaring preview event properties.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`propagation`] was not stopped. It must return `true` if the event arguments are
/// relevant in the context of the widget. If it returns `true` the `handler` closure is called. Note that events that represent
/// an *interaction* with the widget are send for both [`ENABLED`] and [`DISABLED`] targets, event properties should probably distinguish
/// if they fire on normal interactions vs on *disabled* interactions.
///
/// # Route
///
/// The event `handler` is called before the [`on_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent event updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///
/// # Commands
///
/// You can use [`on_pre_command`] to declare command event properties.
///
/// [`propagation`]: AnyEventArgs::propagation
/// [`ENABLED`]: crate::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget::info::Interactivity::DISABLED
pub fn on_pre_event<C, A, F, H>(child: C, event: Event<A>, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    #[cfg(dyn_closure)]
    let filter: Box<dyn FnMut(&A) -> bool + Send> = Box::new(filter);
    on_pre_event_impl(child.cfg_boxed(), event, filter, handler.cfg_boxed()).cfg_boxed()
}
fn on_pre_event_impl<C, A, F, H>(child: C, event: Event<A>, mut filter: F, mut handler: H) -> impl UiNode
where
    C: UiNode,
    A: EventArgs,
    F: FnMut(&A) -> bool + Send + 'static,
    H: WidgetHandler<A>,
{
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&event);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = event.on(update) {
                if !args.propagation().is_stopped() && filter(args) {
                    handler.event(args);
                }
            }
        }
        UiNodeOp::Update { .. } => {
            handler.update();
        }
        _ => {}
    })
}

#[doc(hidden)]
pub struct EventData {
    name: &'static str,
    widget_subs: IdMap<WidgetId, EventHandle>,
    hooks: Vec<EventHook>,
}
impl EventData {
    #[doc(hidden)]
    pub const fn new(name: &'static str) -> Self {
        EventData {
            name,
            widget_subs: IdMap::new(),
            hooks: vec![],
        }
    }
}

/// Represents an event.
pub struct Event<A: EventArgs> {
    local: &'static AppLocal<EventData>,
    _args: PhantomData<fn(A)>,
}
impl<A: EventArgs> fmt::Debug for Event<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Event({})", self.name())
        } else {
            write!(f, "{}", self.name())
        }
    }
}
impl<A: EventArgs> Event<A> {
    #[doc(hidden)]
    pub const fn new(local: &'static AppLocal<EventData>) -> Self {
        Event { local, _args: PhantomData }
    }

    /// Gets the event without the args type.
    pub fn as_any(&self) -> AnyEvent {
        AnyEvent { local: self.local }
    }

    /// Register the widget to receive targeted events from this event.
    ///
    /// Widgets only receive events if they are in the delivery list generated by the event arguments and are
    /// subscribers to the event, app extensions receive all events.
    pub fn subscribe(&self, widget_id: WidgetId) -> EventHandle {
        self.as_any().subscribe(widget_id)
    }

    /// Returns `true` if the widget is subscribed to this event.
    pub fn is_subscriber(&self, widget_id: WidgetId) -> bool {
        self.as_any().is_subscriber(widget_id)
    }

    /// Returns `true`  if at least one widget is subscribed to this event.
    pub fn has_subscribers(&self) -> bool {
        self.as_any().has_subscribers()
    }

    /// Calls `visit` for each widget subscribed to this event.
    ///
    /// Note that trying to subscribe inside `visit` will deadlock, inside `visit` you can notify the event,
    /// generate event updates or even visit recursive.
    pub fn visit_subscribers(&self, visit: impl FnMut(WidgetId)) {
        self.as_any().visit_subscribers(visit)
    }

    /// Event name.
    pub fn name(&self) -> &'static str {
        self.local.read().name
    }

    /// Returns `true` if the update is for this event.
    pub fn has(&self, update: &EventUpdate) -> bool {
        *self == update.event
    }

    /// Get the event update args if the update is for this event.
    pub fn on<'a>(&self, update: &'a EventUpdate) -> Option<&'a A> {
        if *self == update.event {
            update.args.as_any().downcast_ref()
        } else {
            None
        }
    }

    /// Get the event update args if the update is for this event and propagation is not stopped.
    pub fn on_unhandled<'a>(&self, update: &'a EventUpdate) -> Option<&'a A> {
        self.on(update).filter(|a| !a.propagation().is_stopped())
    }

    /// Calls `handler` if the update is for this event and propagation is not stopped, after the handler is called propagation is stopped.
    pub fn handle<R>(&self, update: &EventUpdate, handler: impl FnOnce(&A) -> R) -> Option<R> {
        if let Some(args) = self.on(update) {
            args.handle(handler)
        } else {
            None
        }
    }

    /// Create an event update for this event with delivery list filtered by the event subscribers.
    pub fn new_update(&self, args: A) -> EventUpdate {
        self.new_update_custom(args, UpdateDeliveryList::new(Box::new(self.as_any())))
    }

    /// Create and event update for this event with a custom delivery list.
    pub fn new_update_custom(&self, args: A, mut delivery_list: UpdateDeliveryList) -> EventUpdate {
        args.delivery_list(&mut delivery_list);
        EventUpdate {
            event: self.as_any(),
            delivery_list,
            args: Box::new(args),
            pre_actions: Mutex::new(vec![]),
            pos_actions: Mutex::new(vec![]),
        }
    }

    /// Schedule an event update.
    pub fn notify(&self, args: A) {
        let update = self.new_update(args);
        EVENTS.notify(update);
    }

    /// Creates a preview event handler.
    ///
    /// The event `handler` is called for every update of `E` that has not stopped [`propagation`](AnyEventArgs::propagation).
    /// The handler is called before UI handlers and [`on_event`](Self::on_event) handlers, it is called after all previous registered
    /// preview handlers.
    ///
    /// Returns an [`EventHandle`] that can be dropped to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_app::event::*;
    /// # use zero_ui_app::App;
    /// # use zero_ui_app::handler::app_hn;
    /// # event_args! { pub struct FocusChangedArgs { pub new_focus: bool, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) {} } }
    /// # event! { pub static FOCUS_CHANGED_EVENT: FocusChangedArgs; }
    /// # let _scope = App::minimal();
    /// let handle = FOCUS_CHANGED_EVENT.on_pre_event(app_hn!(|args: &FocusChangedArgs, _| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
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
    /// subsequent event updates, after the event has already propagated, so stopping [`propagation`](AnyEventArgs::propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn_once!`]: crate::handler::async_app_hn_once!
    pub fn on_pre_event<H>(&self, handler: H) -> EventHandle
    where
        H: AppHandler<A>,
    {
        self.on_event_impl(handler, true)
    }

    /// Creates an event handler.
    ///
    /// The event `handler` is called for every update of `E` that has not stopped [`propagation`](AnyEventArgs::propagation).
    /// The handler is called after all [`on_pre_event`](Self::on_pre_event) all UI handlers and all [`on_event`](Self::on_event) handlers
    /// registered before this one.
    ///
    /// Returns an [`EventHandle`] that can be dropped to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_app::event::*;
    /// # use zero_ui_app::App;
    /// # use zero_ui_app::handler::app_hn;
    /// # event_args! { pub struct FocusChangedArgs { pub new_focus: bool, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) {} } }
    /// # event! { pub static FOCUS_CHANGED_EVENT: FocusChangedArgs; }
    /// # let _scope = App::minimal();
    /// let handle = FOCUS_CHANGED_EVENT.on_event(app_hn!(|args: &FocusChangedArgs, _| {
    ///     println!("focused: {:?}", args.new_focus);
    /// }));
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
    /// subsequent event updates, after the event has already propagated, so stopping [`propagation`](AnyEventArgs::propagation)
    /// only causes the desired effect before the first `.await`.
    ///
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn_once!`]: crate::handler::async_app_hn_once!
    pub fn on_event(&self, handler: impl AppHandler<A>) -> EventHandle {
        self.on_event_impl(handler, false)
    }

    fn on_event_impl(&self, handler: impl AppHandler<A>, is_preview: bool) -> EventHandle {
        let handler = Arc::new(Mutex::new(handler));
        let (inner_handle_owner, inner_handle) = zero_ui_handle::Handle::new(());
        self.as_any().hook(move |update| {
            if inner_handle_owner.is_dropped() {
                return false;
            }

            let handle = inner_handle.downgrade();
            update.push_once_action(
                Box::new(clmv!(handler, |update| {
                    let args = update.args().as_any().downcast_ref::<A>().unwrap();
                    if !args.propagation().is_stopped() {
                        handler.lock().event(
                            args,
                            &AppHandlerArgs {
                                handle: &handle,
                                is_preview,
                            },
                        );
                    }
                })),
                is_preview,
            );

            true
        })
    }

    /// Creates a receiver that can listen to the event from another thread. The event updates are sent as soon as the
    /// event update cycle starts in the UI thread.
    ///
    /// Drop the receiver to stop listening.
    pub fn receiver(&self) -> EventReceiver<A>
    where
        A: Send,
    {
        let (sender, receiver) = flume::unbounded();

        self.as_any()
            .hook(move |update| sender.send(update.args().as_any().downcast_ref::<A>().unwrap().clone()).is_ok())
            .perm();

        EventReceiver { receiver, event: *self }
    }

    /// Creates a sender that can raise an event from other threads and without access to [`EVENTS`].
    pub fn sender(&self) -> EventSender<A>
    where
        A: Send,
    {
        EVENTS_SV.write().sender(*self)
    }

    /// Returns `true` if any app level callback is registered for this event.
    ///
    /// This includes [`AnyEvent::hook`], [`Event::on_pre_event`], [`Event::on_event`] and [`Event::receiver`].
    pub fn has_hooks(&self) -> bool {
        self.as_any().has_hooks()
    }
}
impl<A: EventArgs> Clone for Event<A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<A: EventArgs> Copy for Event<A> {}
impl<A: EventArgs> PartialEq for Event<A> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.local, other.local)
    }
}
impl<A: EventArgs> Eq for Event<A> {}

/// Represents an [`Event`] without the args type.
#[derive(Clone, Copy)]
pub struct AnyEvent {
    local: &'static AppLocal<EventData>,
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
    /// Display name.
    pub fn name(&self) -> &'static str {
        self.local.read().name
    }

    /// Returns `true` is `self` is the type erased `event`.
    pub fn is<A: EventArgs>(&self, event: &Event<A>) -> bool {
        self == event
    }

    /// Returns `true` if the update is for this event.
    pub fn has(&self, update: &EventUpdate) -> bool {
        *self == update.event
    }

    /// Register a callback that is called just before an event begins notifying.
    pub fn hook(&self, hook: impl Fn(&mut EventUpdate) -> bool + Send + Sync + 'static) -> EventHandle {
        self.hook_impl(Box::new(hook))
    }
    fn hook_impl(&self, hook: Box<dyn Fn(&mut EventUpdate) -> bool + Send + Sync>) -> EventHandle {
        let (handle, hook) = EventHandle::new(hook);
        self.local.write().hooks.push(hook);
        handle
    }

    /// Register the widget to receive targeted events from this event.
    ///
    /// Widgets only receive events if they are in the delivery list generated by the event arguments and are
    /// subscribers to the event, app extensions receive all events.
    pub fn subscribe(&self, widget_id: WidgetId) -> EventHandle {
        self.local
            .write()
            .widget_subs
            .entry(widget_id)
            .or_insert_with(EventHandle::new_none)
            .clone()
    }

    /// Returns `true` if the widget is subscribed to this event.
    pub fn is_subscriber(&self, widget_id: WidgetId) -> bool {
        self.local.read().widget_subs.contains_key(&widget_id)
    }

    /// Returns `true`  if at least one widget is subscribed to this event.
    pub fn has_subscribers(&self) -> bool {
        !self.local.read().widget_subs.is_empty()
    }

    /// Calls `visit` for each widget subscribed to this event.
    ///
    /// Note that trying to subscribe inside `visit` will deadlock, inside `visit` you can notify the event,
    /// generate event updates or even visit recursive.
    pub fn visit_subscribers(&self, mut visit: impl FnMut(WidgetId)) {
        for sub in self.local.read().widget_subs.keys() {
            visit(*sub);
        }
    }

    /// Returns `true` if any app level callback is registered for this event.
    ///
    /// This includes [`AnyEvent::hook`], [`Event::on_pre_event`], [`Event::on_event`] and [`Event::receiver`].
    pub fn has_hooks(&self) -> bool {
        !self.local.read().hooks.is_empty()
    }

    fn on_update(&self, update: &mut EventUpdate) {
        let mut hooks = mem::take(&mut self.local.write().hooks);
        hooks.retain(|h| h.call(update));

        let mut write = self.local.write();
        hooks.append(&mut write.hooks);
        write.hooks = hooks;
    }
}
impl PartialEq for AnyEvent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.local, other.local)
    }
}
impl Eq for AnyEvent {}
impl<A: EventArgs> PartialEq<AnyEvent> for Event<A> {
    fn eq(&self, other: &AnyEvent) -> bool {
        std::ptr::eq(self.local, other.local)
    }
}
impl<A: EventArgs> PartialEq<Event<A>> for AnyEvent {
    fn eq(&self, other: &Event<A>) -> bool {
        std::ptr::eq(self.local, other.local)
    }
}

impl UpdateSubscribers for AnyEvent {
    fn contains(&self, widget_id: WidgetId) -> bool {
        if let Some(mut write) = self.local.try_write() {
            match write.widget_subs.entry(widget_id) {
                IdEntry::Occupied(e) => {
                    let t = e.get().retain();
                    if !t {
                        let _ = e.remove();
                    }
                    t
                }
                IdEntry::Vacant(_) => false,
            }
        } else {
            // fallback without cleanup
            match self.local.read().widget_subs.get(&widget_id) {
                Some(e) => e.retain(),
                None => false,
            }
        }
    }

    fn to_set(&self) -> IdSet<WidgetId> {
        self.local.read().widget_subs.keys().copied().collect()
    }
}

/// Represents a collection of var handles.
#[must_use = "the event subscriptions or handlers are dropped if the handle is dropped"]
#[derive(Clone, Default)]
pub struct EventHandles(pub Vec<EventHandle>);
impl EventHandles {
    /// Empty collection.
    pub const fn dummy() -> Self {
        EventHandles(vec![])
    }

    /// Returns `true` if empty or all handles are dummy.
    pub fn is_dummy(&self) -> bool {
        self.0.is_empty() || self.0.iter().all(EventHandle::is_dummy)
    }

    /// Drop all handles without stopping their behavior.
    pub fn perm(self) {
        for handle in self.0 {
            handle.perm()
        }
    }

    /// Add `other` handle to the collection.
    pub fn push(&mut self, other: EventHandle) -> &mut Self {
        if !other.is_dummy() {
            self.0.push(other);
        }
        self
    }

    /// Drop all handles.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}
impl FromIterator<EventHandle> for EventHandles {
    fn from_iter<T: IntoIterator<Item = EventHandle>>(iter: T) -> Self {
        EventHandles(iter.into_iter().filter(|h| !h.is_dummy()).collect())
    }
}
impl<const N: usize> From<[EventHandle; N]> for EventHandles {
    fn from(handles: [EventHandle; N]) -> Self {
        handles.into_iter().filter(|h| !h.is_dummy()).collect()
    }
}
impl Extend<EventHandle> for EventHandles {
    fn extend<T: IntoIterator<Item = EventHandle>>(&mut self, iter: T) {
        for handle in iter {
            self.push(handle);
        }
    }
}
impl IntoIterator for EventHandles {
    type Item = EventHandle;

    type IntoIter = std::vec::IntoIter<EventHandle>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

struct EventHandleData {
    perm: AtomicBool,
    hook: Option<Box<dyn Fn(&mut EventUpdate) -> bool + Send + Sync>>,
}

/// Represents an event widget subscription, handler callback or hook.
#[derive(Clone)]
#[must_use = "the event subscription or handler is dropped if the handle is dropped"]
pub struct EventHandle(Option<Arc<EventHandleData>>);
impl PartialEq for EventHandle {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (None, Some(_)) | (Some(_), None) => false,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        }
    }
}
impl Eq for EventHandle {}
impl std::hash::Hash for EventHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let i = match &self.0 {
            Some(rc) => Arc::as_ptr(rc) as usize,
            None => 0,
        };
        state.write_usize(i);
    }
}
impl fmt::Debug for EventHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let i = match &self.0 {
            Some(rc) => Arc::as_ptr(rc) as usize,
            None => 0,
        };
        f.debug_tuple("EventHandle").field(&i).finish()
    }
}
/// Dummy
impl Default for EventHandle {
    fn default() -> Self {
        Self::dummy()
    }
}
impl EventHandle {
    fn new(hook: Box<dyn Fn(&mut EventUpdate) -> bool + Send + Sync>) -> (Self, EventHook) {
        let rc = Arc::new(EventHandleData {
            perm: AtomicBool::new(false),
            hook: Some(hook),
        });
        (Self(Some(rc.clone())), EventHook(rc))
    }

    fn new_none() -> Self {
        Self(Some(Arc::new(EventHandleData {
            perm: AtomicBool::new(false),
            hook: None,
        })))
    }

    /// Handle to no event.
    pub fn dummy() -> Self {
        EventHandle(None)
    }

    /// If the handle is not actually registered in an event.
    pub fn is_dummy(&self) -> bool {
        self.0.is_none()
    }

    /// Drop the handle without un-registering it, the resource it represents will remain registered in the event for the duration of
    /// the process.
    pub fn perm(self) {
        if let Some(rc) = self.0 {
            rc.perm.store(true, Ordering::Relaxed);
        }
    }

    /// Create an [`EventHandles`] collection with `self` and `other`.
    pub fn with(self, other: Self) -> EventHandles {
        [self, other].into()
    }

    fn retain(&self) -> bool {
        let rc = self.0.as_ref().unwrap();
        Arc::strong_count(rc) > 1 || rc.perm.load(Ordering::Relaxed)
    }
}

struct EventHook(Arc<EventHandleData>);
impl EventHook {
    /// Callback, returns `true` if the handle must be retained.
    fn call(&self, update: &mut EventUpdate) -> bool {
        (Arc::strong_count(&self.0) > 1 || self.0.perm.load(Ordering::Relaxed)) && (self.0.hook.as_ref().unwrap())(update)
    }
}
