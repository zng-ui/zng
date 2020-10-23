use super::*;

/// A reference counted bidirectional mapping variable.
#[doc(hidden)]
pub struct RcMapBidiVar<I: VarValue, O: VarValue, V: Var<I>, F: FnMut(&I) -> O, G: FnMut(O) -> I>(Rc<RcMapBidiVarData<I, O, V, F, G>>);
struct RcMapBidiVarData<I: VarValue, O: VarValue, V: Var<I>, F: FnMut(&I) -> O, G: FnMut(O) -> I> {
    _i: PhantomData<I>,
    var: V,
    map: RefCell<F>,
    map_back: RefCell<G>,
    version: Cell<Option<u32>>,
    output: UnsafeCell<MaybeUninit<O>>,
}
impl<I, O, V, F, G> protected::Var for RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O,
    G: FnMut(O) -> I,
{
}
impl<I, O, V, F, G> Clone for RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O,
    G: FnMut(O) -> I,
{
    fn clone(&self) -> Self {
        RcMapBidiVar(Rc::clone(&self.0))
    }
}
impl<I, O, V, F, G> RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
    G: FnMut(O) -> I + 'static,
{
    pub(super) fn new(var: V, map: F, map_back: G) -> Self {
        RcMapBidiVar(Rc::new(RcMapBidiVarData {
            _i: PhantomData,
            var,
            map: RefCell::new(map),
            map_back: RefCell::new(map_back),
            version: Cell::new(None),
            output: UnsafeCell::new(MaybeUninit::uninit()),
        }))
    }
}
impl<I, O, V, F, G> VarObj<O> for RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
    G: FnMut(O) -> I + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        let var_version = Some(self.0.var.version(vars));
        if var_version != self.0.version.get() {
            let value = (&mut *self.0.map.borrow_mut())(self.0.var.get(vars));
            // SAFETY: This is safe because it only happens before the first borrow
            // of this update, and borrows cannot exist across updates because source
            // vars require a &mut Vars for changing version.
            unsafe {
                let m_uninit = &mut *self.0.output.get();
                m_uninit.as_mut_ptr().write(value);
            }
            self.0.version.set(var_version);
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
        self.0.var.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.0.var.version(vars)
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.0.var.is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.0.var.always_read_only()
    }

    fn can_update(&self) -> bool {
        self.0.var.can_update()
    }

    fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
        let new_value = (&mut *self.0.map_back.borrow_mut())(new_value);
        self.0.var.set(vars, new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        let mut new_value = self.get(vars).clone();
        change(&mut new_value);
        self.set(vars, new_value)
    }
}
impl<I, O, V, F, G> Var<O> for RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
    G: FnMut(O) -> I + 'static,
{
    type AsReadOnly = ForceReadOnlyVar<O, Self>;
    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<H>(&self, vars: &Vars, change: H) -> Result<(), VarIsReadOnly>
    where
        H: FnOnce(&mut O) + 'static,
    {
        let mut new_value = self.get(vars).clone();
        change(&mut new_value);
        self.set(vars, new_value)
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn map<O2: VarValue, F2: FnMut(&O) -> O2>(&self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G2: FnMut(O2) -> O + 'static>(
        &self,
        map: F2,
        map_back: G2,
    ) -> RcMapBidiVar<O, O2, Self, F2, G2> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}
