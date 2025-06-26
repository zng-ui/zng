use std::any::TypeId;

use super::arc::WeakArcVar;

use super::*;

/// Represents a single value as [`Var<T>`].
///
/// This is the var target for most [`IntoVar<T>`] implementations.
#[derive(Clone)]
pub struct LocalVar<T: VarValue>(pub T);

impl<T: VarValue> crate::private::Sealed for LocalVar<T> {}

impl<T: VarValue> AnyVar for LocalVar<T> {
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
        Box::new(self.0.clone())
    }

    fn with_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) {
        read(&self.0)
    }

    fn with_new_any(&self, _: &mut dyn FnMut(&dyn AnyVarValue)) -> bool {
        false
    }

    fn set_any(&self, _: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn last_update(&self) -> VarUpdateId {
        VarUpdateId::never()
    }

    fn is_contextual(&self) -> bool {
        false
    }

    fn capabilities(&self) -> VarCapability {
        VarCapability::empty()
    }

    fn hook_any(&self, _: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        VarHandle::dummy()
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        Err(handler)
    }

    fn strong_count(&self) -> usize {
        0
    }

    fn weak_count(&self) -> usize {
        0
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.clone_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakArcVar::<T>::new())
    }

    fn is_animating(&self) -> bool {
        false
    }

    fn modify_importance(&self) -> usize {
        0
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        VarPtr::new_never_eq(self)
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

impl<T: VarValue> IntoVar<T> for LocalVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
impl<T: VarValue> IntoVar<T> for T {
    type Var = LocalVar<T>;

    fn into_var(self) -> Self::Var {
        LocalVar(self)
    }
}

impl<T: VarValue> Var<T> for LocalVar<T> {
    type ReadOnly = Self;

    type ActualVar = Self;

    type Downgrade = WeakArcVar<T>;

    type Map<O: VarValue> = LocalVar<O>;
    type MapBidi<O: VarValue> = LocalVar<O>;

    type FlatMap<O: VarValue, V: Var<O>> = V;

    type FilterMap<O: VarValue> = LocalVar<O>;
    type FilterMapBidi<O: VarValue> = LocalVar<O>;

    type MapRef<O: VarValue> = LocalVar<O>;
    type MapRefBidi<O: VarValue> = LocalVar<O>;

    type Easing = LocalVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        read(&self.0)
    }

    fn modify<F>(&self, _: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + 'static,
    {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn actual_var(self) -> Self::ActualVar {
        self
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakArcVar::new()
    }

    fn into_value(self) -> T {
        self.0
    }

    fn read_only(&self) -> Self::ReadOnly {
        self.clone()
    }

    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
    {
        LocalVar(self.with(map))
    }

    fn map_bidi<O, M, B>(&self, map: M, _: B) -> Self::MapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static,
    {
        self.map(map)
    }

    fn flat_map<O, V, M>(&self, map: M) -> Self::FlatMap<O, V>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + Send + 'static,
    {
        self.with(map)
    }

    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> Self::FilterMap<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        LocalVar(self.with(map).unwrap_or_else(fallback))
    }

    fn filter_map_bidi<O, M, B, I>(&self, map: M, _: B, fallback: I) -> Self::FilterMapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        self.filter_map(map, fallback)
    }

    fn map_ref<O, M>(&self, map: M) -> Self::MapRef<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
    {
        LocalVar(self.with(|v| map(v).clone()))
    }

    fn map_ref_bidi<O, M, B>(&self, map: M, _: B) -> Self::MapRefBidi<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static,
    {
        self.map_ref(map)
    }

    fn easing<F>(&self, _: Duration, _: F) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        self.clone()
    }

    fn easing_with<F, S>(&self, _: Duration, _: F, _: S) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
    {
        self.clone()
    }
}
