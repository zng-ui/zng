use super::{protected, LocalVar, ObjVar, VarIsReadOnly, VarValue};
use crate::core::context::{Updates, Vars};

/// Boxed [`ObjVar`].
pub type BoxVar<T> = Box<dyn ObjVar<T>>;

impl<T: VarValue> protected::Var<T> for BoxVar<T> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        self.as_ref().bind_info(vars)
    }
    fn is_context_var(&self) -> bool {
        self.as_ref().is_context_var()
    }
    fn read_only_prev_version(&self) -> u32 {
        self.as_ref().read_only_prev_version()
    }
}

impl<T: VarValue> ObjVar<T> for BoxVar<T> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.as_ref().get(vars)
    }
    fn get_step<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.as_ref().get_step(vars)
    }
    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.as_ref().update(vars)
    }
    fn update_step<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.as_ref().update_step(vars)
    }
    fn is_new(&self, vars: &Vars) -> bool {
        self.as_ref().is_new(vars)
    }
    fn is_animating(&self, vars: &Vars) -> bool {
        self.as_ref().is_animating(vars)
    }
    fn version(&self, vars: &Vars) -> u32 {
        self.as_ref().version(vars)
    }
    fn read_only(&self, vars: &Vars) -> bool {
        self.as_ref().read_only(vars)
    }
    fn always_read_only(&self, vars: &Vars) -> bool {
        self.as_ref().always_read_only(vars)
    }
    fn push_set(&self, new_value: T, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.as_ref().push_set(new_value, vars, updates)
    }
    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut T) + 'static>,
        vars: &Vars,
        updates: &mut Updates,
    ) -> Result<(), VarIsReadOnly> {
        self.as_ref().push_modify_boxed(modify, vars, updates)
    }
    fn boxed(self) -> BoxVar<T>
    where
        Self: Sized,
    {
        self
    }
}

/// Boxed [`LocalVar`].
pub type BoxLocalVar<T> = Box<dyn LocalVar<T>>;

impl<T: VarValue> protected::Var<T> for BoxLocalVar<T> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        self.as_ref().bind_info(vars)
    }
    fn is_context_var(&self) -> bool {
        self.as_ref().is_context_var()
    }
    fn read_only_prev_version(&self) -> u32 {
        self.as_ref().read_only_prev_version()
    }
}

impl<T: VarValue> ObjVar<T> for BoxLocalVar<T> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.as_ref().get(vars)
    }
    fn get_step<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.as_ref().get_step(vars)
    }
    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.as_ref().update(vars)
    }
    fn update_step<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.as_ref().update_step(vars)
    }
    fn is_new(&self, vars: &Vars) -> bool {
        self.as_ref().is_new(vars)
    }
    fn is_animating(&self, vars: &Vars) -> bool {
        self.as_ref().is_animating(vars)
    }
    fn version(&self, vars: &Vars) -> u32 {
        self.as_ref().version(vars)
    }
    fn read_only(&self, vars: &Vars) -> bool {
        self.as_ref().read_only(vars)
    }
    fn always_read_only(&self, vars: &Vars) -> bool {
        self.as_ref().always_read_only(vars)
    }
    fn push_set(&self, new_value: T, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.as_ref().push_set(new_value, vars, updates)
    }
    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut T) + 'static>,
        vars: &Vars,
        updates: &mut Updates,
    ) -> Result<(), VarIsReadOnly> {
        self.as_ref().push_modify_boxed(modify, vars, updates)
    }
}

impl<T: VarValue> LocalVar<T> for BoxLocalVar<T> {
    fn get_local(&self) -> &T {
        self.as_ref().get_local()
    }
    fn get_local_step(&self) -> &T {
        self.as_ref().get_local_step()
    }
    fn init_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> &'a T {
        self.as_mut().init_local(vars)
    }
    fn update_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> Option<&'a T> {
        self.as_mut().update_local(vars)
    }
    fn update_local_step<'a, 'b>(&'a mut self, vars: &'b Vars) -> Option<&'a T> {
        self.as_mut().update_local_step(vars)
    }
}
