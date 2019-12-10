use super::{FocusKey, LayoutPoint, LayoutSize};
use fnv::FnvHashMap;
use once_cell::sync::OnceCell;
use retain_mut::*;
use std::any::Any;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

macro_rules! ui_value_key {
    ($(
        $(#[$outer:meta])*
        pub struct $Key:ident (struct $Id:ident) { new_lazy() -> pub struct $KeyRef:ident };
    )+) => {$(
        uid! {struct $Id(_);}

        $(#[$outer])*
        #[derive(Debug, PartialEq, Eq, Hash)]
        pub struct $Key<T> ($Id, PhantomData<T>);

        impl<T> Clone for $Key<T> {
            fn clone(&self) -> Self {
                $Key (self.0,self.1)
            }
        }

        impl<T> Copy for $Key<T> {}

        /// Dereferences to a key that is generated on the first deref.
        pub struct $KeyRef<T> (OnceCell<$Key<T>>);

        impl<T: 'static> $Key<T> {
            /// New unique key.
            pub fn new_unique() -> Self {
                $Key ($Id::new_unique(), PhantomData)
            }

            /// New lazy initialized unique key. Use this for public static
            /// variables.
            pub const fn new_lazy() -> $KeyRef<T> {
                $KeyRef(OnceCell::new())
            }

            fn id(&self) -> $Id {
                self.0
            }
        }

        impl<T: 'static> Deref for $KeyRef<T> {
            type Target = $Key<T>;
            fn deref(&self) -> &Self::Target {
                self.0.get_or_init($Key::new_unique)
            }
        }
    )+};
}

ui_value_key! {
    /// Unique key for a value set in a parent Ui to be read in a child Ui.
    pub struct ParentValueKey(struct ParentValueId) {
        new_lazy() -> pub struct ParentValueKeyRef
    };

    /// Unique key for a value set in a child Ui to be read in a parent Ui.
    pub struct ChildValueKey(struct ChildValueId) {
        new_lazy() -> pub struct ChildValueKeyRef
    };
}

uid! {
    /// Identifies a group of nested Uis as a single element.
    pub struct UiItemId(_) { new_lazy() -> pub struct UiItemIdRef };
}

enum UntypedRef {}

/// Contains `ParentValueKey` values from call context and allows returning `ChildValueKey` values.
pub struct UiValues {
    parent_values: FnvHashMap<ParentValueId, *const UntypedRef>,
    child_values: FnvHashMap<ChildValueId, Box<dyn Any>>,

    item: UiItemId,
    window_focus_key: FocusKey,
    mouse_capture_target: Option<UiItemId>,
}
impl UiValues {
    pub fn new(window_item_id: UiItemId, window_focus_key: FocusKey, mouse_capture_target: Option<UiItemId>) -> Self {
        UiValues {
            parent_values: Default::default(),
            child_values: Default::default(),

            item: window_item_id,
            window_focus_key,
            mouse_capture_target,
        }
    }

    /// Gets the current item.
    #[inline]
    pub fn item(&self) -> UiItemId {
        self.item
    }

    /// Calls `action` with self, during that call [UiValues::item] is the `item` argument.
    pub(crate) fn item_scope(&mut self, item: UiItemId, action: impl FnOnce(&mut UiValues)) {
        let old_item = self.item;
        self.item = item;
        action(self);
        self.item = old_item;
    }

    /// Gets a value set by a parent Ui.
    #[inline]
    pub fn parent<T: 'static>(&self, key: ParentValueKey<T>) -> Option<&T> {
        // REFERENCE SAFETY: This is safe because parent_values are only inserted for the duration
        // of [with_parent_value] that holds the reference.
        //
        // TYPE SAFETY: This is safe because [ParentValueId::new] is always unique AND created by
        // [ParentValueKey::new] THAT can only be inserted in [with_parent_value].
        self.parent_values
            .get(&key.id())
            .map(|pointer| unsafe { &*(*pointer as *const T) })
    }

    /// Calls `action` with self, during that call [UiValues::parent] returns the value
    /// set by `key` => `value`.
    #[inline]
    pub fn with_parent_value<T: 'static>(
        &mut self,
        key: ParentValueKey<T>,
        value: &T,
        action: impl FnOnce(&mut UiValues),
    ) {
        let previous_value = self
            .parent_values
            .insert(key.id(), (value as *const T) as *const UntypedRef);

        action(self);

        if let Some(previous_value) = previous_value {
            self.parent_values.insert(key.id(), previous_value);
        } else {
            self.parent_values.remove(&key.id());
        }
    }

    #[inline]
    pub fn child<T: 'static>(&self, key: ChildValueKey<T>) -> Option<&T> {
        self.child_values.get(&key.id()).map(|a| a.downcast_ref::<T>().unwrap())
    }

    #[inline]
    pub fn set_child_value<T: 'static>(&mut self, key: ChildValueKey<T>, value: T) {
        self.child_values.insert(key.id(), Box::new(value));
    }

    pub(crate) fn clear_child_values(&mut self) {
        self.child_values.clear()
    }

    /// Gets the current window focus key.
    #[inline]
    pub fn window_focus_key(&self) -> FocusKey {
        self.window_focus_key
    }

    /// Gets the Ui that is capturing mouse events.
    #[inline]
    pub fn mouse_capture_target(&self) -> Option<UiItemId> {
        self.mouse_capture_target
    }
}

mod private {
    pub trait Sealed {}
}

/// A value used in a `Ui`. Derefs to `T`.
///
/// Use this as a generic constrain to work with both [Owned] values and [Var] references.
///
/// ## See also
/// * [IntoValue]: For making constructors.
pub trait Value<T>: private::Sealed + Deref<Target = T> + 'static {
    /// If the value was set in the last update.
    fn touched(&self) -> bool;

    /// Gets if `self` and `other` point to the same data.
    fn ptr_eq<O: Value<T>>(&self, other: &O) -> bool {
        std::ptr::eq(self.deref(), other.deref())
    }
}

/// An owned `'static` [Value].
///
/// This is used constructed by a [IntoValue].
#[derive(Clone)]
pub struct Owned<T>(pub T);

impl<T> private::Sealed for Owned<T> {}

impl<T> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: 'static> Value<T> for Owned<T> {
    /// Always `false`.
    fn touched(&self) -> bool {
        false
    }
}

type Listener<T> = Box<dyn FnMut(&T) -> ListenerStatus>;

struct VarData<T> {
    value: RefCell<T>,
    pending: Cell<Box<dyn FnOnce(&mut T)>>,
    touched: Cell<bool>,
    listeners: RefCell<Vec<Listener<T>>>,
}

#[derive(PartialEq, Eq)]
enum ListenerStatus {
    Alive,
    Dead,
}

/// A reference counted [Value] that can change.
pub struct Var<T: 'static> {
    r: Rc<VarData<T>>,
}

impl<T> Clone for Var<T> {
    /// Returns a new reference to the value.
    fn clone(&self) -> Self {
        Var { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static> Var<T> {
    /// New var with starting `value`.
    pub fn new(value: T) -> Self {
        Var {
            r: Rc::new(VarData {
                value: RefCell::new(value),
                pending: Cell::new(Box::new(|_| {})),
                touched: Cell::new(false),
                listeners: Default::default(),
            }),
        }
    }

    /// Gets a `Var<B>` that is set using a `map` function every time this var changes.
    pub fn map<B: 'static, F: FnMut(&T) -> B + 'static>(&self, mut map: F) -> Var<B> {
        let target = Var::new(map(self));
        let weak_target = Rc::downgrade(&target.r);

        self.r.listeners.borrow_mut().push(Box::new(move |new_value| {
            if let Some(live_target) = weak_target.upgrade() {
                *live_target.value.borrow_mut() = map(new_value);
                live_target.touched.set(true);
                ListenerStatus::Alive
            } else {
                ListenerStatus::Dead
            }
        }));
        target
    }

    /// Sets the pending value change for the next update.
    pub(crate) fn change_value(&self, change: impl FnOnce(&mut T) + 'static) {
        self.r.pending.set(Box::new(change));
    }
}

impl<T> Deref for Var<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: This is safe because borrow_mut only occurs when committing a change
        // inside a FnOnce : 'static. Because it is 'static it cannot capture a unguarded
        // reference, but it can capture a Var clone, in that case we panic.
        unsafe {
            &self
                .r
                .value
                .try_borrow_unguarded()
                .expect("Cannot deref `Var` while changing the same `Var`")
        }
    }
}

impl<T> private::Sealed for Var<T> {}

impl<T: 'static> Value<T> for Var<T> {
    /// Gets if the var was set in the last update.
    fn touched(&self) -> bool {
        self.r.touched.get()
    }
}

/// Into `[Value]<T>`.
pub trait IntoValue<T> {
    type Value: Value<T>;

    fn into_value(self) -> Self::Value;
}

/// Does nothing. `[Var]<T>` already implements `Value<T>`.
impl<T: 'static> IntoValue<T> for Var<T> {
    type Value = Var<T>;

    fn into_value(self) -> Self::Value {
        self
    }
}

/// Wraps the value in an `[Owned]<T>` value.
impl<T: 'static> IntoValue<T> for T {
    type Value = Owned<T>;

    fn into_value(self) -> Owned<T> {
        Owned(self)
    }
}

/// [Var] updating methods, separated to allow dynamic dispatch.
pub(crate) trait VarChange {
    /// Commits the pending value and set touched to `true`.
    fn commit(&mut self);
    /// Resets touched to `false`.
    fn reset_touched(&mut self);
}

impl<T> VarChange for Var<T> {
    fn commit(&mut self) {
        let change = self.r.pending.replace(Box::new(|_| {}));
        change(&mut self.r.value.borrow_mut());
        self.r.touched.set(true);

        let new_value = self.r.value.borrow();

        self.r
            .listeners
            .borrow_mut()
            .retain_mut(|l| l(&new_value) == ListenerStatus::Alive);
    }

    fn reset_touched(&mut self) {
        self.r.touched.set(false);
    }
}

impl<'s> IntoValue<String> for &'s str {
    type Value = Owned<String>;

    fn into_value(self) -> Owned<String> {
        Owned(self.to_owned())
    }
}

impl IntoValue<Cow<'static, str>> for &'static str {
    type Value = Owned<Cow<'static, str>>;

    fn into_value(self) -> Self::Value {
        Owned(self.into())
    }
}

impl IntoValue<Cow<'static, str>> for String {
    type Value = Owned<Cow<'static, str>>;

    fn into_value(self) -> Self::Value {
        Owned(self.into())
    }
}

impl IntoValue<LayoutPoint> for (f32, f32) {
    type Value = Owned<LayoutPoint>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutPoint::new(self.0, self.1))
    }
}

impl IntoValue<LayoutSize> for (f32, f32) {
    type Value = Owned<LayoutSize>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutSize::new(self.0, self.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_parent_value() {
        let mut ui_values = UiValues::new(UiItemId::new_unique(), FocusKey::new_unique(), None);
        let key1 = ParentValueKey::new_unique();
        let key2 = ParentValueKey::new_unique();

        let val1: u32 = 10;
        let val2: u32 = 11;
        let val3: u32 = 12;

        assert_eq!(ui_values.parent(key1), None);
        assert_eq!(ui_values.parent(key2), None);

        ui_values.with_parent_value(key1, &val1, |ui_values| {
            assert_eq!(ui_values.parent(key1), Some(&val1));
            assert_eq!(ui_values.parent(key2), None);

            ui_values.with_parent_value(key2, &val2, |ui_values| {
                assert_eq!(ui_values.parent(key1), Some(&val1));
                assert_eq!(ui_values.parent(key2), Some(&val2));

                ui_values.with_parent_value(key1, &val3, |ui_values| {
                    assert_eq!(ui_values.parent(key1), Some(&val3));
                    assert_eq!(ui_values.parent(key2), Some(&val2));
                });

                assert_eq!(ui_values.parent(key1), Some(&val1));
                assert_eq!(ui_values.parent(key2), Some(&val2));
            });

            assert_eq!(ui_values.parent(key1), Some(&val1));
            assert_eq!(ui_values.parent(key2), None);
        });

        assert_eq!(ui_values.parent(key1), None);
        assert_eq!(ui_values.parent(key2), None);
    }
}
