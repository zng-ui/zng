#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Hash-map of type erased values, useful for storing assorted dynamic state.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{any::Any, fmt, marker::PhantomData};

use zng_unique_id::unique_id_64;

pub use zng_unique_id::static_id;

/// Represents a type that can be a [`StateId`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
#[diagnostic::on_unimplemented(note = "`StateValue` is implemented for all `T: Any + Send + Sync`")]
pub trait StateValue: Any + Send + Sync {}
impl<T: Any + Send + Sync> StateValue for T {}

unique_id_64! {
    /// Unique identifier of a value in a state map.
    ///
    /// The type `T` is the value type.
    ///
    /// ```
    /// # use zng_state_map::*;
    /// static_id! {
    ///     static ref FOO_ID: StateId<bool>;
    /// }
    ///
    /// # fn demo() {
    /// let mut owned_state = OwnedStateMap::<()>::default();
    /// let foo = owned_state.borrow_mut().set(*FOO_ID, true);
    /// # ; }
    /// ```
    pub struct StateId<T: (StateValue)>;
}
zng_unique_id::impl_unique_id_bytemuck!(StateId<T: (StateValue)>);
impl<T: StateValue> fmt::Debug for StateId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(debug_assertions)]
        let t = pretty_type_name::pretty_type_name::<T>();
        #[cfg(not(debug_assertions))]
        let t = "$T";

        if f.alternate() {
            writeln!(f, "StateId<{t} {{")?;
            writeln!(f, "   id: {},", self.get())?;
            writeln!(f, "   sequential: {}", self.sequential())?;
            writeln!(f, "}}")
        } else {
            write!(f, "StateId<{t}>({})", self.sequential())
        }
    }
}

/// Read-only state map.
///
/// The `U` parameter is tag type that represents the map's *context*.
pub struct StateMapRef<'a, U>(&'a state_map::StateMap, PhantomData<U>);
impl<U> Clone for StateMapRef<'_, U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<U> Copy for StateMapRef<'_, U> {}
impl<U> fmt::Debug for StateMapRef<'_, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StateMapRef<{}>({} entries);",
            pretty_type_name::pretty_type_name::<U>(),
            self.0.len()
        )
    }
}
impl<U> StateMapRef<'static, U> {
    /// Static empty map.
    pub fn empty() -> Self {
        static EMPTY: state_map::StateMap = state_map::StateMap::new();
        Self(&EMPTY, PhantomData)
    }
}
impl<'a, U> StateMapRef<'a, U> {
    /// Gets if the ID is set in this map.
    pub fn contains<T: StateValue>(self, id: impl Into<StateId<T>>) -> bool {
        self.0.contains(id.into())
    }

    /// Reference the ID's value set in this map.
    pub fn get<T: StateValue>(self, id: impl Into<StateId<T>>) -> Option<&'a T> {
        self.0.get(id.into())
    }

    /// Copy the ID's value set in this map.
    pub fn copy<T: StateValue + Copy>(self, id: impl Into<StateId<T>>) -> Option<T> {
        self.get(id.into()).copied()
    }

    /// Clone the ID's value set in this map.
    pub fn get_clone<T: StateValue + Clone>(self, id: impl Into<StateId<T>>) -> Option<T> {
        self.get(id.into()).cloned()
    }

    /// Reference the ID's value set in this map or panics if the key is not set.
    pub fn req<T: StateValue>(self, id: impl Into<StateId<T>>) -> &'a T {
        self.0.req(id.into())
    }

    /// Gets if a state ID without value is set.
    pub fn flagged(self, id: impl Into<StateId<()>>) -> bool {
        self.0.flagged(id.into())
    }

    /// If no state is set.
    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if self and other reference the same map.
    pub fn ptr_eq(self, other: Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

/// Mutable state map.
///
/// The `U` parameter is tag type that represents the map's *context*.
pub struct StateMapMut<'a, U>(&'a mut state_map::StateMap, PhantomData<U>);
impl<U> fmt::Debug for StateMapMut<'_, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateMapMut<{}>({} entries);", std::any::type_name::<U>(), self.0.len())
    }
}
impl<'a, U> StateMapMut<'a, U> {
    /// Gets if the ID is set in this map.
    pub fn contains<T: StateValue>(&self, id: impl Into<StateId<T>>) -> bool {
        self.0.contains(id.into())
    }

    /// Reference the ID's value set in this map.
    pub fn get<T: StateValue>(&self, id: impl Into<StateId<T>>) -> Option<&T> {
        self.0.get(id.into())
    }

    /// Consume the mutable reference to the map and returns a reference to the value in the parent lifetime `'a`.
    pub fn into_get<T: StateValue>(self, id: impl Into<StateId<T>>) -> Option<&'a T> {
        self.0.get(id.into())
    }

    /// Copy the ID's value set in this map.
    pub fn copy<T: StateValue + Copy>(&self, id: impl Into<StateId<T>>) -> Option<T> {
        self.get(id.into()).copied()
    }

    /// Clone the ID's value set in this map.
    pub fn get_clone<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> Option<T> {
        self.get(id).cloned()
    }

    /// Reference the ID's value set in this map or panics if the ID is not set.
    pub fn req<T: StateValue>(&self, id: impl Into<StateId<T>>) -> &T {
        self.0.req(id.into())
    }

    /// Consume the mutable reference to the map and returns a reference to the value in the parent lifetime `'a`.
    pub fn into_req<T: StateValue>(self, id: impl Into<StateId<T>>) -> &'a T {
        self.0.req(id.into())
    }

    /// Gets if a state ID without value is set.
    pub fn flagged(&self, id: impl Into<StateId<()>>) -> bool {
        self.0.flagged(id.into())
    }

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Set the ID's `value`.
    pub fn set<T: StateValue>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>) -> Option<T> {
        self.0.set(id.into(), value.into())
    }

    /// Mutable borrow the ID's value set in this map.
    pub fn get_mut<T: StateValue>(&mut self, id: impl Into<StateId<T>>) -> Option<&mut T> {
        self.0.get_mut(id.into())
    }

    /// Consume the mutable reference to the map and mutable borrow the ID's value in the parent lifetime `'a`.
    pub fn into_get_mut<T: StateValue>(self, id: impl Into<StateId<T>>) -> Option<&'a mut T> {
        self.0.get_mut(id.into())
    }

    /// Mutable borrow the key value set in this map or panics if the ID is not set.
    pub fn req_mut<T: StateValue>(&mut self, id: impl Into<StateId<T>>) -> &mut T {
        self.0.req_mut(id.into())
    }

    /// Consume the mutable reference to the map and mutable borrow the ID value in the parent lifetime `'a`.
    pub fn into_req_mut<T: StateValue>(self, id: impl Into<StateId<T>>) -> &'a mut T {
        self.0.req_mut(id.into())
    }

    /// Gets the given ID's corresponding entry in the map for in-place manipulation.
    pub fn entry<T: StateValue>(&mut self, id: impl Into<StateId<T>>) -> state_map::StateMapEntry<'_, T> {
        self.0.entry(id.into())
    }

    /// Consume the mutable reference to the map and returns a given ID's corresponding entry in the map with the parent lifetime `'a`.
    pub fn into_entry<T: StateValue>(self, id: impl Into<StateId<T>>) -> state_map::StateMapEntry<'a, T> {
        self.0.entry(id.into())
    }

    /// Sets a state ID without value.
    ///
    /// Returns if the state ID was already flagged.
    pub fn flag(&mut self, id: impl Into<StateId<()>>) -> bool {
        self.0.flag(id.into())
    }

    /// Reborrow the mutable reference.
    pub fn reborrow(&mut self) -> StateMapMut<'_, U> {
        StateMapMut(self.0, PhantomData)
    }

    /// Reborrow the reference as read-only.
    pub fn as_ref(&self) -> StateMapRef<'_, U> {
        StateMapRef(self.0, PhantomData)
    }
}

/// Private state map.
///
/// The owner of a state map has full access including to the `remove` and `clear` methods that are not
/// provided in the [`StateMapMut`] type. All mutable references borrowed from this map are also protected to
/// not allow replacement.
///
/// The `U` parameter is tag type that represents the map's *context*.
pub struct OwnedStateMap<U>(state_map::StateMap, PhantomData<U>);
impl<U> fmt::Debug for OwnedStateMap<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OwnedStateMap<{}>({} entries);",
            pretty_type_name::pretty_type_name::<U>(),
            self.0.len()
        )
    }
}
impl<U> Default for OwnedStateMap<U> {
    fn default() -> Self {
        Self::new()
    }
}
impl<U> OwnedStateMap<U> {
    /// New default, empty.
    pub const fn new() -> Self {
        OwnedStateMap(state_map::StateMap::new(), PhantomData)
    }

    /// Remove the key.
    pub fn remove<T: StateValue>(&mut self, id: impl Into<StateId<T>>) -> Option<T> {
        self.0.remove(id.into())
    }

    /// Removes all entries.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Create tagged read-only reference to the map.
    pub fn borrow(&self) -> StateMapRef<'_, U> {
        StateMapRef(&self.0, PhantomData)
    }

    /// Crate tagged mutable reference to the map.
    pub fn borrow_mut(&mut self) -> StateMapMut<'_, U> {
        StateMapMut(&mut self.0, PhantomData)
    }
}

/// State map helper types.
pub mod state_map {
    use std::any::Any;

    use zng_unique_id::*;

    use super::*;

    type AnyMap = IdMap<u64, Box<dyn Any + Send + Sync>>;

    pub(super) struct StateMap {
        map: AnyMap,
    }
    impl StateMap {
        pub(super) const fn new() -> Self {
            StateMap { map: AnyMap::new() }
        }

        pub(super) fn len(&self) -> usize {
            self.map.len()
        }

        pub(super) fn remove<T: StateValue>(&mut self, id: StateId<T>) -> Option<T> {
            self.map.remove(&id.get()).map(|a| *a.downcast().unwrap())
        }

        pub(super) fn clear(&mut self) {
            self.map.clear()
        }

        pub fn set<T: StateValue>(&mut self, id: StateId<T>, value: T) -> Option<T> {
            self.map.insert(id.get(), Box::new(value)).map(|any| *any.downcast().unwrap())
        }

        pub fn contains<T: StateValue>(&self, id: StateId<T>) -> bool {
            self.map.contains_key(&id.get())
        }

        pub fn get<T: StateValue>(&self, id: StateId<T>) -> Option<&T> {
            self.map.get(&id.get()).map(|any| any.downcast_ref().unwrap())
        }

        pub fn get_mut<T: StateValue>(&mut self, id: StateId<T>) -> Option<&mut T> {
            self.map.get_mut(&id.get()).map(|any| any.downcast_mut().unwrap())
        }

        pub fn req<T: StateValue>(&self, id: StateId<T>) -> &T {
            self.get(id).unwrap_or_else(move || panic!("expected `{id:?}` in state map"))
        }

        pub fn req_mut<T: StateValue>(&mut self, id: StateId<T>) -> &mut T {
            self.get_mut(id).unwrap_or_else(move || panic!("expected `{id:?}` in state map"))
        }

        pub fn entry<T: StateValue>(&mut self, id: StateId<T>) -> StateMapEntry<'_, T> {
            match self.map.entry(id.get()) {
                IdEntry::Occupied(e) => StateMapEntry::Occupied(OccupiedStateMapEntry {
                    _type: PhantomData,
                    entry: e,
                }),
                IdEntry::Vacant(e) => StateMapEntry::Vacant(VacantStateMapEntry {
                    _type: PhantomData,
                    entry: e,
                }),
            }
        }

        pub fn flag(&mut self, id: StateId<()>) -> bool {
            self.set(id, ()).is_some()
        }

        pub fn flagged(&self, id: StateId<()>) -> bool {
            self.map.contains_key(&id.get())
        }

        pub fn is_empty(&self) -> bool {
            self.map.is_empty()
        }
    }

    /// A view into an occupied entry in a state map.
    ///
    /// This struct is part of [`StateMapEntry`].
    pub struct OccupiedStateMapEntry<'a, T: StateValue> {
        _type: PhantomData<T>,
        entry: IdOccupiedEntry<'a, u64, Box<dyn Any + Send + Sync>>,
    }
    impl<'a, T: StateValue> OccupiedStateMapEntry<'a, T> {
        /// Gets a reference to the value in the entry.
        pub fn get(&self) -> &T {
            self.entry.get().downcast_ref().unwrap()
        }

        /// Gets a mutable reference to the value in the entry.
        ///
        /// See also [`into_mut`] to get a reference tied to the lifetime of the map directly.
        ///
        /// [`into_mut`]: Self::into_mut
        pub fn get_mut(&mut self) -> &mut T {
            self.entry.get_mut().downcast_mut().unwrap()
        }

        /// Converts the entry into a mutable reference to the value in the entry with a lifetime bound to the map itself.
        pub fn into_mut(self) -> &'a mut T {
            self.entry.into_mut().downcast_mut().unwrap()
        }

        /// Sets the value of the entry, and returns the entry’s old value.
        pub fn insert(&mut self, value: T) -> T {
            *self.entry.insert(Box::new(value)).downcast().unwrap()
        }

        /// Takes the value out of the entry, and returns it.
        pub fn remove(self) -> T {
            *self.entry.remove().downcast().unwrap()
        }
    }
    impl<T: StateValue + fmt::Debug> fmt::Debug for OccupiedStateMapEntry<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let id = StateId::<T>::from_raw(*self.entry.key());
            f.debug_struct("OccupiedStateMapEntry")
                .field("key", &id)
                .field("value", self.get())
                .finish()
        }
    }

    /// A view into a vacant entry in a state map.
    ///
    /// This struct is part of [`StateMapEntry`].
    pub struct VacantStateMapEntry<'a, T: StateValue> {
        _type: PhantomData<T>,
        entry: IdVacantEntry<'a, u64, Box<dyn Any + Send + Sync>>,
    }
    impl<'a, T: StateValue> VacantStateMapEntry<'a, T> {
        /// Sets the value of the entry and returns a mutable reference to it.
        pub fn insert(self, value: impl Into<T>) -> &'a mut T {
            self.entry.insert(Box::new(value.into())).downcast_mut().unwrap()
        }
    }
    impl<T: StateValue + fmt::Debug> fmt::Debug for VacantStateMapEntry<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let id = StateId::<T>::from_raw(*self.entry.key());
            f.debug_struct("VacantStateMapEntry").field("key", &id).finish_non_exhaustive()
        }
    }

    /// A view into a single entry in a state map, which may either be vacant or occupied.
    ///
    /// This `enum` is constructed from the [`entry`] method on [`StateMapMut`].
    ///
    /// [`entry`]: StateMapMut::entry
    pub enum StateMapEntry<'a, T: StateValue> {
        /// An occupied entry.
        Occupied(OccupiedStateMapEntry<'a, T>),
        /// A vacant entry.
        Vacant(VacantStateMapEntry<'a, T>),
    }
    impl<'a, T: StateValue> StateMapEntry<'a, T> {
        /// Ensures a value is in the entry by inserting the default if empty, and
        /// returns a mutable reference to the value in the entry.
        pub fn or_insert(self, default: impl Into<T>) -> &'a mut T {
            match self {
                StateMapEntry::Occupied(e) => e.into_mut(),
                StateMapEntry::Vacant(e) => e.insert(default),
            }
        }

        /// Ensures a value is in the entry by inserting the result of the
        /// default function if empty, and returns a mutable reference to the value in the entry.
        pub fn or_insert_with<F: FnOnce() -> T>(self, default: F) -> &'a mut T {
            match self {
                StateMapEntry::Occupied(e) => e.into_mut(),
                StateMapEntry::Vacant(e) => e.insert(default()),
            }
        }

        /// Provides in-place mutable access to an occupied entry before any potential inserts into the map.
        pub fn and_modify<F: FnOnce(&mut T)>(mut self, f: F) -> Self {
            if let StateMapEntry::Occupied(e) = &mut self {
                f(e.get_mut())
            }
            self
        }
    }
    impl<'a, T: StateValue> StateMapEntry<'a, T>
    where
        T: Default,
    {
        /// Ensures a value is in the entry by inserting the default value if empty,
        /// and returns a mutable reference to the value in the entry.
        pub fn or_default(self) -> &'a mut T {
            self.or_insert_with(Default::default)
        }
    }
    impl<T: StateValue + fmt::Debug> fmt::Debug for StateMapEntry<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Occupied(arg0) => f.debug_tuple("Occupied").field(arg0).finish(),
                Self::Vacant(arg0) => f.debug_tuple("Vacant").field(arg0).finish(),
            }
        }
    }
}
