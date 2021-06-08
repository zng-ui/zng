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

/// Generates a type that can only have a single instance per thread.
macro_rules! thread_singleton {
    ($Singleton:ident) => {
        struct $Singleton {
            _not_send: std::marker::PhantomData<Rc<()>>,
        }
        impl $Singleton {
            std::thread_local! {
                static IN_USE: std::cell::Cell<bool> = std::cell::Cell::new(false);
            }

            fn set(in_use: bool) {
                Self::IN_USE.with(|f| f.set(in_use));
            }

            /// If an instance of this type already exists in this thread.
            pub fn in_use() -> bool {
                Self::IN_USE.with(|f| f.get())
            }

            /// Panics if [`Self::in_use`], otherwise creates the single instance of `Self` for the thread.
            pub fn assert_new(type_name: &str) -> Self {
                if Self::in_use() {
                    panic!("only a single instance of `{}` can exist per thread at a time", type_name)
                }
                Self::set(true);

                Self {
                    _not_send: std::marker::PhantomData,
                }
            }
        }
        impl Drop for $Singleton {
            fn drop(&mut self) {
                Self::set(false);
            }
        }
    };
}

/// Runs a cleanup action once on drop.
pub(crate) struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    pub fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}
