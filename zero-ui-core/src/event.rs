//! App event API.

use unsafe_any::UnsafeAny;

use crate::context::{AppContext, Updates, WidgetContext};
use crate::widget_base::IsEnabled;
use crate::{impl_ui_node, UiNode};
use std::cell::RefCell;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;
use std::time::Instant;
use std::{any::*, collections::VecDeque};

/// [`Event`] arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;
    /// If this event arguments is relevant to the widget context.
    fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool;

    /// Requests that subsequent handlers skip this event.
    ///
    /// Cloned arguments signal stop for all clones.
    fn stop_propagation(&self);

    /// If the handler must skip this event.
    ///
    /// Note that property level handlers don't need to check this, as those handlers are
    /// already not called when this is `true`. [`UiNode`](crate::UiNode) and
    /// [`AppExtension`](crate::app::AppExtension) implementers must check if this is `true`.
    fn stop_propagation_requested(&self) -> bool;
}

/// [`Event`] arguments that can be canceled.
pub trait CancelableEventArgs: EventArgs {
    /// If the originating action must be canceled.
    fn cancel_requested(&self) -> bool;
    /// Cancel the originating action.
    ///
    /// Cloned arguments signal cancel for all clones.
    fn cancel(&self);
}

/// Identifies an event type.
pub trait Event: Debug + Clone + Copy + 'static {
    /// Event arguments type.
    type Args: EventArgs;

    /// Schedule an event update.
    #[doc(hidden)]
    fn notify(events: &Events, args: Self::Args) {
        events.notify::<Self>(args);
    }

    /// Gets the event arguments if the update is for `Self`.
    #[inline(always)]
    fn update<U: EventUpdateArgs>(args: &U) -> Option<&EventUpdate<Self>> {
        args.args_for::<Self>()
    }
}

mod protected {
    pub trait EventUpdateArgs {}
}

/// [`EventUpdateArgs`] for event `E`, dereferences to the argument.
pub struct EventUpdate<E: Event>(E::Args);
impl<E: Event> EventUpdate<E> {
    /// Clone the arguments.
    #[inline]
    #[allow(clippy::should_implement_trait)] // that is what we want
    pub fn clone(&self) -> E::Args {
        self.0.clone()
    }

    fn boxed(self) -> BoxedEventUpdate {
        BoxedEventUpdate {
            event_type: TypeId::of::<E>(),
            event_name: type_name::<E>(),
            args: Box::new(self),
        }
    }
}
impl<E: Event> protected::EventUpdateArgs for EventUpdate<E> {}
impl<E: Event> fmt::Debug for EventUpdate<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "EventUpdate<{}>({:#?})", type_name::<E>(), self.0)
        } else {
            write!(f, "EventUpdate<{}>({:?})", type_name::<E>(), self.0)
        }
    }
}
impl<E: Event> Deref for EventUpdate<E> {
    type Target = E::Args;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Boxed [`EventUpdateArgs`].
pub struct BoxedEventUpdate {
    event_type: TypeId,
    event_name: &'static str,
    args: Box<dyn UnsafeAny>,
}
impl protected::EventUpdateArgs for BoxedEventUpdate {}
impl fmt::Debug for BoxedEventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BoxedEventUpdate for {}", self.event_name)
    }
}
impl EventUpdateArgs for BoxedEventUpdate {
    #[inline(always)]
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if self.event_type == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type
                self.args.downcast_ref_unchecked()
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type: self.event_type,
            event_name: self.event_name,
            args: unsafe {
                // SAFETY: no different then the EventUpdate::as_any()
                self.args.downcast_ref_unchecked()
            },
        }
    }
}

/// Type erased [`EventUpdateArgs`].
pub struct AnyEventUpdate<'a> {
    event_type: TypeId,
    event_name: &'static str,
    args: &'a (),
}
impl<'a> fmt::Debug for AnyEventUpdate<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyEventUpdate for {}", self.event_name)
    }
}
impl<'a> protected::EventUpdateArgs for AnyEventUpdate<'a> {}
impl<'a> EventUpdateArgs for AnyEventUpdate<'a> {
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if self.event_type == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type.
                #[allow(clippy::transmute_ptr_to_ptr)]
                mem::transmute(self.args)
            })
        } else {
            None
        }
    }

    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type: self.event_type,
            event_name: self.event_name,
            args: self.args,
        }
    }
}

/// Represents an event update.
pub trait EventUpdateArgs: protected::EventUpdateArgs + fmt::Debug {
    /// Gets the the update arguments if the event updating is `Q`.
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>>;

    /// Type erased event update.
    fn as_any(&self) -> AnyEventUpdate;
}
impl<E: Event> EventUpdateArgs for EventUpdate<E> {
    #[inline(always)]
    fn args_for<Q: Event>(&self) -> Option<&EventUpdate<Q>> {
        if TypeId::of::<E>() == TypeId::of::<Q>() {
            Some(unsafe {
                // SAFETY: its the same type.
                #[allow(clippy::transmute_ptr_to_ptr)]
                std::mem::transmute(&self)
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn as_any(&self) -> AnyEventUpdate {
        AnyEventUpdate {
            event_type: TypeId::of::<E>(),
            event_name: type_name::<E>(),
            args: unsafe {
                // SAFETY: nothing will be done with it other then a validated restore in `args_for`.
                #[allow(clippy::transmute_ptr_to_ptr)]
                mem::transmute(self)
            },
        }
    }
}

/// Identifies an event type for an action that can be canceled.
///
/// # Auto-Implemented
///
/// This trait is auto-implemented for all events with cancellable arguments.
pub trait CancelableEvent: Event + 'static {
    /// Cancelable event arguments type.
    type CancelableArgs: CancelableEventArgs;
}
impl<A: CancelableEventArgs, E: Event<Args = A>> CancelableEvent for E {
    type CancelableArgs = A;
}

/// A buffered event listener.
///
/// This `struct` is a refence to the buffer, clones of it point to the same buffer. This `struct`
/// is not `Send`, you can use a [`Sync::event_receiver`](crate::sync::Sync::event_receiver) for that.
#[derive(Clone)]
pub struct EventBuffer<T: Clone> {
    queue: Rc<RefCell<VecDeque<T>>>,
}
impl<T: Clone> EventBuffer<T> {
    /// If there are any updates in the buffer.
    #[inline]
    pub fn has_updates(&self) -> bool {
        !RefCell::borrow(&self.queue).is_empty()
    }

    /// Take the oldest event in the buffer.
    #[inline]
    pub fn pop_oldest(&self) -> Option<T> {
        self.queue.borrow_mut().pop_front()
    }

    /// Take the oldest `n` events from the buffer.
    ///
    /// The result is sorted from oldest to newer.
    #[inline]
    pub fn pop_oldest_n(&self, n: usize) -> Vec<T> {
        self.queue.borrow_mut().drain(..n).collect()
    }

    /// Take all the events from the buffer.
    ///
    /// The result is sorted from oldest to newest.
    #[inline]
    pub fn pop_all(&self) -> Vec<T> {
        self.queue.borrow_mut().drain(..).collect()
    }

    /// Create an empty buffer that will always stay empty.
    #[inline]
    pub fn never() -> Self {
        EventBuffer { queue: Default::default() }
    }
}

thread_singleton!(SingletonEvents);

/// Access to application events.
///
/// Only a single instance of this type exists at a time.
pub struct Events {
    updates: RefCell<Vec<BoxedEventUpdate>>,

    #[allow(clippy::type_complexity)]
    buffers: RefCell<Vec<Box<dyn Fn(&BoxedEventUpdate) -> Retain>>>,
    app_pre_handlers: AppHandlers,
    app_handlers: AppHandlers,

    _singleton: SingletonEvents,
}

type AppHandlerWeak = std::rc::Weak<RefCell<dyn FnMut(&mut AppContext, &BoxedEventUpdate)>>;

#[derive(Default)]
struct AppHandlers(RefCell<Vec<AppHandlerWeak>>);
impl AppHandlers {
    pub fn push(&self, handler: &EventHandler) {
        self.0.borrow_mut().push(Rc::downgrade(&handler.0));
    }

    pub fn notify(&self, ctx: &mut AppContext, args: &BoxedEventUpdate) {
        let mut handlers = self.0.borrow_mut();
        let mut live_handlers = Vec::with_capacity(handlers.len());
        handlers.retain(|h| {
            if let Some(handler) = h.upgrade() {
                live_handlers.push(handler);
                true
            } else {
                false
            }
        });
        drop(handlers);

        for handler in live_handlers {
            handler.borrow_mut()(ctx, args);
        }
    }
}

/// A *global* event handler created by [`Events::on_event`].
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct EventHandler(Rc<RefCell<dyn FnMut(&mut AppContext, &BoxedEventUpdate)>>);
impl EventHandler {
    pub(self) fn new(handler: impl FnMut(&mut AppContext, &BoxedEventUpdate) + 'static) -> Self {
        Self(Rc::new(RefCell::new(handler)))
    }
}
impl Events {
    /// If an instance of `Events` already exists in the  current thread.
    #[inline]
    pub fn instantiated() -> bool {
        SingletonEvents::in_use()
    }

    /// Produces the instance of `Events`. Only a single
    /// instance can exist in a thread at a time, panics if called
    /// again before dropping the previous instance.
    #[inline]
    pub fn instance() -> Self {
        Events {
            updates: RefCell::default(),
            buffers: RefCell::default(),
            app_pre_handlers: AppHandlers::default(),
            app_handlers: AppHandlers::default(),
            _singleton: SingletonEvents::assert_new("Events"),
        }
    }

    fn notify<E: Event>(&self, args: E::Args) {
        let update = EventUpdate::<E>(args);
        self.updates.borrow_mut().push(update.boxed());
    }

    /// Creates an event buffer for that listens to `E`.
    ///
    /// Drop the buffer to stop listening.
    pub fn buffer<E: Event>(&self) -> EventBuffer<E::Args> {
        let buf = EventBuffer::never();
        let weak = Rc::downgrade(&buf.queue);
        self.buffers.borrow_mut().push(Box::new(move |args| {
            let mut retain = false;
            if let Some(rc) = weak.upgrade() {
                if let Some(args) = E::update(args) {
                    rc.borrow_mut().push_back(args.clone());
                }
                retain = true;
            }
            retain
        }));
        buf
    }

    /// Creates a preview event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called before UI handlers and [`on_event`](Self::on_event) handlers, it is called after all previous registered
    /// preview handlers.
    ///
    /// Drop all clones of the [`EventHandler`] object to unsubscribe.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::focus::FocusChangedEvent;
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handler = ctx.events.on_pre_event::<FocusChangedEvent, _>(|_ctx, args| {
    ///     println!("focused: {:?}", args.new_focus);
    /// });
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context and before all UI handlers.
    ///
    /// # Panics
    ///
    /// If the event is not registered in the application.
    pub fn on_pre_event<E, H>(&self, mut handler: H) -> EventHandler
    where
        E: Event,
        H: FnMut(&mut AppContext, &E::Args) + 'static,
    {
        let handler = EventHandler::new(move |ctx, args| {
            if let Some(args) = E::update(args) {
                if !args.stop_propagation_requested() {
                    handler(ctx, args);
                }
            }
        });
        self.app_pre_handlers.push(&handler);
        handler
    }

    /// Creates an event handler.
    ///
    /// The event `handler` is called for every update of `E` that are not marked [`stop_propagation`](EventArgs::stop_propagation).
    /// The handler is called after all [`on_pre_event`],(Self::on_pre_event) all UI handlers and all [`on_event`](Self::on_event) handlers
    /// registered before this one.
    ///
    /// Creating a [listener](Events::listen) is slightly more efficient then this and also gives you access to args marked
    /// with [`stop_propagation`](EventArgs::stop_propagation), this method exists for the convenience of listening on
    /// an event at the app level without having to declare an [`AppExtension`](crate::app::AppExtension) or a weird property.
    ///
    /// Drop all clones of the [`EventHandler`] object to unsubscribe.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::event::*;
    /// # use zero_ui_core::focus::FocusChangedEvent;
    /// # fn example(ctx: &mut zero_ui_core::context::AppContext) {
    /// let handler = ctx.events.on_event::<FocusChangedEvent, _>(|_ctx, args| {
    ///     println!("focused: {:?}", args.new_focus);
    /// });
    /// # }
    /// ```
    /// The example listens to all `FocusChangedEvent` events, independent of widget context, after the UI was notified.
    ///
    /// # Panics
    ///
    /// If the event is not registered in the application.
    pub fn on_event<E, H>(&self, mut handler: H) -> EventHandler
    where
        E: Event,
        H: FnMut(&mut AppContext, &E::Args) + 'static,
    {
        let handler = EventHandler::new(move |ctx, args| {
            if let Some(args) = E::update(args) {
                if !args.stop_propagation_requested() {
                    handler(ctx, args);
                }
            }
        });
        self.app_handlers.push(&handler);
        handler
    }

    #[must_use]
    pub(super) fn apply(&mut self, updates: &mut Updates) -> Vec<BoxedEventUpdate> {
        let r = std::mem::take(self.updates.get_mut());
        if !r.is_empty() {
            updates.update();
        }
        r
    }

    pub(super) fn on_pre_events(&self, ctx: &mut AppContext, args: &BoxedEventUpdate) {
        self.buffers.borrow_mut().retain(|buf| buf(args));
        self.app_pre_handlers.notify(ctx, args);
    }

    pub(super) fn on_events(&self, ctx: &mut AppContext, args: &BoxedEventUpdate) {
        self.app_handlers.notify(ctx, args);
    }
}

type Retain = bool;

/// Declares new [`EventArgs`](crate::event::EventArgs) types.
///
/// # Example
///
/// ```
/// # use zero_ui_core::event::event_args;
/// use zero_ui_core::render::WidgetPath;
///
/// event_args! {
///     /// My event arguments.
///     pub struct MyEventArgs {
///         /// My argument.
///         pub arg: String,
///         /// My event target.
///         pub target: WidgetPath,
///
///         ..
///
///         /// If `ctx.path.widget_id()` is in the `self.target` path.
///         fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
///             self.target.contains(ctx.path.widget_id())
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
#[macro_export]
macro_rules! event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:ident: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {$(
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            /// When the event happened.
            pub timestamp: std::time::Instant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*

            // Arc<AtomicBool> so we don't cause the $Args:!Send and block the user from creating event channels.
            stop_propagation: std::sync::Arc<std::sync::atomic::AtomicBool>,
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::sync::Arc::default(),
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }

            /// Requests that subsequent handlers skip this event.
            ///
            /// Cloned arguments signal stop for all clones.
            #[inline]
            pub fn stop_propagation(&self) {
                <Self as $crate::event::EventArgs>::stop_propagation(self)
            }

            /// If the handler must skip this event.
            ///
            /// Note that property level handlers don't need to check this, as those handlers are
            /// already not called when this is `true`. [`UiNode`](crate::UiNode) and
            /// [`AppExtension`](crate::app::AppExtension) implementers must check if this is `true`.
            #[inline]
            pub fn stop_propagation_requested(&self) -> bool {
                <Self as $crate::event::EventArgs>::stop_propagation_requested(self)
            }
        }
        impl $crate::event::EventArgs for $Args {
            #[inline]
            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }

            #[inline]
            $(#[$concerns_widget_outer])*
            fn concerns_widget(&$self, $ctx: &mut $crate::context::WidgetContext) -> bool {
                $($concerns_widget)+
            }

            #[inline]
            fn stop_propagation(&self) {
                self.stop_propagation.store(true, std::sync::atomic::Ordering::Relaxed);
            }

            #[inline]
            fn stop_propagation_requested(&self) -> bool {
                self.stop_propagation.load(std::sync::atomic::Ordering::Relaxed)
            }
        }
    )+};

    // match discard WidgetContext in concerns_widget.
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, _: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {
        $crate::event_args! { $(

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, _ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }

        )+ }
    };
}

#[doc(inline)]
pub use crate::event_args;

/// Declares new [`CancelableEventArgs`](crate::event::CancelableEventArgs) types.
///
/// Same syntax as [`event_args!`](macro.event_args.html) but the generated args is also cancelable.
///
/// # Example
///
/// ```
/// # use zero_ui_core::event::cancelable_event_args;
/// # use zero_ui_core::render::WidgetPath;
/// cancelable_event_args! {
///     /// My event arguments.
///     pub struct MyEventArgs {
///         /// My argument.
///         pub arg: String,
///         /// My event target.
///         pub target: WidgetPath,
///
///         ..
///
///         /// If `ctx.path.widget_id()` is in the `self.target` path.
///         fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
///             self.target.contains(ctx.path.widget_id())
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
#[macro_export]
macro_rules! cancelable_event_args {

    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:ident: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {$(
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            /// When the event happened.
            pub timestamp: std::time::Instant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*
            // Arc<AtomicBool> so we don't cause the $Args:!Send and block the user from creating event channels.
            stop_propagation: std::sync::Arc<std::sync::atomic::AtomicBool>,
            cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::sync::Arc::default(),
                    cancel: std::sync::Arc::default(),
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }

            /// Requests that subsequent handlers skip this event.
            ///
            /// Cloned arguments signal stop for all clones.
            #[inline]
            pub fn stop_propagation(&self) {
                <Self as $crate::event::EventArgs>::stop_propagation(self)
            }

            /// If the handler must skip this event.
            ///
            /// Note that property level handlers don't need to check this, as those handlers are
            /// already not called when this is `true`. [`UiNode`](crate::UiNode) and
            /// [`AppExtension`](crate::app::AppExtension) implementers must check if this is `true`.
            #[inline]
            pub fn stop_propagation_requested(&self) -> bool {
                <Self as $crate::event::EventArgs>::stop_propagation_requested(self)
            }

            /// Cancel the originating action.
            ///
            /// Cloned arguments signal cancel for all clones.
            #[inline]
            pub fn cancel(&self) {
                <Self as $crate::event::CancelableEventArgs>::cancel(self)
            }

            /// If the originating action must be canceled.
            #[inline]
            pub fn cancel_requested(&self) -> bool {
                <Self as $crate::event::CancelableEventArgs>::cancel_requested(self)
            }
        }
        impl $crate::event::EventArgs for $Args {
            #[inline]
            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }

            #[inline]
            $(#[$concerns_widget_outer])*
            fn concerns_widget(&$self, $ctx: &mut $crate::context::WidgetContext) -> bool {
                $($concerns_widget)+
            }

            #[inline]
            fn stop_propagation(&self) {
                self.stop_propagation.store(true, std::sync::atomic::Ordering::Relaxed);
            }

            #[inline]
            fn stop_propagation_requested(&self) -> bool {
                self.stop_propagation.load(std::sync::atomic::Ordering::Relaxed)
            }
        }
        impl $crate::event::CancelableEventArgs for $Args {
            #[inline]
            fn cancel_requested(&self) -> bool {
                self.cancel.load(std::sync::atomic::Ordering::Relaxed)
            }

            #[inline]
            fn cancel(&self) {
                self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    )+};

    // match discard WidgetContext in concerns_widget.
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, _: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {
        $crate::cancelable_event_args! { $(

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, _ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }

        )+ }
    };
}

#[doc(inline)]
pub use crate::cancelable_event_args;

/// Declares new low-pressure [`Event`](crate::event::Event) types.
///
/// # Example
///
/// ```
/// # use zero_ui_core::event::event;
/// # use zero_ui_core::gesture::ClickArgs;
/// event! {
///     /// Event docs.
///     pub ClickEvent: ClickArgs;
///
///     /// Other event docs.
///     pub DoubleClickEvent: ClickArgs;
/// }
/// ```
#[macro_export]
macro_rules! event {
    ($($(#[$outer:meta])* $vis:vis $Event:ident : $Args:path;)+) => {$(
        $(#[$outer])*
        #[derive(Clone, Copy, Debug)]
        $vis struct $Event;
        impl $Event {
            /// Gets the event arguments if the update is for this event.
            #[inline(always)]
            pub fn update<U: $crate::event::EventUpdateArgs>(args: &U) -> Option<&$crate::event::EventUpdate<$Event>> {
                <Self as $crate::event::Event>::update(args)
            }

            /// Schedule an event update.
            #[inline]
            pub fn notify(events: &$crate::event::Events, args: $Args) {
                <Self as $crate::event::Event>::notify(events, args);
            }
        }
        impl $crate::event::Event for $Event {
            type Args = $Args;
        }
    )+};
}
#[doc(inline)]
pub use crate::event;

/* Event Property */

struct OnEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool,
    H: FnMut(&mut WidgetContext, &E::Args),
{
    child: C,
    _event: PhantomData<E>,
    filter: F,
    handler: H,
}
#[impl_ui_node(child)]
impl<C, E, F, H> UiNode for OnEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        if let Some(args) = E::update(args) {
            self.child.event(ctx, args);
            if IsEnabled::get(ctx.vars) && !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                (self.handler)(ctx, args);
            }
        } else {
            self.child.event(ctx, args);
        }
    }
}

struct OnPreviewEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool,
    H: FnMut(&mut WidgetContext, &E::Args),
{
    child: C,
    _event: PhantomData<E>,
    filter: F,
    handler: H,
}
#[impl_ui_node(child)]
impl<C, E, F, H> UiNode for OnPreviewEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        if let Some(args) = E::update(args) {
            if IsEnabled::get(ctx.vars) && !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                (self.handler)(ctx, args);
            }
            self.child.event(ctx, args);
        } else {
            self.child.event(ctx, args);
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __event_property {
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
            filter: $filter:expr,
        }
    ) => { $crate::paste! {
        $(#[$on_event_attrs])*
        ///
        /// # Preview Event
        ///
        #[doc = "You can preview this event using [`on_pre_" $event "`]."]
        #[$crate::property(event, default(|_, _|{}))]
        $vis fn [<on_ $event>](
            child: impl $crate::UiNode,
            handler: impl FnMut(&mut $crate::context::WidgetContext, &$Args) + 'static
        ) -> impl $crate::UiNode {
            $crate::event::on_event(child, $Event, $filter, handler)
        }

        #[doc = "Preview [on_" $event "] event."]
        ///
        /// # Preview Events
        ///
        /// Preview events are fired before the main event, if you stop the propagation of a preview event
        /// the main event does not run. See [`on_pre_event`](crate::properties::events::on_pre_event) for more details.
        #[$crate::property(event, default(|_, _|{}))]
        $vis fn [<on_pre_ $event>](
            child: impl $crate::UiNode,
            handler: impl FnMut(&mut $crate::context::WidgetContext, &$Args) + 'static
        ) -> impl $crate::UiNode {
            $crate::event::on_pre_event(child, $Event, $filter, handler)
        }
    } };
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                filter: |ctx, args| $crate::event::EventArgs::concerns_widget(args, ctx),
            }
        }
    };
}
/// Declare one or more event properties.
///
/// Each declaration expands to a pair of properties `on_$event` and `on_pre_$event`. The preview property
/// calls [`on_pre_event`](crate::event::on_pre_event),
/// the main event property calls [`on_event`](crate::event::on_event).
///
/// # Example
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::event::{event_property, EventArgs};
/// # use zero_ui_core::keyboard::*;
/// event_property! {
///     /// on_key_down docs.
///     pub fn key_down {
///         event: KeyDownEvent,
///         args: KeyInputArgs,
///         // default filter is |ctx, args| args.concerns_widget(ctx)
///     }
///
///     pub(crate) fn space_down {
///         event: KeyDownEvent,
///         args: KeyInputArgs,
///         // optional filter:
///         filter: |ctx, args| args.concerns_widget(ctx) && args.key == Some(Key::Space),
///     }
/// }
/// ```
///
/// # Filter
///
/// App events can be listened from any `UiNode`. An event property must call the event handler only
/// in contexts where the event is relevant. Some event properties can also specialize further on top
/// of a more general app event. To implement this you can use a filter predicate.
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called. See [`on_event`] and [`on_pre_event`] for more information.
///
/// If you don't provide a filter predicate the default [`args.concerns_widget(ctx)`](EventArgs::concerns_widget) is used.
/// So if you want to extend the filter and not fully replace it you must call `args.concerns_widget(ctx)` in your custom filter.
#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path $(,
            filter: $filter:expr)? $(,)?
        }
    )+) => {$(
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                $(filter: $filter,)?
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
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
#[inline]
pub fn on_event<C, E, F, H>(child: C, _event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    OnEventNode {
        child,
        _event: PhantomData::<E>,
        filter,
        handler,
    }
}

/// Helper for declaring preview event properties with a custom filter.
///
/// This function is used by the [`event_property!`] macro.
///
/// # Filter
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested and the
/// widget is [enabled](IsEnabled). It must return `true` if the event arguments are relevant in the context of the
/// widget. If it returns `true` the `handler` closure is called.
///
/// # Route
///
/// The event `handler` is called before the [`on_event`] equivalent at the same context level. If the event
/// `filter` allows more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
pub fn on_pre_event<C, E, F, H>(child: C, _event: E, filter: F, handler: H) -> impl UiNode
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    OnPreviewEventNode {
        child,
        _event: PhantomData::<E>,
        filter,
        handler,
    }
}
