use std::{
    any::{type_name, TypeId},
    fmt,
    marker::PhantomData,
};

use unsafe_any::UnsafeAny;

use crate::{
    context::{InfoContext, WidgetContext},
    crate_util::AnyMap,
    impl_ui_node,
    var::{IntoVar, Var, VarValue},
    widget_info::WidgetSubscriptions,
    UiNode,
};

/// A key to a value in a state map.
///
/// The type that implements this trait is the key. You can use the [`state_key!`]
/// macro to generate a key type.
///
/// [`state_key!`]: crate::context::state_key
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait StateKey: Copy + 'static {
    /// The value type.
    type Type: 'static;
}

///<span data-del-macro-root></span> Declares new [`StateKey`] types.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::context::state_key;
/// state_key! {
///     /// Key docs.
///     pub struct FooKey: u32;
/// }
/// ```
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `Key` suffix. If the value is a singleton-like value the key
/// can be made private and a helper function to require and get the value added directly on the value type, see [`WindowVars::req`]
/// and [`WindowVars::get`] for examples of this.
///
/// [`StateKey`]: crate::context::StateKey
/// [`WindowVars::req`]: crate::window::WindowVars::req
/// [`WindowVars::get`]: crate::window::WindowVars::get
#[macro_export]
macro_rules! state_key {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty;)+) => {$(
        $(#[$outer])*
        /// # StateKey
        ///
        /// This `struct` is a [`StateKey`].
        ///
        /// [`StateKey`]: crate::context::StateKey
        #[derive(Clone, Copy)]
        $vis struct $ident;

        impl $crate::context::StateKey for $ident {
            type Type = $type;
        }
    )+};
}
#[doc(inline)]
pub use crate::state_key;

/// Read-only state map.
///
/// The `U` parameter is tag type that represents the map's *context*.
pub struct StateMapRef<'a, U>(&'a state_map::StateMap, PhantomData<U>);
impl<'a, U> Clone for StateMapRef<'a, U> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}
impl<'a, U> Copy for StateMapRef<'a, U> {}
impl<'a, U> fmt::Debug for StateMapRef<'a, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateMapRef<{}>({} entries);", std::any::type_name::<U>(), self.0.len())
    }
}
impl<'a, U> StateMapRef<'a, U> {
    /// Gets if the key is set in this map.
    pub fn contains<S: StateKey>(self, key: S) -> bool {
        self.0.contains(key)
    }

    /// Reference the key value set in this map.
    pub fn get<S: StateKey>(self, key: S) -> Option<&'a S::Type> {
        self.0.get(key)
    }

    /// Copy the key value set in this map.
    pub fn copy<S: StateKey>(self, key: S) -> Option<S::Type>
    where
        S::Type: Copy,
    {
        self.get(key).copied()
    }

    /// Clone the key value set in this map.
    pub fn get_clone<S: StateKey>(self, key: S) -> Option<S::Type>
    where
        S::Type: Clone,
    {
        self.get(key).cloned()
    }

    /// Reference the key value set in this map or panics if the key is not set.
    pub fn req<S: StateKey>(self, key: S) -> &'a S::Type {
        self.0.req(key)
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(self, key: S) -> bool {
        self.0.flagged(key)
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
impl<'a, U> fmt::Debug for StateMapMut<'a, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateMapMut<{}>({} entries);", std::any::type_name::<U>(), self.0.len())
    }
}
impl<'a, U> StateMapMut<'a, U> {
    /// Gets if the key is set in this map.
    pub fn contains<S: StateKey>(&self, key: S) -> bool {
        self.0.contains(key)
    }

    /// Reference the key value set in this map.
    pub fn get<S: StateKey>(&self, key: S) -> Option<&S::Type> {
        self.0.get(key)
    }

    /// Consume the mutable reference to the map and returns a reference to the value in the parent lifetime `'a`.
    pub fn into_get<S: StateKey>(self, key: S) -> Option<&'a S::Type> {
        self.0.get(key)
    }

    /// Copy the key value set in this map.
    pub fn copy<S: StateKey>(&self, key: S) -> Option<S::Type>
    where
        S::Type: Copy,
    {
        self.get(key).copied()
    }

    /// Clone the key value set in this map.
    pub fn get_clone<S: StateKey>(&self, key: S) -> Option<S::Type>
    where
        S::Type: Clone,
    {
        self.get(key).cloned()
    }

    /// Reference the key value set in this map or panics if the key is not set.
    pub fn req<S: StateKey>(&self, key: S) -> &S::Type {
        self.0.req(key)
    }

    /// Consume the mutable reference to the map and returns a reference to the value in the parent lifetime `'a`.
    pub fn into_req<S: StateKey>(self, key: S) -> &'a S::Type {
        self.0.req(key)
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self, key: S) -> bool {
        self.0.flagged(key)
    }

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Set the key `value`.
    ///
    /// # Key
    ///
    /// Use [`state_key!`](crate::context::state_key) to generate a key, any static type can be a key,
    /// the [type id](TypeId) is the actual key.
    pub fn set<S: StateKey>(&mut self, key: S, value: S::Type) -> Option<S::Type> {
        self.0.set(key, value)
    }

    /// Sets a value that is its own [`StateKey`].
    pub fn set_single<S: StateKey<Type = S>>(&mut self, value: S) -> Option<S> {
        self.0.set_single(value)
    }

    /// Mutable borrow the key value set in this map.
    pub fn get_mut<S: StateKey>(&mut self, key: S) -> Option<&mut S::Type> {
        self.0.get_mut(key)
    }

    /// Consume the mutable reference to the map and mutable borrow the key value in the parent lifetime `'a`.
    pub fn into_get_mut<S: StateKey>(self, key: S) -> Option<&'a mut S::Type> {
        self.0.get_mut(key)
    }

    /// Mutable borrow the key value set in this map or panics if the key is not set.
    pub fn req_mut<S: StateKey>(&mut self, key: S) -> &mut S::Type {
        self.0.req_mut(key)
    }

    /// Consume the mutable reference to the map and mutable borrow the key value in the parent lifetime `'a`.
    pub fn into_req_mut<S: StateKey>(self, key: S) -> &'a mut S::Type {
        self.0.req_mut(key)
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry<S: StateKey>(&mut self, key: S) -> state_map::StateMapEntry<S> {
        self.0.entry(key)
    }

    /// Consume the mutable reference to the map and returns a given key's corresponding entry in the map with the parent lifetime `'a`.
    pub fn into_entry<S: StateKey>(self, key: S) -> state_map::StateMapEntry<'a, S> {
        self.0.entry(key)
    }

    /// Sets a state key without value.
    ///
    /// Returns if the state key was already flagged.
    pub fn flag<S: StateKey<Type = ()>>(&mut self, key: S) -> bool {
        self.0.flag(key)
    }

    /// Reborrow the mutable reference.
    pub fn reborrow(&mut self) -> StateMapMut<U> {
        StateMapMut(self.0, PhantomData)
    }

    /// Reborrow the reference as read-only.
    pub fn as_ref(&self) -> StateMapRef<U> {
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
        write!(f, "OwnedStateMap<{}>({} entries);", std::any::type_name::<U>(), self.0.len())
    }
}
impl<U> Default for OwnedStateMap<U> {
    fn default() -> Self {
        OwnedStateMap(state_map::StateMap::new(), PhantomData)
    }
}
impl<U> OwnedStateMap<U> {
    /// New default, empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove the key.
    pub fn remove<S: StateKey>(&mut self, key: S) -> Option<S::Type> {
        self.0.remove(key)
    }

    /// Removes all entries.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Create tagged read-only reference to the map.
    pub fn borrow(&self) -> StateMapRef<U> {
        StateMapRef(&self.0, PhantomData)
    }

    /// Crate tagged mutable reference to the map.
    pub fn borrow_mut(&mut self) -> StateMapMut<U> {
        StateMapMut(&mut self.0, PhantomData)
    }
}

/// Borrow a read-only reference to a state-map of kind `U`.
pub trait BorrowStateMap<U> {
    /// Borrow a read-only reference to a state-map.
    fn borrow(&self) -> StateMapRef<U>;
}
impl<'a, U> BorrowStateMap<U> for StateMapRef<'a, U> {
    fn borrow(&self) -> StateMapRef<U> {
        *self
    }
}
impl<'a, U> BorrowStateMap<U> for StateMapMut<'a, U> {
    fn borrow(&self) -> StateMapRef<U> {
        self.as_ref()
    }
}
impl<U> BorrowStateMap<U> for OwnedStateMap<U> {
    fn borrow(&self) -> StateMapRef<U> {
        self.borrow()
    }
}

/// Borrow a mutable reference to a state-map of kind `U`.
pub trait BorrowMutStateMap<U> {
    /// Borrow a mutable reference to a state-map.
    fn borrow_mut(&mut self) -> StateMapMut<U>;
}
impl<'a, U> BorrowMutStateMap<U> for StateMapMut<'a, U> {
    fn borrow_mut(&mut self) -> StateMapMut<U> {
        self.reborrow()
    }
}
impl<U> BorrowMutStateMap<U> for OwnedStateMap<U> {
    fn borrow_mut(&mut self) -> StateMapMut<U> {
        self.borrow_mut()
    }
}

macro_rules! impl_borrow_mut_for_ctx {
    ($($Ctx:ident.$field:ident : $Tag:ident;)+) => {$(

        impl<'a> BorrowStateMap<state_map::$Tag> for crate::context::$Ctx<'a> {
            fn borrow(&self) -> StateMapRef<state_map::$Tag> {
                self.$field.as_ref()
            }
        }

        impl<'a> BorrowMutStateMap<state_map::$Tag> for crate::context::$Ctx<'a> {
            fn borrow_mut(&mut self) -> StateMapMut<state_map::$Tag> {
                self.$field.reborrow()
            }
        }

    )+}
}
impl_borrow_mut_for_ctx! {
    AppContext.app_state: App;
    WindowContext.app_state: App;
    WidgetContext.app_state: App;
    LayoutContext.app_state: App;

    WindowContext.window_state: Window;
    WidgetContext.window_state: Window;
    LayoutContext.window_state: Window;

    WidgetContext.widget_state: Widget;
    LayoutContext.widget_state: Widget;

    WindowContext.update_state: Update;
    WidgetContext.update_state: Update;
    LayoutContext.update_state: Update;
    MeasureContext.update_state: Update;
    InfoContext.update_state: Update;
    RenderContext.update_state: Update;
}

macro_rules! impl_borrow_for_ctx {
    ($($Ctx:ident.$field:ident : $Tag:ident;)+) => {$(

        impl<'a> BorrowStateMap<state_map::$Tag> for crate::context::$Ctx<'a> {
            fn borrow(&self) -> StateMapRef<state_map::$Tag> {
                self.$field
            }
        }

    )+}
}
impl_borrow_for_ctx! {
    MeasureContext.app_state: App;
    MeasureContext.window_state: Window;
    MeasureContext.widget_state: Widget;

    InfoContext.app_state: App;
    InfoContext.window_state: Window;
    InfoContext.widget_state: Widget;

    RenderContext.app_state: App;
    RenderContext.window_state: Window;
    RenderContext.widget_state: Widget;
}

macro_rules! impl_borrow_mut_for_test_ctx {
    ($($field:ident : $Tag:ident;)+) => {$(

        #[cfg(any(test, doc, feature = "test_util"))]
        impl BorrowStateMap<state_map::$Tag> for crate::context::TestWidgetContext {
            fn borrow(&self) -> StateMapRef<state_map::$Tag> {
                self.$field.borrow()
            }
        }
        #[cfg(any(test, doc, feature = "test_util"))]
        impl BorrowMutStateMap<state_map::$Tag> for crate::context::TestWidgetContext {
            fn borrow_mut(&mut self) -> StateMapMut<state_map::$Tag> {
                self.$field.borrow_mut()
            }
        }

    )+}
}
impl_borrow_mut_for_test_ctx! {
    app_state: App;
    window_state: Window;
    widget_state: Widget;
    update_state: Update;
}
impl BorrowStateMap<state_map::App> for crate::app::HeadlessApp {
    fn borrow(&self) -> StateMapRef<state_map::App> {
        self.app_state()
    }
}
impl BorrowMutStateMap<state_map::App> for crate::app::HeadlessApp {
    fn borrow_mut(&mut self) -> StateMapMut<state_map::App> {
        self.app_state_mut()
    }
}

/// State map helper types.
pub mod state_map {
    use super::*;

    /// App state-map tag.
    pub enum App {}

    /// Window state-map tag.
    pub enum Window {}

    /// Widget state-map tag.
    pub enum Widget {}

    /// Update state-map tag.
    pub enum Update {}

    pub(super) struct StateMap {
        map: AnyMap,
    }
    impl StateMap {
        pub(super) fn new() -> Self {
            StateMap { map: AnyMap::default() }
        }

        pub(super) fn len(&self) -> usize {
            self.map.len()
        }

        pub(super) fn remove<S: StateKey>(&mut self, _key: S) -> Option<S::Type> {
            self.map.remove(&TypeId::of::<S>()).map(|a| {
                // SAFETY: The type system asserts this is valid.
                unsafe { *a.downcast_unchecked::<S::Type>() }
            })
        }

        pub(super) fn clear(&mut self) {
            self.map.clear()
        }

        pub fn set<S: StateKey>(&mut self, _key: S, value: S::Type) -> Option<S::Type> {
            self.map.insert(TypeId::of::<S>(), Box::new(value)).map(|any| {
                // SAFETY: The type system asserts this is valid.
                unsafe { *any.downcast_unchecked::<S::Type>() }
            })
        }

        pub fn set_single<S: StateKey<Type = S>>(&mut self, value: S) -> Option<S> {
            self.map.insert(TypeId::of::<S>(), Box::new(value)).map(|any| {
                // SAFETY: The type system asserts this is valid.
                unsafe { *any.downcast_unchecked::<S>() }
            })
        }

        pub fn contains<S: StateKey>(&self, _key: S) -> bool {
            self.map.contains_key(&TypeId::of::<S>())
        }

        pub fn get<S: StateKey>(&self, _key: S) -> Option<&S::Type> {
            self.map.get(&TypeId::of::<S>()).map(|any| {
                // SAFETY: The type system asserts this is valid.
                unsafe { any.downcast_ref_unchecked::<S::Type>() }
            })
        }

        pub fn get_mut<S: StateKey>(&mut self, _key: S) -> Option<&mut S::Type> {
            self.map.get_mut(&TypeId::of::<S>()).map(|any| {
                // SAFETY: The type system asserts this is valid.
                unsafe { any.downcast_mut_unchecked::<S::Type>() }
            })
        }

        pub fn req<S: StateKey>(&self, key: S) -> &S::Type {
            self.get(key)
                .unwrap_or_else(|| panic!("expected `{}` in state map", type_name::<S>()))
        }

        pub fn req_mut<S: StateKey>(&mut self, key: S) -> &mut S::Type {
            self.get_mut(key)
                .unwrap_or_else(|| panic!("expected `{}` in state map", type_name::<S>()))
        }

        pub fn entry<S: StateKey>(&mut self, _key: S) -> StateMapEntry<S> {
            match self.map.entry(TypeId::of::<S>()) {
                std::collections::hash_map::Entry::Occupied(e) => StateMapEntry::Occupied(OccupiedStateMapEntry {
                    _key: PhantomData,
                    entry: e,
                }),
                std::collections::hash_map::Entry::Vacant(e) => StateMapEntry::Vacant(VacantStateMapEntry {
                    _key: PhantomData,
                    entry: e,
                }),
            }
        }

        pub fn flag<S: StateKey<Type = ()>>(&mut self, key: S) -> bool {
            self.set(key, ()).is_some()
        }

        pub fn flagged<S: StateKey<Type = ()>>(&self, _key: S) -> bool {
            self.map.contains_key(&TypeId::of::<S>())
        }

        pub fn is_empty(&self) -> bool {
            self.map.is_empty()
        }
    }

    /// A view into an occupied entry in a state map.
    ///
    /// This struct is part of [`StateMapEntry`].
    pub struct OccupiedStateMapEntry<'a, S: StateKey> {
        _key: PhantomData<S>,
        entry: std::collections::hash_map::OccupiedEntry<'a, TypeId, Box<dyn UnsafeAny>>,
    }
    impl<'a, S: StateKey> OccupiedStateMapEntry<'a, S> {
        /// Gets a reference to the value in the entry.
        pub fn get(&self) -> &S::Type {
            // SAFETY: The type system asserts this is valid.
            unsafe { self.entry.get().downcast_ref_unchecked() }
        }

        /// Gets a mutable reference to the value in the entry.
        ///
        /// If you need a reference to the OccupiedEntry which may outlive the destruction of the Entry value, see [`into_mut`].
        ///
        /// [`into_mut`]: Self::into_mut
        pub fn get_mut(&mut self) -> &mut S::Type {
            // SAFETY: The type system asserts this is valid.
            unsafe { self.entry.get_mut().downcast_mut_unchecked() }
        }

        /// Converts the entry into a mutable reference to the value in the entry with a lifetime bound to the map itself.
        ///
        /// If you need multiple references to the OccupiedEntry, see [`get_mut`].
        ///
        /// [`get_mut`]: Self::get_mut
        pub fn into_mut(self) -> &'a mut S::Type {
            // SAFETY: The type system asserts this is valid.
            unsafe { self.entry.into_mut().downcast_mut_unchecked() }
        }

        /// Sets the value of the entry, and returns the entry’s old value.
        pub fn insert(&mut self, value: S::Type) -> S::Type {
            // SAFETY: The type system asserts this is valid.
            unsafe { *self.entry.insert(Box::new(value)).downcast_unchecked() }
        }

        /// Takes the value out of the entry, and returns it.
        pub fn remove(self) -> S::Type {
            // SAFETY: The type system asserts this is valid.
            unsafe { *self.entry.remove().downcast_unchecked() }
        }
    }
    impl<'a, S: StateKey> fmt::Debug for OccupiedStateMapEntry<'a, S>
    where
        S::Type: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("OccupiedStateMapEntry")
                .field("key", &type_name::<S>())
                .field("value", self.get())
                .finish_non_exhaustive()
        }
    }

    /// A view into a vacant entry in a state map.
    ///
    /// This struct is part of [`StateMapEntry`].
    pub struct VacantStateMapEntry<'a, S: StateKey> {
        _key: PhantomData<S>,
        entry: std::collections::hash_map::VacantEntry<'a, TypeId, Box<dyn UnsafeAny>>,
    }
    impl<'a, S: StateKey> VacantStateMapEntry<'a, S> {
        /// Sets the value of the entry and returns a mutable reference to it.
        pub fn insert(self, value: S::Type) -> &'a mut S::Type {
            // SAFETY: The type system asserts this is valid.
            unsafe { self.entry.insert(Box::new(value)).downcast_mut_unchecked() }
        }
    }
    impl<'a, S: StateKey> fmt::Debug for VacantStateMapEntry<'a, S>
    where
        S::Type: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("VacantStateMapEntry")
                .field("key", &type_name::<S>())
                .finish_non_exhaustive()
        }
    }

    /// A view into a single entry in a state map, which may either be vacant or occupied.
    ///
    /// This `enum` is constructed from the [`entry`] method on [`StateMapMut`].
    ///
    /// [`entry`]: StateMapMut::entry
    pub enum StateMapEntry<'a, S: StateKey> {
        /// An occupied entry.
        Occupied(OccupiedStateMapEntry<'a, S>),
        /// A vacant entry.
        Vacant(VacantStateMapEntry<'a, S>),
    }
    impl<'a, S: StateKey> StateMapEntry<'a, S> {
        /// Ensures a value is in the entry by inserting the default if empty, and
        /// returns a mutable reference to the value in the entry.
        pub fn or_insert(self, default: S::Type) -> &'a mut S::Type {
            match self {
                StateMapEntry::Occupied(e) => e.into_mut(),
                StateMapEntry::Vacant(e) => e.insert(default),
            }
        }

        /// Ensures a value is in the entry by inserting the result of the
        /// default function if empty, and returns a mutable reference to the value in the entry.
        pub fn or_insert_with<F: FnOnce() -> S::Type>(self, default: F) -> &'a mut S::Type {
            match self {
                StateMapEntry::Occupied(e) => e.into_mut(),
                StateMapEntry::Vacant(e) => e.insert(default()),
            }
        }

        /// Provides in-place mutable access to an occupied entry before any potential inserts into the map.
        pub fn and_modify<F: FnOnce(&mut S::Type)>(mut self, f: F) -> Self {
            if let StateMapEntry::Occupied(e) = &mut self {
                f(e.get_mut())
            }
            self
        }
    }
    impl<'a, S: StateKey> StateMapEntry<'a, S>
    where
        S::Type: Default,
    {
        /// Ensures a value is in the entry by inserting the default value if empty,
        /// and returns a mutable reference to the value in the entry.
        pub fn or_default(self) -> &'a mut S::Type {
            self.or_insert_with(Default::default)
        }
    }
    impl<'a, S: StateKey> fmt::Debug for StateMapEntry<'a, S>
    where
        S::Type: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Occupied(arg0) => f.debug_tuple("Occupied").field(arg0).finish(),
                Self::Vacant(arg0) => f.debug_tuple("Vacant").field(arg0).finish(),
            }
        }
    }
}

/// Helper for declaring properties that set the widget state.
///
/// The state key is set in [`widget_state`](WidgetContext::widget_state) on init and is kept updated.
///
/// # Examples
///
/// ```
/// # fn main() -> () { }
/// use zero_ui_core::{property, context::{state_key, WidgetContext, set_widget_state}, var::IntoVar, UiNode, Widget};
///
/// state_key! {
///     pub struct FooKey: u32;
/// }
///
/// #[property(context)]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
///     set_widget_state(child, FooKey, value)
/// }
///
/// // after the property is used and the widget initializes:
///
/// /// Get the value from outside the widget.
/// fn get_foo_outer(widget: &impl Widget) -> u32 {
///     widget.state().get(FooKey).copied().unwrap_or_default()
/// }
///
/// /// Get the value from inside the widget.
/// fn get_foo_inner(ctx: &WidgetContext) -> u32 {
///     ctx.widget_state.get(FooKey).copied().unwrap_or_default()
/// }
/// ```
pub fn set_widget_state<U, K, V>(child: U, key: K, value: V) -> impl UiNode
where
    U: UiNode,
    K: StateKey,
    K::Type: VarValue,
    V: IntoVar<K::Type>,
{
    set_widget_state_update(child, key, value, |_, _| {})
}

/// Helper for declaring properties that set the widget state with a custom closure executed when the value updates.
///
/// The `on_update` closure is called every time the `value` variable updates.
///
/// See [`set_widget_state`] for more details.
pub fn set_widget_state_update<U, K, V, H>(child: U, key: K, value: V, on_update: H) -> impl UiNode
where
    U: UiNode,
    K: StateKey,
    K::Type: VarValue,
    V: IntoVar<K::Type>,
    H: FnMut(&mut WidgetContext, &K::Type) + 'static,
{
    struct SetWidgetStateNode<U, K, V, H> {
        child: U,
        key: K,
        var: V,
        on_update: H,
    }
    #[impl_ui_node(child)]
    impl<U, K, V, H> UiNode for SetWidgetStateNode<U, K, V, H>
    where
        U: UiNode,
        K: StateKey,
        K::Type: VarValue,
        V: Var<K::Type>,
        H: FnMut(&mut WidgetContext, &K::Type) + 'static,
    {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.var);
            self.child.subscriptions(ctx, subs);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.widget_state.set(self.key, self.var.get(ctx).clone());
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(new) = self.var.clone_new(ctx) {
                (self.on_update)(ctx, &new);
                ctx.widget_state.set(self.key, new);
            }
            self.child.update(ctx);
        }
    }
    SetWidgetStateNode {
        child: child.cfg_boxed(),
        key,
        var: value.into_var(),
        on_update,
    }
    .cfg_boxed()
}
