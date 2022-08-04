use super::*;

use std::cell::{Cell, RefCell, UnsafeCell};
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

///<span data-del-macro-root></span> Initializes a new [`Var`](crate::var::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](crate::var::Var), minimal 2.
/// * `merge`: A function that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Examples
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) {  }
/// let var0: RcVar<Text> = var_from("Hello");
/// let var1: RcVar<Text> = var_from("World");
///
/// let greeting_text = text(merge_var!(var0, var1, |a, b|formatx!("{a} {b}!")));
/// ```
#[macro_export]
macro_rules! merge_var {
    ($($tt:tt)*) => {
        $crate::merge_var_impl!($($tt)*)
    };
}
#[doc(inline)]
pub use crate::merge_var;

#[macro_export]
#[doc(hidden)]
#[cfg(not(dyn_closure))]
macro_rules! merge_var_impl {
    ($v0: expr, $v1: expr, $merge: expr) => {
        $crate::var::types::RcMerge2Var::new(($v0, $v1), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $merge: expr) => {
        $crate::var::types::RcMerge3Var::new(($v0, $v1, $v2), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $merge: expr) => {
        $crate::var::types::RcMerge4Var::new(($v0, $v1, $v2, $v3), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $merge: expr) => {
        $crate::var::types::RcMerge5Var::new(($v0, $v1, $v2, $v3, $v4), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $merge: expr) => {
        $crate::var::types::RcMerge6Var::new(($v0, $v1, $v2, $v3, $v4, $v5), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $merge: expr) => {
        $crate::var::types::RcMerge7Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $merge: expr) => {
        $crate::var::types::RcMerge8Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $merge: expr) => {
        $crate::var::types::rc_merge_var!($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $merge).as_impl_var()
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (var0, var1, .., merge_fn")
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(dyn_closure)]
macro_rules! merge_var_impl {
    ($($input:expr),+ $(,)?) => {
        $crate::rc_merge_var!($($input),+).as_impl_var()
    }
}

#[cfg(not(dyn_closure))]
macro_rules! impl_rc_merge_var {
    ($(
        $len:tt => $($n:tt),+;
    )+) => {$(
        $crate::paste!{
            impl_rc_merge_var!{
                Var: [<RcMerge $len Var>];// RcMerge2Var
                WeakVar: [<WeakRcMerge $len Var>];// WeakRcMerge2Var
                Data: [<RcMerge $len VarData>];// RcMerge2VarData
                len: $len;//2
                I: $([<I $n>]),+;// I0, I1
                V: $([<V $n>]),+;// V0, V1
                n: $($n),+; // 0, 1
                test_name: [<test_merge_var_ $len>]; // test_merge_var_2
            }
        }
    )+};

    (
        Var: $RcMergeVar:ident;
        WeakVar: $WeakRcMergeVar:ident;
        Data: $RcMergeVarData:ident;
        len: $len:tt;
        I: $($I:ident),+;
        V: $($V:ident),+;
        n: $($n:tt),+;
        test_name: $test_name:ident;
    ) => {
        #[doc(hidden)]
        pub struct $RcMergeVar<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>(
            Rc<$RcMergeVarData<$($I,)+ O, $($V,)+ F>>,
        );

        #[doc(hidden)]
        pub struct $WeakRcMergeVar<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>(
            Weak<$RcMergeVarData<$($I,)+ O, $($V,)+ F>>,
        );

        struct $RcMergeVarData<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static> {
            _i: PhantomData<($($I),+)>,
            vars: ($($V),+),
            f: Rc<RefCell<F>>,
            versions: [VarVersionCell; $len],
            output_version: Cell<u32>,
            output: UnsafeCell<Option<O>>,
        }

        #[allow(missing_docs)]// this is all hidden.
        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static> $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            pub fn new(vars: ($($V),+), f: F) -> Self {
                Self(Rc::new($RcMergeVarData {
                    _i: PhantomData,
                    vars,
                    f: Rc::new(RefCell::new(f)),
                    versions: array_init::array_init(|_|VarVersionCell::new(0)),
                    output_version: Cell::new(0),
                    output: UnsafeCell::new(None),
                }))
            }

            fn update_output(&self, vars: &VarsRead) {
                // SAFETY: This is safe because it only happens before the first borrow
                // of this update, and borrows cannot exist across updates because source
                // vars require a &mut Vars for changing version.

                let versions = ($(self.0.vars.$n.version(vars)),+);

                let update_output = unsafe { &*self.0.output.get() }.is_none() || {
                    $(self.0.versions[$n].get() != versions.$n)||+
                };
                if update_output {
                    let new_value = (&mut *self.0.f.borrow_mut())($(self.0.vars.$n.get(vars)),+);

                    unsafe {
                        *self.0.output.get() = Some(new_value);
                    }

                    self.0.output_version.set(self.0.output_version.get().wrapping_add(1));
                    $(self.0.versions[$n].set(versions.$n);)+
                }
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        crate::private::Sealed for $RcMergeVar<$($I,)+ O, $($V,)+ F> {}

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        crate::private::Sealed for $WeakRcMergeVar<$($I,)+ O, $($V,)+ F> {}

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Clone for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            fn clone(&self) -> Self {
                $RcMergeVar(Rc::clone(&self.0))
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Clone for $WeakRcMergeVar<$($I,)+ O, $($V,)+ F> {
            fn clone(&self) -> Self {
                $WeakRcMergeVar(self.0.clone())
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Var<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            type AsReadOnly = types::ReadOnlyVar<O, Self>;
            type Weak = $WeakRcMergeVar<$($I,)+ O, $($V,)+ F>;

            fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a O {
                self.update_output(vars.as_ref());

                // SAFETY:
                // This is safe because we require &mut Vars for updating.
                unsafe { &*self.0.output.get() }.as_ref().unwrap()
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
                vars.with_vars_read(|vars| {
                    self.update_output(vars);

                    match Rc::try_unwrap(self.0) {
                        Ok(r) => r.output.into_inner().unwrap(),
                        Err(e) => $RcMergeVar(e).get_clone(vars)
                    }
                })
            }

            fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
                $(self.0.vars.$n.is_new(vars))||+
            }

            fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
               vars.with_vars_read(|vars| {
                    self.update_output(vars);
                    VarVersion::normal(self.0.output_version.get())
               })
            }


            fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
                true
            }


            fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
                vars.with_vars_read(|vars| {
                    $(self.0.vars.$n.is_animating(vars))||+
                })
            }


            fn always_read_only(&self) -> bool {
                true
            }

            fn can_update(&self) -> bool {
                $(self.0.vars.$n.can_update())||+
            }

            fn is_contextual(&self) -> bool {
                $(self.0.vars.$n.is_contextual())||+
            }

            fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<O> {
                if self.is_contextual() {
                    vars.with_vars(|vars| {
                        let value = self.get_clone(vars);
                        let var = $RcMergeVar(Rc::new($RcMergeVarData {
                            _i: PhantomData,
                            vars: ($(self.0.vars.$n.actual_var(vars)),+),
                            f: self.0.f.clone(),
                            versions: self.0.versions.clone(),
                            output_version: self.0.output_version.clone(),
                            output: UnsafeCell::new(Some(value)),
                        }));

                        var.boxed()
                    })
                } else {
                    self.clone().boxed()
                }
            }


            fn set<Vw: WithVars, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly> where N: Into<O> {
                Err(VarIsReadOnly)
            }


            fn set_ne<Vw: WithVars, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>  where N: Into<O>, O: PartialEq {
                Err(VarIsReadOnly)
            }


            fn modify<Vw: WithVars, F2: FnOnce(VarModify<O>) + 'static>(&self, _: &Vw, _: F2) -> Result<(), VarIsReadOnly> {
                Err(VarIsReadOnly)
            }


            fn into_read_only(self) -> Self::AsReadOnly {
                types::ReadOnlyVar::new(self)
            }


            fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
                vars.with_vars_read(|vars| {
                    let mut r = UpdateMask::none();
                    $(r |= self.0.vars.$n.update_mask(vars);)+
                    r
                })
            }


            fn is_rc(&self) -> bool {
                true
            }


            fn downgrade(&self) -> Option<Self::Weak> {
                Some($WeakRcMergeVar(Rc::downgrade(&self.0)))
            }


            fn strong_count(&self) -> usize {
                Rc::strong_count(&self.0)
            }


            fn weak_count(&self) -> usize {
                Rc::weak_count(&self.0)
            }


            fn as_ptr(&self) -> *const () {
                Rc::as_ptr(&self.0) as _
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        IntoVar<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            type Var = Self;
            fn into_var(self) -> Self {
                self
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        any::AnyVar for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            fn into_any(self) -> Box<dyn any::AnyVar> {
                Box::new(self)
            }

            any_var_impls!();
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        WeakVar<O> for $WeakRcMergeVar<$($I,)+ O, $($V,)+ F> {
            type Strong = $RcMergeVar<$($I,)+ O, $($V,)+ F>;


            fn upgrade(&self) -> Option<Self::Strong> {
                self.0.upgrade().map($RcMergeVar)
            }


            fn strong_count(&self) -> usize {
                self.0.strong_count()
            }


            fn weak_count(&self) -> usize {
                self.0.weak_count()
            }


            fn as_ptr(&self) -> *const () {
                self.0.as_ptr() as _
            }
        }

        #[test]
        #[allow(non_snake_case)]
        fn $test_name() {
            let vars = [$(var($n)),+];
            let var = merge_var!(
                $(vars[$n].clone(),)+
                |$($I),+| {
                    [$(*$I),+]
                }
            );

            let mut test = crate::context::TestWidgetContext::new();

            let mut expected = [$($n),+];
            assert_eq!(&expected, var.get(&test.vars));

            for i in 0..vars.len() {
                vars[i].set(&test.vars, (i + 1) as i32);
                expected[i] += 1;

                let (_, u) = test.apply_updates();
                assert!(u.update);
                assert_eq!(&expected,  var.get(&test.vars));
            }
        }
    };
}

#[cfg(not(dyn_closure))]
impl_rc_merge_var! {
    2 => 0, 1;
    3 => 0, 1, 2;
    4 => 0, 1, 2, 3;
    5 => 0, 1, 2, 3, 4;
    6 => 0, 1, 2, 3, 4, 5;
    7 => 0, 1, 2, 3, 4, 5, 6;
    8 => 0, 1, 2, 3, 4, 5, 6, 7;
}

type AnyVars = [Box<dyn any::AnyVar>];

struct MergeVarData<O> {
    inputs: Box<AnyVars>,
    versions: Box<[VarVersionCell]>,

    last_update: Cell<u32>,
    #[allow(clippy::type_complexity)]
    merge: Rc<RefCell<Box<dyn FnMut(&VarsRead, &AnyVars) -> O>>>,

    output_version: Cell<u32>,
    output: UnsafeCell<Option<O>>,
}

/// A [`merge_var!`] that uses dynamic dispatch to support any number of variables.
///
/// This type is a reference-counted pointer ([`Rc`]),
/// it implements the full [`Var`] read and write methods.
///
/// Don't use this type directly use the [`merge_var!`] macro to use the best merge var type,
/// or [`rc_merge_var!`] if you need the `RcMergeVar<O>` type.
pub struct RcMergeVar<O>(Rc<MergeVarData<O>>);
impl<O: VarValue> RcMergeVar<O> {
    #[doc(hidden)]
    pub fn new(inputs: Box<AnyVars>, merge: Box<dyn FnMut(&VarsRead, &AnyVars) -> O>) -> Self {
        RcMergeVar(Rc::new(MergeVarData {
            versions: inputs.iter().map(|_| VarVersionCell::new(0)).collect(),
            inputs,
            merge: Rc::new(RefCell::new(merge)),
            last_update: Cell::new(0),
            output_version: Cell::new(0),
            output: UnsafeCell::new(None),
        }))
    }

    fn update_output(&self, vars: &VarsRead) {
        // SAFETY: This is safe because it only happens before the first borrow
        // of this update, and borrows cannot exist across updates because source
        // vars require a &mut Vars for changing version.

        let first = unsafe { &*self.0.output.get() }.is_none();

        if first || self.0.last_update.get() != vars.update_id() {
            self.0.last_update.set(vars.update_id());

            let mut merge = first;
            for (version, var) in self.0.versions.iter().zip(self.0.inputs.iter()) {
                let new_version = var.version_any(vars);

                if version.get() != new_version {
                    version.set(new_version);
                    merge = true;
                }
            }

            if merge {
                let new_value = (self.0.merge.borrow_mut())(vars, &self.0.inputs);

                unsafe {
                    *self.0.output.get() = Some(new_value);
                }

                self.0.output_version.set(self.0.output_version.get().wrapping_add(1));
            }
        }
    }

    #[doc(hidden)]
    pub fn as_impl_var(self) -> impl Var<O> {
        self
    }
}
impl<O: VarValue> crate::private::Sealed for RcMergeVar<O> {}

impl<O: VarValue> Clone for RcMergeVar<O> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<O: VarValue> Var<O> for RcMergeVar<O> {
    type AsReadOnly = Self;

    type Weak = WeakRcMergeVar<O>;

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a O {
        self.update_output(vars.as_ref());

        // SAFETY:
        // This is safe because we require &mut Vars for updating.
        unsafe { &*self.0.output.get() }.as_ref().unwrap()
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
        vars.with_vars(|vars| self.0.inputs.iter().any(|v| v.is_new_any(vars)))
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|vars| {
            self.update_output(vars);
            VarVersion::normal(self.0.output_version.get())
        })
    }

    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn is_contextual(&self) -> bool {
        self.0.inputs.iter().any(|v| v.is_contextual_any())
    }

    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<O> {
        if self.is_contextual() {
            vars.with_vars(|vars| {
                let value = self.get_clone(vars);
                let var = RcMergeVar(Rc::new(MergeVarData {
                    inputs: self.0.inputs.iter().map(|v| v.actual_var_any(vars)).collect(),
                    merge: self.0.merge.clone(),
                    last_update: self.0.last_update.clone(),
                    versions: self.0.versions.clone(),
                    output_version: self.0.output_version.clone(),
                    output: UnsafeCell::new(Some(value)),
                }));

                var.boxed()
            })
        } else {
            self.clone().boxed()
        }
    }

    fn is_rc(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.inputs.iter().any(|v| v.can_update_any())
    }

    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        vars.with_vars_read(|vars| self.0.inputs.iter().any(|v| v.is_animating_any(vars)))
    }

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> O {
        vars.with_vars_read(|vars| {
            self.update_output(vars);

            match Rc::try_unwrap(self.0) {
                Ok(r) => r.output.into_inner().unwrap(),
                Err(e) => RcMergeVar(e).get_clone(vars),
            }
        })
    }

    fn downgrade(&self) -> Option<Self::Weak> {
        Some(WeakRcMergeVar(Rc::downgrade(&self.0)))
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    fn as_ptr(&self) -> *const () {
        Rc::as_ptr(&self.0) as _
    }

    fn modify<Vw, M>(&self, _: &Vw, _: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(VarModify<O>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|vars| {
            let mut mask = UpdateMask::none();
            for var in self.0.inputs.iter() {
                mask |= var.update_mask_any(vars);
            }
            mask
        })
    }
}
impl<O: VarValue> IntoVar<O> for RcMergeVar<O> {
    type Var = Self;

    fn into_var(self) -> Self {
        self
    }
}
impl<O: VarValue> any::AnyVar for RcMergeVar<O> {
    fn into_any(self) -> Box<dyn any::AnyVar> {
        Box::new(self)
    }

    any_var_impls!(Var);
}

/// A weak reference to a [`RcMergeVar`].
pub struct WeakRcMergeVar<T: VarValue>(Weak<MergeVarData<T>>);
impl<T: VarValue> crate::private::Sealed for WeakRcMergeVar<T> {}
impl<T: VarValue> Clone for WeakRcMergeVar<T> {
    fn clone(&self) -> Self {
        WeakRcMergeVar(self.0.clone())
    }
}
impl<O: VarValue> any::AnyWeakVar for WeakRcMergeVar<O> {
    fn into_any(self) -> Box<dyn any::AnyWeakVar> {
        Box::new(self)
    }

    any_var_impls!(WeakVar);
}
impl<T: VarValue> WeakVar<T> for WeakRcMergeVar<T> {
    type Strong = RcMergeVar<T>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(RcMergeVar)
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr() as *const ()
    }
}

#[doc(hidden)]
pub use zero_ui_proc_macros::merge_var as __merge_var;

/// <span data-del-macro-root></span> Instantiate a [`RcMergeVar`].
///
/// The macro syntax is the same as [`merge_var!`], but outputs a [`RcMergeVar`] instead of
/// an optimized opaque var type.
#[macro_export]
macro_rules! rc_merge_var {
    ($($tt:tt)+) => {
        $crate::var::types::__merge_var! {
            $crate::var,
            $($tt)+
        }
    };
}
#[doc(inline)]
pub use crate::rc_merge_var;

#[doc(hidden)]
pub struct RcMergeVarInput<T: VarValue, V: Var<T>>(PhantomData<(V, T)>);
impl<T: VarValue, V: Var<T>> RcMergeVarInput<T, V> {
    pub fn new(_: &V) -> Self {
        RcMergeVarInput(PhantomData)
    }

    #[allow(clippy::borrowed_box)]
    pub fn get<'a>(&self, var: &'a Box<dyn any::AnyVar>, vars: &'a VarsRead) -> &'a T {
        var.as_any().downcast_ref::<V>().unwrap().get(vars)
    }

    #[allow(clippy::borrowed_box)]
    pub fn is_new(&self, var: &Box<dyn any::AnyVar>, vars: &Vars) -> bool {
        var.as_any().downcast_ref::<V>().unwrap().is_new(vars)
    }
}
