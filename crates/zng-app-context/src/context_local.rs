use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    AppLocal, AppLocalId, AppLocalImpl, LocalContext, LocalValueKind, ReadOnlyRwLock, RwLockReadGuardOwned, RwLockWriteGuardOwned,
};

#[doc(hidden)]
pub struct ContextLocalData<T: Send + Sync + 'static> {
    default_init: fn() -> T,
    default_value: Option<Arc<T>>,
}
impl<T: Send + Sync + 'static> ContextLocalData<T> {
    #[doc(hidden)]
    pub const fn new(default_init: fn() -> T) -> Self {
        Self {
            default_init,
            default_value: None,
        }
    }
}

/// Represents an [`AppLocal<T>`] value that can be temporarily overridden in a context.
///
/// The *context* works across threads, as long as the threads are instrumented using [`LocalContext`].
///
/// Use the [`context_local!`] macro to declare a static variable in the same style as [`thread_local!`].
pub struct ContextLocal<T: Send + Sync + 'static> {
    data: AppLocal<ContextLocalData<T>>,
}
impl<T: Send + Sync + 'static> ContextLocal<T> {
    #[doc(hidden)]
    pub const fn new(storage: fn() -> &'static dyn AppLocalImpl<ContextLocalData<T>>) -> Self {
        Self {
            data: AppLocal::new(storage),
        }
    }

    /// Gets an ID for this context local instance that is valid for the lifetime of the process.
    ///
    /// Note that comparing two `&'static CTX_LOCAL` pointers is incorrect, because in `"hot_reload"` builds the statics
    /// can be different and still represent the same app local. This ID identifies the actual inner pointer.
    pub fn id(&'static self) -> AppLocalId {
        self.data.id()
    }

    /// Calls `f` with the `value` loaded in context.
    ///
    /// The `value` is moved into context, `f` is called, then the value is moved back to `value`.
    ///
    /// # Panics
    ///
    /// Panics if `value` is `None`.
    pub fn with_context<R>(&'static self, value: &mut Option<Arc<T>>, f: impl FnOnce() -> R) -> R {
        let mut r = None;
        let f = || r = Some(f());
        #[cfg(feature = "dyn_closure")]
        let f: Box<dyn FnOnce()> = Box::new(f);

        LocalContext::with_value_ctx(self, LocalValueKind::Local, value, f);

        r.unwrap()
    }

    /// Same as [`with_context`], but `value` represents a variable.
    ///
    /// Values loaded with this method are captured by [`CaptureFilter::ContextVars`].
    ///
    /// [`with_context`]: Self::with_context
    pub fn with_context_var<R>(&'static self, value: &mut Option<Arc<T>>, f: impl FnOnce() -> R) -> R {
        let mut r = None;
        let f = || r = Some(f());

        #[cfg(feature = "dyn_closure")]
        let f: Box<dyn FnOnce()> = Box::new(f);

        LocalContext::with_value_ctx(self, LocalValueKind::Var, value, f);

        r.unwrap()
    }

    /// Calls `f` with no value loaded in context.
    pub fn with_default<R>(&'static self, f: impl FnOnce() -> R) -> R {
        let mut r = None;
        let f = || r = Some(f());

        #[cfg(feature = "dyn_closure")]
        let f: Box<dyn FnOnce()> = Box::new(f);
        LocalContext::with_default_ctx(self, f);

        r.unwrap()
    }

    /// Gets if no value is set in the context.
    pub fn is_default(&'static self) -> bool {
        !LocalContext::contains(self.id())
    }

    /// Clone a reference to the current value in the context or the default value.
    pub fn get(&'static self) -> Arc<T> {
        let cl = self.data.read();
        match LocalContext::get(self.id()) {
            Some(c) => Arc::downcast(c.0).unwrap(),
            None => match &cl.default_value {
                Some(d) => d.clone(),
                None => {
                    drop(cl);
                    let mut cl = self.data.write();
                    match &cl.default_value {
                        None => {
                            let d = Arc::new((cl.default_init)());
                            cl.default_value = Some(d.clone());
                            d
                        }
                        Some(d) => d.clone(),
                    }
                }
            },
        }
    }

    /// Clone the current value in the context or the default value.
    pub fn get_clone(&'static self) -> T
    where
        T: Clone,
    {
        let cl = self.data.read();
        match LocalContext::get(self.id()) {
            Some(c) => c.0.downcast_ref::<T>().unwrap().clone(),
            None => match &cl.default_value {
                Some(d) => d.as_ref().clone(),
                None => {
                    drop(cl);
                    let mut cl = self.data.write();
                    match &cl.default_value {
                        None => {
                            let val = (cl.default_init)();
                            let r = val.clone();
                            cl.default_value = Some(Arc::new(val));
                            r
                        }
                        Some(d) => d.as_ref().clone(),
                    }
                }
            },
        }
    }
}

impl<T: Send + Sync + 'static> ContextLocal<RwLock<T>> {
    /// Gets a read-only shared reference to the current context value.
    pub fn read_only(&'static self) -> ReadOnlyRwLock<T> {
        ReadOnlyRwLock::new(self.get())
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::read` for more details.
    pub fn read(&'static self) -> RwLockReadGuardOwned<T> {
        RwLockReadGuardOwned::lock(self.get())
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// Unlike `read`, this method is guaranteed to succeed without blocking if
    /// another read lock is held at the time of the call.
    ///
    /// See `parking_lot::RwLock::read` for more details.
    pub fn read_recursive(&'static self) -> RwLockReadGuardOwned<T> {
        RwLockReadGuardOwned::lock_recursive(self.get())
    }

    /// Locks this `RwLock` with exclusive write access, blocking the current
    /// thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::write` for more details.
    pub fn write(&'static self) -> RwLockWriteGuardOwned<T> {
        RwLockWriteGuardOwned::lock(self.get())
    }

    /// Try lock this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::try_read` for more details.
    pub fn try_read(&'static self) -> Option<RwLockReadGuardOwned<T>> {
        RwLockReadGuardOwned::try_lock(self.get())
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::try_read_recursive` for more details.
    pub fn try_read_recursive(&'static self) -> Option<RwLockReadGuardOwned<T>> {
        RwLockReadGuardOwned::try_lock_recursive(self.get())
    }

    /// Locks this `RwLock` with exclusive write access, blocking the current
    /// thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::try_write` for more details.
    pub fn try_write(&'static self) -> Option<RwLockWriteGuardOwned<T>> {
        RwLockWriteGuardOwned::try_lock(self.get())
    }
}

///<span data-del-macro-root></span> Declares new app and context local variable.
///
/// # Examples
///
/// ```
/// # use zng_app_context::*;
/// context_local! {
///     /// A public documented value.
///     pub static FOO: u8 = 10u8;
///
///     // A private value.
///     static BAR: String = "Into!";
/// }
/// ```
///
/// # Default Value
///
/// All contextual values must have a fallback value that is used when no context is loaded.
///
/// The default value is instantiated once per app, the expression can be any static value that converts [`Into<T>`].
///
/// # Usage
///
/// After you declare the context local you can use it by loading a contextual value for the duration of a closure call.
///
/// ```
/// # use zng_app_context::*;
/// # use std::sync::Arc;
/// context_local! { static FOO: String = "default"; }
///
/// fn print_value() {
///     println!("value is {}!", FOO.get());
/// }
///
/// let _scope = LocalContext::start_app(AppId::new_unique());
///
/// let mut value = Some(Arc::new(String::from("other")));
/// FOO.with_context(&mut value, || {
///     print!("in context, ");
///     print_value();
/// });
///
/// print!("out of context, ");
/// print_value();
/// ```
///
/// The example above prints:
///
/// ```text
/// in context, value is other!
/// out of context, value is default!
/// ```
///
/// See [`ContextLocal<T>`] for more details.
#[macro_export]
macro_rules! context_local {
    ($(
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    )+) => {$(
        $crate::context_local_impl! {
            $(#[$meta])*
            $vis static $IDENT: $T = $init;
        }
    )+};
}

#[doc(hidden)]
#[macro_export]
macro_rules! context_local_impl_single {
    ($(
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    )+) => {$(
        $(#[$meta])*
        $vis static $IDENT: $crate::ContextLocal<$T> = {
            fn s() -> &'static dyn $crate::AppLocalImpl<$crate::ContextLocalData<$T>> {
                fn init() -> $T {
                    std::convert::Into::into($init)
                }
                $crate::hot_static! {
                    static IMPL: $crate::AppLocalConst<$crate::ContextLocalData<$T>> =
                    $crate::AppLocalConst::new(
                        $crate::ContextLocalData::new(init)
                    );
                }
                $crate::hot_static_ref!(IMPL)
            }
            $crate::ContextLocal::new(s)
        };
    )+};
}

#[doc(hidden)]
#[macro_export]
macro_rules! context_local_impl_multi {
    ($(
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    )+) => {$(
        $(#[$meta])*
        $vis static $IDENT: $crate::ContextLocal<$T> = {
            fn s() -> &'static dyn $crate::AppLocalImpl<$crate::ContextLocalData<$T>> {
                fn init() -> $T {
                    std::convert::Into::into($init)
                }
                $crate::hot_static! {
                    static IMPL: $crate::AppLocalVec<$crate::ContextLocalData<$T>> =
                    $crate::AppLocalVec::new(
                        || $crate::ContextLocalData::new(init)
                    );
                }
                $crate::hot_static_ref!(IMPL)
            }
            $crate::ContextLocal::new(s)
        };
    )+};
}

#[cfg(feature = "multi_app")]
#[doc(hidden)]
pub use context_local_impl_multi as context_local_impl;

#[cfg(not(feature = "multi_app"))]
#[doc(hidden)]
pub use context_local_impl_single as context_local_impl;
