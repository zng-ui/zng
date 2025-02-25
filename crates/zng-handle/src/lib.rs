#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Resource handle type.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::hash::Hash;
use std::{
    fmt,
    hash::Hasher,
    sync::{
        Arc, Weak,
        atomic::{AtomicU8, Ordering},
    },
};

/// Represents a resource handle.
///
/// The resource stays in memory as long as a handle clone is alive. After the handle
/// is dropped the resource will be removed after an indeterminate time at the discretion
/// of the resource manager.
///
/// You can *forget* a handle by calling [`perm`](Self::perm), this releases the handle memory
/// but the resource stays alive for the duration of the app, unlike calling [`std::mem::forget`] no memory is leaked.
///
/// Any handle can also [`force_drop`](Self::force_drop), meaning that even if there are various handles active the
/// resource will be dropped regardless.
///
/// The parameter type `D` is any [`Sync`] data type that will be shared using the handle.
#[must_use = "the resource id dropped if the handle is dropped"]
#[repr(transparent)]
pub struct Handle<D: Send + Sync>(Arc<HandleState<D>>);
struct HandleState<D> {
    state: AtomicU8,
    data: D,
}
impl<D: Send + Sync> Handle<D> {
    /// Create a handle with owner pair.
    pub fn new(data: D) -> (HandleOwner<D>, Handle<D>) {
        let handle = Handle(Arc::new(HandleState {
            state: AtomicU8::new(NONE),
            data,
        }));
        (HandleOwner(handle.clone()), handle)
    }

    /// Create a handle to nothing, the handle always in the *dropped* state.
    ///
    /// Note that `Option<Handle<D>>` takes up the same space as `Handle<D>` and avoids an allocation.
    pub fn dummy(data: D) -> Self {
        Handle(Arc::new(HandleState {
            state: AtomicU8::new(FORCE_DROP),
            data,
        }))
    }

    /// Reference the attached data.
    pub fn data(&self) -> &D {
        &self.0.data
    }

    /// Mark the handle as permanent and drops this clone of it. This causes the resource to stay in memory
    /// until the app exits, no need to hold a handle somewhere.
    pub fn perm(self) {
        self.0.state.fetch_or(PERMANENT, Ordering::Relaxed);
    }

    /// If [`perm`](Self::perm) was called in another clone of this handle.
    ///
    /// If `true` the resource will stay in memory for the duration of the app, unless [`force_drop`](Self::force_drop)
    /// is also called.
    pub fn is_permanent(&self) -> bool {
        self.0.state.load(Ordering::Relaxed) == PERMANENT
    }

    /// Force drops the handle, meaning the resource will be dropped even if there are other handles active.
    pub fn force_drop(self) {
        self.0.state.store(FORCE_DROP, Ordering::Relaxed);
    }

    /// If the handle is in *dropped* state.
    ///
    /// The handle is considered dropped when all handle and clones are dropped or when [`force_drop`](Handle::force_drop)
    /// was called in any of the clones.
    ///
    /// Note that in this method it can only be because [`force_drop`](Handle::force_drop) was called.
    pub fn is_dropped(&self) -> bool {
        self.0.state.load(Ordering::Relaxed) == FORCE_DROP
    }

    /// Create a [`WeakHandle`] to this handle.
    pub fn downgrade(&self) -> WeakHandle<D> {
        WeakHandle(Arc::downgrade(&self.0))
    }
}
impl<D: Send + Sync> Clone for Handle<D> {
    fn clone(&self) -> Self {
        Handle(Arc::clone(&self.0))
    }
}
impl<D: Send + Sync> PartialEq for Handle<D> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<D: Send + Sync> Eq for Handle<D> {}
impl<D: Send + Sync> Hash for Handle<D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.0) as usize;
        ptr.hash(state);
    }
}
impl<D: Send + Sync> Drop for Handle<D> {
    fn drop(&mut self) {
        if !self.is_permanent() && Arc::strong_count(&self.0) == 2 {
            // if we are about to drop the last handle and it is not permanent, force-drop
            // this causes potential weak-handles to not reanimate a dropping resource because
            // of the handle that HandleOwner holds.
            self.0.state.store(FORCE_DROP, Ordering::Relaxed);
        }
    }
}
impl<D: Send + Sync> fmt::Debug for Handle<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_permanent() {
            write!(f, "permanent")
        } else if self.is_dropped() {
            write!(f, "dropped")
        } else {
            write!(f, "holding")
        }
    }
}

/// A weak reference to a [`Handle`].
pub struct WeakHandle<D: Send + Sync>(Weak<HandleState<D>>);
impl<D: Send + Sync> WeakHandle<D> {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        WeakHandle(Weak::new())
    }

    /// Get a live handle if it was not dropped or force-dropped.
    pub fn upgrade(&self) -> Option<Handle<D>> {
        if let Some(arc) = self.0.upgrade() {
            let handle = Handle(arc);
            if handle.is_dropped() { None } else { Some(handle) }
        } else {
            None
        }
    }
}
impl<D: Send + Sync> Default for WeakHandle<D> {
    fn default() -> Self {
        Self::new()
    }
}
impl<D: Send + Sync> Clone for WeakHandle<D> {
    fn clone(&self) -> Self {
        WeakHandle(self.0.clone())
    }
}
impl<D: Send + Sync> PartialEq for WeakHandle<D> {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}
impl<D: Send + Sync> Eq for WeakHandle<D> {}
impl<D: Send + Sync> Hash for WeakHandle<D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = self.0.as_ptr() as usize;
        ptr.hash(state);
    }
}
impl<D: Send + Sync> fmt::Debug for WeakHandle<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.strong_count() > 0 {
            write!(f, "can-upgrade")
        } else {
            write!(f, "dropped")
        }
    }
}

/// A [`Handle`] owner.
///
/// Use [`Handle::new`] to create.
///
/// Dropping the [`HandleOwner`] marks all active handles as *force-drop*.
pub struct HandleOwner<D: Send + Sync>(Handle<D>);
impl<D: Send + Sync> HandleOwner<D> {
    /// If the handle is in *dropped* state.
    ///
    /// The handle is considered dropped when all handle and clones are dropped or when [`force_drop`](Handle::force_drop)
    /// was called in any of the clones.
    pub fn is_dropped(&self) -> bool {
        let state = self.0.0.state.load(Ordering::Relaxed);
        state == FORCE_DROP || (state != PERMANENT && Arc::strong_count(&self.0.0) <= 1)
    }

    /*
    /// New handle owner in the dropped state.
    pub fn dropped(data: D) -> HandleOwner<D> {
        HandleOwner(Handle(Arc::new(HandleState {
            state: AtomicU8::new(FORCE_DROP),
            data,
        })))
    }

    /// Gets a new handle and resets the state if it was *force-drop*.
    ///
    /// Note that handles are permanently dropped when the last handle is dropped.
    pub fn reanimate(&self) -> Handle<D> {
        if self.is_dropped() {
            self.0 .0.state.store(NONE, Ordering::Relaxed);
        }
        self.0.clone()
    }

    */

    /// Gets an weak handle that may-not be able to upgrade.
    pub fn weak_handle(&self) -> WeakHandle<D> {
        self.0.downgrade()
    }

    /// Reference the attached data.
    pub fn data(&self) -> &D {
        self.0.data()
    }
}
impl<D: Send + Sync> Drop for HandleOwner<D> {
    fn drop(&mut self) {
        self.0.0.state.store(FORCE_DROP, Ordering::Relaxed);
    }
}

const NONE: u8 = 0;
const PERMANENT: u8 = 0b01;
const FORCE_DROP: u8 = 0b11;
