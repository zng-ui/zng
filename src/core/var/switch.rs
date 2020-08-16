use super::{
    protected, CloningLocalVar, IntoVar, MapBiDiSharedVar, MapSharedVar, MapVar, MapVarBiDi, MapVarBiDiInner, MapVarInner, ObjVar,
    ReadOnlyVar, Var, VarIsReadOnly, VarValue,
};
use crate::core::context::{Updates, Vars};
use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

macro_rules! impl_switch_vars {
    ($($SwitchVar:ident<$N:expr,$($VN:ident),+> {
        $SwitchVarInner:ident {
            $($n:expr => $vn:ident, $version: ident;)+
        }
    })+) => {$(
        struct $SwitchVarInner<I: Var<usize>, T: VarValue, $($VN: Var<T>),+> {
            _t: PhantomData<T>,
            $($vn: $VN,)+

            index: I,
            index_version: Cell<u32>,

            $($version: Cell<u32>,)+

            version: Cell<u32>,
        }

        #[doc(hidden)]
        pub struct $SwitchVar<I: Var<usize>, T: VarValue, $($VN: Var<T>),+> {
            r: Rc<$SwitchVarInner<I, T, $($VN),+>>,
        }

        impl<I: Var<usize>, T: VarValue, $($VN: Var<T>),+> $SwitchVar<I, T, $($VN),+> {
            #[allow(clippy::too_many_arguments)]
            pub fn new(index: impl IntoVar<usize, Var=I>, $($vn: impl IntoVar<T, Var=$VN>),+) -> Self {
                $SwitchVar {
                    r: Rc::new($SwitchVarInner {
                        _t: PhantomData,
                        index: index.into_var(),
                        index_version: Cell::new(0),
                        $($version: Cell::new(0),)+
                        version: Cell::new(0),
                        $($vn: $vn.into_var(),)+
                    })
                }
            }
        }

        impl<I: Var<usize>, T: VarValue, $($VN: Var<T>),+> protected::Var<T> for $SwitchVar<I, T, $($VN),+> {
            fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
                let is_new = self.is_new(vars);
                let version = self.version(vars);
                let inner_info = match *self.r.index.get(vars) {
                    $($n => self.r.$vn.bind_info(vars),)+
                    i => panic!("switch_var index `{}` out of range", i),
                };

                match inner_info {
                    protected::BindInfo::Var(value, _, _) => protected::BindInfo::Var(value, is_new, version),
                    protected::BindInfo::ContextVar(var_id, default, _) => {
                        protected::BindInfo::ContextVar(var_id, default, Some((is_new, version)))
                    }
                }
            }

            fn read_only_prev_version(&self) -> u32 {
                self.r.version.get().wrapping_sub(1)
            }
        }

        impl<I: Var<usize>, T: VarValue, $($VN: Var<T>),+> ObjVar<T> for $SwitchVar<I, T, $($VN),+> {
            fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
                match *self.r.index.get(vars) {
                    $($n => self.r.$vn.get(vars),)+
                    i => panic!("switch_var index `{}` out of range", i),
                }
            }

            fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
                if self.r.index.is_new(vars) {
                    Some(self.get(vars))
                } else {
                    match *self.r.index.get(vars) {
                        $($n => self.r.$vn.update(vars),)+
                        i => panic!("switch_var index `{}` out of range", i),
                    }
                }
            }

            fn can_update(&self) -> bool {
                self.r.index.can_update()
            }

            fn is_new(&self, vars: &Vars) -> bool {
                self.r.index.is_new(vars)
                    || match *self.r.index.get(vars) {
                        $($n => self.r.$vn.is_new(vars),)+
                        i => panic!("switch_var index `{}` out of range", i),
                    }
            }

            fn version(&self, vars: &Vars) -> u32 {
                let mut increment_ver = false;
                match *self.r.index.get(vars) {
                    $($n => {
                        let $version = self.r.$vn.version(vars);
                        if $version != self.r.$version.get() {
                            self.r.$version.set($version);
                            increment_ver = true;
                        }
                    },)+
                    i => panic!("switch_var index `{}` out of range", i),
                }
                let version = self.r.index.version(vars);
                if version != self.r.index_version.get(){
                    self.r.index_version.set(version);
                    increment_ver = true;
                }
                if increment_ver{
                    self.r.version.set(self.r.version.get().wrapping_add(1));
                }
                self.r.version.get()
            }

            fn read_only(&self, vars: &Vars) -> bool {
                match *self.r.index.get(vars) {
                    $($n => self.r.$vn.read_only(vars),)+
                    i => panic!("switch_var index `{}` out of range", i),
                }
            }

            fn always_read_only(&self, vars: &Vars) -> bool {
                $(self.r.$vn.always_read_only(vars)) && +
            }

            fn push_set(&self, new_value: T, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
                match *self.r.index.get(vars) {
                    $($n => self.r.$vn.push_set(new_value, vars, updates),)+
                    i => panic!("switch_var index `{}` out of range", i),
                }
            }

            fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut T) + 'static>, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
                match *self.r.index.get(vars) {
                    $($n => self.r.$vn.push_modify_boxed(modify, vars, updates),)+
                    i => panic!("switch_var index `{}` out of range", i),
                }
            }
        }

        impl<I: Var<usize>, T: VarValue, $($VN: Var<T>),+> Clone for $SwitchVar<I, T, $($VN),+> {
            fn clone(&self) -> Self {
                $SwitchVar { r: Rc::clone(&self.r) }
            }
        }

        impl<I: Var<usize>, T: VarValue, $($VN: Var<T>),+> Var<T> for $SwitchVar<I, T, $($VN),+> {
            type AsReadOnly = ReadOnlyVar<T, Self>;
            type AsLocal = CloningLocalVar<T, Self>;

            fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
                match *self.r.index.get(vars) {
                    $($n => self.r.$vn.push_modify(modify, vars, updates),)+
                    i => panic!("switch_var index `{}` out of range", i),
                }
            }

            fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
            where
                M: FnMut(&T) -> O + 'static,
                O: VarValue,
            {
                self.clone().into_map(map)
            }

            fn into_map<O, M>(self, map: M) -> MapVar<T, Self, O, M>
            where
                M: FnMut(&T) -> O + 'static,
                O: VarValue,
            {
                let prev_version = self.r.version.get().wrapping_sub(1);
                MapVar::new(MapVarInner::Shared(MapSharedVar::new(
                    self,
                    map,
                    prev_version,
                )))
            }

            fn map_bidi<O: VarValue, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T>(
                &self,
                map: M,
                map_back: N,
            ) -> MapVarBiDi<T, Self, O, M, N> {
                MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
                    self.clone(),
                    map,
                    map_back,
                    self.r.version.get().wrapping_sub(1),
                )))
            }

            fn as_read_only(self) -> Self::AsReadOnly {
                ReadOnlyVar::new(self)
            }

            fn as_local(self) -> Self::AsLocal {
                CloningLocalVar::new(self)
            }
        }

        impl<I: Var<usize>, T: VarValue, $($VN: Var<T>),+> IntoVar<T> for $SwitchVar<I, T, $($VN),+> {
            type Var = Self;

            fn into_var(self) -> Self::Var {
                self
            }
        }
    )+};
}

impl_switch_vars! {
    SwitchVar2<2, V0, V1> {
        SwitchVar2Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
        }
    }
    SwitchVar3<3, V0, V1, V2> {
        SwitchVar3Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
        }
    }
    SwitchVar4<4, V0, V1, V2, V3> {
        SwitchVar4Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
        }
    }
    SwitchVar5<5, V0, V1, V2, V3, V4> {
        SwitchVar5Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
            4 => v4, v4_version;
        }
    }
    SwitchVar6<6, V0, V1, V2, V3, V4, V5> {
        SwitchVar6Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
            4 => v4, v4_version;
            5 => v5, v5_version;
        }
    }
    SwitchVar7<7, V0, V1, V2, V3, V4, V5, V6> {
        SwitchVar7Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
            4 => v4, v4_version;
            5 => v5, v5_version;
            6 => v6, v6_version;
        }
    }
    SwitchVar8<8, V0, V1, V2, V3, V4, V5, V6, V7> {
        SwitchVar8Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
            4 => v4, v4_version;
            5 => v5, v5_version;
            6 => v6, v6_version;
            7 => v7, v7_version;
        }
    }
}

struct SwitchVarDynInner<I: Var<usize>, T: 'static> {
    _t: PhantomData<T>,
    vars: Vec<Box<dyn ObjVar<T>>>,
    versions: Vec<Cell<u32>>,
    index_version: Cell<u32>,

    index: I,

    version: Cell<u32>,
}

/// A dynamically-sized set of variables that can be switched on. See [`switch_var!`](macro.switch_var.html) for
/// the full documentation.
pub struct SwitchVarDyn<I: Var<usize>, T: VarValue> {
    r: Rc<SwitchVarDynInner<I, T>>,
}

impl<I: Var<usize>, T: VarValue> SwitchVarDyn<I, T> {
    pub fn new(index: I, vars: Vec<Box<dyn ObjVar<T>>>) -> Self {
        assert!(!vars.is_empty());

        SwitchVarDyn {
            r: Rc::new(SwitchVarDynInner {
                _t: PhantomData,
                index,
                index_version: Cell::new(0),
                versions: vec![Cell::new(0); vars.len()],
                version: Cell::new(0),
                vars,
            }),
        }
    }
}

impl<I: Var<usize>, T: VarValue> protected::Var<T> for SwitchVarDyn<I, T> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        let is_new = self.is_new(vars);
        let version = self.version(vars);
        let inner_info = self.r.vars[*self.r.index.get(vars)].bind_info(vars);

        match inner_info {
            protected::BindInfo::Var(value, _, _) => protected::BindInfo::Var(value, is_new, version),
            protected::BindInfo::ContextVar(var_id, default, _) => {
                protected::BindInfo::ContextVar(var_id, default, Some((is_new, version)))
            }
        }
    }

    fn read_only_prev_version(&self) -> u32 {
        self.r.version.get().wrapping_sub(1)
    }
}

impl<I: Var<usize>, T: VarValue> ObjVar<T> for SwitchVarDyn<I, T> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.r.vars[*self.r.index.get(vars)].get(vars)
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        if self.r.index.is_new(vars) {
            Some(self.get(vars))
        } else {
            self.r.vars[*self.r.index.get(vars)].update(vars)
        }
    }

    fn can_update(&self) -> bool {
        self.r.index.can_update()
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.r.index.is_new(vars) || self.r.vars[*self.r.index.get(vars)].is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        let mut increment_ver = false;
        let index = *self.r.index.get(vars);

        let version = self.r.vars[index].version(vars);
        if version != self.r.versions[index].get() {
            self.r.versions[index].set(version);
            increment_ver = true;
        }
        let version = self.r.index.version(vars);
        if version != self.r.index_version.get() {
            self.r.index_version.set(version);
            increment_ver = true;
        }

        if increment_ver {
            self.r.version.set(self.r.version.get().wrapping_add(1));
        }

        self.r.version.get()
    }

    fn read_only(&self, vars: &Vars) -> bool {
        self.r.vars[*self.r.index.get(vars)].read_only(vars)
    }

    fn always_read_only(&self, vars: &Vars) -> bool {
        self.r.vars.iter().all(|v| v.always_read_only(vars))
    }

    fn push_set(&self, new_value: T, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.r.vars[*self.r.index.get(vars)].push_set(new_value, vars, updates)
    }

    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut T) + 'static>,
        vars: &Vars,
        updates: &mut Updates,
    ) -> Result<(), VarIsReadOnly> {
        self.r.vars[*self.r.index.get(vars)].push_modify_boxed(modify, vars, updates)
    }
}

impl<I: Var<usize>, T: VarValue> Clone for SwitchVarDyn<I, T> {
    fn clone(&self) -> Self {
        SwitchVarDyn { r: Rc::clone(&self.r) }
    }
}

impl<I: Var<usize>, T: VarValue> Var<T> for SwitchVarDyn<I, T> {
    type AsReadOnly = ReadOnlyVar<T, Self>;
    type AsLocal = CloningLocalVar<T, Self>;

    fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.push_modify_boxed(Box::new(modify), vars, updates)
    }

    fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        self.clone().into_map(map)
    }

    fn into_map<O, M>(self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        let prev_version = self.r.version.get().wrapping_sub(1);
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(self, map, prev_version)))
    }

    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static,
        O: VarValue,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            self.r.version.get().wrapping_sub(1),
        )))
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<I: Var<usize>, T: VarValue> IntoVar<T> for SwitchVarDyn<I, T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

pub use zero_ui_macros::switch_var;
