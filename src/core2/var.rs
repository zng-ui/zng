use super::UpdateNotice;
use std::cell::Ref;
use std::ops::Deref;

mod private {
    pub trait Sealed {}
}

/// Abstraction over a direct owned `T` or an `UpdateNotice<T>`.
pub trait Var<T>: private::Sealed {
    type RefType: for<'a> VarRefType<'a, T>;

    /// Borrows the value. Returns `&T` when owned or `Ref<T>` when it is an update notice.
    fn borrow(&self) -> <Self::RefType as VarRefType<'_, T>>::Type;

    /// If the value was just updated. Always false if owned or the same as [UpdateNotice::is_new].
    fn is_new(&self) -> bool;
}

#[doc(hidden)]
pub trait VarRefType<'a, T: 'a> {
    type Type: Deref<Target = T>;
}

/// A `[Var<T>]` that onwns the value.
pub struct OwnedVar<T>(pub T);

impl<'a, T: 'a> VarRefType<'a, T> for OwnedVar<T> {
    type Type = &'a T;
}

impl<'a, T: 'a> VarRefType<'a, T> for UpdateNotice<T> {
    type Type = Ref<'a, T>;
}

impl<T> private::Sealed for OwnedVar<T> {}
impl<T: 'static> Var<T> for OwnedVar<T> {
    type RefType = Self;

    fn borrow(&self) -> &T {
        &self.0
    }

    fn is_new(&self) -> bool {
        false
    }
}

impl<T> private::Sealed for UpdateNotice<T> {}
impl<T: 'static> Var<T> for UpdateNotice<T> {
    type RefType = Self;

    fn borrow(&self) -> Ref<T> {
        UpdateNotice::last_update(self).expect("no `last_update` found in `Var` context")
    }

    fn is_new(&self) -> bool {
        UpdateNotice::is_new(self)
    }
}
