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
        const __KEY: $crate::hot_reload::PatchKey = $crate::hot_reload::PatchKey {
            file: std::file!(),
            line: std::line!(),
            column: std::column!(),
            item_name: stringify!($IDENT),
        };
        $crate::paste! {
            static [<$IDENT _COLD>] : $Ty = $init;
            static mut $IDENT: &$Ty = &[<$IDENT _COLD>];
            unsafe fn [<$IDENT _INIT>](static_ptr: *const ()) -> *const () {
                $crate::hot_reload::init_static(&mut $IDENT, static_ptr)
            }

            $crate::hot_reload::HOT_STATICS! {
                static [<$IDENT _REGISTER>]: ($crate::hot_reload::PatchKey, unsafe fn(*const ()) -> *const ()) = (
                    __KEY,
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
pub static HOT_STATICS: [(PatchKey, unsafe fn(*const ()) -> *const ())];

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatchKey {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
    pub item_name: &'static str,
}

impl std::fmt::Debug for PatchKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}#{}", self.file, self.line, self.column, self.item_name)
    }
}
