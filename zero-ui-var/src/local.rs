use std::any::TypeId;

use super::arc::WeakArcVar;

use super::*;

/// Represents a single value as [`Var<T>`].
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

    fn set_any(&self, _: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn last_update(&self) -> VarUpdateId {
        VarUpdateId::never()
    }

    fn capabilities(&self) -> VarCapabilities {
        VarCapabilities::empty()
    }

    fn hook(&self, _: Box<dyn Fn(&VarHookArgs) -> bool + Send + Sync>) -> VarHandle {
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
        usize::MAX
    }

    fn var_ptr(&self) -> VarPtr {
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

macro_rules! impl_into_var_option {
    (
        $($T:ty),* $(,)?
    ) => {
        $(
            impl IntoVar<Option<$T>> for $T {
                type Var = LocalVar<Option<$T>>;

                fn into_var(self) -> Self::Var {
                    LocalVar(Some(self))
                }
            }

            impl IntoValue<Option<$T>> for $T { }
        )*
    }
}
impl_into_var_option! {
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64,
    char, bool,
}

impl<T: VarValue> Var<T> for LocalVar<T> {
    type ReadOnly = Self;

    type ActualVar = Self;

    type Downgrade = WeakArcVar<T>;

    type Map<O: VarValue> = LocalVar<O>;

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
}
