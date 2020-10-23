use super::*;

/// A [`VarLocal`] that keeps a cloned copy of the value locally.
#[doc(hidden)]
#[derive(Clone)]
pub struct CloningLocalVar<T: VarValue, V: Var<T>> {
    var: V,
    local_version: u32,
    local: Option<T>,
}
impl<T: VarValue, V: Var<T>> protected::Var for CloningLocalVar<T, V> {}
impl<T: VarValue, V: Var<T>> CloningLocalVar<T, V> {
    pub(super) fn new(var: V) -> Self {
        CloningLocalVar {
            var,
            local_version: 0,
            local: None,
        }
    }
}
impl<T: VarValue, V: Var<T>> VarObj<T> for CloningLocalVar<T, V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.var.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.var.get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.var.version(vars)
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.var.is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.var.always_read_only()
    }

    fn can_update(&self) -> bool {
        self.var.can_update()
    }

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.var.set(vars, new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        self.var.modify_boxed(vars, change)
    }
}
impl<T: VarValue, V: Var<T>> Var<T> for CloningLocalVar<T, V> {
    type AsReadOnly = ForceReadOnlyVar<T, Self>;
    type AsLocal = Self;

    fn as_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        self
    }

    fn modify<F: FnOnce(&mut T) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        self.var.modify(vars, change)
    }

    fn map<O: VarValue, F: FnMut(&T) -> O>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}
impl<T: VarValue, V: Var<T>> VarLocal<T> for CloningLocalVar<T, V> {
    fn get_local(&self) -> &T {
        self.local.as_ref().expect("local variable not initialized")
    }

    fn init_local(&mut self, vars: &Vars) -> &T {
        self.local_version = self.var.version(vars);
        self.local = Some(self.var.get(vars).clone());
        self.local.as_ref().unwrap()
    }

    fn update_local(&mut self, vars: &Vars) -> Option<&T> {
        let var_version = self.var.version(vars);
        if var_version != self.local_version {
            self.local_version = var_version;
            self.local = Some(self.var.get(vars).clone());
            self.local.as_ref()
        } else {
            None
        }
    }
}

impl<T: VarValue, V: Var<T>> IntoVar<T> for CloningLocalVar<T, V> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
