use std::sync::{Arc, atomic::AtomicBool};

use zng_var::VarValue;

use crate::widget::WidgetId;
use atomic::Ordering::Relaxed;

/// Represents an event update.
pub trait EventArgs: VarValue {
    /// Instant this event update happened.
    fn timestamp(&self) -> crate::DInstant;

    /// Propagation handle associated with this event instance.
    ///
    /// Cloned arguments share the same handle, some arguments may also share the handle
    /// of another event if they share the same cause.
    fn propagation(&self) -> &EventPropagationHandle;

    /// Gets if the widget is in any of the target paths in this update.
    fn is_in_target(&self, widget: WidgetId) -> bool;
}

///<span data-del-macro-root></span> Declares new [`EventArgs`] types.
///
/// The macro syntax is similar to `struct` declaration, but after the args struct members you must add `..` and then
/// the `fn is_in_target(&self, widget: WidgetId) -> bool { }` method that matches the widget target.
///
/// After the `is_in_target` method you can also optionally add a `fn validate(&self) -> Result<(), Txt> { }` method
/// that validates the arguments.
///
/// The macro expansion implements the [`EventArgs`] trait for the new structs, it generates a public `timestamp`
/// member and a `new` and `now` associated functions. The `new` function instantiates args with custom timestamp and propagation handle,
/// the `now` function provides the timestamp and propagation handle and is the primary way to instantiate args.
///
/// # Examples
///
/// ```
/// # use zng_app::{event::event_args, widget::info::WidgetPath};
/// # use zng_txt::*;
/// #
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
///         fn is_in_target(&self, widget: WidgetId) -> bool { } {
///             self.target.contains(widget)
///         }
///
///         /// Optional validation, if defined the generated `new` and `now` functions call it and unwrap the result.
///         ///
///         /// The error type can be any type that implement `Debug`.
///         fn validate(&self) -> Result<(), Txt> {
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
            $(#[$is_in_target_outer:meta])*
            fn is_in_target(&$self:ident, $is_in_target_id:ident: WidgetId) -> bool { $($is_in_target:tt)* }

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

                $(#[$is_in_target_outer])*
                fn is_in_target(&$self, $is_in_target_id: WidgetId) -> bool { $($is_in_target)* }

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
            $(#[$is_in_target_outer:meta])*
            fn is_in_target(&$self:ident, $is_in_target_id:ident: WidgetId) -> bool { $($is_in_target:tt)* }

            $(#[$validate_outer:meta])*
            fn validate(&$self_v:ident) -> Result<(), $ValidationError:path> { $($validate:tt)+ }
        }
    ) => {
        $crate::__event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$is_in_target_outer])*
                fn is_in_target(&$self, $is_in_target_id: WidgetId) -> bool { $($is_in_target)* }
            }
        }
        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[allow(clippy::too_many_arguments)]
            pub fn new(
                timestamp: impl Into<$crate::DInstant>,
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
                timestamp: impl Into<$crate::DInstant>,
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

            /// Arguments for event that happened now (`INSTANT.now`).
            ///
            /// # Panics
            ///
            /// Panics if the arguments are invalid.
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new($crate::INSTANT.now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
            }

            /// Arguments for event that happened now (`INSTANT.now`).
            ///
            /// Returns an error if the constructed arguments are invalid.
            #[allow(clippy::too_many_arguments)]
            pub fn try_now($($arg : impl Into<$arg_ty>),*) -> Result<Self, $ValidationError> {
                Self::try_new($crate::INSTANT.now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
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
            $(#[$is_in_target_outer:meta])*
            fn is_in_target(&$self:ident, $is_in_target_id:ident: WidgetId) -> bool { $($is_in_target:tt)* }
        }
    ) => {
        $crate::__event_args! {common=>

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$is_in_target_outer])*
                fn is_in_target(&$self, $is_in_target_id: WidgetId) -> bool { $($is_in_target)* }
            }
        }

        impl $Args {
            /// New args from values that convert [into](Into) the argument types.
            #[allow(clippy::too_many_arguments)]
            pub fn new(
                timestamp: impl Into<$crate::DInstant>,
                propagation_handle: $crate::event::EventPropagationHandle,
                $($arg : impl Into<$arg_ty>),*
            ) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    propagation_handle,
                }
            }

            /// Arguments for event that happened now (`INSTANT.now`).
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new($crate::INSTANT.now(), $crate::event::EventPropagationHandle::new(), $($arg),*)
            }
        }
    };

    // common code between validating and not.
    (common=>

        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$is_in_target_outer:meta])*
            fn is_in_target(&$self:ident, $is_in_target_id:ident: WidgetId) -> bool { $($is_in_target:tt)* }
        }
    ) => {
        $(#[$outer])*
        #[derive(Debug, Clone, PartialEq)]
        $vis struct $Args {
            /// Instant the event happened.
            pub timestamp: $crate::DInstant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*

            propagation_handle: $crate::event::EventPropagationHandle,
        }
        impl $crate::event::EventArgs for $Args {
            fn timestamp(&self) -> $crate::DInstant {
                self.timestamp
            }

            $(#[$is_in_target_outer])*
            fn is_in_target(&$self, $is_in_target_id: $crate::widget::WidgetId) -> bool {
                let _ = $is_in_target_id;
                $($is_in_target)*
            }

            fn propagation(&self) -> &$crate::event::EventPropagationHandle {
                &self.propagation_handle
            }
        }
    };
}
#[doc(inline)]
pub use crate::event_args;

/// Event propagation handle associated with one or multiple [`EventArgs`].
///
/// Event handlers can use this to signal subsequent handlers that the event is already handled and they should
/// operate as if the event was not received.
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
        self.0.store(true, Relaxed);
    }

    /// If the handler must skip this event instance.
    ///
    /// Note that property level handlers don't need to check this, as those handlers are
    /// not called when this is `true`. Direct event listeners in [`UiNode`] and [`AppExtension`]
    /// must check if this is `true`.
    ///
    /// [`UiNode`]: crate::widget::node::UiNode
    /// [`AppExtension`]: crate::AppExtension
    pub fn is_stopped(&self) -> bool {
        self.0.load(Relaxed)
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
