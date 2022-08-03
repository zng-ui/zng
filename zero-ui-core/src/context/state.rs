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

/// A key to a value in a [`StateMap`].
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

/// A map of [state keys](StateKey) to values of their associated types that exists for
/// a stage of the application.
///
/// # No Remove
///
/// Note that there is no way to clear the map, remove a key or replace the map with a new empty one.
/// This is by design, if you want to make a key *removable* make its value `Option<T>`.
pub struct StateMap {
    map: AnyMap,
}
impl fmt::Debug for StateMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateMap[{} entries]", self.map.len())
    }
}
impl StateMap {
    pub(crate) fn new() -> Self {
        StateMap { map: AnyMap::default() }
    }

    /// Set the key `value`.
    ///
    /// # Key
    ///
    /// Use [`state_key!`](crate::context::state_key) to generate a key, any static type can be a key,
    /// the [type id](TypeId) is the actual key.
    pub fn set<S: StateKey>(&mut self, _key: S, value: S::Type) -> Option<S::Type> {
        self.map.insert(TypeId::of::<S>(), Box::new(value)).map(|any| {
            // SAFETY: The type system asserts this is valid.
            unsafe { *any.downcast_unchecked::<S::Type>() }
        })
    }

    /// Sets a value that is its own [`StateKey`].
    pub fn set_single<S: StateKey<Type = S>>(&mut self, value: S) -> Option<S> {
        self.map.insert(TypeId::of::<S>(), Box::new(value)).map(|any| {
            // SAFETY: The type system asserts this is valid.
            unsafe { *any.downcast_unchecked::<S>() }
        })
    }

    /// Gets if the key is set in this map.
    pub fn contains<S: StateKey>(&self, _key: S) -> bool {
        self.map.contains_key(&TypeId::of::<S>())
    }

    /// Reference the key value set in this map.
    pub fn get<S: StateKey>(&self, _key: S) -> Option<&S::Type> {
        self.map.get(&TypeId::of::<S>()).map(|any| {
            // SAFETY: The type system asserts this is valid.
            unsafe { any.downcast_ref_unchecked::<S::Type>() }
        })
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

    /// Mutable borrow the key value set in this map.
    pub fn get_mut<S: StateKey>(&mut self, _key: S) -> Option<&mut S::Type> {
        self.map.get_mut(&TypeId::of::<S>()).map(|any| {
            // SAFETY: The type system asserts this is valid.
            unsafe { any.downcast_mut_unchecked::<S::Type>() }
        })
    }

    /// Reference the key value set in this map or panics if the key is not set.
    pub fn req<S: StateKey>(&self, key: S) -> &S::Type {
        self.get(key)
            .unwrap_or_else(|| panic!("expected `{}` in state map", type_name::<S>()))
    }

    /// Mutable borrow the key value set in this map or panics if the key is not set.
    pub fn req_mut<S: StateKey>(&mut self, key: S) -> &mut S::Type {
        self.get_mut(key)
            .unwrap_or_else(|| panic!("expected `{}` in state map", type_name::<S>()))
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
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

    /// Sets a state key without value.
    ///
    /// Returns if the state key was already flagged.
    pub fn flag<S: StateKey<Type = ()>>(&mut self, key: S) -> bool {
        self.set(key, ()).is_some()
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self, _key: S) -> bool {
        self.map.contains_key(&TypeId::of::<S>())
    }

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// A view into an occupied entry in a [`StateMap`].
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

/// A view into a vacant entry in a [`StateMap`].
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
/// This `enum` is constructed from the [`entry`] method on [`StateMap`].
///
/// [`entry`]: StateMap::entry
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

/// Private [`StateMap`].
///
/// The owner of a state map has full access including to the `remove` and `clear` function that is not
/// provided in the [`StateMap`] type.
pub struct OwnedStateMap(pub(crate) StateMap);
impl Default for OwnedStateMap {
    fn default() -> Self {
        OwnedStateMap(StateMap::new())
    }
}
impl OwnedStateMap {
    /// New default, empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove the key.
    pub fn remove<S: StateKey>(&mut self, _key: S) -> Option<S::Type> {
        self.0.map.remove(&TypeId::of::<S>()).map(|a| {
            // SAFETY: The type system asserts this is valid.
            unsafe { *a.downcast_unchecked::<S::Type>() }
        })
    }

    /// Removes all entries.
    pub fn clear(&mut self) {
        self.0.map.clear()
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

    /// Gets if the key is set in this map.
    pub fn contains<S: StateKey>(&self, key: S) -> bool {
        self.0.contains(key)
    }

    /// Reference the key value set in this map.
    pub fn get<S: StateKey>(&self, key: S) -> Option<&S::Type> {
        self.0.get(key)
    }

    /// Mutable borrow the key value set in this map.
    pub fn get_mut<S: StateKey>(&mut self, key: S) -> Option<&mut S::Type> {
        self.0.get_mut(key)
    }

    /// Reference the key value set in this map, or panics if the key is not set.
    pub fn req<S: StateKey>(&self, key: S) -> &S::Type {
        self.0.req(key)
    }

    /// Mutable borrow the key value set in this map, or panics if the key is not set.
    pub fn req_mut<S: StateKey>(&mut self, key: S) -> &mut S::Type {
        self.0.req_mut(key)
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry<S: StateKey>(&mut self, key: S) -> StateMapEntry<S> {
        self.0.entry(key)
    }

    /// Sets a state key without value.
    ///
    /// Returns if the state key was already flagged.
    pub fn flag<S: StateKey<Type = ()>>(&mut self, key: S) -> bool {
        self.0.flag(key)
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self, key: S) -> bool {
        self.0.flagged(key)
    }

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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
