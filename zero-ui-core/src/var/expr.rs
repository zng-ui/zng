///<span data-del-macro-root></span> New variable from an expression with interpolated *vars*.
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
/// # use zero_ui_core::var::*;
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
/// In the example a `var_eq` of type `impl Var<bool>` is created. When either `var_a` or `var_b` are set
/// the value of `var_eq` is updated on the next read. Normal variables like `name` are moved in, like a closure capture.
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
/// The interpolation result value is the [`Var::get`] return value.
///
/// # Expansion
///
/// The expression is transformed into different types of vars depending on the number of interpolated variables.
///
/// ## No Variables
///
/// An expression with no interpolation is simply evaluated into a var using [`IntoVar`].
///
/// # Single Variable
///
/// An expression with a single variable is transformed in a [`map`] operation, unless the expression
/// is only the variable without any extra operation.
///
/// # Multiple Variables
///
/// An expression with multiple variables is transformed into a [`merge_var!`] call.
/// 
/// [`Var::get`]: crate::var::Var::get
/// [`map`]: crate::var::Var::map
/// [`IntoVar`]: crate::var::IntoVar
/// [`merge_var!`]: crate::var::merge_var
#[macro_export]
macro_rules! expr_var {
    ($($expr:tt)+) => {
        $crate::var::types::__expr_var! { $crate::var, $($expr)+ }
    };
}
#[doc(inline)]
pub use crate::expr_var;

#[doc(hidden)]
pub use zero_ui_proc_macros::expr_var as __expr_var;
