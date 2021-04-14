//! Crate visible macros.

/// Declare a new unique id type.
macro_rules! unique_id {
    ($(#[$docs:meta])* $vis:vis struct $Type:ident;) => {

        $(#[$docs])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis struct $Type(std::num::NonZeroU64);

        impl $Type {
            fn next() -> &'static std::sync::atomic::AtomicU64 {
                use std::sync::atomic::AtomicU64;
                static NEXT: AtomicU64 = AtomicU64::new(1);
                &NEXT
            }

            /// Generates a new unique ID.
            ///
            /// # Panics
            /// Panics if called more then `u64::MAX` times.
            pub fn new_unique() -> Self {
                use std::sync::atomic::Ordering;

                let id = Self::next().fetch_add(1, Ordering::Relaxed);

                if let Some(id) = std::num::NonZeroU64::new(id) {
                    $Type(id)
                } else {
                    Self::next().store(0, Ordering::SeqCst);
                    panic!("`{}` reached `u64::MAX` IDs.", stringify!($Type))
                }
            }

            /// Retrieve the underlying `u64` value.
            #[allow(dead_code)]
            #[inline]
            pub fn get(self) -> u64 {
                self.0.get()
            }

            /// Creates an id from a raw value.
            ///
            /// # Safety
            ///
            /// This is only safe if called with a value provided by [`get`](Self::get).
            #[allow(dead_code)]
            pub unsafe fn from_raw(raw: u64) -> $Type {
                $Type(std::num::NonZeroU64::new_unchecked(raw))
            }

            /// Creates an id from a raw value.
            ///
            /// Checks if `raw` is in the range of generated widgets.
            #[inline]
            #[allow(dead_code)]
            pub fn new(raw: u64) -> Option<$Type> {
                use std::sync::atomic::Ordering;

                if raw >= 1 && raw < Self::next().load(Ordering::Relaxed) {
                    // SAFETY: we just validated raw.
                    Some(unsafe { Self::from_raw(raw) })
                } else {
                    None
                }
            }
        }
    };
}

/// Calls `eprintln!("error: {}", format_args!($))` with `error` colored bright red and bold.
#[allow(unused)]
macro_rules! error_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "error".bright_red().bold(), format_args!($($tt)*))
    }}
}

/// Calls `eprintln!("warning: {}", format_args!($))` with `warning` colored bright yellow and bold.
#[allow(unused)]
macro_rules! warn_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "warning".bright_yellow().bold(), format_args!($($tt)*))
    }}
}

#[allow(unused)]
#[cfg(debug_assertions)]
macro_rules! print_backtrace {
    () => {
        eprintln!("\n\n\n=========BACKTRACE=========\n{:?}", backtrace::Backtrace::new())
    };
}

/// Implements From and IntoVar without boilerplate.
macro_rules! impl_from_and_into_var {
    ($(
        $(#[$docs:meta])*
        fn from $(< $($T:ident  $(: $TConstrain:path)?),+ $(,)?>)? (
            $($name:ident)? // single ident OR
            $( ( // tuple deconstruct of
                $(
                    $($tuple_names:ident)? // single idents OR
                    $( ( // another tuple deconstruct of
                        $($tuple_inner_names:ident ),+ // inner idents
                    ) )?
                ),+
            ) )?
            : $From:ty) -> $To:ty
            $convert_block:block
    )+) => {
        $(
            impl $(< $($T $(: $TConstrain)?),+ >)? From<$From> for $To {
                $(#[$docs])*
                #[inline]
                fn from(
                    $($name)?
                    $( (
                        $(
                            $($tuple_names)?
                            $( (
                                $($tuple_inner_names),+
                            ) )?
                        ),+
                    ) )?
                    : $From) -> Self
                    $convert_block

            }

            impl $(< $($T $(: $TConstrain + Clone)?),+ >)? $crate::var::IntoVar<$To> for $From {
                type Var = $crate::var::OwnedVar<$To>;

                $(#[$docs])*
                fn into_var(self) -> Self::Var {
                    $crate::var::OwnedVar(self.into())
                }
            }
        )+
    };
}

/// Generates a type that can only have a single instance at a time.
macro_rules! singleton_assert {
    ($Singleton:ident) => {
        struct $Singleton {}

        impl $Singleton {
            fn flag() -> &'static std::sync::atomic::AtomicBool {
                static ALIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                &ALIVE
            }

            pub fn assert_new() -> Self {
                if Self::flag().load(std::sync::atomic::Ordering::SeqCst) {
                    panic!("only a single instance of `{}` can exist at at time", stringify!($Singleton))
                }

                Self::flag().store(true, std::sync::atomic::Ordering::SeqCst);

                $Singleton {}
            }
        }

        impl Drop for $Singleton {
            fn drop(&mut self) {
                Self::flag().store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
    };
}
