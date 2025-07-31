use std::{
    any::{Any, TypeId},
    fmt, ops,
    sync::Arc,
};

use smallbox::*;

/// Small box for [`AnyVarValue`] values.
pub struct BoxAnyVarValue(SmallBox<dyn AnyVarValue, space::S4>);
impl ops::Deref for BoxAnyVarValue {
    type Target = dyn AnyVarValue;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl ops::DerefMut for BoxAnyVarValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}
impl BoxAnyVarValue {
    /// Box `value`.
    pub fn new(value: impl AnyVarValue) -> Self {
        BoxAnyVarValue(smallbox!(value))
    }

    /// Downcast to value.
    pub fn downcast<T: VarValue>(self) -> Result<T, Self> {
        // Can't cast to `SmallBox<dyn Any>` in stable, so need to clone here for now.
        match self.downcast_ref::<T>() {
            Some(v) => Ok(v.clone()),
            None => Err(self),
        }
    }

    /// Gets the [`TypeId`] of the boxed value.
    ///
    /// Note that if you call [`Any::type_id`] directly on this type you will get the `BoxAnyVarValue` type id, not
    /// the id of the actual value.
    pub fn type_id(&self) -> TypeId {
        (*self.0).type_id()
    }

    /// Alternate formatter that writes detailed debug info about the box and value type name.
    ///
    /// The `std::fmt::Debug` implementation simply redirects to the value debug,
    /// this wrapper provides more info.
    pub fn detailed_debug(&self) -> impl fmt::Debug {
        struct DetailedDebug<'a>(&'a BoxAnyVarValue);
        impl<'a> fmt::Debug for DetailedDebug<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut b = f.debug_struct("BoxAnyVarValue");
                b.field("value", &*self.0.0);
                #[cfg(feature = "type_names")]
                b.field("type_name()", &self.0.0.type_name());
                #[cfg(not(feature = "type_names"))]
                b.field("type_id()", &self.0.type_id());
                b.field("is_heap()", &SmallBox::is_heap(&self.0.0));
                b.finish()
            }
        }
        DetailedDebug(self)
    }
}
impl Clone for BoxAnyVarValue {
    fn clone(&self) -> Self {
        self.0.clone_boxed()
    }
}
impl fmt::Debug for BoxAnyVarValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}
// don't implement PartialEq for BoxAnyVarValue,
// that would cause it to become a AnyVarValue itself and cause all sort of issues

/// Represents any variable value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it can apply to.
///
/// See [`VarValue<T>`] for more details.
pub trait AnyVarValue: fmt::Debug + Any + Send + Sync {
    /// Clone the value.
    fn clone_boxed(&self) -> BoxAnyVarValue;
    /// Gets if `self` and `other` are equal.
    fn eq_any(&self, other: &dyn AnyVarValue) -> bool;
    /// Value type name.
    #[cfg(feature = "type_names")]
    fn type_name(&self) -> &'static str;

    /// Swap value with `other` if both are of the same type.
    fn try_swap(&mut self, other: &mut dyn AnyVarValue) -> bool;
}
impl dyn AnyVarValue {
    /// Returns some reference to the inner value if it is of type `T`, or
    /// `None` if it isn't.
    pub fn downcast_ref<T: VarValue>(&self) -> Option<&T> {
        let any: &dyn Any = self;
        any.downcast_ref()
    }

    /// Returns some mutable reference to the inner value if it is of type `T`, or
    /// `None` if it isn't.
    pub fn downcast_mut<T: VarValue>(&mut self) -> Option<&mut T> {
        let any: &mut dyn Any = self;
        any.downcast_mut()
    }

    /// Returns `true` if the inner type is the same as `T`.
    pub fn is<T: VarValue>(&self) -> bool {
        let any: &dyn Any = self;
        any.is::<T>()
    }
}
impl PartialEq for dyn AnyVarValue {
    fn eq(&self, other: &Self) -> bool {
        self.eq_any(other)
    }
}
impl<T> AnyVarValue for T
where
    T: fmt::Debug + PartialEq + Clone + Any + Send + Sync,
{
    fn clone_boxed(&self) -> BoxAnyVarValue {
        BoxAnyVarValue::new(self.clone())
    }

    fn eq_any(&self, other: &dyn AnyVarValue) -> bool {
        match other.downcast_ref::<T>() {
            Some(o) => self == o,
            None => false,
        }
    }

    #[cfg(feature = "type_names")]
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn try_swap(&mut self, other: &mut dyn AnyVarValue) -> bool {
        if let Some(other) = other.downcast_mut::<T>() {
            std::mem::swap(self, other);
            return true;
        }
        false
    }
}

/// Represents a type that can be a [`Var<T>`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it can apply to.
///
/// # Implementing
///
/// Types need to be `Debug + Clone + PartialEq + Send + Sync + Any` to auto-implement this trait,
/// if you want to place an external type in a variable and it does not implement all the traits
/// you may need to declare a *newtype* wrapper.
///
/// If the external type is at least `Debug + Send + Sync + Any` you can use the [`ArcEq<T>`] wrapper
/// to quickly implement `Clone + PartialEq`, this is particularly useful for error types in [`ResponseVar<Result<_, E>>`].
///
/// If you want to use another variable as value use the [`VarEq<T>`] wrapper to use [`Var::var_eq`] as `PartialEq`.
/// Vars are not allowed to be values directly as that causes type inference issues.
///
/// [`Var<T>`]: crate::Var
pub trait VarValue: AnyVarValue + Clone + PartialEq {}
impl<T: AnyVarValue + Clone + PartialEq> VarValue for T {}

/// Arc value that implements equality by pointer comparison.
///
/// This type allows external types that are only `Debug + Send + Sync` to become
/// a full [`VarValue`] to be allowed as a variable value.
pub struct ArcEq<T: fmt::Debug + Send + Sync>(pub Arc<T>);
impl<T: fmt::Debug + Send + Sync> ops::Deref for ArcEq<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: fmt::Debug + Send + Sync> ArcEq<T> {
    /// Constructs a new `ArcEq<T>`.
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}
impl<T: fmt::Debug + Send + Sync> PartialEq for ArcEq<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: fmt::Debug + Send + Sync> Eq for ArcEq<T> {}
impl<T: fmt::Debug + Send + Sync> Clone for ArcEq<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<T: fmt::Debug + Send + Sync> fmt::Debug for ArcEq<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

/// Represents a [`Var<T>`] as a value inside another variable.
///
/// Variable values must implement `PartialEq + Debug + Clone + Send + Sync + Any`. Variable types
/// implement all of those except `PartialEq`, this type wraps a variable and adds equality using [`Var::var_eq`].
///
/// Variables cannot be be values directly because that breaks the [`IntoVar<T>`] blanket implementation for value types,
/// as variables also implement `IntoVar<T>`. This could be solved with the *default impl* Rust feature, but it is not yet stable.
/// This type is a workaround that limitation, it derefs to the wrapped var so it should require minimal refactoring as a drop-in replacement
/// for `Var<T>` in struct fields.
///
/// [`Var<T>`]: crate::Var
/// [`Var::var_eq`]: crate::Var::var_eq
/// [`IntoVar<T>`]: crate::IntoVar
#[derive(Clone)]
pub struct VarEq<T: VarValue>(pub crate::Var<T>);
impl<T: VarValue> ops::Deref for VarEq<T> {
    type Target = crate::Var<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: VarValue> fmt::Debug for VarEq<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl<T: VarValue> PartialEq for VarEq<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.var_eq(&other.0)
    }
}

/// A property value that is not a variable but can be inspected.
///
/// # Implementing
///
/// The trait is only auto-implemented for `T: Into<T> + VarValue`, unfortunately actual type conversions
/// must be manually implemented, note that the [`impl_from_and_into_var!`] macro auto-implements this conversion.
///
/// [`Debug`]: std::fmt::Debug
/// [`impl_from_and_into_var`]: impl_from_and_into_var
#[diagnostic::on_unimplemented(
    note = "`IntoValue<T>` is implemented for all `T: VarValue`",
    note = "you can use `impl_from_and_into_var!` to implement conversions"
)]
pub trait IntoValue<T: VarValue>: Into<T> {}
impl<T: VarValue> IntoValue<T> for T {}
