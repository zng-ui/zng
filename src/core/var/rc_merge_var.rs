use super::*;

pub use zero_ui_macros::merge_var;

macro_rules! impl_rc_merge_var {
    ($(
        $len:tt => $($n:tt),+;
    )+) => {$(
        paste::paste!{
            impl_rc_merge_var!{
                Var: [<RcMerge $len Var>];// RcMerge2Var
                Data: [<RcMerge $len VarData>];// RcMerge2VarData
                len: $len;//2
                I: $([<I $n>]),+;// I0, I1
                V: $([<V $n>]),+;// V0, V1
                n: $($n),+; // 0, 1
            }
        }
    )+};

    (
        Var: $RcMergeVar:ident;
        Data: $RcMergeVarData:ident;
        len: $len:tt;
        I: $($I:ident),+;
        V: $($V:ident),+;
        n: $($n:tt),+;
    ) => {
        #[doc(hidden)]
        pub struct $RcMergeVar<$($I: VarValue,)+ O: VarValue, $($V: VarObj<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>(
            Rc<$RcMergeVarData<$($I,)+ O, $($V,)+ F>>,
        );

        struct $RcMergeVarData<$($I: VarValue,)+ O: VarValue, $($V: VarObj<$I>,)+ F: FnMut($(&$I),+) -> O + 'static> {
            _i: PhantomData<($($I),+)>,
            vars: ($($V),+),
            f: RefCell<F>,
            versions: [Cell<u32>; $len],
            output_version: Cell<u32>,
            output: UnsafeCell<MaybeUninit<O>>, // TODO: Need to manually drop?
            last_update_id: Cell<Option<u32>>,
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: VarObj<$I>,)+ F: FnMut($(&$I),+) -> O + 'static> $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            pub fn new(vars: ($($V),+), f: F) -> Self {
                Self(Rc::new($RcMergeVarData {
                    _i: PhantomData,
                    vars,
                    f: RefCell::new(f),
                    versions: array_init::array_init(|_|Cell::new(0)),
                    output_version: Cell::new(0),
                    output: UnsafeCell::new(MaybeUninit::uninit()),
                    last_update_id: Cell::new(None),
                }))
            }

            pub fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
                <Self as VarObj<O>>::get(self, vars)
            }

            pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
                <Self as VarObj<O>>::get_new(self, vars)
            }

            pub fn is_new(&self, vars: &Vars) -> bool {
                <Self as VarObj<O>>::is_new(self, vars)
            }

            pub fn version(&self, vars: &Vars) -> u32 {
                <Self as VarObj<O>>::version(self, vars)
            }

            pub fn can_update(&self) -> bool {
                <Self as VarObj<O>>::can_update(self)
            }

            fn output_uninit(&self) -> bool {
                self.0.last_update_id.get().is_none()
            }

            fn update_output(&self, vars: &Vars) {
                let last_update_id = Some(vars.update_id());
                if self.0.last_update_id.get() != last_update_id {
                    let versions = ($(self.0.vars.$n.version(vars)),+);
                    if $(self.0.versions[$n].get() != versions.$n)||+ || self.output_uninit() {
                        let value = (&mut *self.0.f.borrow_mut())($(self.0.vars.$n.get(vars)),+);

                        // SAFETY: This is safe because it only happens before the first borrow
                        // of this update, and borrows cannot exist across updates because source
                        // vars require a &mut Vars for changing version.
                        unsafe {
                            let m_uninit = &mut *self.0.output.get();
                            m_uninit.as_mut_ptr().write(value);
                        }

                        self.0.output_version.set(self.0.output_version.get().wrapping_add(1));
                        $(self.0.versions[$n].set(versions.$n);)+
                    }
                    self.0.last_update_id.set(last_update_id);
                }
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: VarObj<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Clone for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            fn clone(&self) -> Self {
                $RcMergeVar(Rc::clone(&self.0))
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: VarObj<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        protected::Var for $RcMergeVar<$($I,)+ O, $($V,)+ F> { }

        impl<$($I: VarValue,)+ O: VarValue, $($V: VarObj<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        VarObj<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
                self.update_output(vars);

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
                $(self.0.vars.$n.is_new(vars))||+
            }

            fn version(&self, vars: &Vars) -> u32 {
                self.update_output(vars);
                self.0.output_version.get()
            }

            fn is_read_only(&self, _: &Vars) -> bool {
                true
            }

            fn always_read_only(&self) -> bool {
                true
            }

            fn can_update(&self) -> bool {
                $(self.0.vars.$n.can_update())||+
            }

            fn set(&self, _: &Vars, _: O) -> Result<(), VarIsReadOnly> {
                Err(VarIsReadOnly)
            }

            fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
                Err(VarIsReadOnly)
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: VarObj<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Var<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
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

        impl<$($I: VarValue,)+ O: VarValue, $($V: VarObj<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        IntoVar<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            type Var = Self;
            fn into_var(self) -> Self {
                self
            }
        }
    };
}

impl_rc_merge_var! {
    2 => 0, 1;
    3 => 0, 1, 2;
    4 => 0, 1, 2, 3;
    5 => 0, 1, 2, 3, 4;
    6 => 0, 1, 2, 3, 4, 5;
    7 => 0, 1, 2, 3, 4, 5, 6;
    8 => 0, 1, 2, 3, 4, 5, 6, 7;
    9 => 0, 1, 2, 3, 4, 5, 6, 7, 8;

    10 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9;
    11 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10;
    12 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11;
    13 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12;
    14 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13;
    15 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14;
    16 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15;
    17 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16;
    18 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17;
    19 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18;

    20 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19;
    21 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20;
    22 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21;
    23 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22;
    24 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23;
    25 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24;
    26 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25;
    27 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26;
    28 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27;
    29 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28;

    30 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29;
    31 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30;
    32 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31;
}

/* TODO
fn merge_20_test<V: Var<u8>>(
    v0: V,
    v1: V,
    v2: V,
    v3: V,
    v4: V,
    v5: V,
    v6: V,
    v7: V,
    v8: V,
    v9: V,

    v10: V,
    v11: V,
    v12: V,
    v13: V,
    v14: V,
    v15: V,
    v16: V,
    v17: V,
    v18: V,
    v19: V,
) -> impl Var<usize> {
    let a = RcMerge16Var::new(
        (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15),
        |v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15| {
            (
                v0.clone(),
                v1.clone(),
                v2.clone(),
                v3.clone(),
                v4.clone(),
                v5.clone(),
                v6.clone(),
                v7.clone(),
                v8.clone(),
                v9.clone(),
                v10.clone(),
                v11.clone(),
                v12.clone(),
                v13.clone(),
                v14.clone(),
                v15.clone(),
            )
        },
    );

    RcMerge5Var::new(
        (a, v16, v17, v18, v19),
        |(v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15), v16, v17, v18, v19| {
            [
                v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15, v16, v17, v18, v19,
            ]
            .iter()
            .map(|u| u as usize)
            .sum()
        },
    )
}
*/
