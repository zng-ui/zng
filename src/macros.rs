/// Declare and implement a unique ID type. Optionally also declare
/// a lazy initialization type for static variables.
///
/// # Examples
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() {
/// uid! { pub struct PublicId(_); }
/// uid! { struct PrivateId(_); }
///
/// let unique_id = PublicId::new_unique();
/// let underlying_value = unique_id.get();
/// ```
///
/// ## Lazy Initialization
/// ```
/// uid! { pub struct PublicId(_) { new_lazy() -> pub struct PublicIdRef } }
///
/// static UNIQUE_ID: PublicIdRef = PublicId::new_lazy();
/// let unique_id = *UNIQUE_ID;
/// assert_eq!(unique_id, *UNIQUE_ID);
/// # }
/// ```
macro_rules! uid {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Type:ident (_);
    )+) => {
        $(
            $(#[$outer])*
            /// # Details
            /// Underlying value is a `NonZeroU64` generated using a relaxed global atomic `fetch_add`,
            /// so IDs are unique for the process duration, but order is not guaranteed.
            ///
            /// Panics if you somehow reach `u64::max_value()` calls to `new`.
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
            $vis struct $Type(std::num::NonZeroU64);

            impl $Type {
                /// Generates a new unique ID.
                ///
                /// # Panics
                /// Panics if called more then `u64::max_value()` times.
                pub fn new_unique() -> Self {
                    use std::sync::atomic::{AtomicU64, Ordering};
                    static NEXT: AtomicU64 = AtomicU64::new(1);

                    let id = NEXT.fetch_add(1, Ordering::Relaxed);

                    if let Some(id) = std::num::NonZeroU64::new(id) {
                        $Type(id)
                    } else {
                        NEXT.store(0, Ordering::SeqCst);
                        panic!("`{}` reached `u64::max_value()` IDs.",  stringify!($Type))
                    }
                }

                /// Retrieve the underlying `u64` value.
                #[allow(dead_code)]
                #[inline]
                pub fn get(self) -> u64 {
                    self.0.get()
                }
            }
        )+
    };

    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Type:ident (_) { new_lazy() -> $vis_ref:vis struct $TypeRef:ident };
    )+) => {$(
        uid! {$vis struct $Type(_);}

        /// Dereferences to an unique ID that is generated on the first deref.
        $vis_ref struct $TypeRef (once_cell::sync::OnceCell<$Type>);

        impl $Type {
            /// New lazy initialized unique key. Use this for static
            /// variables.
            #[inline]
            pub const fn new_lazy() -> $TypeRef {
                $TypeRef(once_cell::sync::OnceCell::new())
            }
        }

        impl std::ops::Deref for $TypeRef {
            type Target = $Type;
            #[inline]
            fn deref(&self) -> &Self::Target {
                self.0.get_or_init($Type::new_unique)
            }
        }
    )+};
}

/// Declares a [`ProfileScope`](crate::core::profiler::ProfileScope) variable if
/// the `app_profiler` feature is active.
///
/// # Example
///
/// If compiled with the `app_profiler` feature, this will register a "do-things" scope
/// that starts when the macro was called and has the duration of the block.
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() {
/// # fn do_thing() { }
/// # fn do_another_thing() { }
/// {
///     profile_scope!("do-things");
///
///     do_thing();
///     do_another_thing();
/// }
/// # }
/// ```
///
/// You can also format strings:
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() {
/// # let thing = "";
/// profile_scope!("do-{}", thing);
/// # }
/// ```
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "app_profiler")]
        let _profile_scope =
            $crate::core::profiler::ProfileScope::new($name);
    };
    ($($args:tt)+) => {
        #[cfg(feature = "app_profiler")]
        let _profile_scope =
            $crate::core::profiler::ProfileScope::new(format!($($args)+));
    };
}

/// Declares new [`StateKey`](crate::core::context::StateKey) types.
#[macro_export]
macro_rules! state_key {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty;)+) => {$(
        $(#[$outer])*
        /// # StateKey
        /// This `struct` is a [`StateKey`](zero_ui::core::context::StateKey).
        $vis struct $ident;

        impl $crate::core::context::StateKey for $ident {
            type Type = $type;
        }
    )+};
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
}

/// Declares new [`CancelableEventArgs`](crate::core::event::CancelableEventArgs) types.
///
/// Same syntax as [`event_args!`](event_args) but the generated args is also cancelable.
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
}

/// Creates a [`Text`](crate::core::types::Text) by calling the `format!` macro and
/// wrapping the result in a `Cow::Owned`.
///
/// # Example
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() {
/// use zero_ui::core::types::Text;
///
/// let text: Text = formatx!("Hello {}", "World!");
/// # }
/// ```
#[macro_export]
macro_rules! formatx {
    ($($tt:tt)*) => {
        std::borrow::Cow::Owned(format!($($tt)*))
    };
}

/// Calls `eprintln!("error: {}", format_args!($))` with `error` colored bright red and bold.
macro_rules! error_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "error".bright_red().bold(), format_args!($($tt)*))
    }}
}
