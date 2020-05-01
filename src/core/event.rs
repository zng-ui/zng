use crate::core::context::{Events, WidgetContext};
use std::cell::{Cell, UnsafeCell};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Instant;

/// [`Event`](Event) arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;
    /// If this event arguments is relevant to the widget context.
    fn concerns_widget(&self, _: &mut WidgetContext) -> bool;
}

/// [`Event`](Event) arguments that can be canceled.
pub trait CancelableEventArgs: EventArgs {
    /// If the originating action must be canceled.
    fn cancel_requested(&self) -> bool;
    /// Cancel the originating action.
    fn cancel(&self);
}

/// Identifies an event type.
pub trait Event: 'static {
    /// Event arguments.
    type Args: EventArgs;

    const IS_HIGH_PRESSURE: bool = false;
}

/// Identifies an event type for an action that
/// can be canceled.
pub trait CancelableEvent: Event + 'static {
    /// Event arguments.
    type Args: CancelableEventArgs;
}

struct EventChannelInner<T> {
    data: UnsafeCell<Vec<T>>,
    listener_count: Cell<usize>,
    is_high_pressure: bool,
}

struct EventChannel<T: 'static> {
    r: Rc<EventChannelInner<T>>,
}
impl<T: 'static> Clone for EventChannel<T> {
    fn clone(&self) -> Self {
        EventChannel { r: Rc::clone(&self.r) }
    }
}
impl<T: 'static> EventChannel<T> {
    pub(crate) fn notify(self, new_update: T, _assert_events_not_borrowed: &mut Events, cleanup: &mut Vec<Box<dyn FnOnce()>>) {
        // SAFETY: This is safe because borrows are bound to the `Events` instance
        // so if we have a mutable reference to it no event value is borrowed.
        let data = unsafe { &mut *self.r.data.get() };
        data.push(new_update);

        if data.len() == 1 {
            // register for cleanup once
            cleanup.push(Box::new(move || {
                unsafe { &mut *self.r.data.get() }.clear();
            }))
        }
    }

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::core::UiNode::update).
    pub fn updates<'a>(&'a self, _events: &'a Events) -> &'a [T] {
        // SAFETY: This is safe because we are bounding the value lifetime with
        // the `Events` lifetime and we require a mutable reference to `Events` to
        // modify the value.
        unsafe { &*self.r.data.get() }.as_ref()
    }

    /// If this update is notified using the [`UiNode::update_hp`](crate::core::UiNode::update_hp) method.
    pub fn is_high_pressure(&self) -> bool {
        self.r.is_high_pressure
    }

    pub fn listener_count(&self) -> usize {
        self.r.listener_count.get()
    }

    pub fn has_listeners(&self) -> bool {
        self.listener_count() > 0
    }

    pub fn on_new_listener(&self) {
        self.r.listener_count.set(self.r.listener_count.get() + 1)
    }

    pub fn on_drop_listener(&self) {
        self.r.listener_count.set(self.r.listener_count.get() - 1)
    }
}

/// Read-only reference to an event channel.
pub struct EventListener<T: 'static> {
    chan: EventChannel<T>,
}
impl<T: 'static> Clone for EventListener<T> {
    fn clone(&self) -> Self {
        EventListener::new(self.chan.clone())
    }
}
impl<T: 'static> EventListener<T> {
    fn new(chan: EventChannel<T>) -> Self {
        chan.on_new_listener();
        EventListener { chan }
    }

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::core::UiNode::update).
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        self.chan.updates(events)
    }

    /// If [`updates`](EventListener::updates) is not empty.
    pub fn has_updates<'a>(&'a self, events: &'a Events) -> bool {
        !self.updates(events).is_empty()
    }

    /// If this update is notified using the [`UiNode::update_hp`](crate::core::UiNode::update_hp) method.
    pub fn is_high_pressure(&self) -> bool {
        self.chan.is_high_pressure()
    }

    /// Listener that never updates.
    pub fn never(is_high_pressure: bool) -> Self {
        EventEmitter::new(is_high_pressure).into_listener()
    }
}

impl<T: 'static> Drop for EventListener<T> {
    fn drop(&mut self) {
        self.chan.on_drop_listener();
    }
}

/// Read-write reference to an event channel.
pub struct EventEmitter<T: 'static> {
    chan: EventChannel<T>,
}
impl<T: 'static> Clone for EventEmitter<T> {
    fn clone(&self) -> Self {
        EventEmitter { chan: self.chan.clone() }
    }
}
impl<T: 'static> EventEmitter<T> {
    /// New event emitter.
    ///
    /// # Arguments
    /// * `is_high_pressure`: If this event is notified using the [`UiNode::update_hp`](crate::core::UiNode::update_hp) method.
    pub fn new(is_high_pressure: bool) -> Self {
        EventEmitter {
            chan: EventChannel {
                r: Rc::new(EventChannelInner {
                    data: UnsafeCell::default(),
                    listener_count: Cell::new(0),
                    is_high_pressure,
                }),
            },
        }
    }

    /// Number of listener to this event emitter.
    pub fn listener_count(&self) -> usize {
        self.chan.listener_count()
    }

    /// If this event emitter has any listeners.
    pub fn has_listeners(&self) -> bool {
        self.chan.has_listeners()
    }

    /// Gets a reference to the updates that happened in between calls of [`UiNode::update`](crate::core::UiNode::update).
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        self.chan.updates(events)
    }

    /// If [`updates`](EventEmitter::updates) is not empty.
    pub fn has_updates<'a>(&'a self, events: &'a Events) -> bool {
        !self.updates(events).is_empty()
    }

    /// If this event is notified using the [`UiNode::update_hp`](crate::core::UiNode::update_hp) method.
    pub fn is_high_pressure(&self) -> bool {
        self.chan.is_high_pressure()
    }

    /// Gets a new event listener linked with this emitter.
    pub fn listener(&self) -> EventListener<T> {
        EventListener::new(self.chan.clone())
    }

    /// Converts this emitter instance into a listener.
    pub fn into_listener(self) -> EventListener<T> {
        EventListener::new(self.chan)
    }

    pub(crate) fn notify(self, new_update: T, assert_events_not_borrowed: &mut Events, cleanup: &mut Vec<Box<dyn FnOnce()>>) {
        self.chan.notify(new_update, assert_events_not_borrowed, cleanup);
    }
}

/// Declares new [`EventArgs`](crate::core::event::EventArgs) types.
///
/// # Example
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() {
/// use zero_ui::core::render::WidgetPath;
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
///         /// If `ctx.widget_id` is in the `self.target` path.
///         fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
///             self.target.contains(ctx.widget_id)
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// # }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui::core::render::WidgetPath;
/// #
/// /// My event arguments.
/// #[derive(Debug, Clone)]
/// pub struct MyEventArgs {
///     /// When the event happened.
///     pub timestamp: std::time::Instant,
///     /// My argument.
///     pub arg: String,
///     /// My event target.
///     pub target: WidgetPath,
/// }
///
/// impl MyEventArgs {
///     #[inline]
///     pub fn new(
///         timestamp: impl Into<std::time::Instant>,
///         arg: impl Into<String>,
///         target: impl Into<WidgetPath>,
///     ) -> Self {
///         MyEventArgs {
///             timestamp: timestamp.into(),
///             arg: arg.into(),
///             target: target.into(),
///         }
///     }
///
///     /// Arguments for event that happened now (`Instant::now`).
///     #[inline]
///     pub fn now(arg: impl Into<String>, target: impl Into<WidgetPath>) -> Self {
///         Self::new(std::time::Instant::now(), arg, target)
///     }
/// }
///
/// impl zero_ui::core::event::EventArgs for MyEventArgs {
///     #[inline]
///     fn timestamp(&self) -> std::time::Instant {
///         self.timestamp
///     }
///
///     #[inline]
///     /// If `ctx.widget_id` is in the `self.target` path.
///     fn concerns_widget(&self, ctx: &mut zero_ui::core::context::WidgetContext) -> bool {
///         self.target.contains(ctx.widget_id)
///     }
/// }
/// ```
#[macro_export]
macro_rules! __event_args {
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
        }
        impl $Args {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }
        }
        impl $crate::core::event::EventArgs for $Args {
            #[inline]
            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }

            #[inline]
            $(#[$concerns_widget_outer])*
            fn concerns_widget(&$self, $ctx: &mut $crate::core::context::WidgetContext) -> bool {
                $($concerns_widget)+
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
pub use __event_args as event_args;

/// Declares new [`CancelableEventArgs`](crate::core::event::CancelableEventArgs) types.
///
/// Same syntax as [`event_args!`](macro.event_args.html) but the generated args is also cancelable.
///
/// # Example
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() {
/// use zero_ui::core::render::WidgetPath;
///
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
///         /// If `ctx.widget_id` is in the `self.target` path.
///         fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
///             self.target.contains(ctx.widget_id)
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// # }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui::core::render::WidgetPath;
/// #
/// /// My event arguments.
/// #[derive(Debug, Clone)]
/// pub struct MyEventArgs {
///     /// When the event happened.
///     pub timestamp: std::time::Instant,
///     /// My argument.
///     pub arg: String,
///     /// My event target.
///     pub target: WidgetPath,
///
///     cancel: std::rc::Rc<std::cell::Cell<bool>>
/// }
///
/// impl MyEventArgs {
///     #[inline]
///     pub fn new(
///         timestamp: impl Into<std::time::Instant>,
///         arg: impl Into<String>,
///         target: impl Into<WidgetPath>,
///     ) -> Self {
///         MyEventArgs {
///             timestamp: timestamp.into(),
///             arg: arg.into(),
///             target: target.into(),
///             cancel: std::rc::Rc::default()
///         }
///     }
///
///     /// Arguments for event that happened now (`Instant::now`).
///     #[inline]
///     pub fn now(arg: impl Into<String>, target: impl Into<WidgetPath>) -> Self {
///         Self::new(std::time::Instant::now(), arg, target)
///     }
/// }
///
/// impl zero_ui::core::event::EventArgs for MyEventArgs {
///     #[inline]
///     fn timestamp(&self) -> std::time::Instant {
///         self.timestamp
///     }
///
///     #[inline]
///     /// If `ctx.widget_id` is in the `self.target` path.
///     fn concerns_widget(&self, ctx: &mut zero_ui::core::context::WidgetContext) -> bool {
///         self.target.contains(ctx.widget_id)
///     }
/// }
///
/// impl zero_ui::core::event::CancelableEventArgs for MyEventArgs {
///     /// If a listener canceled the action.
///     #[inline]
///     fn cancel_requested(&self) -> bool {
///         self.cancel.get()
///     }
///
///     /// Cancel the action.
///     ///
///     /// Cloned args are still linked, canceling one will cancel the others.
///     #[inline]
///     fn cancel(&self) {
///         self.cancel.set(true);
///     }
/// }
/// ```
#[macro_export]
macro_rules! __cancelable_event_args {

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
            cancel: std::rc::Rc<std::cell::Cell<bool>>
        }
        impl $Args {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    cancel: std::rc::Rc::default()
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }
        }
        impl $crate::core::event::EventArgs for $Args {
            #[inline]
            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }

            #[inline]
            $(#[$concerns_widget_outer])*
            fn concerns_widget(&$self, $ctx: &mut $crate::core::context::WidgetContext) -> bool {
                $($concerns_widget)+
            }
        }
        impl $crate::core::event::CancelableEventArgs for $Args {
            /// If a listener canceled the action.
            #[inline]
            fn cancel_requested(&self) -> bool {
                self.cancel.get()
            }

            /// Cancel the action.
            ///
            /// Cloned args are still linked, canceling one will cancel the others.
            #[inline]
            fn cancel(&self) {
                self.cancel.set(true);
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
        $crate::__cancelable_event_args! { $(

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
pub use __cancelable_event_args as cancelable_event_args;
