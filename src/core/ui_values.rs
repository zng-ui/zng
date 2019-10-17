use super::{LayoutPoint, LayoutSize};
use fnv::FnvHashMap;
use once_cell::sync::OnceCell;
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
            pub fn new() -> Self {
                $Key ($Id::new(), PhantomData)
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
                self.0.get_or_init(|| $Key::new())
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

enum UntypedRef {}

/// Contains `ParentValueKey` values from call context and allows returning `ChildValueKey` values.
#[derive(new)]
pub struct UiValues {
    #[new(default)]
    parent_values: FnvHashMap<ParentValueId, *const UntypedRef>,
    #[new(default)]
    child_values: FnvHashMap<ChildValueId, Box<dyn Any>>,
}
impl UiValues {
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

    pub fn child<T: 'static>(&self, key: ChildValueKey<T>) -> Option<&T> {
        self.child_values.get(&key.id()).map(|a| a.downcast_ref::<T>().unwrap())
    }

    pub fn set_child_value<T: 'static>(&mut self, key: ChildValueKey<T>, value: T) {
        self.child_values.insert(key.id(), Box::new(value));
    }

    pub(crate) fn clear_child_values(&mut self) {
        self.child_values.clear()
    }
}

mod private {
    pub trait Sealed {}
}

pub trait Value<T>: private::Sealed + Deref<Target = T> {
    fn changed(&self) -> bool;

    /// Gets if `self` and `other` derefs to the same data.
    fn is_same<O: Value<T>>(&self, other: &O) -> bool {
        std::ptr::eq(self.deref(), other.deref())
    }
}

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
    fn changed(&self) -> bool {
        false
    }
}

struct VarData<T> {
    value: RefCell<T>,
    pending: Cell<Box<dyn FnOnce(&mut T)>>,
    changed: Cell<bool>,
}

pub struct Var<T> {
    r: Rc<VarData<T>>,
}

impl<T: 'static> Var<T> {
    pub fn new(value: T) -> Self {
        Var {
            r: Rc::new(VarData {
                value: RefCell::new(value),
                pending: Cell::new(Box::new(|_| {})),
                changed: Cell::new(false),
            }),
        }
    }

    pub(crate) fn change_value(&self, change: impl FnOnce(&mut T) + 'static) {
        self.r.pending.set(Box::new(change));
    }
}

impl<T> Clone for Var<T> {
    fn clone(&self) -> Self {
        Var { r: Rc::clone(&self.r) }
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

impl<T> Value<T> for Var<T> {
    fn changed(&self) -> bool {
        self.r.changed.get()
    }
}

pub trait IntoValue<T> {
    type Value: Value<T>;

    fn into_value(self) -> Self::Value;
}

/// Does nothing. `Var<T>` already implements `Value<T>`.
impl<T> IntoValue<T> for Var<T> {
    type Value = Var<T>;

    fn into_value(self) -> Self::Value {
        self
    }
}

/// Wraps the value in an `Owned<T>` value.
impl<T: 'static> IntoValue<T> for T {
    type Value = Owned<T>;

    fn into_value(self) -> Owned<T> {
        Owned(self)
    }
}

pub(crate) trait VarChange {
    fn commit(&mut self);
    fn reset_changed(&mut self);
}

impl<T> VarChange for Var<T> {
    fn commit(&mut self) {
        let change = self.r.pending.replace(Box::new(|_| {}));
        change(&mut self.r.value.borrow_mut());
        self.r.changed.set(true);
    }

    fn reset_changed(&mut self) {
        self.r.changed.set(false);
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
mod ui_values {
    use super::*;

    #[test]
    fn with_parent_value() {
        let mut ui_values = UiValues::new();
        let key1 = ParentValueKey::new();
        let key2 = ParentValueKey::new();

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
