use super::*;

pub struct RcMerge2Var<I0: VarValue, I1: VarValue, O: VarValue, V0: Var<I0>, V1: Var<I1>, F: FnMut(&I0, &I1) -> O + 'static>(
    Rc<RcMerge2VarData<I0, I1, O, V0, V1, F>>,
);

struct RcMerge2VarData<I0: VarValue, I1: VarValue, O: VarValue, V0: Var<I0>, V1: Var<I1>, F: FnMut(&I0, &I1) -> O + 'static> {
    _i: PhantomData<(I0, I1)>,
    vars: (V0, V1),
    f: RefCell<F>,
    versions: (Cell<u32>, Cell<u32>),
    output_version: Cell<u32>,
    output: UnsafeCell<MaybeUninit<O>>, // TODO: Need to manually drop?
    last_updated: Cell<Option<u32>>,
}

impl<I0, I1, O, V0, V1, F> RcMerge2Var<I0, I1, O, V0, V1, F>
where
    I0: VarValue,
    I1: VarValue,
    O: VarValue,
    V0: Var<I0>,
    V1: Var<I1>,
    F: FnMut(&I0, &I1) -> O + 'static,
{
    pub fn new(var: (V0, V1), f: F) -> Self {
        Self(Rc::new(RcMerge2VarData {
            _i: PhantomData,
            vars: var,
            f: RefCell::new(f),
            versions: (Cell::new(0), Cell::new(0)),
            output_version: Cell::new(0),
            output: UnsafeCell::new(MaybeUninit::uninit()),
            last_updated: Cell::new(None),
        }))
    }
}

impl<I0, I1, O, V0, V1, F> Clone for RcMerge2Var<I0, I1, O, V0, V1, F>
where
    I0: VarValue,
    I1: VarValue,
    O: VarValue,
    V0: Var<I0>,
    V1: Var<I1>,
    F: FnMut(&I0, &I1) -> O + 'static,
{
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<I0, I1, O, V0, V1, F> protected::Var for RcMerge2Var<I0, I1, O, V0, V1, F>
where
    I0: VarValue,
    I1: VarValue,
    O: VarValue,
    V0: Var<I0>,
    V1: Var<I1>,
    F: FnMut(&I0, &I1) -> O + 'static,
{
}

impl<I0, I1, O, V0, V1, F> VarObj<O> for RcMerge2Var<I0, I1, O, V0, V1, F>
where
    I0: VarValue,
    I1: VarValue,
    O: VarValue,
    V0: Var<I0>,
    V1: Var<I1>,
    F: FnMut(&I0, &I1) -> O + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        let self_last_updated = self.0.last_updated.get();
        let last_updated = Some(vars.update_id());
        if self_last_updated != last_updated {
            // TODO: This implementation is breaking with [is_new].
            self.0.last_updated.set(last_updated);

            let versions = (self.0.vars.0.version(vars), self.0.vars.1.version(vars));
            if versions.0 != self.0.versions.0.get() || versions.1 != self.0.versions.1.get() || self_last_updated.is_none() {
                let value = (&mut *self.0.f.borrow_mut())(self.0.vars.0.get(vars), self.0.vars.1.get(vars));

                // SAFETY: This is safe because it only happens before the first borrow
                // of this update, and borrows cannot exist across updates because source
                // vars require a &mut Vars for changing version.
                unsafe {
                    let m_uninit = &mut *self.0.output.get();
                    m_uninit.as_mut_ptr().write(value);
                }

                self.0.output_version.set(self.0.output_version.get().wrapping_add(1));
                self.0.versions.0.set(versions.0);
                self.0.versions.1.set(versions.1);
            }
        }

        // SAFETY:
        // This is safe because source require &mut Vars for updating.
        unsafe {
            let inited = &*self.0.output.get();
            &*inited.as_ptr()
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
        self.0.last_updated.get() == Some(vars.update_id())
    }

    fn version(&self, _: &Vars) -> u32 {
        self.0.output_version.get()
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.vars.0.can_update() || self.0.vars.1.can_update()
    }

    fn set(&self, _: &Vars, _: O) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}

impl<I0, I1, O, V0, V1, F> Var<O> for RcMerge2Var<I0, I1, O, V0, V1, F>
where
    I0: VarValue,
    I1: VarValue,
    O: VarValue,
    V0: Var<I0>,
    V1: Var<I1>,
    F: FnMut(&I0, &I1) -> O + 'static,
{
    type AsReadOnly = ForceReadOnlyVar<O, Self>;

    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<F2: FnOnce(&mut O) + 'static>(&self, _: &Vars, _: F2) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn map<O2: VarValue, F2: FnMut(&O) -> O2 + 'static>(&self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        &self,
        map: F2,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F2, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}
