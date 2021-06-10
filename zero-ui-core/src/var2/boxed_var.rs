use super::*;

/// A [`Var`] in a box.
///
/// This type uses dynamic dispatch to access the generic methods of [`Var`], in exchange
/// it can store any type of var.
pub type BoxedVar<T> = Box<dyn VarBoxed<T>>;

/// A [`VarLocal`] in a box.
///
/// This type uses dynamic dispatch to access the generic methods of [`Var`], in exchange
/// it can store any type of var.
pub type BoxedLocalVar<T> = Box<dyn VarLocalBoxed<T>>;

#[doc(hidden)]
pub trait VarBoxed<T: VarValue> {
    fn get_boxed<'a>(&'a self, vars: &'a VarsRead) -> &'a T;
    fn get_new_boxed<'a>(&'a self, vars: &'a Vars) -> Option<&'a T>;
    fn version_boxed<'a>(&'a self, vars: &'a VarsRead) -> u32;
    fn is_read_only_boxed(&self, vars: &VarsRead) -> bool;
    fn always_read_only_boxed(&self) -> bool;
    fn can_update_boxed(&self) -> bool;
    fn modify_boxed(&self, vars: &Vars, modify: Box<dyn FnOnce(&mut VarModify<T>)>) -> Result<(), VarIsReadOnly>;
    fn set_boxed(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>;
    fn into_read_only_boxed(self) -> BoxedVar<T>;
    fn into_local_boxed(self) -> BoxedLocalVar<T>;
    fn clone_boxed(&self) -> BoxedVar<T>;
}
impl<T: VarValue, V: Var<T>> VarBoxed<T> for V {
    fn get_boxed<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.get(vars)
    }

    fn get_new_boxed<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.get_new(vars)
    }

    fn version_boxed<'a>(&'a self, vars: &'a VarsRead) -> u32 {
        self.version(vars)
    }

    fn is_read_only_boxed(&self, vars: &VarsRead) -> bool {
        self.is_read_only(vars)
    }

    fn always_read_only_boxed(&self) -> bool {
        self.always_read_only()
    }

    fn can_update_boxed(&self) -> bool {
        self.can_update()
    }

    fn modify_boxed(&self, vars: &Vars, modify: Box<dyn FnOnce(&mut VarModify<T>)>) -> Result<(), VarIsReadOnly> {
        self.modify(vars, modify)
    }

    fn set_boxed(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.set(vars, new_value)
    }

    fn into_read_only_boxed(self) -> BoxedVar<T> {
        self.into_read_only().boxed()
    }

    fn into_local_boxed(self) -> BoxedLocalVar<T> {
        Box::new(Var::into_local(self))
    }

    fn clone_boxed(&self) -> BoxedVar<T> {
        self.clone().boxed()
    }
}
impl<T: VarValue> Clone for BoxedVar<T> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}
impl<T: VarValue> Var<T> for BoxedVar<T> {
    type AsReadOnly = BoxedVar<T>;
    type AsLocal = BoxedLocalVar<T>;

    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.get_boxed(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.get_new_boxed(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.version_boxed(vars)
    }

    fn is_read_only(&self, vars: &VarsRead) -> bool {
        self.is_read_only_boxed(vars)
    }

    fn always_read_only(&self) -> bool {
        self.always_read_only_boxed()
    }

    fn can_update(&self) -> bool {
        self.can_update_boxed()
    }

    fn modify<M>(&self, vars: &Vars, modify: M) -> Result<(), VarIsReadOnly>
    where
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        self.modify_boxed(vars, Box::new(modify))
    }

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.set_boxed(vars, new_value)
    }

    fn set_ne(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>
    where
        T: PartialEq,
    {
        self.modify(vars, move |v| {
            if v.eq(&new_value) {
                **v = new_value;
            }
        })
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        if self.always_read_only() {
            self
        } else {
            self.into_read_only_boxed()
        }
    }

    fn into_local(self) -> Self::AsLocal {
        self.into_local_boxed()
    }
}
impl<T: VarValue> IntoVar<T> for BoxedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
#[doc(hidden)]
pub trait VarLocalBoxed<T: VarValue>: VarBoxed<T> {
    fn get_local_boxed(&self) -> &T;
    fn init_local_boxed<'a>(&'a mut self, vars: &'a Vars) -> &'a T;
    fn update_local_boxed<'a>(&'a mut self, vars: &'a Vars) -> Option<&'a T>;
    fn clone_local_boxed(&self) -> BoxedLocalVar<T>;
}
impl<T: VarValue, V: VarLocal<T>> VarLocalBoxed<T> for V {
    fn get_local_boxed(&self) -> &T {
        self.get_local()
    }

    fn init_local_boxed<'a>(&'a mut self, vars: &'a Vars) -> &'a T {
        self.init_local(vars)
    }

    fn update_local_boxed<'a>(&'a mut self, vars: &'a Vars) -> Option<&'a T> {
        self.update_local(vars)
    }

    fn clone_local_boxed(&self) -> BoxedLocalVar<T> {
        Box::new(self.clone())
    }
}

impl<T: VarValue> Clone for BoxedLocalVar<T> {
    fn clone(&self) -> Self {
        self.clone_local_boxed()
    }
}
impl<T: VarValue> Var<T> for BoxedLocalVar<T> {
    type AsReadOnly = BoxedVar<T>;
    type AsLocal = Self;

    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.get_boxed(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.get_new_boxed(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.version_boxed(vars)
    }

    fn is_read_only(&self, vars: &VarsRead) -> bool {
        self.is_read_only_boxed(vars)
    }

    fn always_read_only(&self) -> bool {
        self.always_read_only_boxed()
    }

    fn can_update(&self) -> bool {
        self.can_update_boxed()
    }

    fn modify<M>(&self, vars: &Vars, modify: M) -> Result<(), VarIsReadOnly>
    where
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        self.modify_boxed(vars, Box::new(modify))
    }

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.set_boxed(vars, new_value)
    }

    fn set_ne(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>
    where
        T: PartialEq,
    {
        self.modify(vars, move |v| {
            if v.eq(&new_value) {
                **v = new_value;
            }
        })
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self.into_read_only_boxed()
    }

    fn into_local(self) -> Self::AsLocal {
        self
    }
}
impl<T: VarValue> IntoVar<T> for BoxedLocalVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
impl<T: VarValue> VarLocal<T> for BoxedLocalVar<T> {
    fn get_local(&self) -> &T {
        self.as_ref().get_local_boxed()
    }

    fn init_local<'a>(&'a mut self, vars: &'a Vars) -> &'a T {
        self.init_local_boxed(vars)
    }

    fn update_local<'a>(&'a mut self, vars: &'a Vars) -> Option<&'a T> {
        self.update_local_boxed(vars)
    }
}
