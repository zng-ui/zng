use std::any::Any;

use super::*;

#[doc(hidden)]
pub trait AnyVar: Any + crate::private::Sealed {
    fn into_any(self) -> Box<dyn AnyVar>;
    fn is_new_any(&self, vars: &Vars) -> bool;
    fn version_any(&self, vars: &VarsRead) -> VarVersion;
    fn is_read_only_any(&self, vars: &Vars) -> bool;
    fn always_read_only_any(&self) -> bool;
    fn is_contextual_any(&self) -> bool;
    fn can_update_any(&self) -> bool;
    fn is_animating_any(&self, vars: &VarsRead) -> bool;
    fn update_mask_any(&self, vars: &VarsRead) -> UpdateMask;
    fn actual_var_any(&self, vars: &Vars) -> Box<dyn AnyVar>;
}

macro_rules! any_var_impls {
    () => {
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
    };
}
