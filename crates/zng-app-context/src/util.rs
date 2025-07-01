use std::{mem, ops, sync::Arc};

use parking_lot::RwLock;

/// Represents a read guard for an `Arc<RwLock<T>>` that owns a reference to it.
pub struct RwLockReadGuardOwned<T: 'static> {
    lock: parking_lot::RwLockReadGuard<'static, T>,
    _owned: Arc<RwLock<T>>,
}
impl<T> RwLockReadGuardOwned<T> {
    /// Lock owned.    
    ///
    /// See `parking_lot::RwLock::read` for more details.
    pub fn lock(own: Arc<RwLock<T>>) -> Self {
        Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockReadGuard<'_, T>, parking_lot::RwLockReadGuard<'static, T>>(own.read()) },
            _owned: own,
        }
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::read_recursive` for more details.
    pub fn lock_recursive(own: Arc<RwLock<T>>) -> Self {
        Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe {
                mem::transmute::<parking_lot::RwLockReadGuard<'_, T>, parking_lot::RwLockReadGuard<'static, T>>(own.read_recursive())
            },
            _owned: own,
        }
    }

    /// Try lock owned.
    ///
    /// See `parking_lot::RwLock::try_read` for more details.
    pub fn try_lock(own: Arc<RwLock<T>>) -> Option<Self> {
        let lock = own.try_read()?;
        Some(Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockReadGuard<'_, T>, parking_lot::RwLockReadGuard<'static, T>>(lock) },
            _owned: own,
        })
    }

    /// Try lock owned.
    ///
    /// See `parking_lot::RwLock::try_read` for more details.
    pub fn try_lock_recursive(own: Arc<RwLock<T>>) -> Option<Self> {
        let lock = own.try_read_recursive()?;
        Some(Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockReadGuard<'_, T>, parking_lot::RwLockReadGuard<'static, T>>(lock) },
            _owned: own,
        })
    }

    /// Make a new `MappedRwLockReadGuardOwned` for a component of the locked data.
    ///
    /// This is an associated function that needs to be
    /// used as `RwLockReadGuardOwned::map(...)`. A method would interfere with methods of
    /// the same name on the contents of the locked data.
    pub fn map<O>(guard: Self, map: impl FnOnce(&T) -> &O) -> MappedRwLockReadGuardOwned<T, O> {
        MappedRwLockReadGuardOwned {
            lock: parking_lot::RwLockReadGuard::map(guard.lock, map),
            _owned: guard._owned,
        }
    }
}
impl<T> ops::Deref for RwLockReadGuardOwned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}

/// Represents a read guard for an `Arc<RwLock<T>>` that owns a reference to it, mapped from another read guard.
pub struct MappedRwLockReadGuardOwned<T: 'static, O: 'static> {
    lock: parking_lot::MappedRwLockReadGuard<'static, O>,
    _owned: Arc<RwLock<T>>,
}
impl<T, O> MappedRwLockReadGuardOwned<T, O> {
    /// Make a new `MappedRwLockReadGuardOwned` for a component of the locked data.
    ///
    /// This is an associated function that needs to be
    /// used as `MappedRwLockReadGuardOwned::map(...)`. A method would interfere with methods of
    /// the same name on the contents of the locked data.
    pub fn map<O2>(guard: Self, map: impl FnOnce(&O) -> &O2) -> MappedRwLockReadGuardOwned<T, O2> {
        MappedRwLockReadGuardOwned {
            lock: parking_lot::MappedRwLockReadGuard::map(guard.lock, map),
            _owned: guard._owned,
        }
    }
}
impl<T, O> ops::Deref for MappedRwLockReadGuardOwned<T, O> {
    type Target = O;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}

/// Represents a read guard for an `Arc<RwLock<T>>` that owns a reference to it.
pub struct RwLockWriteGuardOwned<T: 'static> {
    lock: parking_lot::RwLockWriteGuard<'static, T>,
    _owned: Arc<RwLock<T>>,
}
impl<T> RwLockWriteGuardOwned<T> {
    /// Lock owned.
    ///
    /// See `parking_lot::RwLock::write` for more details.
    pub fn lock(own: Arc<RwLock<T>>) -> Self {
        Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockWriteGuard<'_, T>, parking_lot::RwLockWriteGuard<'static, T>>(own.write()) },
            _owned: own,
        }
    }

    /// Lock owned.
    ///
    /// See `parking_lot::RwLock::try_write` for more details.
    pub fn try_lock(own: Arc<RwLock<T>>) -> Option<Self> {
        let lock = own.try_write()?;
        Some(Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockWriteGuard<'_, T>, parking_lot::RwLockWriteGuard<'static, T>>(lock) },
            _owned: own,
        })
    }

    /// Make a new `MappedRwLockReadGuardOwned` for a component of the locked data.
    ///
    /// This is an associated function that needs to be
    /// used as `MappedRwLockReadGuardOwned::map(...)`. A method would interfere with methods of
    /// the same name on the contents of the locked data.
    pub fn map<O>(guard: Self, map: impl FnOnce(&mut T) -> &mut O) -> MappedRwLockWriteGuardOwned<T, O> {
        MappedRwLockWriteGuardOwned {
            lock: parking_lot::RwLockWriteGuard::map(guard.lock, map),
            _owned: guard._owned,
        }
    }
}
impl<T> ops::Deref for RwLockWriteGuardOwned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}
impl<T> ops::DerefMut for RwLockWriteGuardOwned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lock.deref_mut()
    }
}

/// Represents a write guard for an `Arc<RwLock<T>>` that owns a reference to it, mapped from another read guard.
pub struct MappedRwLockWriteGuardOwned<T: 'static, O: 'static> {
    lock: parking_lot::MappedRwLockWriteGuard<'static, O>,
    _owned: Arc<RwLock<T>>,
}
impl<T, O> MappedRwLockWriteGuardOwned<T, O> {
    /// Make a new `MappedRwLockWriteGuardOwned` for a component of the locked data.
    ///
    /// This is an associated function that needs to be
    /// used as `MappedRwLockWriteGuardOwned::map(...)`. A method would interfere with methods of
    /// the same name on the contents of the locked data.
    pub fn map<O2>(guard: Self, map: impl FnOnce(&mut O) -> &mut O2) -> MappedRwLockWriteGuardOwned<T, O2> {
        MappedRwLockWriteGuardOwned {
            lock: parking_lot::MappedRwLockWriteGuard::map(guard.lock, map),
            _owned: guard._owned,
        }
    }
}
impl<T, O> ops::Deref for MappedRwLockWriteGuardOwned<T, O> {
    type Target = O;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}
impl<T, O> ops::DerefMut for MappedRwLockWriteGuardOwned<T, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lock.deref_mut()
    }
}

/// Read-only wrapper on an `Arc<RwLock<T>>` contextual value.
pub struct ReadOnlyRwLock<T>(Arc<RwLock<T>>);
impl<T> Clone for ReadOnlyRwLock<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> ReadOnlyRwLock<T> {
    /// New.
    pub fn new(l: Arc<RwLock<T>>) -> Self {
        Self(l)
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::read` for more details.
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, T> {
        self.0.read()
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// Unlike `read`, this method is guaranteed to succeed without blocking if
    /// another read lock is held at the time of the call.
    ///
    /// See `parking_lot::RwLock::read_recursive` for more details.
    pub fn read_recursive(&self) -> parking_lot::RwLockReadGuard<'_, T> {
        self.0.read_recursive()
    }

    /// Attempts to acquire this `RwLock` with shared read access.
    ///
    /// See `parking_lot::RwLock::try_read` for more details.
    pub fn try_read(&self) -> Option<parking_lot::RwLockReadGuard<'_, T>> {
        self.0.try_read()
    }

    /// Attempts to acquire this `RwLock` with shared read access.
    ///
    /// See `parking_lot::RwLock::try_read_recursive` for more details.
    pub fn try_read_recursive(&self) -> Option<parking_lot::RwLockReadGuard<'_, T>> {
        self.0.try_read_recursive()
    }

    /// Gets if the read-only shared reference is to the same lock as `other`.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Helper, runs a cleanup action once on drop.
pub struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    /// New with closure that will run once on drop.
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

pub(crate) fn panic_str<'s>(payload: &'s Box<dyn std::any::Any + Send + 'static>) -> &'s str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s
    } else {
        "<unknown-panic-message-type>"
    }
}
