use std::{any::Any, fmt, marker::PhantomData};

use crate::{
    context::{WidgetContext, WidgetUpdates},
    ui_node,
    var::{IntoVar, Var, VarValue},
    widget_instance::UiNode,
};

/// A type that can be a [`StateId`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait StateValue: Any + Send + 'static {}
impl<T: Any + Send + 'static> StateValue for T {}

unique_id_64! {
    /// Unique identifier of a value in a state map.
    ///
    /// The type `T` is the value type.
    ///
    /// ```
    /// # use zero_ui_core::context::*;
    /// static FOO_ID: StaticStateId<bool> = StateId::new_static();
    ///
    /// # fn demo(ctx: &mut WidgetContext) {
    /// let foo = ctx.widget_state.get(&FOO_ID);
    /// # ; }
    /// ```
    pub struct StateId<T: (StateValue)>;
}
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
impl<'a, U> Clone for StateMapRef<'a, U> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}
impl<'a, U> Copy for StateMapRef<'a, U> {}
impl<'a, U> fmt::Debug for StateMapRef<'a, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StateMapRef<{}>({} entries);",
            pretty_type_name::pretty_type_name::<U>(),
            self.0.len()
        )
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
impl<'a, U> fmt::Debug for StateMapMut<'a, U> {
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
    pub fn entry<T: StateValue>(&mut self, id: impl Into<StateId<T>>) -> state_map::StateMapEntry<T> {
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
        OwnedStateMap(state_map::StateMap::new(), PhantomData)
    }
}
impl<U> OwnedStateMap<U> {
    /// New default, empty.
    pub fn new() -> Self {
        Self::default()
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
    use std::any::Any;

    use super::*;

    type AnyMap = crate::crate_util::IdMap<u64, Box<dyn Any + Send>>;

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
            self.get(id).unwrap_or_else(move || panic!("expected `{:?}` in state map", id))
        }

        pub fn req_mut<T: StateValue>(&mut self, id: StateId<T>) -> &mut T {
            self.get_mut(id).unwrap_or_else(move || panic!("expected `{:?}` in state map", id))
        }

        pub fn entry<T: StateValue>(&mut self, id: StateId<T>) -> StateMapEntry<T> {
            match self.map.entry(id.get()) {
                std::collections::hash_map::Entry::Occupied(e) => StateMapEntry::Occupied(OccupiedStateMapEntry {
                    _type: PhantomData,
                    entry: e,
                }),
                std::collections::hash_map::Entry::Vacant(e) => StateMapEntry::Vacant(VacantStateMapEntry {
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
        entry: std::collections::hash_map::OccupiedEntry<'a, u64, Box<dyn Any + Send>>,
    }
    impl<'a, T: StateValue> OccupiedStateMapEntry<'a, T> {
        /// Gets a reference to the value in the entry.
        pub fn get(&self) -> &T {
            self.entry.get().downcast_ref().unwrap()
        }

        /// Gets a mutable reference to the value in the entry.
        ///
        /// If you need a reference to the OccupiedEntry which may outlive the destruction of the Entry value, see [`into_mut`].
        ///
        /// [`into_mut`]: Self::into_mut
        pub fn get_mut(&mut self) -> &mut T {
            self.entry.get_mut().downcast_mut().unwrap()
        }

        /// Converts the entry into a mutable reference to the value in the entry with a lifetime bound to the map itself.
        ///
        /// If you need multiple references to the OccupiedEntry, see [`get_mut`].
        ///
        /// [`get_mut`]: Self::get_mut
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
    impl<'a, T: StateValue + fmt::Debug> fmt::Debug for OccupiedStateMapEntry<'a, T> {
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
        entry: std::collections::hash_map::VacantEntry<'a, u64, Box<dyn Any + Send>>,
    }
    impl<'a, T: StateValue> VacantStateMapEntry<'a, T> {
        /// Sets the value of the entry and returns a mutable reference to it.
        pub fn insert(self, value: impl Into<T>) -> &'a mut T {
            self.entry.insert(Box::new(value.into())).downcast_mut().unwrap()
        }
    }
    impl<'a, T: StateValue + fmt::Debug> fmt::Debug for VacantStateMapEntry<'a, T> {
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
    impl<'a, T: StateValue + fmt::Debug> fmt::Debug for StateMapEntry<'a, T> {
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
/// The state ID is set in [`widget_state`](WidgetContext::widget_state) on init and is kept updated.
///
/// # Examples
///
/// ```
/// # fn main() -> () { }
/// use zero_ui_core::{property, context::*, var::IntoVar, widget_instance::UiNode};
///
/// pub static FOO_ID: StaticStateId<u32> = StateId::new_static();
///
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
///     set_widget_state(child, &FOO_ID, value)
/// }
///
/// // after the property is used and the widget initializes:
///
/// /// Get the value from outside the widget.
/// fn get_foo_outer(widget: &impl UiNode) -> u32 {
///     widget.with_context(|ctx| ctx.widget_state.get(&FOO_ID).copied()).flatten().unwrap_or_default()
/// }
///
/// /// Get the value from inside the widget.
/// fn get_foo_inner(ctx: &WidgetContext) -> u32 {
///     ctx.widget_state.get(&FOO_ID).copied().unwrap_or_default()
/// }
/// ```
pub fn set_widget_state<U, T>(child: U, id: impl Into<StateId<T>>, value: impl IntoVar<T>) -> impl UiNode
where
    U: UiNode,
    T: StateValue + VarValue,
{
    set_widget_state_update(child, id, value, |_, _| {})
}

/// Helper for declaring properties that set the widget state with a custom closure executed when the value updates.
///
/// The `on_update` closure is called every time the `value` variable updates.
///
/// See [`set_widget_state`] for more details.
pub fn set_widget_state_update<U, T, H>(child: U, id: impl Into<StateId<T>>, value: impl IntoVar<T>, on_update: H) -> impl UiNode
where
    U: UiNode,
    T: StateValue + VarValue,
    H: FnMut(&mut WidgetContext, &T) + Send + 'static,
{
    #[ui_node(struct SetWidgetStateNode<T: StateValue + VarValue> {
        child: impl UiNode,
        id: StateId<T>,
        #[var] value: impl Var<T>,
        on_update: impl FnMut(&mut WidgetContext, &T) + Send + 'static,
    })]
    impl UiNode for SetWidgetStateNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.init_handles(ctx);
            ctx.widget_state.set(self.id, self.value.get());
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Some(new) = self.value.get_new(ctx) {
                (self.on_update)(ctx, &new);
                ctx.widget_state.set(self.id, new);
            }
            self.child.update(ctx, updates);
        }
    }
    SetWidgetStateNode {
        child: child.cfg_boxed(),
        id: id.into(),
        value: value.into_var(),
        on_update,
    }
    .cfg_boxed()
}
