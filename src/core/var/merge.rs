use super::{
    protected, CloningLocalVar, IntoVar, MapBiDiSharedVar, MapSharedVar, MapVar, MapVarBiDi, MapVarBiDiInner, MapVarInner, ObjVar, Var,
    VarValue,
};
use crate::core::context::Vars;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::rc::Rc;

macro_rules! impl_merge_vars {
    ($($MergeVar:ident<$($VN:ident),+> {
        $MergeVarInner:ident<$($TN:ident),+> {
            _t: $($_t: ident),+;
            v: $($vn:ident),+;
            version: $($version:ident),+;
        }
    })+) => {$(
        struct $MergeVarInner<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> {
            $($_t: PhantomData<$TN>,)+
            $($vn: $VN,)+
            $($version: Cell<u32>,)+
            merge: RefCell<M>,
            output: UnsafeCell<MaybeUninit<O>>,
            version: Cell<u32>
        }

        #[doc(hidden)]
        pub struct $MergeVar<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> {
            r: Rc<$MergeVarInner<$($TN,)+ $($VN,)+ O, M>>
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            #[allow(clippy::too_many_arguments)]
            pub fn new($($vn: $VN,)+ merge: M) -> Self {
                $MergeVar {
                    r: Rc::new($MergeVarInner {
                        $($_t: PhantomData,)+
                        $($version: Cell::new(0),)+ // TODO prev_version
                        $($vn,)+
                        merge: RefCell::new(merge),
                        output: UnsafeCell::new(MaybeUninit::uninit()),
                        version: Cell::new(0)
                    })
                }
            }

            fn sync(&self, vars: &Vars) {
                let mut sync = false;

                $(
                    let version = self.r.$vn.version(vars);
                    if version != self.r.$version.get() {
                        sync = true;
                        self.r.$version.set(version);
                    }
                )+

                if self.r.version.get() == 0 {
                    sync = true;
                }

                if sync {
                    self.r.version.set(self.r.version.get().wrapping_add(1));
                    let value = (&mut *self.r.merge.borrow_mut())($(self.r.$vn.get(vars)),+);

                    // SAFETY: This is safe because it only happens before the first borrow
                    // of this update, and borrows cannot exist across updates because source
                    // vars require a &mut Vars for changing version.
                    unsafe {
                        let m_uninit = &mut *self.r.output.get();
                        m_uninit.as_mut_ptr().write(value);
                    }
                }
            }

            fn borrow<'a>(&'a self, vars: &'a Vars) -> &'a O {
                self.sync(vars);
                // SAFETY:
                // * Value will not change here because we require a mutable reference to
                // `Vars` for changing values in source variables.
                // * Memory is initialized here because we start from the prev_version.
                unsafe {
                    let inited = &*self.r.output.get();
                    &*inited.as_ptr()
                }
            }

            fn any_is_new(&self, vars: &Vars) -> bool {
                 $(self.r.$vn.is_new(vars))||+
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> Clone
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn clone(&self) -> Self {
                $MergeVar { r: Rc::clone(&self.r) }
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> protected::Var<O>
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
                protected::BindInfo::Var(self.borrow(vars), self.any_is_new(vars), self.r.version.get())
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> ObjVar<O>
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
                self.borrow(vars)
            }

            fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
                if self.any_is_new(vars) {
                    Some(self.borrow(vars))
                } else {
                    None
                }
            }

            fn is_new(&self, vars: &Vars) -> bool {
                self.any_is_new(vars)
            }

            fn version(&self, vars: &Vars) -> u32 {
                self.sync(vars);
                self.r.version.get()
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> Var<O>
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            type AsReadOnly = Self;
            type AsLocal = CloningLocalVar<O, Self>;

            fn map<O2: VarValue, M2: FnMut(&O) -> O2>(&self, map: M2) -> MapVar<O, Self, O2, M2> {
                MapVar::new(MapVarInner::Shared(MapSharedVar::new(
                    self.clone(),
                    map,
                    self.r.version.get().wrapping_sub(1),
                )))
            }

            fn map_bidi<O2: VarValue, M2: FnMut(&O) -> O2, N: FnMut(&O2) -> O>(
                &self,
                map: M2,
                map_back: N,
            ) -> MapVarBiDi<O, Self, O2, M2, N> {
                MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
                    self.clone(),
                    map,
                    map_back,
                    self.r.version.get().wrapping_sub(1),
                )))
            }

            fn as_read_only(self) -> Self {
                self
            }

            fn as_local(self) -> Self::AsLocal {
                CloningLocalVar::new(self)
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> IntoVar<O>
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            type Var = Self;

            fn into_var(self) -> Self::Var {
                self
            }
        }
    )+}
}

impl_merge_vars! {
    MergeVar2<V0, V1> {
        MergeVar2Inner<T0, T1> {
            _t: _t0, _t1;
            v: v0, v1;
            version: v0_version, v1_version;
        }
    }
    MergeVar3<V0, V1, V2> {
        MergeVar3Inner<T0, T1, T2> {
            _t: _t0, _t1, _t2;
            v: v0, v1, v2;
            version: v0_version, v1_version, v2_version;
        }
    }
    MergeVar4<V0, V1, V2, V3> {
        MergeVar4Inner<T0, T1, T2, T3> {
            _t: _t0, _t1, _t2, _t3;
            v: v0, v1, v2, v3;
            version: v0_version, v1_version, v2_version, v3_version;
        }
    }
    MergeVar5<V0, V1, V2, V3, V4> {
        MergeVar5Inner<T0, T1, T2, T3, T4> {
            _t: _t0, _t1, _t2, _t3, _t4;
            v: v0, v1, v2, v3, v4;
            version: v0_version, v1_version, v2_version, v3_version, v4_version;
        }
    }
    MergeVar6<V0, V1, V2, V3, V4, V5> {
        MergeVar6Inner<T0, T1, T2, T3, T4, T5> {
            _t: _t0, _t1, _t2, _t3, _t4, _t5;
            v: v0, v1, v2, v3, v4, v5;
            version: v0_version, v1_version, v2_version, v3_version, v4_version, v5_version;
        }
    }
    MergeVar7<V0, V1, V2, V3, V4, V5, V6> {
        MergeVar7Inner<T0, T1, T2, T3, T4, T5, T6> {
            _t: _t0, _t1, _t2, _t3, _t4, _t5, _t6;
            v: v0, v1, v2, v3, v4, v5, v6;
            version: v0_version, v1_version, v2_version, v3_version, v4_version, v5_version, v6_version;
        }
    }
    MergeVar8<V0, V1, V2, V3, V4, V5, V6, V7> {
        MergeVar8Inner<T0, T1, T2, T3, T4, T5, T6, T7> {
            _t: _t0, _t1, _t2, _t3, _t4, _t5, _t6, _t7;
            v: v0, v1, v2, v3, v4, v5, v6, v7;
            version: v0_version, v1_version, v2_version, v3_version, v4_version, v5_version, v6_version, v7_version;
        }
    }
}

/// Initializes a new [`Var`](crate::core::var::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](crate::core::var::Var), minimal 2.
/// * `merge`: A function that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Example
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # use zero_ui::prelude::{var, text, Text};
/// # use zero_ui::core::var::SharedVar;
/// # fn main() {
/// let var0: SharedVar<Text> = var("Hello");
/// let var1: SharedVar<Text> = var("World");
///
/// let greeting_text = text(merge_var!(var0, var1, |a, b|formatx!("{} {}!", a, b)));
/// # }
/// ```
#[macro_export]
macro_rules! merge_var {
    ($v0: expr, $v1: expr, $merge: expr) => {
        $crate::core::var::MergeVar2::new($v0, $v1, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $merge: expr) => {
        $crate::core::var::MergeVar3::new($v0, $v1, $v2, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $merge: expr) => {
        $crate::core::var::MergeVar4::new($v0, $v1, $v2, $v3, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $merge: expr) => {
        $crate::core::var::MergeVar5::new($v0, $v1, $v2, $v3, $v4, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $merge: expr) => {
        $crate::core::var::MergeVar6::new($v0, $v1, $v2, $v3, $v4, $v5, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $merge: expr) => {
        $crate::core::var::MergeVar7::new($v0, $v1, $v2, $v3, $v4, $v5, $v6, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $merge: expr) => {
        $crate::core::var::MergeVar8::new($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $($more_args:tt),+) => {
        compile_error!("merge_var is only implemented to a maximum of 8 variables")
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (var0, var1, .., merge_fn")
    };
}
