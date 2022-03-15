use super::*;

use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

///<span data-inline></span> Initializes a new switch var.
///
/// A switch var updates when the *index* var updates or when the *indexed* var updates.  
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `$index`: A positive integer that is the initial switch index.
/// * `$v0..$vn`: A list of [vars](crate::var::Var), minimal 2.
///
/// [`RcSwitchVar`](crate::var::RcSwitchVar) is used for more then 8 variables.
///
/// All arguments are [`IntoVar`](crate::var::RcSwitchVar).
///
/// # Example
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) { }
/// let index = var(0);
/// let var0 = var("Read-write".to_text());
/// let var1 = "Read-only";
///
/// let t = text(switch_var!(index.clone(), var0.clone(), var1));
/// ```
///
/// In the example if `index` or `var0` are modified afterwards the text updates.
#[macro_export]
macro_rules! switch_var {
    ($index: expr $(, $v0: expr)? $(,)?) => {
        compile_error!{"switch_var requires at least 2 variables"}
    };
    ($index: expr, $v0: expr, $v1: expr) => {
        $crate::var::RcSwitch2Var::new($index, ($v0, $v1))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr) => {
        $crate::var::RcSwitch3Var::new($index, ($v0, $v1, $v2))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr) => {
        $crate::var::RcSwitch4Var::new($index, ($v0, $v1, $v2, $v3))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr) => {
        $crate::var::RcSwitch5Var::new($index, ($v0, $v1, $v2, $v3, $v4))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr) => {
        $crate::var::RcSwitch6Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr) => {
        $crate::var::RcSwitch7Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5, $v6))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr) => {
        $crate::var::RcSwitch8Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7))
    };
    ($index: expr, $($v:expr),+) => {
        // we need a builder to have $v be IntoVar and work like the others.
        $crate::var::RcSwitchVarBuilder::new($index)
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
        $crate::paste!{
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
        pub struct $RcSwitchVar<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>>(Rc<$RcSwitchVarData<O, $($V,)+ VI>>);
        struct $RcSwitchVarData<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>> {
            _o: PhantomData<O>,
            vars: ($($V),+),
            versions: [VarVersionCell; $len],
            index: VI,
            index_version: VarVersionCell,
            self_version: Cell<u32>,
        }

        #[allow(missing_docs)] // this is hidden
        impl<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>> $RcSwitchVar<O, $($V,)+ VI> {
            pub fn new<$($IV: IntoVar<O, Var=$V>),+>(index: VI, vars: ($($IV),+)) -> Self {
                Self::from_vars(index, ($(vars.$n.into_var()),+))
            }
        }

        #[allow(missing_docs)] // this is hidden
        impl<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>> $RcSwitchVar<O, $($V,)+ VI> {
            pub fn from_vars(index: VI, vars: ($($V),+)) -> Self {
                Self(Rc::new($RcSwitchVarData {
                    _o: PhantomData,
                    vars,
                    versions: array_init::array_init(|_|VarVersionCell::new(0)),
                    index,
                    index_version: VarVersionCell::new(0),
                    self_version: Cell::new(0),
                }))
            }
        }

        impl<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>>
        Clone for $RcSwitchVar<O, $($V,)+ VI> {
            fn clone(&self) -> Self {
                Self(Rc::clone(&self.0))
            }
        }

        impl<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>>
        crate::private::Sealed for $RcSwitchVar<O, $($V,)+ VI> { }

        impl<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>>
        Var<O> for $RcSwitchVar<O, $($V,)+ VI> {
            type AsReadOnly = ReadOnlyVar<O, Self>;

            fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a O {
                let vars = vars.as_ref();
                match *self.0.index.get(vars) {
                    $($n => self.0.vars.$n.get(vars),)+
                    _ => panic!("switch_var index out of range"),
                }
            }

            fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a O> {
                let vars = vars.as_ref();
                if self.is_new(vars) {
                    Some(self.get(vars))
                } else {
                    None
                }
            }

            fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> O {
                match Rc::try_unwrap(self.0) {
                    Ok(r) => {
                        vars.with_vars_read(move |vars| {
                            match *r.index.get(vars) {
                                $($n => r.vars.$n.into_value(vars),)+
                                _ => panic!("switch_var index out of range"),
                            }
                        })
                    },
                    Err(e) => $RcSwitchVar(e).get_clone(vars)
                }
            }

            fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
                vars.with_vars(|vars| {
                    self.0.index.is_new(vars)
                    || match *self.0.index.get(vars) {
                        $($n => self.0.vars.$n.is_new(vars),)+
                        _ => panic!("switch_var index out of range"),
                    }
                })
            }

            fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
                vars.with_vars_read(|vars| {
                    let i_ver = self.0.index.version(vars);
                    let var_vers = ($(self.0.vars.$n.version(vars)),+);

                    if i_ver != self.0.index_version.get() || $(var_vers.$n != self.0.versions[$n].get())||+ {
                        self.0.self_version.set(self.0.self_version.get().wrapping_add(1));
                        self.0.index_version.set(i_ver);
                        $(self.0.versions[$n].set(var_vers.$n);)+
                    }

                    VarVersion::normal(self.0.self_version.get())
                })
            }

            fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
               vars.with_vars(|vars| {
                    match *self.0.index.get(vars) {
                        $($n => self.0.vars.$n.is_read_only(vars),)+
                        _ => panic!("switch_var index out of range"),
                    }
               })
            }

            fn is_contextual(&self) -> bool {
                self.0.index.is_contextual() || $(self.0.vars.$n.is_contextual())||+
            }

            fn always_read_only(&self) -> bool {
                $(self.0.vars.$n.always_read_only())&&+
            }

            #[inline]
            fn can_update(&self) -> bool {
                self.0.index.can_update() || $(self.0.vars.$n.can_update())||+
            }

            #[inline]
            fn strong_count(&self) -> usize {
                Rc::strong_count(&self.0)
            }

            fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
            where
                Vw: WithVars,
                N: Into<O>
            {
                vars.with_vars(|vars| {
                    match *self.0.index.get(vars) {
                        $($n => self.0.vars.$n.set(vars, new_value),)+
                        _ => panic!("switch_var index out of range"),
                    }
                })
            }

            fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, VarIsReadOnly>
            where
                Vw: WithVars,
                N: Into<O>,
                O : PartialEq
            {
                vars.with_vars(|vars| {
                    match *self.0.index.get(vars) {
                        $($n => self.0.vars.$n.set_ne(vars, new_value),)+
                        _ => panic!("switch_var index out of range")
                    }
                })
            }

            fn modify<Vw, F>(&self, vars: &Vw, change: F) -> Result<(), VarIsReadOnly>
            where
                Vw: WithVars,
                F: FnOnce(&mut VarModify<O>) + 'static
            {
                vars.with_vars(|vars| {
                    match *self.0.index.get(vars) {
                        $($n => self.0.vars.$n.modify(vars, change),)+
                        _ => panic!("switch_var index out of range"),
                    }
                })
            }

            #[inline]
            fn into_read_only(self) -> Self::AsReadOnly {
                ReadOnlyVar::new(self)
            }

            fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
                vars.with_vars_read(|vars| {
                    let mut r = self.0.index.update_mask(vars);
                    $(r |= self.0.vars.$n.update_mask(vars);)+
                    r
                })
            }
        }

        impl<O: VarValue, $($V: Var<O>,)+ VI: Var<usize>>
        IntoVar<O> for $RcSwitchVar<O, $($V,)+ VI> {
            type Var = Self;

            #[inline]
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
pub struct RcSwitchVar<O: VarValue, VI: Var<usize>>(Rc<RcSwitchVarData<O, VI>>);
struct RcSwitchVarData<O: VarValue, VI: Var<usize>> {
    vars: Box<[BoxedVar<O>]>,
    var_versions: Box<[VarVersionCell]>,

    index: VI,
    index_version: VarVersionCell,

    self_version: Cell<u32>,
}
impl<O: VarValue, VI: Var<usize>> RcSwitchVar<O, VI> {
    #[doc(hidden)]
    pub fn from_vars(index: VI, vars: Box<[BoxedVar<O>]>) -> Self {
        assert!(vars.len() >= 2);
        Self(Rc::new(RcSwitchVarData {
            var_versions: vars.iter().map(|_| VarVersionCell::new(0)).collect(),
            vars,
            index,
            index_version: VarVersionCell::new(0),
            self_version: Cell::new(0),
        }))
    }

    /// Gets the indexed variable value.
    pub fn get<'a>(&'a self, vars: &'a Vars) -> &O {
        <Self as Var<O>>::get(self, vars)
    }

    /// Gets if the index is new or the indexed variable value is new.
    pub fn is_new(&self, vars: &Vars) -> bool {
        <Self as Var<O>>::is_new(self, vars)
    }

    /// Gets the version.
    ///
    /// The version is new when the index variable changes
    /// or when the indexed variable changes.
    pub fn version(&self, vars: &Vars) -> VarVersion {
        <Self as Var<O>>::version(self, vars)
    }

    /// Gets if the indexed variable is read-only.
    pub fn is_read_only(&self, vars: &Vars) -> bool {
        <Self as Var<O>>::is_read_only(self, vars)
    }

    /// Gets if all alternate variables are always read-only.
    pub fn always_read_only(&self) -> bool {
        <Self as Var<O>>::always_read_only(self)
    }

    /// Tries to set the indexed variable.
    pub fn set<N>(&self, vars: &Vars, new_value: N) -> Result<(), VarIsReadOnly>
    where
        N: Into<O>,
    {
        <Self as Var<O>>::set(self, vars, new_value)
    }

    /// Tries to set the indexed variable, but only sets if the value is not equal.
    pub fn set_ne<N>(&self, vars: &Vars, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        N: Into<O>,
        O: PartialEq,
    {
        <Self as Var<O>>::set_ne(self, vars, new_value)
    }

    /// Modify the indexed variable.
    pub fn modify<F: FnOnce(&mut VarModify<O>) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        <Self as Var<O>>::modify(self, vars, change)
    }
}
impl<O: VarValue, VI: Var<usize>> Clone for RcSwitchVar<O, VI> {
    fn clone(&self) -> Self {
        RcSwitchVar(Rc::clone(&self.0))
    }
}
impl<O: VarValue, VI: Var<usize>> crate::private::Sealed for RcSwitchVar<O, VI> {}
impl<O: VarValue, VI: Var<usize>> Var<O> for RcSwitchVar<O, VI> {
    type AsReadOnly = ReadOnlyVar<O, Self>;

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a O {
        let vars = vars.as_ref();
        self.0.vars[self.0.index.copy(vars)].get(vars)
    }

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a O> {
        let vars = vars.as_ref();
        if self.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.0.vars[self.0.index.copy(vars)].is_new(vars))
    }

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> O {
        match Rc::try_unwrap(self.0) {
            Ok(r) => vars.with_vars_read(move |vars| Vec::from(r.vars).swap_remove(r.index.copy(vars)).into_value(vars)),
            Err(e) => RcSwitchVar(e).get_clone(vars),
        }
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|vars| {
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

            VarVersion::normal(self.0.self_version.get())
        })
    }

    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.0.vars[*self.0.index.get(vars)].is_read_only(vars))
    }

    fn always_read_only(&self) -> bool {
        self.0.vars.iter().all(|v| v.always_read_only())
    }

    #[inline]
    fn can_update(&self) -> bool {
        self.0.index.can_update() || self.0.vars.iter().any(|v| v.can_update())
    }

    #[inline]
    fn is_contextual(&self) -> bool {
        self.0.index.is_contextual() || self.0.vars.iter().any(|v| v.is_contextual())
    }

    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<O>,
    {
        vars.with_vars(|vars| self.0.vars[*self.0.index.get(vars)].set(vars, new_value))
    }

    fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<O>,
        O: PartialEq,
    {
        vars.with_vars(|vars| self.0.vars[*self.0.index.get(vars)].set_ne(vars, new_value))
    }

    fn modify<Vw: WithVars, F: FnOnce(&mut VarModify<O>) + 'static>(&self, vars: &Vw, change: F) -> Result<(), VarIsReadOnly> {
        vars.with_vars(|vars| self.0.vars[*self.0.index.get(vars)].modify(vars, change))
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|vars| {
            let mut r = self.0.index.update_mask(vars);
            for var in self.0.vars.iter() {
                r |= var.update_mask(vars);
            }
            r
        })
    }
}
impl<O: VarValue, VI: Var<usize>> IntoVar<O> for RcSwitchVar<O, VI> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

#[doc(hidden)]
pub struct RcSwitchVarBuilder<O: VarValue, VI: Var<usize>> {
    index: VI,
    vars: Vec<BoxedVar<O>>,
}
#[allow(missing_docs)] // this is all hidden
impl<O: VarValue, VI: Var<usize>> RcSwitchVarBuilder<O, VI> {
    pub fn new(index: VI) -> Self {
        RcSwitchVarBuilder { index, vars: vec![] }
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
