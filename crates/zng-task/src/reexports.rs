/// Recommended blocking lock primitives.
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

#[doc(no_inline)]
pub use rayon;
