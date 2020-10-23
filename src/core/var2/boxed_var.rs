use super::*;

/// A boxed [`VarObj`].
pub type BoxedVar<T> = Box<dyn VarObj<T>>;

impl<T: VarValue> protected::Var for BoxedVar<T> {}

impl<T: VarValue> VarObj<T> for BoxedVar<T> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.as_ref().get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.as_ref().get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.as_ref().is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.as_ref().version(vars)
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.as_ref().is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.as_ref().always_read_only()
    }

    fn can_update(&self) -> bool {
        self.as_ref().can_update()
    }

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.as_ref().set(vars, new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        self.as_ref().modify_boxed(vars, change)
    }

    fn boxed(self) -> Box<dyn VarObj<T>>
    where
        Self: Sized,
    {
        self
    }
}

/// A boxes [`VarLocal`].
pub type BoxedLocalVar<T> = Box<dyn VarLocal<T>>;

impl<T: VarValue> protected::Var for BoxedLocalVar<T> {}

impl<T: VarValue> VarObj<T> for BoxedLocalVar<T> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.as_ref().get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.as_ref().get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.as_ref().is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.as_ref().version(vars)
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.as_ref().is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.as_ref().always_read_only()
    }

    fn can_update(&self) -> bool {
        self.as_ref().can_update()
    }

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.as_ref().set(vars, new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        self.as_ref().modify_boxed(vars, change)
    }
}

impl<T: VarValue> VarLocal<T> for BoxedLocalVar<T> {
    fn get_local(&self) -> &T {
        self.as_ref().get_local()
    }

    fn init_local(&mut self, vars: &Vars) -> &T {
        self.as_mut().init_local(vars)
    }

    fn update_local(&mut self, vars: &Vars) -> Option<&T> {
        self.as_mut().update_local(vars)
    }

    fn boxed_local(self) -> Box<dyn VarLocal<T>>
    where
        Self: Sized,
    {
        self
    }
}
