use super::{protected, LocalVar, ObjVar, Var, VarIsReadOnly, VarValue};
use crate::core::context::{Updates, Vars};

/// Variable that keeps a local clone of the current value.
pub struct CloningLocalVar<T: VarValue, V: Var<T>> {
    var: V,
    local: Option<T>,
    local_step: Option<T>,
}

impl<T: VarValue, V: Var<T>> CloningLocalVar<T, V> {
    pub(crate) fn new(var: V) -> Self {
        CloningLocalVar {
            var,
            local: None,
            local_step: None,
        }
    }
}

impl<T: VarValue, V: Var<T>> protected::Var<T> for CloningLocalVar<T, V> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        self.var.bind_info(vars)
    }

    fn is_context_var(&self) -> bool {
        self.var.is_context_var()
    }

    fn read_only_prev_version(&self) -> u32 {
        self.var.read_only_prev_version()
    }
}

impl<T: VarValue, V: Var<T>> ObjVar<T> for CloningLocalVar<T, V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.var.get(vars)
    }

    fn get_step<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.var.get_step(vars)
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.var.update(vars)
    }

    fn update_step<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.var.update_step(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    fn is_animating(&self, vars: &Vars) -> bool {
        self.var.is_animating(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.var.version(vars)
    }

    fn can_update(&self) -> bool {
        self.var.can_update()
    }

    fn read_only(&self, vars: &Vars) -> bool {
        self.var.read_only(vars)
    }

    fn always_read_only(&self, vars: &Vars) -> bool {
        self.var.always_read_only(vars)
    }

    fn push_set(&self, new_value: T, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.var.push_set(new_value, vars, updates)
    }

    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut T) + 'static>,
        vars: &Vars,
        updates: &mut Updates,
    ) -> Result<(), VarIsReadOnly> {
        self.var.push_modify_boxed(modify, vars, updates)
    }
}

impl<T: VarValue, V: Var<T>> LocalVar<T> for CloningLocalVar<T, V> {
    fn get_local(&self) -> &T {
        self.local.as_ref().expect("``init_local` must be called before using `LocalVar`")
    }

    fn get_local_step(&self) -> &T {
        self.local_step.as_ref().unwrap_or_else(|| self.get_local())
    }

    fn init_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> &'a T {
        self.local = Some(self.var.get(vars).clone());
        if self.var.is_animating(vars) {
            self.local_step = Some(self.var.get_step(vars).clone());
        }
        self.get_local()
    }

    fn update_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> Option<&'a T> {
        match self.var.update(vars) {
            Some(update) => {
                self.local = Some(update.clone());
                self.local.as_ref()
            }
            None => None,
        }
    }

    fn update_local_step<'a, 'b>(&'a mut self, vars: &'b Vars) -> Option<&'a T> {
        match self.var.update_step(vars) {
            Some(update) => {
                self.local_step = Some(update.clone());
                self.local_step.as_ref()
            }
            None => None,
        }
    }
}
