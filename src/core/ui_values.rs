use super::{FocusKey, LayoutPoint, LayoutSize};
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
    pub trait ValueMutSet<T> {
        fn change_value(&self, change: impl FnOnce(&mut T) + 'static);
    }
    pub trait SwitchSet {
        fn change_index(&self, new_index: usize);
    }
}

/// Commits a [ValueMut] change.
pub trait ValueMutCommit {
    /// Commits the pending value and set touched to `true`.
    fn commit(&self);
    /// Resets touched to `false`.
    fn reset_touched(&self);
}

/// Comits a [SwitchValue] change.
pub trait SwitchCommit {
    /// Commits the pending index change and set touched to `true`.
    fn commit(&self);
    /// Resets index change touched to `false`.
    fn reset_touched(&self);
}

/// A value used in a `Ui`. Derefs to `T`.
///
/// Use this as a generic constrain to work with both [Owned] values and [Var] or [SwitchValue] references.
///
/// ## See also
/// * [IntoValue]: For making constructors.
pub trait Value<T>: private::Sealed + Deref<Target = T> + 'static {
    /// If the value was set in the last update.
    fn touched(&self) -> bool;

    /// Gets the value version. It is different every time the value gets [touched].
    fn version(&self) -> u64;

    /// Returns a maping `Value<B>` that stays in sync with this `Value<T>`.
    fn map<B, M: FnMut(&T) -> B>(&self, f: M) -> ValueMap<T, Self, B, M>
    where
        Self: Sized;

    /// Gets if `self` and `other` point to the same data.
    fn ptr_eq<O: Value<T>>(&self, other: &O) -> bool {
        std::ptr::eq(self.deref(), other.deref())
    }
}

struct ValueMapSource<T: 'static, V: Value<T>, B: 'static, M: FnMut(&T) -> B + 'static> {
    _t: PhantomData<T>,
    source: V,
    map: RefCell<M>,
    cached_version: Cell<u64>,
}

struct ValueMapData<T: 'static, V: Value<T>, B: 'static, M: FnMut(&T) -> B + 'static> {
    source: Option<Box<ValueMapSource<T, V, B, M>>>,
    cached: Cell<B>,
}

/// Result of [Value::map]. Implements [Value].
pub struct ValueMap<T: 'static, V: Value<T>, B: 'static, M: FnMut(&T) -> B + 'static> {
    r: Rc<ValueMapData<T, V, B, M>>,
}

impl<T: 'static, V: Value<T>, B: 'static, M: FnMut(&T) -> B + 'static> ValueMap<T, V, B, M> {
    fn with_source(source: V, mut map: M) -> Self {
        let cached = map(&*source);
        let cached_version = source.version();
        ValueMap {
            r: Rc::new(ValueMapData {
                source: Some(Box::new(ValueMapSource {
                    _t: PhantomData,
                    source,
                    map: RefCell::new(map),
                    cached_version: Cell::new(cached_version),
                })),
                cached: Cell::new(cached),
            }),
        }
    }

    fn once(source: &V, mut map: M) -> Self {
        ValueMap {
            r: Rc::new(ValueMapData {
                source: None,
                cached: Cell::new(map(&*source)),
            }),
        }
    }
}

impl<T: 'static, V: Value<T>, B: 'static, M: FnMut(&T) -> B + 'static> private::Sealed for ValueMap<T, V, B, M> {}

impl<T: 'static, V: Value<T>, B: 'static, M: FnMut(&T) -> B + 'static> Clone for ValueMap<T, V, B, M> {
    fn clone(&self) -> Self {
        ValueMap { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static, V: Value<T>, B: 'static, M: FnMut(&T) -> B + 'static> Deref for ValueMap<T, V, B, M> {
    type Target = B;

    fn deref(&self) -> &B {
        if let Some(s) = &self.r.source {
            let source_version = s.source.version();
            if source_version != s.cached_version.get() {
                self.r.cached.set((&mut *s.map.borrow_mut())(&*s.source));
                s.cached_version.set(source_version);
            }
        }
        // TODO safe?
        unsafe { &*self.r.cached.as_ptr() }
    }
}

impl<T: 'static, V: Value<T>, B: 'static, M: FnMut(&T) -> B + 'static> Value<B> for ValueMap<T, V, B, M> {
    fn touched(&self) -> bool {
        self.r.source.as_ref().map(|s| s.source.touched()).unwrap_or_default()
    }

    fn version(&self) -> u64 {
        self.r.source.as_ref().map(|s| s.source.version()).unwrap_or_default()
    }

    /// Returns a follow up mapping. If `self` stayed in sync with original source this
    /// map also syncs.
    fn map<C, N: FnMut(&B) -> C>(&self, mut map: N) -> ValueMap<B, Self, C, N>
    where
        Self: Sized,
    {
        if self.r.source.is_some() {
            ValueMap::with_source(Self::clone(self), map)
        } else {
            // TODO not safe? [deref] can be called inside map?
            let cached = unsafe { map(&*self.r.cached.as_ptr()) };

            ValueMap {
                r: Rc::new(ValueMapData {
                    source: None,
                    cached: Cell::new(cached),
                }),
            }
        }
    }
}

/// A [Value] that can be set.
///
/// Use this a generic constrain to work with [Var] or [SwitchValue] references.
pub trait ValueMut<T>: Value<T> + private::ValueMutSet<T> + ValueMutCommit + Clone + 'static {}

/// An owned `'static` [Value].
///
/// This is usually constructed by a [IntoValue].
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

    /// Always `0`.
    fn version(&self) -> u64 {
        0
    }

    /// Returns an instance of [ValueMap] that does not `self` or `map` in memory.
    /// The function is called imediatly only once.
    fn map<B, M: FnMut(&T) -> B>(&self, map: M) -> ValueMap<T, Self, B, M> {
        ValueMap::once(self, map)
    }
}

struct VarData<T> {
    value: RefCell<T>,
    pending: Cell<Box<dyn FnOnce(&mut T)>>,
    touched: Cell<bool>,
    version: Cell<u64>,
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
                version: Cell::new(0),
            }),
        }
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

    /// Gets the var value version.
    fn version(&self) -> u64 {
        self.r.version.get()
    }

    /// Returns an instance of [ValueMap] that holds a strong reference to this
    /// variable and applies the `map` function every time this variable is [touched].
    fn map<B, M: FnMut(&T) -> B>(&self, map: M) -> ValueMap<T, Self, B, M> {
        ValueMap::with_source(Var::clone(self), map)
    }
}

impl<T: 'static> private::ValueMutSet<T> for Var<T> {
    fn change_value(&self, change: impl FnOnce(&mut T) + 'static) {
        self.r.pending.set(Box::new(change));
    }
}

impl<T: 'static> ValueMutCommit for Var<T> {
    fn commit(&self) {
        let change = self.r.pending.replace(Box::new(|_| {}));
        change(&mut self.r.value.borrow_mut());

        self.r.touched.set(true);

        let version = self.r.version.get();
        self.r.version.set(version.wrapping_add(1));
    }

    fn reset_touched(&self) {
        self.r.touched.set(false);
    }
}

impl<T: 'static> ValueMut<T> for Var<T> {}

/// Into `[Value]<T>`.
pub trait IntoValue<T> {
    type Value: Value<T>;

    fn into_value(self) -> Self::Value;
}

/// Does nothing. `[Var]<T>` already implements `Value<T>`.
impl<T: 'static> IntoValue<T> for Var<T> {
    type Value = Self;

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

/// Does nothing. `[SwitchVar2]<T>` already implements `Value<T>`.
impl<T: 'static, V0: Value<T>, V1: Value<T>> IntoValue<T> for SwitchVar2<T, V0, V1> {
    type Value = Self;

    fn into_value(self) -> Self::Value {
        self
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

///
#[allow(clippy::len_without_is_empty)]
pub trait SwitchValue: private::SwitchSet + SwitchCommit + Clone + 'static {
    /// Current variable.
    fn index(&self) -> usize;

    /// Switch table length.
    fn len(&self) -> usize;
}

struct SwitchVar2Data<T: 'static, V0: Value<T>, V1: Value<T>> {
    t: PhantomData<T>,

    index: Cell<u8>,
    pending_index: Cell<u8>,

    /// Previous index, same as index if no commit happen or if touched reseted.
    ///
    /// We need this because of the order [ValueMutCommit] and
    /// [SwitchCommit] are applied.
    ///
    /// * First value changes are commited so the value touched flag is set.
    /// * Then switch changes are commited so [index] potentially no longer points to
    ///   a value with touched flat set.
    /// * After change notification, ValueMutCommits that where commited get touched flag reset
    ///   requests, so we use this index to find the right value.
    /// * Then switchs commited get the switch flag reset, this is when we set
    ///   this to be equal to `index`.
    prev_index: Cell<u8>,

    v0: V0,
    v1: V1,
}

pub struct SwitchVar2<T: 'static, V0: Value<T>, V1: Value<T>> {
    r: Rc<SwitchVar2Data<T, V0, V1>>,
}

impl<T: 'static, V0: Value<T>, V1: Value<T>> SwitchVar2<T, V0, V1> {
    pub fn new(index: u8, v0: V0, v1: V1) -> Self {
        assert!(index < 2);

        SwitchVar2 {
            r: Rc::new(SwitchVar2Data {
                t: PhantomData,
                index: Cell::new(index),
                pending_index: Cell::new(index),
                prev_index: Cell::new(index),

                v0,
                v1,
            }),
        }
    }

    fn index_impl(&self) -> u8 {
        self.r.index.get()
    }

    fn touched_impl(&self) -> bool {
        self.r.index.get() != self.r.prev_index.get()
    }
}

impl<T: 'static, V0: Value<T>, V1: Value<T>> private::SwitchSet for SwitchVar2<T, V0, V1> {
    fn change_index(&self, new_value: usize) {
        self.r.pending_index.set(new_value as u8);
    }
}

impl<T: 'static, V0: Value<T>, V1: Value<T>> SwitchCommit for SwitchVar2<T, V0, V1> {
    fn commit(&self) {
        self.r.prev_index.set(self.r.index.get());
        self.r.index.set(self.r.pending_index.get());
    }

    fn reset_touched(&self) {
        self.r.prev_index.set(self.r.index.get());
    }
}

impl<T: 'static, V0: Value<T>, V1: Value<T>> SwitchValue for SwitchVar2<T, V0, V1> {
    fn index(&self) -> usize {
        self.index_impl() as usize
    }

    fn len(&self) -> usize {
        2
    }
}

impl<T: 'static, V0: ValueMut<T>, V1: ValueMut<T>> private::ValueMutSet<T> for SwitchVar2<T, V0, V1> {
    fn change_value(&self, change: impl FnOnce(&mut T) + 'static) {
        match self.index() {
            0 => self.r.v0.change_value(change),
            1 => self.r.v1.change_value(change),
            _ => unreachable!(),
        }
    }
}

impl<T: 'static, V0: Value<T>, V1: Value<T>> Clone for SwitchVar2<T, V0, V1> {
    fn clone(&self) -> Self {
        SwitchVar2 { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static, V0: ValueMut<T>, V1: ValueMut<T>> ValueMutCommit for SwitchVar2<T, V0, V1> {
    fn commit(&self) {
        match self.index_impl() {
            0 => self.r.v0.commit(),
            1 => self.r.v1.commit(),
            _ => unreachable!(),
        }
    }

    fn reset_touched(&self) {
        match self.r.prev_index.get() {
            0 => self.r.v0.reset_touched(),
            1 => self.r.v1.reset_touched(),
            _ => unreachable!(),
        }
    }
}

impl<T: 'static, V0: ValueMut<T>, V1: ValueMut<T>> ValueMut<T> for SwitchVar2<T, V0, V1> {}

impl<T: 'static, V0: Value<T>, V1: Value<T>> Deref for SwitchVar2<T, V0, V1> {
    type Target = T;

    fn deref(&self) -> &T {
        match self.index_impl() {
            0 => &*self.r.v0,
            1 => &*self.r.v1,
            _ => unreachable!(),
        }
    }
}

impl<T: 'static, V0: Value<T>, V1: Value<T>> private::Sealed for SwitchVar2<T, V0, V1> {}

impl<T: 'static, V0: Value<T>, V1: Value<T>> Value<T> for SwitchVar2<T, V0, V1> {
    fn touched(&self) -> bool {
        self.touched_impl()
            || match self.index() {
                0 => self.r.v0.touched(),
                1 => self.r.v1.touched(),
                _ => unreachable!(),
            }
    }

    fn version(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut version = DefaultHasher::new();
        let index = self.index();
        version.write_usize(index);
        version.write_u64(match index {
            0 => self.r.v0.version(),
            1 => self.r.v1.version(),
            _ => unreachable!(),
        });
        version.finish()
    }

    /// Returns an instance of [ValueMap] that holds a strong reference to this
    /// variable and applies the `map` function every time this variable is [touched].
    fn map<B, M: FnMut(&T) -> B>(&self, map: M) -> ValueMap<T, Self, B, M> {
        ValueMap::with_source(Self::clone(self), map)
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
