//#[macro_export]
//macro_rules! ui {
//    ($($mtd:ident: $($arg:expr),+;)+ => $child:expr) => {
//        {
//            let child = $child;
//            $(let child = $mtd(child, $($arg),+);)+
//            {child}
//        }
//    };
//}

///The enclose macro for easier cloning
#[macro_export]
macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

/// Declare and implement a unique ID type. Optionally also declare
/// a lazy initialization type for static variables.
///
/// # Examples
/// ```
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
            /// so IDs are unique for the process duration but order is not garanteed.
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

#[macro_export]
macro_rules! profile_scope {
    ($($args:tt)+) => {
        #[cfg(feature = "app_profiler")]
        let _profile_scope =
            $crate::core::profiler::ProfileScope::new(format!($($args)+));
    };
}

/// Declares new [StateKey](crate::core::context::StateKey) types.
#[macro_export]
macro_rules! state_key {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty;)+) => {$(
        $(#[$outer])*
        $vis struct $ident;

        impl $crate::core::context::StateKey for $ident {
            type Type = $type;
        }
    )+};
}

/// Declares new [ContextVar] types.
#[macro_export]
macro_rules! context_var {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $default:expr;)+) => {$(
        $(#[$outer])*
        #[derive(Clone, Copy)]
        $vis struct $ident;

        impl $crate::core::var::ContextVar for $ident {
            type Type = $type;

            fn default() -> &'static Self::Type {
                static DEFAULT: $type = $default;
                &DEFAULT
            }
        }
    )+};
}

/// Declares event args `struct`s that follows the basic initialization
/// pattern and implement `EventArgs`.
///
/// # Example
/// ```
/// event_args! {
///     /// My event arguments.
///     pub struct MyEventArgs {
///         pub arg: String
///     }
/// }
/// ```
#[macro_export]
macro_rules! event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $(pub $arg:ident : $arg_ty:ty,)*
            concerns_widget: $($concerns_widget:tt)+
        }
    )+) => {$(
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            pub timestamp: std::time::Instant,
            $(pub $arg : $arg_ty,)*
        }
        impl $Args {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: std::time::Instant, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp,
                    $($arg: $arg.into(),)*
                }
            }

            /// Arguments for event that happened `Instant::now`.
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
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                let impl_ = $($concerns_widget)+;
                impl_(self, ctx)
            }
        }
    )+};
}

/// Declares event args `struct`s for events whos originating action can be
/// cancelled.
///
/// Same sintax as `[event_args!]` but the generated code also implements
/// [CancelableEventArgs]
#[macro_export]
macro_rules! cancelable_event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $(pub $arg:ident : $arg_ty:ty,)*
            concerns_widget: $($concerns_widget:tt)+
        }
    )+) => {$(
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            pub timestamp: std::time::Instant,
            $(pub $arg : $arg_ty,)*
            cancel: std::cell::Cell<bool>
        }
        impl $Args {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: std::time::Instant, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp,
                    $($arg: $arg.into(),)*
                    cancel: std::cell::Cell::new(false)
                }
            }

            /// Arguments for event that happened `Instant::now`.
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
            fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
                let impl_ = $($concerns_widget)+;
                impl_(self, ctx)
            }
        }
        impl $crate::core::event::CancelableEventArgs for $Args {
            /// If a handler canceled the action.
            #[inline]
            fn cancel_requested(&self) -> bool {
                self.cancel.get()
            }

            /// Cancel the action.
            #[inline]
            fn cancel(&self) {
                self.cancel.set(true);
            }
        }
    )+};
}
