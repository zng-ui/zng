use std::{fmt, time::Instant, sync::{atomic::{AtomicBool, self}, Arc}, cell::{RefCell, Cell}, mem};

use crate::{WidgetId, WidgetPath, context::WindowContext, window::WindowId};

/// [`Event`] arguments.
pub trait EventArgs: fmt::Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;

    /// Generate an [`EventDeliveryList`] that defines all targets of the event.
    fn delivery_list(&self) -> EventDeliveryList;

    /// Propagation handle associated with this event instance.
    ///
    /// Cloned arguments share the same handle, some arguments may also share the handle
    /// of another event if they share the same cause.
    fn propagation(&self) -> &EventPropagationHandle;

    /// Calls `handler` and stops propagation if propagation is still allowed.
    ///
    /// Returns the `handler` result if it was called.
    fn handle<F, R>(&self, handler: F) -> Option<R>
    where
        F: FnOnce(&Self) -> R,
    {
        if self.propagation().is_stopped() {
            None
        } else {
            let r = handler(self);
            self.propagation().stop();
            Some(r)
        }
    }
}

/// Event propagation handle associated with one or multiple [`EventArgs`].
///
/// Event handlers can use this handle to signal subsequent handlers that they should skip handling the event.
///
/// You can get the propagation handle of any event argument by using the [`EventArgs::propagation`] method.
#[derive(Debug, Clone)]
pub struct EventPropagationHandle(Arc<AtomicBool>);
impl EventPropagationHandle {
    /// New in the not stopped default state.
    pub fn new() -> Self {
        EventPropagationHandle(Arc::new(AtomicBool::new(false)))
    }

    /// Signal subsequent handlers that the event is already handled.
    pub fn stop(&self) {
        // Is `Arc` to make `EventArgs` send, but stop handle is only useful in the UI thread, so
        // we don't need any ordering.
        self.0.store(true, atomic::Ordering::Relaxed);
    }

    /// If the handler must skip this event instance.
    ///
    /// Note that property level handlers don't need to check this, as those handlers are
    /// not called when this is `true`. Direct event listeners in [`UiNode`] and [`AppExtension`]
    /// must check if this is `true`.
    ///
    /// [`UiNode`]: crate::UiNode
    /// [`AppExtension`]: crate::app::AppExtension
    pub fn is_stopped(&self) -> bool {
        self.0.load(atomic::Ordering::Relaxed)
    }
}
impl Default for EventPropagationHandle {
    fn default() -> Self {
        EventPropagationHandle::new()
    }
}
impl PartialEq for EventPropagationHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for EventPropagationHandle {}
impl std::hash::Hash for EventPropagationHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.0) as usize;
        std::hash::Hash::hash(&ptr, state);
    }
}

#[derive(Debug)]
struct WindowDelivery {
    id: WindowId,
    widgets: Vec<WidgetPath>,
    all: bool,
}

/// Delivery list for an [`EventArgs`].
///
/// Windows and widgets use this list to find all targets of the event.
pub struct EventDeliveryList {
    windows: RefCell<Vec<WindowDelivery>>,
    all: bool,
    window: Cell<usize>,
    depth: Cell<usize>,

    search: RefCell<Vec<WidgetId>>,
}
impl fmt::Debug for EventDeliveryList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "EventDeliveryList {{")?;
        if self.all {
            writeln!(f, "   <all-widgets>")?;
        } else {
            for w in self.windows.borrow().iter() {
                if w.all {
                    writeln!(f, "   {}//<all-widgets>", w.id)?;
                } else {
                    for wgt in w.widgets.iter() {
                        writeln!(f, "   {wgt}")?;
                    }
                }
            }
        }
        writeln!(f, "}}")
    }
}
impl Default for EventDeliveryList {
    /// None.
    fn default() -> Self {
        Self::none()
    }
}
impl EventDeliveryList {
    /// Target no widgets or windows.
    ///
    /// Only app extensions receive the event.
    pub fn none() -> Self {
        Self {
            windows: RefCell::new(vec![]),
            all: false,
            window: Cell::new(0),
            depth: Cell::new(0),

            search: RefCell::new(vec![]),
        }
    }

    /// Target all widgets and windows.
    ///
    /// The event is broadcast to everyone.
    pub fn all() -> Self {
        let mut s = Self::none();
        s.all = true;
        s
    }

    /// All widgets inside the window.
    pub fn window(window_id: WindowId) -> Self {
        Self::none().with_window(window_id)
    }

    /// All widgets inside the window.
    pub fn window_opt(window_id: Option<WindowId>) -> Self {
        Self::none().with_window_opt(window_id)
    }

    /// All widgets in the path.
    pub fn widgets(widget_path: &WidgetPath) -> Self {
        Self::none().with_widgets(widget_path)
    }

    /// All widgets in each path on the list.
    pub fn widgets_list<'a>(list: impl IntoIterator<Item = &'a WidgetPath>) -> Self {
        Self::none().with_widgets_list(list)
    }

    /// All widgets in the path.
    pub fn widgets_opt(widget_path: Option<&WidgetPath>) -> Self {
        Self::none().with_widgets_opt(widget_path)
    }

    /// A widget ID to be searched before send.
    ///
    /// The windows info trees are searched before the event is send for delivery.
    pub fn find_widget(widget_id: WidgetId) -> Self {
        Self::none().with_find_widget(widget_id)
    }

    /// Add all widgets inside the window for delivery.
    ///
    /// The event is broadcast inside the window.
    pub fn with_window(mut self, window_id: WindowId) -> Self {
        if self.all {
            return self;
        }

        if let Some(w) = self.windows.get_mut().iter_mut().find(|w| w.id == window_id) {
            w.widgets.clear();
            w.all = true;
        } else {
            self.windows.get_mut().push(WindowDelivery {
                id: window_id,
                widgets: vec![],
                all: true,
            });
        }
        self
    }

    /// All the widgets in the window if it is some.
    pub fn with_window_opt(self, window_id: Option<WindowId>) -> Self {
        if let Some(window_id) = window_id {
            self.with_window(window_id)
        } else {
            self
        }
    }

    /// Add the widgets in the path to the delivery.
    pub fn with_widgets(mut self, widget_path: &WidgetPath) -> Self {
        if self.all {
            return self;
        }

        if let Some(w) = self.windows.get_mut().iter_mut().find(|w| w.id == widget_path.window_id()) {
            if !w.all {
                w.widgets.push(widget_path.clone());
            }
        } else {
            self.windows.get_mut().push(WindowDelivery {
                id: widget_path.window_id(),
                widgets: vec![widget_path.clone()],
                all: false,
            })
        }
        self
    }

    /// All the widgets in each path on the list.
    pub fn with_widgets_list<'a>(mut self, list: impl IntoIterator<Item = &'a WidgetPath>) -> Self {
        for path in list {
            self = self.with_widgets(path);
        }
        self
    }

    /// Add the widgets in the path if it is some.
    pub fn with_widgets_opt(self, widget_path: Option<&WidgetPath>) -> Self {
        if let Some(path) = widget_path {
            self.with_widgets(path)
        } else {
            self
        }
    }

    /// A widget ID to be searched before send.
    ///
    /// The windows info trees are searched before the event is send for delivery.
    pub fn with_find_widget(mut self, widget_id: WidgetId) -> Self {
        self.search.get_mut().push(widget_id);
        self
    }

    /// Returns `true` if the event has target in the window.
    pub fn enter_window(&self, ctx: &mut WindowContext) -> bool {
        if self.all {
            return true;
        }

        self.find_widgets(ctx);

        let window_id = *ctx.window_id;

        if let Some(i) = self.windows.borrow().iter().position(|w| w.id == window_id) {
            self.window.set(i);
            self.depth.set(0);
            true
        } else {
            false
        }
    }

    /// Returns `true` if the event has targets in the widget or targets the widget.
    pub fn enter_widget(&self, widget_id: WidgetId) -> bool {
        if self.all {
            self.depth.set(self.depth.get() + 1);
            return true;
        }

        let windows = self.windows.borrow();

        if windows.is_empty() {
            self.depth.set(self.depth.get() + 1);
            return false;
        }

        let window = &windows[self.window.get()];
        if window.all {
            self.depth.set(self.depth.get() + 1);
            true
        } else {
            for path in &window.widgets {
                let path = path.widgets_path();
                if path.len() > self.depth.get() && path[self.depth.get()] == widget_id {
                    self.depth.set(self.depth.get() + 1);
                    return true;
                }
            }
            false
        }
    }

    /// Must be called if [`enter_widget`] returned `true`.
    ///
    /// [`enter_widget`]: Self::enter_widget
    pub fn exit_widget(&self) {
        self.depth.set(self.depth.get() - 1);
    }

    /// Must be called if [`exit_window`] returned `true`.
    ///
    /// [`exit_window`]: Self::exit_window
    pub fn exit_window(&self) {
        self.depth.set(0);
    }

    /// Resolve `find_widget` pending queries.
    fn find_widgets(&self, ctx: &mut WindowContext) {
        let search = mem::take(&mut *self.search.borrow_mut());

        if self.all || search.is_empty() {
            return;
        }

        if let Some(windows) = ctx.services.get::<crate::window::Windows>() {
            let mut self_windows = self.windows.borrow_mut();
            'search: for wgt in search {
                for win in windows.widget_trees() {
                    if let Some(info) = win.get(wgt) {
                        if let Some(w) = self_windows.iter_mut().find(|w| w.id == win.window_id()) {
                            if !w.all {
                                w.widgets.push(info.path());
                            }
                        } else {
                            self_windows.push(WindowDelivery {
                                id: win.window_id(),
                                widgets: vec![info.path()],
                                all: false,
                            });
                        }

                        continue 'search;
                    }
                }
            }
        }
    }
}

///<span data-del-macro-root></span> Declares new [`EventArgs`] types.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{event::event_args, WidgetPath, text::{Text, formatx}};
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
///         /// If `target` starts with the current path.
///         fn delivery_list(&self) -> EventDeliveryList {
///             EventDeliveryList::widgets(&self.target)
///         }
///
///         /// Optional validation, if defined the generated `new` and `now` functions call it and unwrap the result.
///         ///
///         /// The error type can be any type that implement `Debug`.
///         fn validate(&self) -> Result<(), Text> {
///             if self.arg.contains("error") {
///                 return Err(formatx!("invalid arg `{}`", self.arg));
///             }
///             Ok(())
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
///
/// [`EventArgs`]: crate::event::EventArgs
#[macro_export]
macro_rules! event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$delivery_list_outer:meta])*
            fn delivery_list(&$self:ident) -> EventDeliveryList { $($delivery_list:tt)+ }

            $(
                $(#[$validate_outer:meta])*
                fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
            )?
        }
    )+) => {$(
        $crate::__event_args! {
            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*

                ..

                $(#[$delivery_list_outer])*
                fn delivery_list(&$self) -> EventDeliveryList { $($delivery_list)+ }

                $(
                    $(#[$validate_outer])*
                    fn validate(&$self_v) -> Result<(), $ValidationError> { $($validate)+ }
                )?
            }
        }
    )+};
}
#[doc(hidden)]
#[macro_export]
macro_rules! __event_args {
    // match validate
    (
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$delivery_list_outer:meta])*
            fn delivery_list(&$self:ident) -> EventDeliveryList { $($delivery_list:tt)+ }

            $(#[$validate_outer:meta])*
            fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
        }
    ) => {
        $crate::__event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$delivery_list_outer])*
                fn delivery_list(&$self) -> EventDeliveryList { $($delivery_list)+ }
            }
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[track_caller]
            #[allow(clippy::too_many_arguments)]
            pub fn new(
                timestamp: impl Into<std::time::Instant>,
                propagation_handle: $crate::event::EventPropagationHandle,
                $($arg : impl Into<$arg_ty>),*
            ) -> Self {
                let args = $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    propagation_handle,
                };
                args.assert_valid();
                args
            }

            /// New args from values that convert [into](Into) the argument types.
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[allow(clippy::too_many_arguments)]
            pub fn try_new(
                timestamp: impl Into<std::time::Instant>,
                propagation_handle: $crate::event::EventPropagationHandle,
                $($arg : impl Into<$arg_ty>),*
            ) -> Result<Self, $ValidationError> {
                let args = $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    propagation_handle,
                };
                args.validate()?;
                Ok(args)
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[track_caller]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
            }

            /// Arguments for event that happened now (`Instant::now`).
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[allow(clippy::too_many_arguments)]
            pub fn try_now($($arg : impl Into<$arg_ty>),*) -> Result<Self, $ValidationError> {
                Self::try_new(std::time::Instant::now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
            }

            $(#[$validate_outer])*
            pub fn validate(&$self_v) -> Result<(), $ValidationError> {
                $($validate)+
            }

            /// Panics if the arguments are invalid.
            #[track_caller]
            pub fn assert_valid(&self) {
                if let Err(e) = self.validate() {
                    panic!("invalid `{}`, {e:?}", stringify!($Args));
                }
            }
        }
    };

    // match no validate
    (
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$delivery_list_outer:meta])*
            fn delivery_list(&$self:ident) -> EventDeliveryList { $($delivery_list:tt)+ }
        }
    ) => {
        $crate::__event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$delivery_list_outer])*
                fn delivery_list(&$self) -> EventDeliveryList { $($delivery_list)+  }
            }
        }

        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[allow(clippy::too_many_arguments)]
            pub fn new(
                timestamp: impl Into<std::time::Instant>,
                propagation_handle: $crate::event::EventPropagationHandle,
                $($arg : impl Into<$arg_ty>),*
            ) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    propagation_handle,
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
            }
        }
    };

    // common code between validating and not.
    (common=>

        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$delivery_list_outer:meta])*
            fn delivery_list(&$self:ident) -> EventDeliveryList { $($delivery_list:tt)+ }
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            /// When the event happened.
            pub timestamp: std::time::Instant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*

            propagation_handle: $crate::event::EventPropagationHandle,
        }
        impl $Args {
            /// Propagation handle associated with this event instance.
            #[allow(unused)]
            pub fn propagation(&self) -> &$crate::event::EventPropagationHandle {
                <Self as $crate::event::EventArgs>::propagation(self)
            }

            $(#[$delivery_list_outer])*
            #[allow(unused)]
            pub fn delivery_list(&self) -> $crate::event::EventDeliveryList {
                <Self as $crate::event::EventArgs>::delivery_list(self)
            }

            /// Calls `handler` and stops propagation if propagation is still allowed.
            ///
            /// Returns the `handler` result if it was called.
            #[allow(unused)]
            pub fn handle<F, R>(&self, handler: F) -> Option<R>
            where
                F: FnOnce(&Self) -> R,
            {
                <Self as $crate::event::EventArgs>::handle(self, handler)
            }
        }
        impl $crate::event::EventArgs for $Args {

            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }


            $(#[$delivery_list_outer])*
            fn delivery_list(&$self) -> $crate::event::EventDeliveryList {
                use $crate::event::EventDeliveryList;

                $($delivery_list)+
            }


            fn propagation(&self) -> &$crate::event::EventPropagationHandle {
                &self.propagation_handle
            }
        }
    };
}
#[doc(inline)]
pub use crate::event_args;