use super::*;

#[doc(hidden)]
pub struct RcSwitch2Var<O: VarValue, V0: Var<O>, V1: Var<O>, VI: Var<usize>>(Rc<RcSwitch2VarData<O, V0, V1, VI>>);
struct RcSwitch2VarData<O: VarValue, V0: Var<O>, V1: Var<O>, VI: Var<usize>> {
    _o: PhantomData<O>,
    vars: (V0, V1),
    var_versions: (Cell<u32>, Cell<u32>),
    index: VI,
    index_version: Cell<u32>,
    self_version: Cell<u32>,
}

impl<O, V0, V1, VI> RcSwitch2Var<O, V0, V1, VI>
where
    O: VarValue,
    V0: Var<O>,
    V1: Var<O>,
    VI: Var<usize>,
{
    pub fn new(v0: V0, v1: V1, index: VI) -> Self {
        Self(Rc::new(RcSwitch2VarData {
            _o: PhantomData,
            vars: (v0, v1),
            var_versions: (Cell::new(0), Cell::new(0)),
            index,
            index_version: Cell::new(0),
            self_version: Cell::new(0),
        }))
    }
}

impl<O, V0, V1, VI> Clone for RcSwitch2Var<O, V0, V1, VI>
where
    O: VarValue,
    V0: Var<O>,
    V1: Var<O>,
    VI: Var<usize>,
{
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<O, V0, V1, VI> protected::Var for RcSwitch2Var<O, V0, V1, VI>
where
    O: VarValue,
    V0: Var<O>,
    V1: Var<O>,
    VI: Var<usize>,
{
}

impl<O, V0, V1, VI> VarObj<O> for RcSwitch2Var<O, V0, V1, VI>
where
    O: VarValue,
    V0: Var<O>,
    V1: Var<O>,
    VI: Var<usize>,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        match *self.0.index.get(vars) {
            0 => self.0.vars.0.get(vars),
            1 => self.0.vars.1.get(vars),
            _ => panic!("switch_var index out of range"),
        }
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        if self.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.index.is_new(vars)
            || match *self.0.index.get(vars) {
                0 => self.0.vars.0.is_new(vars),
                1 => self.0.vars.1.is_new(vars),
                _ => panic!("switch_var index out of range"),
            }
    }

    fn version(&self, vars: &Vars) -> u32 {
        let i_ver = self.0.index.version(vars);
        let var_vers = (self.0.vars.0.version(vars), self.0.vars.1.version(vars));

        if i_ver != self.0.index_version.get() || var_vers.0 != self.0.var_versions.0.get() || var_vers.1 != self.0.var_versions.1.get() {
            self.0.self_version.set(self.0.self_version.get().wrapping_add(1));
            self.0.index_version.set(i_ver);
            self.0.var_versions.0.set(var_vers.0);
            self.0.var_versions.1.set(var_vers.1);
        }

        self.0.self_version.get()
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        match *self.0.index.get(vars) {
            0 => self.0.vars.0.is_read_only(vars),
            1 => self.0.vars.1.is_read_only(vars),
            _ => panic!("switch_var index out of range"),
        }
    }

    fn always_read_only(&self) -> bool {
        self.0.vars.0.always_read_only() && self.0.vars.1.always_read_only()
    }

    fn can_update(&self) -> bool {
        // you could make one that doesn't but we don't care.
        true
    }

    fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
        match *self.0.index.get(vars) {
            0 => self.0.vars.0.set(vars, new_value),
            1 => self.0.vars.1.set(vars, new_value),
            _ => panic!("switch_var index out of range"),
        }
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        match *self.0.index.get(vars) {
            0 => self.0.vars.0.modify_boxed(vars, change),
            1 => self.0.vars.1.modify_boxed(vars, change),
            _ => panic!("switch_var index out of range"),
        }
    }
}

impl<O, V0, V1, VI> Var<O> for RcSwitch2Var<O, V0, V1, VI>
where
    O: VarValue,
    V0: Var<O>,
    V1: Var<O>,
    VI: Var<usize>,
{
    type AsReadOnly = ForceReadOnlyVar<O, Self>;
    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<F: FnOnce(&mut O) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        match *self.0.index.get(vars) {
            0 => self.0.vars.0.modify(vars, change),
            1 => self.0.vars.1.modify(vars, change),
            _ => panic!("switch_var index out of range"),
        }
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn map<O2: VarValue, F: FnMut(&O) -> O2 + 'static>(&self, map: F) -> RcMapVar<O, O2, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O2: VarValue, F: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}
