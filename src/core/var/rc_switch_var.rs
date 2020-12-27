use super::*;

/// Initializes a new switch var.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `$index`: A positive integer that is the initial switch index.
/// * `$v0..$vn`: A list of [vars](crate::core::var::VarObj), minimal 2.
///
/// [`RcSwitchVar`](crate::core::var::RcSwitchVar) is used for more then 8 variables.
///
/// All arguments are [`IntoVar`](crate::core::var::RcSwitchVar).
///
/// # Example
/// ```
/// # use zero_ui::core::var::switch_var;
/// # use zero_ui::prelude::{var, text, ToText};
/// let index = var(0);
/// let var0 = var("Read-write".to_text());
/// let var1 = "Read-only";
///
/// let t = text(switch_var!(index, var0, var1));
/// ```
#[macro_export]
macro_rules! switch_var {
    ($index: expr, $v0: expr, $v1: expr) => {
        $crate::core::var::RcSwitch2Var::new($index, ($v0, $v1))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr) => {
        $crate::core::var::RcSwitch3Var::new($index, ($v0, $v1, $v2))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr) => {
        $crate::core::var::RcSwitch4Var::new($index, ($v0, $v1, $v2, $v3))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr) => {
        $crate::core::var::RcSwitch5Var::new($index, ($v0, $v1, $v2, $v3, $v4))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr) => {
        $crate::core::var::RcSwitch6Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr) => {
        $crate::core::var::RcSwitch7Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5, $v6))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr) => {
        $crate::core::var::RcSwitch8Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7))
    };
    ($index: expr, $($v:expr),+) => {
        // we need a builder to have $v be IntoVar and work like the others.
        $crate::core::var::RcSwitchVarBuilder::new($index)
        $(.push($v))+
        .build()
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (initial_index, var0, var1, ..)")
    };
}
#[doc(inline)]
pub use crate::switch_var;

macro_rules! impl_rc_switch_var {
    ($(
        $len:tt => $($n:tt),+;
    )+) => {$(
        paste::paste!{
            impl_rc_switch_var!{
                Var: [<RcSwitch $len Var>];// RcSwitch2Var
                Data: [<RcSwitch $len VarData>];// RcSwitch2VarData
                len: $len;//2
                V: $([<V $n>]),+;// V0, V1
                IV: $([<IV $n>]),+;// IV0, IV1
                n: $($n),+; // 0, 1
            }
        }
    )+};

    (
        Var: $RcSwitchVar:ident;
        Data: $RcSwitchVarData:ident;
        len: $len:tt;
        V: $($V:ident),+;
        IV: $($IV:ident),+;
        n: $($n:tt),+;
    ) => {
        #[doc(hidden)]
        pub struct $RcSwitchVar<O: VarValue, $($V: VarObj<O>,)+ VI: VarObj<usize>>(Rc<$RcSwitchVarData<O, $($V,)+ VI>>);
        struct $RcSwitchVarData<O: VarValue, $($V: VarObj<O>,)+ VI: VarObj<usize>> {
            _o: PhantomData<O>,
            vars: ($($V),+),
            versions: [Cell<u32>; $len],
            index: VI,
            index_version: Cell<u32>,
            self_version: Cell<u32>,
        }

        impl<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>> $RcSwitchVar<O, $($V,)+ VI> {
            pub fn new<$($IV: IntoVar<O, Var=$V>),+>(index: VI, vars: ($($IV),+)) -> Self {
                Self::from_vars(index, ($(vars.$n.into_var()),+))
            }
        }

        impl<O: VarValue, $($V: VarObj<O>,)+ VI: VarObj<usize>> $RcSwitchVar<O, $($V,)+ VI> {
            pub fn from_vars(index: VI, vars: ($($V),+)) -> Self {
                Self(Rc::new($RcSwitchVarData {
                    _o: PhantomData,
                    vars,
                    versions: array_init::array_init(|_|Cell::new(0)),
                    index,
                    index_version: Cell::new(0),
                    self_version: Cell::new(0),
                }))
            }
        }

        impl<O: VarValue, $($V: VarObj<O>,)+ VI: VarObj<usize>>
        Clone for $RcSwitchVar<O, $($V,)+ VI> {
            fn clone(&self) -> Self {
                Self(Rc::clone(&self.0))
            }
        }

        impl<O: VarValue, $($V: VarObj<O>,)+ VI: VarObj<usize>>
        protected::Var for $RcSwitchVar<O, $($V,)+ VI> { }

        impl<O: VarValue, $($V: VarObj<O>,)+ VI: VarObj<usize>>
        VarObj<O> for $RcSwitchVar<O, $($V,)+ VI> {
            fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
                match *self.0.index.get(vars) {
                    $($n => self.0.vars.$n.get(vars),)+
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
                        $($n => self.0.vars.$n.is_new(vars),)+
                        _ => panic!("switch_var index out of range"),
                    }
            }

            fn version(&self, vars: &Vars) -> u32 {
                let i_ver = self.0.index.version(vars);
                let var_vers = ($(self.0.vars.$n.version(vars)),+);

                if i_ver != self.0.index_version.get() || $(var_vers.$n != self.0.versions[$n].get())||+ {
                    self.0.self_version.set(self.0.self_version.get().wrapping_add(1));
                    self.0.index_version.set(i_ver);
                    $(self.0.versions[$n].set(var_vers.$n);)+
                }

                self.0.self_version.get()
            }

            fn is_read_only(&self, vars: &Vars) -> bool {
                match *self.0.index.get(vars) {
                    $($n => self.0.vars.$n.is_read_only(vars),)+
                    _ => panic!("switch_var index out of range"),
                }
            }

            fn always_read_only(&self) -> bool {
                $(self.0.vars.$n.always_read_only())&&+
            }

            fn can_update(&self) -> bool {
                // you could make one that doesn't but we don't care.
                true
            }

            fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
                match *self.0.index.get(vars) {
                    $($n => self.0.vars.$n.set(vars, new_value),)+
                    _ => panic!("switch_var index out of range"),
                }
            }

            fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
                match *self.0.index.get(vars) {
                    $($n => self.0.vars.$n.modify_boxed(vars, change),)+
                    _ => panic!("switch_var index out of range"),
                }
            }
        }

        impl<O: VarValue, $($V: Var<O>,)+ VI: VarObj<usize>>
        Var<O> for $RcSwitchVar<O, $($V,)+ VI> {
            type AsReadOnly = ForceReadOnlyVar<O, Self>;
            type AsLocal = CloningLocalVar<O, Self>;

            fn modify<F: FnOnce(&mut O) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
                match *self.0.index.get(vars) {
                    $($n => self.0.vars.$n.modify(vars, change),)+
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
                self.clone().into_map(map)
            }

            fn map_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static>(&self, map: F) -> MapRefVar<O, O2, Self, F> {
                self.clone().into_map_ref(map)
            }

            fn map_bidi<O2: VarValue, F: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
                &self,
                map: F,
                map_back: G,
            ) -> RcMapBidiVar<O, O2, Self, F, G> {
                self.clone().into_map_bidi(map, map_back)
            }

            fn into_map<O2: VarValue, F: FnMut(&O) -> O2 + 'static>(self, map: F) -> RcMapVar<O, O2, Self, F> {
                RcMapVar::new(self, map)
            }

            fn into_map_bidi<O2: VarValue, F: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
                self,
                map: F,
                map_back: G,
            ) -> RcMapBidiVar<O, O2, Self, F, G> {
                RcMapBidiVar::new(self, map, map_back)
            }

            fn into_map_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static>(self, map: F) -> MapRefVar<O, O2, Self, F> {
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

        impl<O: VarValue, $($V: Var<O>,)+ VI: VarObj<usize>>
        IntoVar<O> for $RcSwitchVar<O, $($V,)+ VI> {
            type Var = Self;
            fn into_var(self) -> Self {
                self
            }
        }
    };
}

impl_rc_switch_var! {
    2 => 0, 1;
    3 => 0, 1, 2;
    4 => 0, 1, 2, 3;
    5 => 0, 1, 2, 3, 4;
    6 => 0, 1, 2, 3, 4, 5;
    7 => 0, 1, 2, 3, 4, 5, 6;
    8 => 0, 1, 2, 3, 4, 5, 6, 7;
}

/// A [`switch_var!`] that uses dynamic dispatch to support any number of variables.
///
/// This type is a reference-counted pointer ([`Rc`]),
/// it implements the full [`Var`] read and write methods.
///
/// Don't use this type directly use the [macro](switch_var!) instead.
pub struct RcSwitchVar<O: VarValue, VI: VarObj<usize>>(Rc<RcSwitchVarData<O, VI>>);
struct RcSwitchVarData<O: VarValue, VI: VarObj<usize>> {
    vars: Box<[BoxedVar<O>]>,
    var_versions: Box<[Cell<u32>]>,

    index: VI,
    index_version: Cell<u32>,

    self_version: Cell<u32>,
}
impl<O: VarValue, VI: VarObj<usize>> RcSwitchVar<O, VI> {
    pub fn from_vars(index: VI, vars: Box<[BoxedVar<O>]>) -> Self {
        assert!(vars.len() >= 2);
        Self(Rc::new(RcSwitchVarData {
            var_versions: vars.iter().map(|_| Cell::new(0)).collect(),
            vars,
            index,
            index_version: Cell::new(0),
            self_version: Cell::new(0),
        }))
    }

    /// Gets the indexed variable value.
    pub fn get<'a>(&'a self, vars: &'a Vars) -> &O {
        <Self as VarObj<O>>::get(self, vars)
    }

    /// Gets if the index is new or the indexed variable value is new.
    pub fn is_new(&self, vars: &Vars) -> bool {
        <Self as VarObj<O>>::is_new(self, vars)
    }

    /// Gets the version.
    ///
    /// The version is new when the index variable changes
    /// or when the indexed variable changes.
    pub fn version(&self, vars: &Vars) -> u32 {
        <Self as VarObj<O>>::version(self, vars)
    }

    /// Gets if the indexed variable is read-only.
    pub fn is_read_only(&self, vars: &Vars) -> bool {
        <Self as VarObj<O>>::is_read_only(self, vars)
    }

    /// Gets if all alternate variables are always read-only.
    pub fn always_read_only(&self) -> bool {
        <Self as VarObj<O>>::always_read_only(self)
    }

    /// Tries to set the indexed variable.
    pub fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
        <Self as VarObj<O>>::set(self, vars, new_value)
    }

    /// Tries to set the indexed variable.
    pub fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        <Self as VarObj<O>>::modify_boxed(self, vars, change)
    }

    /// Calls [`modify_boxed`](Self::modify_boxed).
    ///
    /// This is because the alternate variables are boxed.
    pub fn modify<F: FnOnce(&mut O) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        <Self as Var<O>>::modify(self, vars, change)
    }
}
impl<O: VarValue, VI: VarObj<usize>> protected::Var for RcSwitchVar<O, VI> {}
impl<O: VarValue, VI: VarObj<usize>> Clone for RcSwitchVar<O, VI> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<O: VarValue, VI: VarObj<usize>> VarObj<O> for RcSwitchVar<O, VI> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        self.0.vars[*self.0.index.get(vars)].get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        if self.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.vars[*self.0.index.get(vars)].is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        let mut changed = false;

        let i_ver = self.0.index.version(vars);
        if i_ver != self.0.index_version.get() {
            self.0.index_version.set(i_ver);
            changed = true;
        }

        let i = *self.0.index.get(vars);
        let v_ver = self.0.vars[i].version(vars);
        if v_ver != self.0.var_versions[i].get() {
            self.0.var_versions[i].set(v_ver);
            changed = true;
        }

        if changed {
            self.0.self_version.set(self.0.self_version.get().wrapping_add(1));
        }

        self.0.self_version.get()
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.0.vars[*self.0.index.get(vars)].is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.0.vars.iter().all(|v| v.always_read_only())
    }

    fn can_update(&self) -> bool {
        true
    }

    fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
        self.0.vars[*self.0.index.get(vars)].set(vars, new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        self.0.vars[*self.0.index.get(vars)].modify_boxed(vars, change)
    }
}

impl<O: VarValue, VI: VarObj<usize>> Var<O> for RcSwitchVar<O, VI> {
    type AsReadOnly = ForceReadOnlyVar<O, Self>;
    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<F: FnOnce(&mut O) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        self.0.vars[*self.0.index.get(vars)].modify_boxed(vars, Box::new(change))
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn map<O2: VarValue, F: FnMut(&O) -> O2 + 'static>(&self, map: F) -> RcMapVar<O, O2, Self, F> {
        self.clone().into_map(map)
    }

    fn map_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static>(&self, map: F) -> MapRefVar<O, O2, Self, F> {
        self.clone().into_map_ref(map)
    }

    fn map_bidi<O2: VarValue, F: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F, G> {
        self.clone().into_map_bidi(map, map_back)
    }

    fn into_map<O2: VarValue, F: FnMut(&O) -> O2 + 'static>(self, map: F) -> RcMapVar<O, O2, Self, F> {
        RcMapVar::new(self, map)
    }

    fn into_map_bidi<O2: VarValue, F: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static>(self, map: F) -> MapRefVar<O, O2, Self, F> {
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

impl<O: VarValue, VI: VarObj<usize>> IntoVar<O> for RcSwitchVar<O, VI> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

#[doc(hidden)]
pub struct RcSwitchVarBuilder<O: VarValue, VI: Var<usize>> {
    index: VI,
    vars: Vec<BoxedVar<O>>,
}
impl<O: VarValue, VI: Var<usize>> RcSwitchVarBuilder<O, VI> {
    pub fn new(index: VI) -> Self {
        RcSwitchVarBuilder {
            index,
            vars: Vec::with_capacity(9),
        }
    }

    pub fn push<IO: IntoVar<O>>(mut self, var: IO) -> Self {
        self.vars.push(var.into_var().boxed());
        self
    }

    pub fn build(self) -> RcSwitchVar<O, VI> {
        debug_assert!(self.vars.len() >= 2);
        RcSwitchVar::from_vars(self.index, self.vars.into_boxed_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small() {
        let _: RcSwitch2Var<u32, OwnedVar<u32>, OwnedVar<u32>, RcVar<usize>> = switch_var!(var(0usize), 0, 1);
        var_type_hint(switch_var!(var(0usize), 0, 1));
    }

    #[test]
    fn large() {
        let _: RcSwitchVar<u32, RcVar<usize>> = switch_var!(var(0usize), 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
        var_type_hint(switch_var!(var(0usize), 0, 1, 2, 3, 4, 5, 6, 7, 8, 9));
    }

    fn var_type_hint(_var: impl Var<u32>) {}
}
