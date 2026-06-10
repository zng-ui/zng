/// Recommended blocking locks.
///
/// The `Mutex` and `RwLock` types reexported here are recommended, they are
/// more optimized than `std` alternatives because they don't implement lock poisoning,
/// have `const` initialization and integrate with the `zng` feature `"deadlock_detection"`.
///
/// # Full API
///
/// See the [`parking_lot`] crate for the full API.
///
/// [`parking_lot`]: https://docs.rs/parking_lot
pub mod parking_lot {
    #[doc(no_inline)]
    pub use ::parking_lot::{
        ArcMutexGuard, ArcRwLockReadGuard, ArcRwLockWriteGuard, Condvar, MappedMutexGuard, MappedRwLockReadGuard, MappedRwLockWriteGuard,
        Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard,
    };
}

/// Parallel iterators.
///
/// This module mostly reexports the primary traits from the [`rayon`] crate.
///
/// This module also includes [`ParallelIteratorWithCtx`] that propagates the
/// zng app context to rayon tasks.
///
/// # Full API
///
/// See the [`rayon`] crate for the full API.
///
/// [`rayon`]: https://docs.rs/rayon
/// [`ParallelIteratorWithCtx`]: crate::rayon::ParallelIteratorWithCtx
pub mod rayon {
    #[doc(no_inline)]
    pub use ::rayon::{iter, slice, str};

    pub use crate::rayon_ctx::*;

    /// Rayon traits imported `as _`.
    pub mod prelude {
        #[doc(no_inline)]
        pub use ::rayon::{
            iter::FromParallelIterator as _, iter::IndexedParallelIterator as _, iter::IntoParallelIterator as _,
            iter::IntoParallelRefIterator as _, iter::IntoParallelRefMutIterator as _, iter::ParallelBridge as _,
            iter::ParallelDrainFull as _, iter::ParallelDrainRange as _, iter::ParallelExtend as _, iter::ParallelIterator as _,
            slice::ParallelSlice as _, slice::ParallelSliceMut as _, str::ParallelString as _,
        };

        pub use super::ParallelIteratorExt as _;
    }
}
