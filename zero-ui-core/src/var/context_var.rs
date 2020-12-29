use super::*;

/// A [`Var`] that represents a [`ContextVar`].
///
/// `PhantomData` is public here because we can't implement a `const fn new()` on stable.
/// We need to generate a const value to implement `ContextVar::var()`.
#[derive(Clone)]
pub struct ContextVarProxy<C: ContextVar>(pub PhantomData<C>);
impl<C: ContextVar> ContextVarProxy<C> {
    /// References the value in the current `vars` context.
    pub fn get<'a>(&'a self, vars: &'a Vars) -> &'a C::Type {
        <Self as VarObj<C::Type>>::get(self, vars)
    }

    /// References the value in the current `vars` context if it is marked as new.
    pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a C::Type> {
        <Self as VarObj<C::Type>>::get_new(self, vars)
    }

    /// If the value in the current `vars` context is marked as new.
    pub fn is_new(&self, vars: &Vars) -> bool {
        <Self as VarObj<C::Type>>::is_new(self, vars)
    }

    /// Gets the version of the value in the current `vars` context.
    pub fn version(&self, vars: &Vars) -> u32 {
        <Self as VarObj<C::Type>>::version(self, vars)
    }
}
impl<C: ContextVar> protected::Var for ContextVarProxy<C> {}
impl<C: ContextVar> Default for ContextVarProxy<C> {
    fn default() -> Self {
        ContextVarProxy(PhantomData)
    }
}
impl<C: ContextVar> VarObj<C::Type> for ContextVarProxy<C> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a C::Type {
        vars.context_var::<C>().0
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a C::Type> {
        let (value, is_new, _) = vars.context_var::<C>();
        if is_new {
            Some(value)
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        vars.context_var::<C>().1
    }

    fn version(&self, vars: &Vars) -> u32 {
        vars.context_var::<C>().2
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        true
    }

    fn set(&self, _: &Vars, _: C::Type) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut C::Type)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<C: ContextVar> Var<C::Type> for ContextVarProxy<C> {
    type AsReadOnly = Self;

    type AsLocal = CloningLocalVar<C::Type, Self>;

    fn as_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn modify<F: FnOnce(&mut C::Type) + 'static>(&self, _: &Vars, _: F) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn map<O: VarValue, F: FnMut(&C::Type) -> O>(&self, map: F) -> RcMapVar<C::Type, O, Self, F> {
        self.clone().into_map(map)
    }

    fn map_ref<O: VarValue, F: Fn(&C::Type) -> &O + Clone + 'static>(&self, map: F) -> MapRefVar<C::Type, O, Self, F> {
        self.clone().into_map_ref(map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&C::Type) -> O + 'static, G: FnMut(O) -> C::Type + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<C::Type, O, Self, F, G> {
        self.clone().into_map_bidi(map, map_back)
    }

    fn into_map<O: VarValue, F: FnMut(&C::Type) -> O>(self, map: F) -> RcMapVar<C::Type, O, Self, F> {
        RcMapVar::new(self, map)
    }

    fn into_map_bidi<O: VarValue, F: FnMut(&C::Type) -> O + 'static, G: FnMut(O) -> C::Type + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<C::Type, O, Self, F, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_ref<O: VarValue, F: Fn(&C::Type) -> &O + Clone + 'static>(self, map: F) -> MapRefVar<C::Type, O, Self, F> {
        MapRefVar::new(self, map)
    }

    fn map_bidi_ref<O: VarValue, F: Fn(&C::Type) -> &O + Clone + 'static, G: Fn(&mut C::Type) -> &mut O + Clone + 'static>(
        &self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<C::Type, O, Self, F, G> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map_bidi_ref<O: VarValue, F: Fn(&C::Type) -> &O + Clone + 'static, G: Fn(&mut C::Type) -> &mut O + Clone + 'static>(
        self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<C::Type, O, Self, F, G> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

impl<C: ContextVar> IntoVar<C::Type> for ContextVarProxy<C> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
/// See `ContextVar::thread_local_value`.
pub struct ContextVarValue<V: ContextVar> {
    _var: PhantomData<V>,
    value: Cell<(*const V::Type, bool, u32)>,
}
impl<V: ContextVar> ContextVarValue<V> {
    #[inline]
    pub fn init() -> Self {
        ContextVarValue {
            _var: PhantomData,
            value: Cell::new((V::default_value() as *const V::Type, false, 0)),
        }
    }
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarLocalKey<V: ContextVar> {
    local: &'static LocalKey<ContextVarValue<V>>,
}
impl<V: ContextVar> ContextVarLocalKey<V> {
    #[inline]
    pub fn new(local: &'static LocalKey<ContextVarValue<V>>) -> Self {
        ContextVarLocalKey { local }
    }

    pub(super) fn get(&self) -> (*const V::Type, bool, u32) {
        self.local.with(|l| l.value.get())
    }

    pub(super) fn set(&self, value: (*const V::Type, bool, u32)) {
        self.local.with(|l| l.value.set(value))
    }

    pub(super) fn replace(&self, value: (*const V::Type, bool, u32)) -> (*const V::Type, bool, u32) {
        self.local.with(|l| l.value.replace(value))
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __context_var_inner {
    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = const $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {

            static DEFAULT: $type = $default;
            &DEFAULT

        };);
    };

    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = once $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {

            static DEFAULT: once_cell::sync::OnceCell<$type> = once_cell::sync::OnceCell::new();
            DEFAULT.get_or_init(||{
                $default
            })

        };);
    };

    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = return $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {
            $default
        };);
    };


    (gen => $(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $DEFAULT:expr;) => {
        $(#[$outer])*
        /// # ContextVar
        /// This `struct` is a [`ContextVar`](zero_ui::core::var::ContextVar).
        #[derive(Debug, Clone, Copy)]
        $vis struct $ident;

        impl $ident {
            std::thread_local! {
                static THREAD_LOCAL_VALUE: $crate::var::ContextVarValue<$ident> = $crate::var::ContextVarValue::init();
            }

            /// [`Var`](zero_ui::core::var::Var) that represents this context var.
            #[inline]
            pub fn var() -> &'static $crate::var::ContextVarProxy<Self> {
                const VAR: $crate::var::ContextVarProxy<$ident> = $crate::var::ContextVarProxy(std::marker::PhantomData);
                &VAR
            }

            /// Default value, used when the variable is not set in a context.
            #[inline]
            pub fn default_value() -> &'static $type {
                $DEFAULT
            }
        }

        impl $crate::var::ContextVar for $ident {
            type Type = $type;

            #[inline]
            fn default_value() -> &'static Self::Type {
               Self::default_value()
            }

            #[inline]
            fn var() -> &'static $crate::var::ContextVarProxy<Self> {
               Self::var()
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
    };
}

/// Declares new [`ContextVar`](crate::core::var::ContextVar) types.
///
/// # Examples
/// ```
/// # use zero_ui::core::var::context_var;
/// # #[derive(Debug, Clone)]
/// # struct NotConst(u8);
/// # fn init_val() -> NotConst { NotConst(10) }
/// #
/// context_var! {
///     /// A public documented context var.
///     pub struct FooVar: u8 = const 10;
///
///     // A private context var.
///     struct BarVar: NotConst = once init_val();
/// }
/// ```
///
/// # Default Value
///
/// All context variable have a default fallback value that is used when the variable is not setted in the context.
///
/// The default value is a `&'static T` where `T` is the variable value type that must auto-implement [`VarValue`](crate::core::var::VarValue).
///
/// There are three different ways of specifying how the default value is stored. The way is selected by a keyword
/// after the `=` and before the default value expression.
///
/// ## `const`
///
/// The default expression is evaluated to a `static` item that is referenced when the variable default is requested.
///
/// Required a constant expression.
///
/// ## `return`
///
/// The default expression is returned when the variable default is requested.
///
/// Requires an expression of type `&'static T` where `T` is the variable value type.
///
/// ## `once`
///
/// The default expression is evaluated once during the first request and the value is cached for the lifetime of the process.
///
/// Requires an expression of type `T` where `T` is the variable value type.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `Var` suffix.
#[macro_export]
macro_rules! context_var {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $mode:ident $default:expr;)+) => {$(
        $crate::__context_var_inner!($(#[$outer])* $vis struct $ident: $type = $mode $default;);
    )+};
}
#[doc(inline)]
pub use crate::context_var;
