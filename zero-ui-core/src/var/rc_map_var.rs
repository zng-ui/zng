use super::*;

/// A reference counted mapping variable.
///
/// The variable is read-only, the value is generated from another value and updates with
/// the other variable.
#[doc(hidden)]
pub struct RcMapVar<I: VarValue, O: VarValue, V: Var<I>, F: FnMut(&I) -> O + 'static>(Rc<RcMapVarData<I, O, V, F>>);
struct RcMapVarData<I: VarValue, O: VarValue, V: Var<I>, F: FnMut(&I) -> O + 'static> {
    _i: PhantomData<I>,
    var: V,
    f: RefCell<F>,
    version: Cell<Option<u32>>,
    output: UnsafeCell<MaybeUninit<O>>,
}
impl<I, O, V, F> protected::Var for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O,
{
}
impl<I, O, V, F> Clone for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O,
{
    fn clone(&self) -> Self {
        RcMapVar(Rc::clone(&self.0))
    }
}
impl<I, O, V, F> RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
{
    pub(super) fn new(var: V, f: F) -> Self {
        RcMapVar(Rc::new(RcMapVarData {
            _i: PhantomData,
            var,
            f: RefCell::new(f),
            version: Cell::new(None),
            output: UnsafeCell::new(MaybeUninit::uninit()),
        }))
    }

    /// References the current value.
    pub fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        <Self as VarObj<O>>::get(self, vars)
    }

    /// References the current value if it is new.
    pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        <Self as VarObj<O>>::get_new(self, vars)
    }

    /// If [`set`](Self::set) or [`modify`](Var::modify) was called in the previous update.
    pub fn is_new(&self, vars: &Vars) -> bool {
        <Self as VarObj<O>>::is_new(self, vars)
    }

    /// Version of the current value.
    ///
    /// The version is incremented every update
    /// that [`set`](Self::set) or [`modify`](Var::modify) are called.
    pub fn version(&self, vars: &Vars) -> u32 {
        <Self as VarObj<O>>::version(self, vars)
    }

    /// If the source variable can update.
    pub fn can_update(&self) -> bool {
        <Self as VarObj<O>>::can_update(self)
    }
}
impl<I, O, V, F> VarObj<O> for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
{
    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a O {
        let var_version = Some(self.0.var.version(vars));
        if var_version != self.0.version.get() {
            let value = (&mut *self.0.f.borrow_mut())(self.0.var.get(vars));
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

    fn version(&self, vars: &VarsRead) -> u32 {
        self.0.var.version(vars)
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.var.can_update()
    }

    fn set(&self, _: &Vars, _: O) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn set_ne(&self, _: &Vars, _: O) -> Result<bool, VarIsReadOnly>
    where
        O: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<I, O, V, F> Var<O> for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
{
    type AsReadOnly = Self;

    type AsLocal = CloningLocalVar<O, Self>;

    fn into_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn modify<G>(&self, _: &Vars, _: G) -> Result<(), VarIsReadOnly>
    where
        G: FnOnce(&mut O) + 'static,
    {
        Err(VarIsReadOnly)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn map<O2: VarValue, F2: FnMut(&O) -> O2 + 'static>(&self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        self.clone().into_map(map)
    }

    fn map_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static>(&self, map: F2) -> MapRefVar<O, O2, Self, F2> {
        self.clone().into_map_ref(map)
    }

    fn map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        &self,
        map: F2,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F2, G> {
        self.clone().into_map_bidi(map, map_back)
    }

    fn into_map<O2: VarValue, F2: FnMut(&O) -> O2 + 'static>(self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        RcMapVar::new(self, map)
    }

    fn into_map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        self,
        map: F2,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F2, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static>(self, map: F2) -> MapRefVar<O, O2, Self, F2> {
        MapRefVar::new(self, map)
    }

    fn map_bidi_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static, G2: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        &self,
        map: F2,
        map_mut: G2,
    ) -> MapBidiRefVar<O, O2, Self, F2, G2> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map_bidi_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static, G2: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        self,
        map: F2,
        map_mut: G2,
    ) -> MapBidiRefVar<O, O2, Self, F2, G2> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

impl<I, O, V, F> IntoVar<O> for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
