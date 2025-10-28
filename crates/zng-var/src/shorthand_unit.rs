// Declared in `zng-var` to avoid having to create a `zng-unit-proc-macros`.

use std::fmt;

use zng_txt::Txt;

#[doc(hidden)]
pub use zng_var_proc_macros::shorthand_unit;

/// Represents a shorthand unit that converts to a property input type.
///
/// In property assigns you can set `ident!` to use a shorthand. In other code you can
/// use this macro `shorthand_unit![ident]`. This macro expands to both the shorthand type and unit instance
/// so you may use it when implementing conversions too.
///
/// # Why
///
/// There are many properties with many input types, having to import every type or using the full type path for
/// every property assign is tedius. Numeric and boolean related types implement conversions from primitives and
/// tuples, are easy to set and look neat. Enums and types with associated consts and functions can implement conversions
/// from shorthand units to achieve the same effect.
///
/// # Guidelines
///
/// A shorthand unit must always match the ident (name) of an associated item from the target type. If there was a
/// way to enforce this with proc-macros, if would be a compile error if the unit was not case sensitive equal
/// to an associated const, variant or function ident.
///
/// The shorthand unit must be documented in the associated item of the same ident. The idea is for users to begin
/// using the full type path and then discover the shorthand.
///
/// The shorthand conversions must be `doc(hidden)`. The expanded unit type can pollute the impl section in the docs page. The
/// [`impl_from_and_into_var!`] macro automatically hides generated its impls.
#[macro_export]
macro_rules! ShorthandUnit {
    ($ident:ident) => {
        // needs to be a proc-macro, we cal pass the type path.
        $crate::shorthand_unit!($crate::ShorthandUnit, $ident)
    };
}

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ShorthandUnit<
    const C00: char,
    const C01: char = '\0',
    const C02: char = '\0',
    const C03: char = '\0',
    const C04: char = '\0',
    const C05: char = '\0',
    const C06: char = '\0',
    const C07: char = '\0',
    const C08: char = '\0',
    const C09: char = '\0',
    const C10: char = '\0',
    const C11: char = '\0',
    const C12: char = '\0',
    const C13: char = '\0',
    const C14: char = '\0',
    const C15: char = '\0',
    const C16: char = '\0',
    const C17: char = '\0',
    const C18: char = '\0',
    const C19: char = '\0',
    const C20: char = '\0',
    const C21: char = '\0',
    const C22: char = '\0',
    const C23: char = '\0',
    const C24: char = '\0',
    const C25: char = '\0',
    const C26: char = '\0',
    const C27: char = '\0',
    const C28: char = '\0',
    const C29: char = '\0',
    const C30: char = '\0',
    const C31: char = '\0',
>;
macro_rules! impl_ShorthandUnit {
    ($($trait:path)? { $($impl:tt)* }) => {
impl<
    const C00: char,
    const C01: char,
    const C02: char,
    const C03: char,
    const C04: char,
    const C05: char,
    const C06: char,
    const C07: char,
    const C08: char,
    const C09: char,
    const C10: char,
    const C11: char,
    const C12: char,
    const C13: char,
    const C14: char,
    const C15: char,
    const C16: char,
    const C17: char,
    const C18: char,
    const C19: char,
    const C20: char,
    const C21: char,
    const C22: char,
    const C23: char,
    const C24: char,
    const C25: char,
    const C26: char,
    const C27: char,
    const C28: char,
    const C29: char,
    const C30: char,
    const C31: char,
> $($trait for)?
    ShorthandUnit<
        C00,
        C01,
        C02,
        C03,
        C04,
        C05,
        C06,
        C07,
        C08,
        C09,
        C10,
        C11,
        C12,
        C13,
        C14,
        C15,
        C16,
        C17,
        C18,
        C19,
        C20,
        C21,
        C22,
        C23,
        C24,
        C25,
        C26,
        C27,
        C28,
        C29,
        C30,
        C31,
    >
{
    $($impl)*
}

    };
}
impl_ShorthandUnit!({
    /// Gets the shorthand value as text.
    ///
    /// Does not include the trailing `!`, the `fmt::Display` printer includes it.
    pub fn ident(self) -> Txt {
        let r: String = [
            C00, C01, C02, C03, C04, C05, C06, C07, C08, C09, C10, C11, C12, C13, C14, C15, C16, C17, C18, C19, C20, C21, C22, C23, C24,
            C25, C26, C27, C28, C29, C30, C31,
        ]
        .into_iter()
        .take_while(|&c| c != '\0')
        .collect();
        r.into()
    }
});
impl_ShorthandUnit!(fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}!", self.ident())
    }
});
impl_ShorthandUnit!(fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ShorthandUnit![{}]", self.ident())
        } else {
            write!(f, "{}!", self.ident())
        }
    }
});
