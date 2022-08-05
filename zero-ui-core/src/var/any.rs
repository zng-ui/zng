use std::any::Any;

use super::*;

/// Type erased var.
///
/// All `Var<T>` types implement this as well, you can use [`AnyVar::into_any`] to store variables of different types
/// in the same collection and retain access to methods that do not need the variable type to function.
pub trait AnyVar: Any + crate::private::Sealed {
    /// Erase the variable type.
    fn into_any(self) -> Box<dyn AnyVar>;
    /// Cast to [`Any`].
    fn as_any(&self) -> &dyn Any;
    /// Type erased [`Var::boxed`].
    ///
    /// Returns a value that can be down-casted to `BoxedVar<T>` if the value type is known.
    fn boxed_any(self: Box<Self>) -> Box<dyn AnyVar>;

    /// Type erased [`Var::is_new`].
    fn is_new_any(&self, vars: &Vars) -> bool;
    /// Type erased [`Var::version`].
    fn version_any(&self, vars: &VarsRead) -> VarVersion;
    /// Type erased [`Var::is_read_only`].
    fn is_read_only_any(&self, vars: &Vars) -> bool;
    /// Type erased [`Var::always_read_only`].
    fn always_read_only_any(&self) -> bool;
    /// Type erased [`Var::is_contextual`].
    fn is_contextual_any(&self) -> bool;
    /// Type erased [`Var::is_rc`].
    fn is_rc_any(&self) -> bool;
    /// Type erased [`Var::can_update`].
    fn can_update_any(&self) -> bool;
    /// Type erased [`Var::is_animating`].
    fn is_animating_any(&self, vars: &VarsRead) -> bool;
    /// Type erased [`Var::update_mask`].
    fn update_mask_any(&self, vars: &VarsRead) -> UpdateMask;
    /// Type erased [`Var::actual_var`].
    fn actual_var_any(&self, vars: &Vars) -> Box<dyn AnyVar>;
    /// Type erased [`Var::as_ptr`].
    fn as_ptr_any(&self) -> *const ();

    /// Type erased [`Var::strong_count`].
    fn strong_count_any(&self) -> usize;
    /// Type erased [`Var::weak_count`].
    fn weak_count_any(&self) -> usize;
    /// Type erased [`Var::downgrade`].
    fn downgrade_any(&self) -> Option<Box<dyn AnyWeakVar>>;
}

/// Type erased weak var.
///
/// All `WeakVar<T>` types implement this trait, you can use [`AnyWeakVar::into_any`] to store weak references to variables
/// of different types in the same collection and retain access to methods that do not need the variable type to function.
pub trait AnyWeakVar: Any + crate::private::Sealed {
    /// Erase the weak var type.
    fn into_any(self) -> Box<dyn AnyWeakVar>;
    /// Cast to [`Any`].
    fn as_any(&self) -> &dyn Any;
    /// Type erased [`WeakVar::boxed`].
    ///
    /// Returns a value that can be down-casted to `BoxedWeakVar<T>` if the value type is known.
    fn boxed_any(self: Box<Self>) -> Box<dyn AnyWeakVar>;

    /// Type erased [`WeakVar::as_ptr`].
    fn as_ptr_any(&self) -> *const ();

    /// Type erased [`WeakVar::strong_count`].
    fn strong_count_any(&self) -> usize;
    /// Type erased [`WeakVar::weak_count`].
    fn weak_count_any(&self) -> usize;
    /// Type erased [`WeakVar::upgrade`].
    fn upgrade_any(&self) -> Option<Box<dyn AnyVar>>;
}

macro_rules! any_var_impls {
    (Var) => {
        fn into_any(self) -> Box<dyn any::AnyVar> {
            // we can end-up double boxing here, this is needed to allow down-casting to the input var type that may be generic,
            // if we delegate to an "into_any_boxed" for `BoxVar<T>` the down-cast fails because the inner boxed var type becomes the `Any` type.
            Box::new(self)
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn boxed_any(self: Box<Self>) -> Box<dyn any::AnyVar> {
            self.boxed().into_any()
        }

        fn is_new_any(&self, vars: &Vars) -> bool {
            Var::is_new(self, vars)
        }

        fn version_any(&self, vars: &crate::var::VarsRead) -> crate::var::VarVersion {
            Var::version(self, vars)
        }

        fn is_read_only_any(&self, vars: &crate::var::Vars) -> bool {
            Var::is_read_only(self, vars)
        }

        fn always_read_only_any(&self) -> bool {
            Var::always_read_only(self)
        }

        fn is_contextual_any(&self) -> bool {
            Var::is_contextual(self)
        }
        fn is_rc_any(&self) -> bool {
            Var::is_rc(self)
        }

        fn can_update_any(&self) -> bool {
            Var::can_update(self)
        }

        fn is_animating_any(&self, vars: &crate::var::VarsRead) -> bool {
            Var::is_animating(self, vars)
        }

        fn update_mask_any(&self, vars: &crate::var::VarsRead) -> crate::widget_info::UpdateMask {
            Var::update_mask(self, vars)
        }

        fn actual_var_any(&self, vars: &Vars) -> Box<dyn any::AnyVar> {
            any::AnyVar::into_any(Var::actual_var(self, vars))
        }

        fn as_ptr_any(&self) -> *const () {
            Var::as_ptr(self)
        }

        fn strong_count_any(&self) -> usize {
            Var::strong_count(self)
        }

        fn weak_count_any(&self) -> usize {
            Var::weak_count(self)
        }

        fn downgrade_any(&self) -> Option<Box<dyn any::AnyWeakVar>> {
            Var::downgrade(self).map(any::AnyWeakVar::into_any)
        }
    };
    (WeakVar) => {
        fn into_any(self) -> Box<dyn AnyWeakVar> {
            Box::new(self)
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn boxed_any(self: Box<Self>) -> Box<dyn any::AnyWeakVar> {
            self.boxed().into_any()
        }

        fn as_ptr_any(&self) -> *const () {
            WeakVar::as_ptr(self)
        }

        fn strong_count_any(&self) -> usize {
            WeakVar::strong_count(self)
        }

        fn weak_count_any(&self) -> usize {
            WeakVar::weak_count(self)
        }

        fn upgrade_any(&self) -> Option<Box<dyn any::AnyVar>> {
            WeakVar::upgrade(self).map(any::AnyVar::into_any)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downcast_rc() {
        let any_var = var(true).into_any();
        assert!(any_var.as_any().downcast_ref::<RcVar<bool>>().is_some())
    }

    #[test]
    fn downcast_boxed() {
        let any_var = var(true).boxed().into_any();
        assert!(any_var.as_any().downcast_ref::<BoxedVar<bool>>().is_some())
    }

    #[test]
    fn downcast_context_var() {
        context_var! {
            struct FooVar: bool = true;
        }
        let any_var = FooVar::new().into_any();
        assert!(any_var.as_any().downcast_ref::<ContextVarProxy<FooVar>>().is_some());
    }

    #[test]
    fn downcast_any_boxed() {
        let any_var = var(true).into_any().boxed_any();
        assert!(any_var.as_any().downcast_ref::<BoxedVar<bool>>().is_some())
    }
}
