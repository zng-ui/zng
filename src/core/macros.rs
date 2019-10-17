/// Declare and implement a unique ID type.
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
                pub fn new() -> Self {
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
                pub fn get(self) -> u64 {
                    self.0.get()
                }
            }
        )+
    };
}