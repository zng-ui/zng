use super::*;

/// Initializes a new conditional var.
///
/// A condition var updates when the first `true` condition changes or the mapped var for the current condition changes.
///
/// # Syntax
///
/// The macro expects a list of `condition-var => condition-value-var`, the list is separated by comma.
/// The first condition must be the `default` keyword that maps to the value for when none of the conditions are `true`.
///
/// The `condition-var` must be an expression that evaluates to an `impl Var<bool>` type. The `condition-value-var` must
/// by any type that implements `IntoVar`. All condition values must be of the same [`VarValue`] type.
///
/// # Example
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) { }
/// let condition = var(true);
/// let when_false = var("condition: false".to_text());
/// 
/// let t = text(when_var! {
///     default => when_false.clone(),
///     condition.clone() => "condition: true".to_text()
/// });
/// ```
///
/// In the example if `condition` or `when_false` are modified the text updates.
#[macro_export]
macro_rules! when_var {
    (
        default => $default:expr,
        $condition:expr => $value:expr $(,)?
    ) => {
        $crate::var::RcWhen1Var::new($default, $condition, $value)
    };
    (
        default => $default:expr,
        $condition0:expr => $value0:expr,
        $condition1:expr => $value1:expr $(,)?
    ) => {
        $crate::var::RcWhen2Var::new($default, ($condition0, $condition1), ($value0, $value1))
    };
    (
        default => $default:expr,
        $condition0:expr => $value0:expr,
        $condition1:expr => $value1:expr,
        $condition2:expr => $value2:expr $(,)?
    ) => {
        $crate::var::RcWhen2Var::new(
            $default, 
            ($condition0, $condition1, $condition2), 
            ($value0, $value1, $value2)
        )
    };
    (
        default => $default:expr,
        $condition0:expr => $value0:expr,
        $condition1:expr => $value1:expr,
        $condition2:expr => $value2:expr, 
        $condition3:expr => $value3:expr 
        $(,)?
    ) => {
        $crate::var::RcWhen2Var::new(
            $default, 
            ($condition0, $condition1, $condition2, $condition3), 
            ($value0, $value1, $value2, $value3)
        )
    };
    (
        default => $default:expr,
        $condition0:expr => $value0:expr,
        $condition1:expr => $value1:expr,
        $condition2:expr => $value2:expr, 
        $condition3:expr => $value3:expr, 
        $condition4:expr => $value4:expr 
        $(,)?
    ) => {
        $crate::var::RcWhen2Var::new(
            $default, 
            ($condition0, $condition1, $condition2, $condition3, $condition4), 
            ($value0, $value1, $value2, $value3, , $value4)
        )
    };
    (
        default => $default:expr,
        $condition0:expr => $value0:expr,
        $condition1:expr => $value1:expr,
        $condition2:expr => $value2:expr, 
        $condition3:expr => $value3:expr, 
        $condition4:expr => $value4:expr,
        $condition5:expr => $value5:expr
        $(,)?
    ) => {
        $crate::var::RcWhen2Var::new(
            $default, 
            ($condition0, $condition1, $condition2, $condition3, $condition4, $condition5), 
            ($value0, $value1, $value2, $value3, , $value4, $value5)
        )
    };
    (
        default => $default:expr,
        $condition0:expr => $value0:expr,
        $condition1:expr => $value1:expr,
        $condition2:expr => $value2:expr, 
        $condition3:expr => $value3:expr, 
        $condition4:expr => $value4:expr,
        $condition5:expr => $value5:expr,
        $condition6:expr => $value6:expr
        $(,)?
    ) => {
        $crate::var::RcWhen2Var::new(
            $default, 
            ($condition0, $condition1, $condition2, $condition3, $condition4, $condition5, $condition6), 
            ($value0, $value1, $value2, $value3, , $value4, $value5, $value6)
        )
    };
    (
        default => $default:expr,
        $condition0:expr => $value0:expr,
        $condition1:expr => $value1:expr,
        $condition2:expr => $value2:expr, 
        $condition3:expr => $value3:expr, 
        $condition4:expr => $value4:expr,
        $condition5:expr => $value5:expr,
        $condition7:expr => $value7:expr
        $(,)?
    ) => {
        $crate::var::RcWhen2Var::new(
            $default, 
            ($condition0, $condition1, $condition2, $condition3, $condition4, $condition5, $condition6, $condition7), 
            ($value0, $value1, $value2, $value3, , $value4, $value5, $value6, $value7)
        )
    };
    (
        default => $default:expr,
        $condition0:expr => $value0:expr,
        $($condition:expr => $value:expr),+ $(,)?
    ) => {
        // we need a builder to have $value be IntoVar and work like the others.
        $crate::var::RcWhenVarBuilder::new($default, $condition0, $value0)
        $(.push($condition, $value))+
        .build()
    };
}

#[doc(hidden)]
pub struct RcWhen1Var<O: VarValue, D: VarObj<O>, C0: VarObj<bool>, V0: VarObj<O>>(Rc<RcWhen1VarData<O, D, C0, V0>>);
struct RcWhen1VarData<O: VarValue, D: VarObj<O>, C0: VarObj<bool>, V0: VarObj<O>> {
    _o: PhantomData<O>,

    default_: D,
    default_ver: Cell<u32>,

    condition: C0,
    condition_ver: Cell<u32>,

    value: V0,
    value_ver: Cell<u32>,

    self_version: Cell<u32>,
}

impl<O: VarValue, D: Var<O>, C0: Var<bool>, V0: Var<O>> RcWhen1Var<O, D, C0, V0> {
    pub fn new<ID: IntoVar<O, Var = D>, IV0: IntoVar<O, Var = V0>>(default_: ID, condition: C0, value: IV0) -> Self {
        RcWhen1Var(Rc::new(RcWhen1VarData {
            _o: PhantomData,

            default_: default_.into_var(),
            default_ver: Cell::new(0),

            condition,
            condition_ver: Cell::new(0),

            value: value.into_var(),
            value_ver: Cell::new(0),

            self_version: Cell::new(0),
        }))
    }
}
impl<O: VarValue, D: VarObj<O>, C0: VarObj<bool>, V0: VarObj<O>> protected::Var for RcWhen1Var<O, D, C0, V0> {}
impl<O: VarValue, D: VarObj<O>, C0: VarObj<bool>, V0: VarObj<O>> Clone for RcWhen1Var<O, D, C0, V0> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<O: VarValue, D: VarObj<O>, C0: VarObj<bool>, V0: VarObj<O>> VarObj<O> for RcWhen1Var<O, D, C0, V0> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        if *self.0.condition.get(vars) {
            self.0.value.get(vars)
        } else {
            self.0.default_.get(vars)
        }
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        if let Some(&c) = self.0.condition.get_new(vars) {
            if c {
                Some(self.0.value.get(vars))
            } else {
                Some(self.0.default_.get(vars))
            }
        } else if *self.0.condition.get(vars) {
            self.0.value.get_new(vars)
        } else {
            self.0.default_.get_new(vars)
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.condition.is_new(vars)
            || if *self.0.condition.get(vars) {
                self.0.value.is_new(vars)
            } else {
                self.0.default_.is_new(vars)
            }
    }

    fn version(&self, vars: &Vars) -> u32 {
        let c_ver = self.0.condition.version(vars);
        let d_ver = self.0.default_.version(vars);
        let v_ver = self.0.value.version(vars);

        if c_ver != self.0.condition_ver.get() || d_ver != self.0.default_ver.get() || v_ver != self.0.value_ver.get() {
            self.0.self_version.set(self.0.self_version.get().wrapping_add(1));
            self.0.condition_ver.set(c_ver);
            self.0.default_ver.set(d_ver);
            self.0.value_ver.set(v_ver);
        }

        self.0.self_version.get()
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        if *self.0.condition.get(vars) {
            self.0.value.is_read_only(vars)
        } else {
            self.0.default_.is_read_only(vars)
        }
    }

    fn always_read_only(&self) -> bool {
        self.0.default_.always_read_only() && self.0.value.always_read_only()
    }

    fn can_update(&self) -> bool {
        // technically not always true but this is only a hint.
        true
    }

    fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
        if *self.0.condition.get(vars) {
            self.0.value.set(vars, new_value)
        } else {
            self.0.default_.set(vars, new_value)
        }
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        if *self.0.condition.get(vars) {
            self.0.value.modify_boxed(vars, change)
        } else {
            self.0.default_.modify_boxed(vars, change)
        }
    }
}
impl<O: VarValue, D: Var<O>, C0: VarObj<bool>, V0: Var<O>> Var<O> for RcWhen1Var<O, D, C0, V0> {
    type AsReadOnly = ForceReadOnlyVar<O, Self>;
    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<F: FnOnce(&mut O) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        if *self.0.condition.get(vars) {
            self.0.value.modify(vars, change)
        } else {
            self.0.default_.modify(vars, change)
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

    fn map_bidi_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static, G: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        &self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<O, O2, Self, F, G> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map<O2: VarValue, F: FnMut(&O) -> O2 + 'static>(self, map: F) -> RcMapVar<O, O2, Self, F> {
        RcMapVar::new(self, map)
    }

    fn into_map_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static>(self, map: F) -> MapRefVar<O, O2, Self, F> {
        MapRefVar::new(self, map)
    }

    fn into_map_bidi<O2: VarValue, F: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_bidi_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static, G: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<O, O2, Self, F, G> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

macro_rules! impl_rc_when_var {
    ($(
        $len:tt => $($n:tt),+ <|> $($n_rev:tt),+;
    )+) => {$(
        $crate::paste!{
            impl_rc_when_var!{
                Var: [<RcWhen $len Var>];// RcWhen2Var
                Data: [<RcWhen $len VarData>];// RcWhen2VarData
                len: $len;//2
                C: $([<C $n>]),+;// C0, C1
                V: $([<V $n>]),+;// V0, V1
                IV: $([<IV $n>]),+;// IV0, IV1
                n: $($n),+; // 0, 1
                n_rev: $($n_rev),+; // 1, 0
            }
        }
    )+};
    (
        Var: $RcMergeVar:ident;
        Data: $RcMergeVarData:ident;
        len: $len:tt;
        C: $($C:ident),+;
        V: $($V:ident),+;
        IV: $($IV:ident),+;
        n: $($n:tt),+;
        n_rev: $($n_rev:tt),+;
    ) => {
        #[doc(hidden)]
        pub struct $RcMergeVar<O: VarValue, D: Var<O>, $($C),+ , $($V),+>(Rc<$RcMergeVarData<O, D, $($C),+ , $($V),+>>);
        struct $RcMergeVarData<O: VarValue, D: Var<O>, $($C),+ , $($V),+> {
            _o: PhantomData<O>,
            
            default_value: D,
            default_version: Cell<u32>,

            conditions: ( $($C),+ ),
            condition_versions: [Cell<u32>; $len],

            values: ( $($V),+ ),
            value_versions: [Cell<u32>; $len],

            self_version: Cell<u32>,
        }
        impl<O: VarValue, D: Var<O>, $($C: Var<bool>),+ , $($V: Var<O>),+> $RcMergeVarData<O, D, $($C),+ , $($V),+> {
            pub fn new<ID: IntoVar<O, Var=D>, $($IV : IntoVar<O, Var=$V>),+>(default_: ID, conditions: ($($C),+), values: ($($IV),+)) -> Self {
                Self {
                    _o: PhantomData,

                    default_value: default_.into_var(),
                    default_version: Cell::new(0),

                    conditions,
                    condition_versions: array_init::array_init(|_|Cell::new(0)),

                    values: ($(values.$n.into_var()),+),
                    value_versions: array_init::array_init(|_|Cell::new(0)),
                
                    self_version: Cell::new(0),
                }
            }
        }
    };
}
impl_rc_when_var! {
    2 => 0, 1 <|> 1, 0;
    3 => 0, 1, 2 <|> 2, 1, 0;
    4 => 0, 1, 2, 3 <|> 3, 2, 1, 0;
    5 => 0, 1, 2, 3, 4 <|> 5, 3, 2, 1, 0;
    6 => 0, 1, 2, 3, 4, 5 <|> 5, 4, 3, 2, 1, 0;
    7 => 0, 1, 2, 3, 4, 5, 6 <|> 6, 5, 4, 3, 2, 1, 0;
    8 => 0, 1, 2, 3, 4, 5, 6, 7 <|> 7, 6, 5, 4, 3, 2, 1, 0;
}

pub struct RcWhenVar<O: VarValue>(Rc<RcWhenVarData<O>>);
struct RcWhenVarData<O: VarValue> {
    default_: BoxedVar<O>,
    default_version: Cell<u32>,

    whens: Box<[(BoxedVar<bool>, BoxedVar<O>)]>,
    when_versions: Box<[(Cell<u32>, Cell<u32>)]>,

    self_version: Cell<u32>,
}
impl<O: VarValue> RcWhenVar<O> {
    pub fn new(default_: BoxedVar<O>, whens: Box<[(BoxedVar<bool>, BoxedVar<O>)]>) -> Self {
        RcWhenVar(Rc::new(RcWhenVarData {
            default_,
            default_version: Cell::new(0),

            when_versions: whens.iter().map(|_|(Cell::new(0), Cell::new(0))).collect(),
            whens,
            
            self_version: Cell::new(0),
        }))
    }
}
impl<O: VarValue> protected::Var for RcWhenVar<O> { }
impl<O: VarValue> Clone for RcWhenVar<O> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<O: VarValue> VarObj<O> for RcWhenVar<O> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        for (c, v) in self.0.whens.iter().rev() {
            if *c.get(vars) {
                return v.get(vars);
            }
        }
        self.0.default_.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        for (c, v) in self.0.whens.iter().rev() {
            if *c.get(vars) {
                if c.is_new(vars) {
                    return Some(v.get(vars));
                } else {
                    return v.get_new(vars);
                }
            }
        }
        self.0.default_.get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        for (c, v) in self.0.whens.iter().rev() {
            if *c.get(vars) {
                return c.is_new(vars) || v.is_new(vars);
            }
        }
        self.0.default_.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        let mut changed = false;

        let dv = self.0.default_.version(vars);
        if dv != self.0.default_version.get() {
            changed = true;
            self.0.default_version.set(dv);
        }

        for ((c, v), (w_cv, w_vv)) in self.0.whens.iter().zip(self.0.when_versions.iter()) {
            let cv = c.version(vars);
            if cv != w_cv.get() {
                changed = true;
                w_cv.set(cv);
            }
            let vv = v.version(vars);
            if vv != w_vv.get() {
                changed = true;
                w_vv.set(vv);
            }
        }

        if changed {
            self.0.self_version.set(self.0.self_version.get().wrapping_add(1));
        }

        self.0.self_version.get()
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        for (c, v) in self.0.whens.iter().rev() {
            if *c.get(vars) {
                return v.is_read_only(vars);
            }
        }
        self.0.default_.is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.0.whens.iter().all(|(_, v)|v.always_read_only())
    }

    fn can_update(&self) -> bool {
        true
    }

    fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
        for (c, v) in self.0.whens.iter().rev() {
            if *c.get(vars) {
                return v.set(vars, new_value);
            }
        }
        self.0.default_.set(vars, new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        for (c, v) in self.0.whens.iter().rev() {
            if *c.get(vars) {
                return v.modify_boxed(vars, change);
            }
        }
        self.0.default_.modify_boxed(vars, change)
    }
}
impl<O: VarValue> Var<O> for RcWhenVar<O> {
    type AsReadOnly = ForceReadOnlyVar<O, Self>;
    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<F: FnOnce(&mut O) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        self.modify_boxed(vars, Box::new(change))
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

    fn map_bidi_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static, G: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        &self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<O, O2, Self, F, G> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map<O2: VarValue, F: FnMut(&O) -> O2 + 'static>(self, map: F) -> RcMapVar<O, O2, Self, F> {
        RcMapVar::new(self, map)
    }

    fn into_map_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static>(self, map: F) -> MapRefVar<O, O2, Self, F> {
        MapRefVar::new(self, map)
    }

    fn into_map_bidi<O2: VarValue, F: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_bidi_ref<O2: VarValue, F: Fn(&O) -> &O2 + Clone + 'static, G: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<O, O2, Self, F, G> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

#[doc(hidden)]
pub struct RcWhenVarBuilder<O: VarValue> {
    default_: BoxedVar<O>,
    whens: Vec<(BoxedVar<bool>, BoxedVar<O>)>
}
impl<O: VarValue> RcWhenVarBuilder<O> {
    pub fn new<D: IntoVar<O>, C0: Var<bool>, V0: IntoVar<O>>(default_ : D, condition0: C0, value0: V0) -> Self {
        Self {
            default_: default_.into_var().boxed(),
            whens: vec![(condition0.boxed(), value0.into_var().boxed())]
        }
    }

    pub fn push<C: Var<bool>, V: IntoVar<O>>(mut self, condition: C, value: V) -> Self {
        self.whens.push((condition.boxed(), value.into_var().boxed()));
        self
    }

    pub fn build(self) -> RcWhenVar<O> {
        RcWhenVar::new(self.default_, self.whens.into_boxed_slice())
    }
}