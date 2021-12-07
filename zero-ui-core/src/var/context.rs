use std::{cell::Cell, marker::PhantomData, thread::LocalKey};

use super::*;

/// A [`Var`] that represents a [`ContextVar`].
///
/// Context var types don't implement [`Var`] directly, to avoid problems with overlapping generics
/// this *proxy* zero-sized type is used.
#[derive(Clone)]
pub struct ContextVarProxy<C: ContextVar>(PhantomData<C>);
impl<C: ContextVar> ContextVarProxy<C> {
    /// New context var proxy.
    ///
    /// Prefer using [`ContextVar::new`] or the `new` generated using the [`context_var!`] macro.
    #[inline]
    pub fn new() -> Self {
        ContextVarProxy(PhantomData)
    }

    #[doc(hidden)]
    #[inline]
    pub fn static_ref() -> &'static Self {
        &ContextVarProxy(PhantomData)
    }
}

impl<C: ContextVar> Default for ContextVarProxy<C> {
    fn default() -> Self {
        ContextVarProxy(PhantomData)
    }
}

impl<C: ContextVar> crate::private::Sealed for ContextVarProxy<C> {}
impl<C: ContextVar> Var<C::Type> for ContextVarProxy<C> {
    type AsReadOnly = Self;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a C::Type {
        let v = vars.as_ref();
        v.context_var::<C>().value(v)
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a C::Type> {
        let info = vars.as_ref().context_var::<C>();
        if info.1 {
            Some(info.0)
        } else {
            None
        }
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|v| v.context_var::<C>().is_new(v))
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> C::Type {
        self.get_clone(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        vars.with_vars_read(|v| v.context_var::<C>().version(v))
    }

    #[inline]
    fn is_read_only<Vr: WithVars>(&self, _: &Vr) -> bool {
        true
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        true
    }

    #[inline]
    fn can_update(&self) -> bool {
        true
    }

    #[inline]
    fn strong_count(&self) -> usize {
        0
    }

    #[inline]
    fn modify<Vw, M>(&self, _: &Vw, _: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<C::Type>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set<Vw, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<C::Type>,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set_ne<Vw, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<C::Type>,
        C::Type: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|v| v.context_var::<C>().update_mask(v))
    }
}

impl<C: ContextVar> IntoVar<C::Type> for ContextVarProxy<C> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

/// Backing data bound to a [`ContextVar`] in a context.
pub trait ContextVarSource<T: VarValue> {
    /// Value for [`Var::get`].
    fn value<'a>(&'a self, vars: &'a VarsRead) -> &'a T;

    /// Value for [`Var::is_new`].
    fn is_new(&self, vars: &Vars) -> bool;

    /// Value for [`Var::version`].
    fn version(&self, vars: &Vars) -> u32;

    /// Value for [`Var::update_mask`].
    fn update_mask(&self, vars: &VarsRead) -> UpdateMask;
}

/// Represents a context var source that is a `Var<T>`.
pub struct ContextVarSourceVar<'a, T: VarValue, V: Var<T>> {
    var: &'a V,
    _t: PhantomData<&'a T>,
}
impl<'a, T: VarValue, V: Var<T>> ContextVarSourceVar<'a, T, V> {
    /// New from var.
    pub fn new(var: &'a V) -> Self {
        Self { var, _t: PhantomData }
    }
}
impl<'a, T, V> ContextVarSource<T> for ContextVarSourceVar<'a, T, V>
where
    T: VarValue,
    V: Var<T>,
{
    fn value<'b>(&'b self, vars: &'b VarsRead) -> &'b T {
        self.var.get(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.var.version(vars)
    }

    fn update_mask(&self, vars: &VarsRead) -> UpdateMask {
        self.var.update_mask(vars)
    }
}

/// Represents context var source that is a value generated from a different context var source
/// such that all the update tracking metadata is the same.
pub struct ContextVarSourceMap<'a, T: VarValue, ST: VarValue, SV: ContextVarSource<ST>> {
    value: &'a T,
    meta: &'a SV,
    _st: PhantomData<&'a ST>,
}
impl<'a, T, ST, SV> ContextVarSourceMap<'a, T, ST, SV>
where
    T: VarValue,
    ST: VarValue,
    SV: ContextVarSource<ST>,
{
    /// New mapping.
    pub fn new(value: &'a T, meta: &'a SV) -> Self {
        Self {
            value,
            meta,
            _st: PhantomData,
        }
    }
}
impl<'a, T, ST, SV> ContextVarSource<T> for ContextVarSourceMap<'a, T, ST, SV>
where
    T: VarValue,
    ST: VarValue,
    SV: ContextVarSource<ST>,
{
    fn value<'b>(&'b self, _: &'b VarsRead) -> &'b T {
        self.value
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.meta.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.meta.version(vars)
    }

    fn update_mask(&self, vars: &VarsRead) -> UpdateMask {
        self.meta.update_mask(vars)
    }
}

struct ContextVarValueDefault<T: VarValue>(T);
impl<T: VarValue> ContextVarSource<T> for ContextVarValueDefault<T> {
    fn value<'a>(&'a self, _: &'a VarsRead) -> &'a T {
        &self.0
    }

    fn is_new(&self, _: &Vars) -> bool {
        false
    }

    fn version(&self, _: &Vars) -> u32 {
        0
    }

    fn update_mask(&self, _: &VarsRead) -> UpdateMask {
        UpdateMask::none()
    }
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarValue<V: ContextVar> {
    _var: PhantomData<V>,
    _default_value: Box<ContextVarValueDefault<V::Type>>,
    value: Cell<*const dyn ContextVarSource<V::Type>>,
}

#[allow(missing_docs)]
impl<V: ContextVar> ContextVarValue<V> {
    #[inline]
    pub fn init() -> Self {
        let default_value = Box::new(ContextVarValueDefault(V::default_value()));
        ContextVarValue {
            _var: PhantomData,
            value: Cell::new(&*default_value as _),
            _default_value: default_value,
        }
    }
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarLocalKey<V: ContextVar> {
    local: &'static LocalKey<ContextVarValue<V>>,
}
#[allow(missing_docs)]
impl<V: ContextVar> ContextVarLocalKey<V> {
    #[inline]
    pub fn new(local: &'static LocalKey<ContextVarValue<V>>) -> Self {
        ContextVarLocalKey { local }
    }

    pub(super) fn get(&self) -> *const dyn ContextVarSource<V::Type> {
        self.local.with(|l| l.value.get())
    }

    pub(super) fn set(&self, value: *const (dyn ContextVarSource<V::Type> + '_)) {
        self.local.with(|l| l.value.set(value))
    }

    pub(super) fn replace(&self, value: *const (dyn ContextVarSource<V::Type> + '_)) -> *const dyn ContextVarSource<V::Type> {
        self.local.with(|l| l.value.replace(value))
    }
}

///<span data-inline></span> Declares new [`ContextVar`](crate::var::ContextVar) types.
///
/// # Examples
/// ```
/// # use zero_ui_core::var::context_var;
/// # #[derive(Debug, Clone)]
/// # struct NotConst(u8);
/// # fn init_val() -> NotConst { NotConst(10) }
/// #
/// context_var! {
///     /// A public documented context var.
///     pub struct FooVar: u8 = 10;
///
///     // A private context var.
///     struct BarVar: NotConst = init_val();
/// }
/// ```
///
/// # Default Value
///
/// All context variable have a default fallback value that is used when the variable is not setted in the context.
///
/// The default value is instantiated once per app thread and is the value of the variable when it is not set in the context.
/// Other instances of the default value can be created by calls to [`ContextVar::default_value`], so the code after the `=` should
/// always generate a value equal to the first value generated.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `Var` suffix.
#[macro_export]
macro_rules! context_var {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $default:expr;)+) => {$(
        $(#[$outer])*
        ///
        /// # ContextVar
        ///
        /// This `struct` is a [`ContextVar`](crate::var::ContextVar).
        #[derive(Debug, Clone, Copy)]
        $vis struct $ident;

        impl $ident {
            std::thread_local! {
                static THREAD_LOCAL_VALUE: $crate::var::ContextVarValue<$ident> = $crate::var::ContextVarValue::init();
            }

            /// [`Var`](crate::var::Var) implementer that represents this context var.
            #[inline]
            #[allow(unused)]
            pub fn new() -> $crate::var::ContextVarProxy<Self> {
                $crate::var::ContextVarProxy::new()
            }

            /// New default value.
            ///
            /// Returns a value that is equal to the variable value when it is not set in any context.
            #[inline]
            pub fn default_value() -> $type {
                $default
            }

            /// References the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn get<'a, Vr: AsRef<$crate::var::VarsRead>>(vars: &'a Vr) -> &'a $type {
                $crate::var::Var::get($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Returns a clone of the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn get_clone<Vr: $crate::var::WithVarsRead>(vars: &Vr) -> $type {
                $crate::var::Var::get_clone($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// References the value in the current `vars` context if it is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn get_new<'a, Vw: AsRef<$crate::var::Vars>>(vars: &'a Vw) -> Option<&'a $type> {
                $crate::var::Var::get_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Returns a clone of the value in the current `vars` context if it is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn clone_new<Vw: $crate::var::WithVars>(vars: &Vw) -> Option<$type> {
                $crate::var::Var::clone_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            // TODO generate copy fns when https://github.com/rust-lang/rust/issues/48214 is stable

            /// If the value in the current `vars` context is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn is_new<Vw: $crate::var::WithVars>(vars: &Vw) -> bool {
                $crate::var::Var::is_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Gets the version of the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn version<Vr: $crate::var::WithVarsRead>(vars: &Vr) -> u32 {
                $crate::var::Var::version($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }
        }

        impl $crate::var::ContextVar for $ident {
            type Type = $type;

            #[inline]
            fn default_value() -> Self::Type {
               Self::default_value()
            }

            #[inline]
            fn thread_local_value() -> $crate::var::ContextVarLocalKey<Self> {
                $crate::var::ContextVarLocalKey::new(&Self::THREAD_LOCAL_VALUE)
            }
        }

        impl $crate::var::IntoVar<$type> for $ident {
            type Var = $crate::var::ContextVarProxy<Self>;
            #[inline]
            fn into_var(self) -> Self::Var {
                $crate::var::ContextVarProxy::default()
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::context_var;
