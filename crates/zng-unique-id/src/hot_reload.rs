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
        $crate::paste! {
            struct [<_K $IDENT:camel>];
            impl $crate::hot_reload::PatchKey for [<_K $IDENT:camel>] {
                fn id(&'static self) -> &'static str {
                    std::any::type_name::<[<_K $IDENT:camel>]>()
                }
            }

            static [<$IDENT _COLD>] : $Ty = $init;
            #[allow(non_camel_case_types)]
            static mut $IDENT: &$Ty = &[<$IDENT _COLD>];
            #[allow(non_snake_case)]
            unsafe fn [<$IDENT _INIT>](static_ptr: *const ()) -> *const () {
                unsafe { $crate::hot_reload::init_static(&mut $IDENT, static_ptr) }
            }

            // expanded from:
            #[$crate::hot_reload::__linkme::distributed_slice($crate::hot_reload::HOT_STATICS)]
            #[linkme(crate=$crate::hot_reload::__linkme)]
            #[doc(hidden)]
            static [<$IDENT _REGISTER>]: (&'static dyn $crate::hot_reload::PatchKey, unsafe fn(*const ()) -> *const ()) = (
                &[<_K $IDENT:camel>],
                [<$IDENT _INIT>]
            );

        }
    };
}

#[doc(hidden)]
pub unsafe fn init_static<T>(s: &mut &'static T, static_ptr: *const ()) -> *const () {
    if static_ptr.is_null() {
        *s as *const T as *const ()
    } else {
        *s = unsafe { &*(static_ptr as *const T) };
        std::ptr::null()
    }
}

use std::{any::Any, fmt, ops};

#[doc(hidden)]
#[cfg(feature = "hot_reload")]
pub use linkme as __linkme;

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
pub use once_cell::sync::OnceCell as OnceCellLazy;

#[doc(hidden)]
pub struct Lazy<T: 'static> {
    #[cfg(feature = "hot_reload")]
    inner: fn(&mut Option<T>) -> &'static T,
    #[cfg(not(feature = "hot_reload"))]
    inner: (OnceCellLazy<T>, fn() -> T),
}

impl<T: 'static> Lazy<T> {
    #[doc(hidden)]
    #[cfg(feature = "hot_reload")]
    pub const fn new(inner: fn(&mut Option<T>) -> &'static T) -> Self {
        Self { inner }
    }

    #[doc(hidden)]
    #[cfg(not(feature = "hot_reload"))]
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            inner: (OnceCellLazy::new(), init),
        }
    }
}
impl<T: 'static> ops::Deref for Lazy<T> {
    type Target = T;

    #[cfg(feature = "hot_reload")]
    fn deref(&self) -> &Self::Target {
        (self.inner)(&mut None)
    }

    #[cfg(not(feature = "hot_reload"))]
    fn deref(&self) -> &Self::Target {
        self.inner.0.get_or_init(|| (self.inner.1)())
    }
}

/// Initializes a [`lazy_static!`] with a custom value if it is not yet inited.
///
/// [`lazy_static!`]: crate::lazy_static
pub fn lazy_static_init<T>(lazy_static: &'static Lazy<T>, value: T) -> Result<&'static T, T> {
    let mut value = Some(value);

    #[cfg(feature = "hot_reload")]
    let r = (lazy_static.inner)(&mut value);
    #[cfg(not(feature = "hot_reload"))]
    let r = {
        let (lazy, _) = &lazy_static.inner;
        lazy.get_or_init(|| value.take().unwrap())
    };

    match value {
        Some(v) => Err(v),
        None => Ok(r),
    }
}

#[doc(hidden)]
#[cfg(feature = "hot_reload")]
pub fn lazy_static_ref<T>(lazy_static: &'static OnceCellLazy<T>, init: fn() -> T, override_init: &mut Option<T>) -> &'static T {
    lazy_static.get_or_init(|| match override_init.take() {
        Some(o) => o,
        None => init(),
    })
}

/// Implementation of `lazy_static!` that supports hot reloading.
///
/// The syntax is similar to the [`lazy_static`](https://docs.rs/lazy_static) crate,
/// but the generated code uses the [`once_cell::sync::Lazy`](https://docs.rs/once_cell/once_cell/sync/struct.Lazy.html)
/// type internally.
#[macro_export]
macro_rules! lazy_static {
    ($(
        $(#[$attr:meta])*
        $vis:vis static ref $N:ident : $T:ty = $e:expr;
    )+) => {
        $(
           $crate::hot_reload::lazy_static_impl! {
                $(#[$attr])*
                $vis static ref $N : $T = $e;
           }
        )+
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! lazy_static_patchable {
    (
        $(#[$attr:meta])*
        $vis:vis static ref $N:ident : $T:ty = $e:expr;
    ) => {
        $crate::paste! {
            fn [<_ $N:lower _hot>](__override: &mut Option<$T>) -> &'static $T {
                fn __init() -> $T {
                    $e
                }
                $crate::hot_static! {
                    static IMPL: $crate::hot_reload::OnceCellLazy<$T> = $crate::hot_reload::OnceCellLazy::new();
                }
                $crate::hot_reload::lazy_static_ref($crate::hot_static_ref!(IMPL), __init, __override)
            }

            $(#[$attr])*
            $vis static $N: $crate::hot_reload::Lazy<$T> = $crate::hot_reload::Lazy::new([<_ $N:lower _hot>]);
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! lazy_static_not_patchable {
    (
        $(#[$attr:meta])*
        $vis:vis static ref $N:ident : $T:ty = $e:expr;
    ) => {
        $crate::paste! {
            fn [<_ $N:lower _init>]() -> $T {
                $e
            }

            $(#[$attr])*
            $vis static $N: $crate::hot_reload::Lazy<$T> = $crate::hot_reload::Lazy::new([<_ $N:lower _init>]);
        }
    };
}

#[doc(hidden)]
#[cfg(not(feature = "hot_reload"))]
pub use crate::lazy_static_not_patchable as lazy_static_impl;

#[doc(hidden)]
#[cfg(feature = "hot_reload")]
pub use crate::lazy_static_patchable as lazy_static_impl;
