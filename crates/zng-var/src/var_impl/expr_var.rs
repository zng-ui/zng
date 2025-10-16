///<span data-del-macro-root></span> New variable from an expression with interpolated vars.
///
/// # Interpolation
///
/// Other variables can be interpolated by quoting the var with `#{..}`. When
/// an expression contains other interpolated vars the expression var updates when
/// any of the interpolated vars update.
///
/// # Examples
///
/// ```
/// # use zng_var::*;
/// let var_a = var(10);
/// let var_b = var(10);
/// let name = "var_eq";
/// let var_eq = expr_var! {
///     let eq = #{var_a} == #{var_b};
///     println!("{} updated: {} == {}: {}", name, #{var_a}, #{var_b}, eq);
///     eq
/// };
/// ```
///
/// In the example a `var_eq` var of type `Var<bool>` is created. When either `var_a` or `var_b` are set
/// the value of `var_eq` is updated. Normal variables like `name` are moved in, like a closure capture.
///
/// # Capture Mode
///
/// The expression operates like a closure that captures by `move`. Both the interpolated variables and any
/// other `let` binding referenced from the scope are moved into the resulting variable.
///
/// # Interpolation
///
/// Variable interpolation is done by quoting the variable with `#{<var-expr>}`, the braces are required.
///
/// The `<var-expr>` is evaluated before *capturing* starts so if you interpolate `#{var_a.clone()}` `var_a`
/// will still be available after the `expr_var` call. Equal `<var-expr>` only evaluate once.
///
/// # Expansion
///
/// The expression is transformed into different types of vars depending on the number of interpolated variables.
///
/// ##### No Variables
///
/// An expression with no interpolation is simply evaluated into a var using [`IntoVar`].
///
/// ##### Single Variable
///
/// An expression with a single variable is transformed in a [`map`] operation, unless the expression
/// is only the variable without any extra operation.
///
/// ##### Multiple Variables
///
/// An expression with multiple variables is transformed into a [`merge_var!`] call.
///
/// [`Var::get`]: crate::Var::get
/// [`map`]: crate::Var::map
/// [`IntoVar`]: crate::IntoVar
/// [`merge_var!`]: crate::merge_var
#[macro_export]
macro_rules! expr_var {
    ($($expr:tt)+) => {
        $crate::__expr_var! { $crate, $($expr)+ }
    };
}

///<span data-del-macro-root></span> New variable from an expression with interpolated vars that produces another variable.
///
/// This macro is very similar to [`expr_var!`], it just expect an expression that produces another variable and flattens it.
#[macro_export]
macro_rules! flat_expr_var {
    ($($expr:tt)+) => {
        $crate::expr_var! {
            $crate::VarEq($($expr)+)
        }.flatten()
    };
}

#[doc(hidden)]
pub use zng_var_proc_macros::expr_var as __expr_var;

use crate::{IntoVar, MergeInput, Var, VarValue};

#[doc(hidden)]
pub fn expr_var_into<T: VarValue>(expr: impl IntoVar<T>) -> Var<T> {
    expr.into_var()
}

#[doc(hidden)]
pub fn expr_var_as<T: VarValue>(var: impl MergeInput<T>) -> Var<T> {
    var.into_merge_input()
}

#[doc(hidden)]
pub fn expr_var_map<I: VarValue, O: VarValue>(input: impl MergeInput<I>, map: impl FnMut(&I) -> O + Send + 'static) -> Var<O> {
    input.into_merge_input().map(map)
}
