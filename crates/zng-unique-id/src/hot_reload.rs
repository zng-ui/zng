// # Full API
//
// The full API is implemented in `zng-ext-hot-reload`, `HOT_STATICS` and `hot_static` are declared
// in this crate to only to be reachable in more workspace crates.

/// Declares a patchable static.
///
/// In builds with Cargo feature `hot_reload` this generates an unsafe static double reference that can be addressed by name and
/// patched in a dynamically loaded build of the exact same crate.
///
/// Use [`hot_static_ref!`] to safely reference the static, attempting to access the variable in any other way is undefined behavior.
///
/// Note that you can only declare private static items, this is by design, you can share the [`hot_static_ref!`] output at
/// a higher visibility.
///
/// See `zng::hot_reload` for more details and links to the full API. This macro is declared on the `zng-unique-id` crate
/// only to avoid circular dependencies in the Zng workspace.
///
/// [`hot_static_ref!`]: crate::hot_static_ref!
#[macro_export]
macro_rules! hot_static {
    (
        static $IDENT:ident: $Ty:ty = $init:expr;
    ) => {
        $crate::hot_reload::hot_static_impl! {
            static $IDENT: $Ty = $init;
        }
    };
}

/// Static reference to a [`hot_static!`].
///
/// [`hot_static!`]: crate::hot_static!
#[macro_export]
macro_rules! hot_static_ref {
    ($PATH:path) => {
        $crate::hot_reload::hot_static_ref_impl!($PATH)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! hot_static_not_patchable {
    (
        static $IDENT:ident: $Ty:ty = $init:expr;
    ) => {
        static $IDENT: $Ty = $init;
    };
}
#[doc(hidden)]
#[macro_export]
macro_rules! hot_static_patchable {
    (
        $vis:vis static $IDENT:ident: $Ty:ty = $init:expr;
    ) => {
        struct _K;
        impl $crate::hot_reload::PatchKey for _K {
            fn id(&'static self) -> &'static str {
                std::any::type_name::<_K>()
            }
        }
        $crate::paste! {
            static [<$IDENT _COLD>] : $Ty = $init;
            static mut $IDENT: &$Ty = &[<$IDENT _COLD>];
            unsafe fn [<$IDENT _INIT>](static_ptr: *const ()) -> *const () {
                $crate::hot_reload::init_static(&mut $IDENT, static_ptr)
            }

            $crate::hot_reload::HOT_STATICS! {
                static [<$IDENT _REGISTER>]: (&'static dyn $crate::hot_reload::PatchKey, unsafe fn(*const ()) -> *const ()) = (
                    &_K,
                    [<$IDENT _INIT>]
                );
            }
        }
    };
}

#[doc(hidden)]
pub unsafe fn init_static<T>(s: &mut &'static T, static_ptr: *const ()) -> *const () {
    if static_ptr.is_null() {
        *s as *const T as *const ()
    } else {
        *s = &*(static_ptr as *const T);
        std::ptr::null()
    }
}

use std::{any::Any, fmt};

#[doc(hidden)]
#[cfg(not(feature = "hot_reload"))]
pub use crate::hot_static_not_patchable as hot_static_impl;

#[doc(hidden)]
#[cfg(feature = "hot_reload")]
pub use crate::hot_static_patchable as hot_static_impl;

#[doc(hidden)]
#[macro_export]
macro_rules! hot_static_ref_not_patchable {
    ($PATH:path) => {
        &$PATH
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! hot_static_ref_patchable {
    ($PATH:path) => {
        // SAFETY: hot_static does not mutate after dylib init.
        unsafe { $PATH }
    };
}

#[doc(hidden)]
#[cfg(not(feature = "hot_reload"))]
pub use crate::hot_static_ref_not_patchable as hot_static_ref_impl;

#[doc(hidden)]
#[cfg(feature = "hot_reload")]
pub use crate::hot_static_ref_patchable as hot_static_ref_impl;

#[doc(hidden)]
#[cfg(feature = "hot_reload")]
#[linkme::distributed_slice]
pub static HOT_STATICS: [(&'static dyn PatchKey, unsafe fn(*const ()) -> *const ())];

#[doc(hidden)]
pub trait PatchKey: Send + Sync + Any {
    fn id(&'static self) -> &'static str;
}
impl PartialEq for &'static dyn PatchKey {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
impl Eq for &'static dyn PatchKey {}
impl std::hash::Hash for &'static dyn PatchKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(self.id(), state)
    }
}
impl fmt::Debug for &'static dyn PatchKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.id(), f)
    }
}

#[doc(hidden)]
pub use lazy_static::{__Deref, lazy, LazyStatic};

#[macro_export]
#[doc(hidden)]
macro_rules! __lazy_static_create {
    ($NAME:ident, $T:ty) => {
        $crate::hot_static! {
            static $NAME: $crate::hot_reload::lazy::Lazy<$T> = $crate::hot_reload::lazy::Lazy::INIT;
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __lazy_static_internal {
    // optional visibility restrictions are wrapped in `()` to allow for
    // explicitly passing otherwise implicit information about private items
    ($(#[$attr:meta])* ($($vis:tt)*) static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        $crate::__lazy_static_internal!(@MAKE TY, $(#[$attr])*, ($($vis)*), $N);
        $crate::__lazy_static_internal!(@TAIL, $N : $T = $e);
        $crate::lazy_static!($($t)*);
    };
    (@TAIL, $N:ident : $T:ty = $e:expr) => {
        impl $crate::hot_reload::__Deref for $N {
            type Target = $T;
            fn deref(&self) -> &$T {
                #[inline(always)]
                fn __static_ref_initialize() -> $T { $e }

                #[inline(always)]
                fn __stability() -> &'static $T {
                    $crate::__lazy_static_create!(LAZY, $T);
                    $crate::hot_static_ref!(LAZY).get(__static_ref_initialize)
                }
                __stability()
            }
        }
        impl $crate::hot_reload::LazyStatic for $N {
            fn initialize(lazy: &Self) {
                let _ = &**lazy;
            }
        }
    };
    // `vis` is wrapped in `()` to prevent parsing ambiguity
    (@MAKE TY, $(#[$attr:meta])*, ($($vis:tt)*), $N:ident) => {
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        $(#[$attr])*
        $($vis)* struct $N {__private_field: ()}
        #[doc(hidden)]
        $($vis)* static $N: $N = $N {__private_field: ()};
    };
    () => ()
}

/// Implementation of `lazy_static!` that supports hot reloading.
///
/// The syntax is identical as the [`lazy_static`](https://docs.rs/lazy_static) crate,
/// the `lazy_static::LazyStatic` trait is also implemented.
#[macro_export(local_inner_macros)]
macro_rules! lazy_static {
    ($(#[$attr:meta])* static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        // use `()` to explicitly forward the information about private items
        $crate::__lazy_static_internal!($(#[$attr])* () static ref $N : $T = $e; $($t)*);
    };
    ($(#[$attr:meta])* $vis:vis static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        $crate::__lazy_static_internal!($(#[$attr])* ($vis) static ref $N : $T = $e; $($t)*);
    };
    () => ()
}
