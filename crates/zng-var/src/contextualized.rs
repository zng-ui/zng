use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use parking_lot::{RwLock, RwLockReadGuard};

use super::{types::WeakContextInitHandle, *};

#[cfg(feature = "dyn_closure")]
macro_rules! ActualLock {
    ($T:ident) => {
        parking_lot::RwLock<Vec<(WeakContextInitHandle, Box<dyn Any + Send + Sync>)>>
    }
}
#[cfg(not(feature = "dyn_closure"))]
macro_rules! ActualLock {
    ($T:ident) => {
        parking_lot::RwLock<Vec<(WeakContextInitHandle, BoxedVar<$T>)>>
    }
}

#[cfg(feature = "dyn_closure")]
macro_rules! ActualInit {
    ($T:ident) => {
        Arc<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>
    }
}
#[cfg(not(feature = "dyn_closure"))]
macro_rules! ActualInit {
    ($T:ident) => {
        Arc<dyn Fn() -> BoxedVar<$T> + Send + Sync>
    }
}

#[cfg(feature = "dyn_closure")]
macro_rules! ActualReadGuard {
    ($a:tt, $T:ident) => {
        parking_lot::MappedRwLockReadGuard<$a, Box<dyn Any + Send + Sync>>
    }
}
#[cfg(not(feature = "dyn_closure"))]
macro_rules! ActualReadGuard {
    ($a:tt, $T:ident) => {
        parking_lot::MappedRwLockReadGuard<$a, BoxedVar<$T>>
    }
}

/// Represents a variable that delays initialization until the first usage.
///
/// Usage that initializes the variable are all [`AnyVar`] and [`Var<T>`] methods except `read_only`, `downgrade` and `boxed`.
/// The variable re-initializes when the [`ContextInitHandle::current`] is different on usage.
///
/// This variable is used in the [`Var::map`] and other mapping methods to support mapping from [`ContextVar<T>`].
///
/// ```
/// # macro_rules! fake{($($tt:tt)*) => {}}
/// # fake! {
/// let wgt = MyWgt! {
///     my_property = MY_CTX_VAR.map(|&b| !b);
/// };
/// # }
/// ```
///
/// In the example above the mapping var will bind with the `MY_CTX_VAR` context inside the property node, not
/// the context at the moment the widget is instantiated.
///
/// # Clone
///
/// Note that a clone of this variable may call the init closure again for the same context, the inited actual var
/// is only reused if it is already inited when clone is called and clone is called on the same context.
pub struct ContextualizedVar<T> {
    _type: PhantomData<T>,

    init: ActualInit![T],
    actual: ActualLock![T],
}

#[expect(clippy::extra_unused_type_parameters)]
fn borrow_init_impl<'a, T>(
    actual: &'a ActualLock![T],
    init: &ActualInit![T],
    #[cfg(debug_assertions)] type_name: &'static str,
) -> ActualReadGuard!['a, T] {
    let current_ctx = ContextInitHandle::current();
    let current_ctx = current_ctx.downgrade();

    let act = actual.read_recursive();

    if let Some(i) = act.iter().position(|(h, _)| h == &current_ctx) {
        return RwLockReadGuard::map(act, move |m| &m[i].1);
    }
    drop(act);

    let mut actual_mut = actual.write();
    actual_mut.retain(|(h, _)| h.is_alive());
    let i = actual_mut.len();

    #[cfg(debug_assertions)]
    if i == 200 {
        tracing::debug!("variable of type `{type_name}` actualized >200 times");
    }

    if !actual_mut.iter().any(|(c, _)| c == &current_ctx) {
        actual_mut.push((current_ctx.clone(), init()));
    }
    drop(actual_mut);

    let actual = actual.read_recursive();
    RwLockReadGuard::map(actual, move |m| {
        if i < m.len() && m[i].0 == current_ctx {
            &m[i].1
        } else if let Some(i) = m.iter().position(|(h, _)| h == &current_ctx) {
            &m[i].1
        } else {
            unreachable!()
        }
    })
}

impl<T: VarValue> ContextualizedVar<T> {
    /// New with initialization function.
    ///
    /// The `init` closure will be called on the first usage of the var, once after the var is cloned and any time
    /// a parent contextualized var is initializing.
    pub fn new<V: Var<T>>(init: impl Fn() -> V + Send + Sync + 'static) -> Self {
        Self {
            _type: PhantomData,

            #[cfg(feature = "dyn_closure")]
            init: Arc::new(move || Box::new(init().boxed())),
            #[cfg(not(feature = "dyn_closure"))]
            init: Arc::new(move || init().boxed()),

            actual: RwLock::new(Vec::with_capacity(1)),
        }
    }

    /// New with initialization function that produces a value.
    ///
    /// The `init` closure will be called on the first usage of the var, once after the var is cloned and any time
    /// a parent contextualized var is initializing.
    pub fn new_value(init: impl Fn() -> T + Send + Sync + 'static) -> Self {
        Self::new(move || init().into_var())
    }

    /// Borrow/initialize the actual var.
    pub fn borrow_init(&self) -> parking_lot::MappedRwLockReadGuard<'_, BoxedVar<T>> {
        #[cfg(feature = "dyn_closure")]
        {
            parking_lot::MappedRwLockReadGuard::map(
                borrow_init_impl::<()>(
                    &self.actual,
                    &self.init,
                    #[cfg(debug_assertions)]
                    std::any::type_name::<T>(),
                ),
                |v| v.downcast_ref().unwrap(),
            )
        }
        #[cfg(not(feature = "dyn_closure"))]
        {
            borrow_init_impl(
                &self.actual,
                &self.init,
                #[cfg(debug_assertions)]
                std::any::type_name::<T>(),
            )
        }
    }

    /// Unwraps the initialized actual var or initializes it now.
    pub fn into_init(self) -> BoxedVar<T> {
        let mut act = self.actual.into_inner();
        let current_ctx = ContextInitHandle::current().downgrade();

        if let Some(i) = act.iter().position(|(h, _)| h == &current_ctx) {
            #[cfg(feature = "dyn_closure")]
            {
                *act.swap_remove(i).1.downcast().unwrap()
            }
            #[cfg(not(feature = "dyn_closure"))]
            {
                act.swap_remove(i).1
            }
        } else {
            #[cfg(feature = "dyn_closure")]
            {
                *(self.init)().downcast().unwrap()
            }
            #[cfg(not(feature = "dyn_closure"))]
            {
                (self.init)()
            }
        }
    }
}

/// Weak var that upgrades to an uninitialized [`ContextualizedVar<T, S>`].
pub struct WeakContextualizedVar<T> {
    _type: PhantomData<T>,

    #[cfg(feature = "dyn_closure")]
    init: Weak<dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync>,

    #[cfg(not(feature = "dyn_closure"))]
    init: Weak<dyn Fn() -> BoxedVar<T> + Send + Sync>,
}

impl<T: VarValue> Clone for ContextualizedVar<T> {
    fn clone(&self) -> Self {
        let current_ctx_id = ContextInitHandle::current().downgrade();
        let act = self.actual.read_recursive();
        if let Some(i) = act.iter().position(|(id, _)| *id == current_ctx_id) {
            return Self {
                _type: PhantomData,
                init: self.init.clone(),
                #[cfg(feature = "dyn_closure")]
                actual: RwLock::new(vec![(
                    act[i].0.clone(),
                    Box::new(act[i].1.downcast_ref::<BoxedVar<T>>().unwrap().clone()),
                )]),
                #[cfg(not(feature = "dyn_closure"))]
                actual: RwLock::new(vec![act[i].clone()]),
            };
        }
        Self {
            _type: PhantomData,
            init: self.init.clone(),
            actual: RwLock::default(),
        }
    }
}
impl<T: VarValue> Clone for WeakContextualizedVar<T> {
    fn clone(&self) -> Self {
        Self {
            _type: PhantomData,
            init: self.init.clone(),
        }
    }
}

impl<T: VarValue> crate::private::Sealed for ContextualizedVar<T> {}
impl<T: VarValue> crate::private::Sealed for WeakContextualizedVar<T> {}

impl<T: VarValue> AnyVar for ContextualizedVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_unboxed_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<T> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn with_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.borrow_init().with_any(read)
    }

    fn with_new_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) -> bool {
        self.borrow_init().with_new_any(read)
    }

    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.borrow_init().last_update()
    }

    fn is_contextual(&self) -> bool {
        true
    }

    fn capabilities(&self) -> VarCapability {
        self.borrow_init().capabilities()
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        self.borrow_init().hook_any(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.borrow_init().hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.init)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.init)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.borrow_init().actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }

    fn is_animating(&self) -> bool {
        self.borrow_init().is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.borrow_init().modify_importance()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        VarPtr::new_arc(&self.init)
    }

    fn get_debug(&self) -> Txt {
        self.with(var_debug)
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        Var::modify(self, var_update)
    }

    fn map_debug(&self) -> BoxedVar<Txt> {
        Var::map(self, var_debug).boxed()
    }
}
impl<T: VarValue> AnyWeakVar for WeakContextualizedVar<T> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.init.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.init.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.upgrade().map(|c| Box::new(c) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue> IntoVar<T> for ContextualizedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for ContextualizedVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = BoxedVar<T>;

    type Downgrade = WeakContextualizedVar<T>;

    type Map<O: VarValue> = contextualized::ContextualizedVar<O>;
    type MapBidi<O: VarValue> = contextualized::ContextualizedVar<O>;

    type FlatMap<O: VarValue, V: Var<O>> = contextualized::ContextualizedVar<O>;

    type FilterMap<O: VarValue> = contextualized::ContextualizedVar<O>;
    type FilterMapBidi<O: VarValue> = contextualized::ContextualizedVar<O>;

    type MapRef<O: VarValue> = types::MapRef<T, O, Self>;
    type MapRefBidi<O: VarValue> = types::MapRefBidi<T, O, Self>;

    type Easing = types::ContextualizedVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.borrow_init().with(read)
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        self.borrow_init().modify(modify)
    }

    fn actual_var(self) -> Self::ActualVar {
        self.into_init().actual_var()
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakContextualizedVar {
            _type: PhantomData,
            init: Arc::downgrade(&self.init),
        }
    }

    fn into_value(self) -> T {
        self.into_init().into_value()
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }

    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
    {
        var_map_ctx(self, map)
    }

    fn map_bidi<O, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static,
    {
        var_map_bidi_ctx(self, map, map_back)
    }

    fn flat_map<O, V, M>(&self, map: M) -> Self::FlatMap<O, V>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + Send + 'static,
    {
        var_flat_map_ctx(self, map)
    }

    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> Self::FilterMap<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        var_filter_map_ctx(self, map, fallback)
    }

    fn filter_map_bidi<O, M, B, I>(&self, map: M, map_back: B, fallback: I) -> Self::FilterMapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        var_filter_map_bidi_ctx(self, map, map_back, fallback)
    }

    fn map_ref<O, M>(&self, map: M) -> Self::MapRef<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
    {
        var_map_ref(self, map)
    }

    fn map_ref_bidi<O, M, B>(&self, map: M, map_mut: B) -> Self::MapRefBidi<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static,
    {
        var_map_ref_bidi(self, map, map_mut)
    }

    fn easing<F>(&self, duration: Duration, easing: F) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        var_easing_ctx(self, duration, easing)
    }

    fn easing_with<F, SE>(&self, duration: Duration, easing: F, sampler: SE) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        SE: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
    {
        var_easing_with_ctx(self, duration, easing, sampler)
    }
}
impl<T: VarValue> WeakVar<T> for WeakContextualizedVar<T> {
    type Upgrade = ContextualizedVar<T>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        Some(ContextualizedVar {
            _type: PhantomData,
            init: self.init.upgrade()?,
            actual: RwLock::default(),
        })
    }
}
