use super::*;

use std::cell::{Cell, RefCell, UnsafeCell};
use std::marker::PhantomData;
use std::rc::Rc;

///<span data-inline></span> Initializes a new [`Var`](crate::var::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](crate::var::Var), minimal 2.
/// * `merge`: A function that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Example
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) {  }
/// let var0: RcVar<Text> = var_from("Hello");
/// let var1: RcVar<Text> = var_from("World");
///
/// let greeting_text = text(merge_var!(var0, var1, |a, b|formatx!("{} {}!", a, b)));
/// ```
#[macro_export]
macro_rules! merge_var {
    ($v0: expr, $v1: expr, $merge: expr) => {
        $crate::var::RcMerge2Var::new(($v0, $v1), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $merge: expr) => {
        $crate::var::RcMerge3Var::new(($v0, $v1, $v2), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $merge: expr) => {
        $crate::var::RcMerge4Var::new(($v0, $v1, $v2, $v3), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $merge: expr) => {
        $crate::var::RcMerge5Var::new(($v0, $v1, $v2, $v3, $v4), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $merge: expr) => {
        $crate::var::RcMerge6Var::new(($v0, $v1, $v2, $v3, $v4, $v5), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $merge: expr) => {
        $crate::var::RcMerge7Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $merge: expr) => {
        $crate::var::RcMerge8Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $merge: expr) => {
        $crate::var::RcMerge9Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr, $merge: expr) => {
        $crate::var::RcMerge10Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9), $merge)
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge11Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10), $merge)
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge12Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11), $merge)
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge13Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12), $merge)
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge14Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13), $merge)
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge15Var::new(
            ($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge16Var::new(
            ($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge17Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge18Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge19Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge20Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge21Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge22Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge23Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge24Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge25Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23, $v24,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr, $v25: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge26Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23, $v24, $v25,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr, $v25: expr, $v26: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge27Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23, $v24, $v25, $v26,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr, $v25: expr, $v26: expr, $v27: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge28Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23, $v24, $v25, $v26, $v27,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr, $v25: expr, $v26: expr, $v27: expr,
        $v28: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge29Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23, $v24, $v25, $v26, $v27, $v28,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr, $v25: expr, $v26: expr, $v27: expr,
        $v28: expr, $v29: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge30Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23, $v24, $v25, $v26, $v27, $v28, $v29,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr, $v25: expr, $v26: expr, $v27: expr,
        $v28: expr, $v29: expr, $v30: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge31Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23, $v24, $v25, $v26, $v27, $v28, $v29, $v30,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr, $v25: expr, $v26: expr, $v27: expr,
        $v28: expr, $v29: expr, $v30: expr, $v31: expr,
        $merge: expr
    ) => {
        $crate::var::RcMerge32Var::new(
            (
                $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9, $v10, $v11, $v12, $v13, $v14, $v15, $v16, $v17, $v18, $v19, $v20, $v21,
                $v22, $v23, $v24, $v25, $v26, $v27, $v28, $v29, $v30, $v31,
            ),
            $merge,
        )
    };
    (
        $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $v9: expr,
        $v10: expr, $v11: expr, $v12: expr, $v13: expr, $v14: expr, $v15: expr, $v16: expr, $v17: expr, $v18: expr,
        $v19: expr, $v20: expr, $v21: expr, $v22: expr, $v23: expr, $v24: expr, $v25: expr, $v26: expr, $v27: expr,
        $v28: expr, $v29: expr, $v30: expr, $v31: expr, $v32: expr,
        $merge: expr
    ) => {
        compile_error!("merge_var is only implemented to a maximum of 32 variables")
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (var0, var1, .., merge_fn")
    };
}
#[doc(inline)]
pub use crate::merge_var;

macro_rules! impl_rc_merge_var {
    ($(
        $len:tt => $($n:tt),+;
    )+) => {$(
        $crate::paste!{
            impl_rc_merge_var!{
                Var: [<RcMerge $len Var>];// RcMerge2Var
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

        struct $RcMergeVarData<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static> {
            _i: PhantomData<($($I),+)>,
            vars: ($($V),+),
            f: RefCell<F>,
            versions: [Cell<u32>; $len],
            output_version: Cell<u32>,
            output: UnsafeCell<Option<O>>,
            last_update_id: Cell<u32>,
        }

        #[allow(missing_docs)]// this is all hidden.
        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static> $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            pub fn new(vars: ($($V),+), f: F) -> Self {
                Self(Rc::new($RcMergeVarData {
                    _i: PhantomData,
                    vars,
                    f: RefCell::new(f),
                    versions: array_init::array_init(|_|Cell::new(0)),
                    output_version: Cell::new(0),
                    output: UnsafeCell::new(None),
                    last_update_id: Cell::new(0),
                }))
            }

            fn update_output(&self, vars: &VarsRead) {
                // SAFETY: This is safe because it only happens before the first borrow
                // of this update, and borrows cannot exist across updates because source
                // vars require a &mut Vars for changing version.

                let update_id = vars.update_id();
                let mut update_output = unsafe { &*self.0.output.get() }.is_none();
                if update_output || self.0.last_update_id.get() != update_id {
                    self.0.last_update_id.set(update_id);

                    let versions = ($(self.0.vars.$n.version(vars)),+);
                    update_output |= $(self.0.versions[$n].get() != versions.$n)||+;

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
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Clone for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            fn clone(&self) -> Self {
                $RcMergeVar(Rc::clone(&self.0))
            }
        }
        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        crate::private::Sealed for $RcMergeVar<$($I,)+ O, $($V,)+ F> {}

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Var<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            type AsReadOnly = ReadOnlyVar<O, Self>;

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

            #[inline]
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


            #[inline]
            fn strong_count(&self) -> usize {
                Rc::strong_count(&self.0)
            }

            fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
               vars.with_vars_read(|vars| {
                    self.update_output(vars);
                    self.0.output_version.get()
               })
            }

            #[inline]
            fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
                true
            }

            #[inline]
            fn always_read_only(&self) -> bool {
                true
            }

            fn can_update(&self) -> bool {
                $(self.0.vars.$n.can_update())||+
            }

            #[inline]
            fn set<Vw: WithVars, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly> where N: Into<O> {
                Err(VarIsReadOnly)
            }

            #[inline]
            fn set_ne<Vw: WithVars, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>  where N: Into<O>, O: PartialEq {
                Err(VarIsReadOnly)
            }

            #[inline]
            fn modify<Vw: WithVars, F2: FnOnce(&mut VarModify<O>) + 'static>(&self, _: &Vw, _: F2) -> Result<(), VarIsReadOnly> {
                Err(VarIsReadOnly)
            }

            #[inline]
            fn into_read_only(self) -> Self::AsReadOnly {
                ReadOnlyVar::new(self)
            }

            #[inline]
            fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
                vars.with_vars_read(|vars| {
                    let mut r = UpdateMask::none();
                    $(r |= self.0.vars.$n.update_mask(vars);)+
                    r
                })
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        IntoVar<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            type Var = Self;
            fn into_var(self) -> Self {
                self
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
