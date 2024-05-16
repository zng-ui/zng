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
