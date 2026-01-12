//! Context vars

use std::{any::TypeId, fmt, ops, sync::Arc};

use smallbox::SmallBox;
use zng_app_context::{AppLocalId, ContextLocal, ContextLocalKeyProvider};

use crate::{
    AnyVar, AnyVarHookArgs, AnyVarModify, AnyVarValue, BoxAnyVarValue, ContextInitHandle, DynAnyVar, DynWeakAnyVar, IntoVar, Var,
    VarCapability, VarHandle, VarImpl, VarInstanceTag, VarUpdateId, VarValue, WeakVarImpl,
};

///<span data-del-macro-root></span> Declares new [`ContextVar<T>`] static items.
///
/// # Examples
///
/// ```
/// # use zng_var::context_var;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct NotConst(u8);
/// # fn init_val() -> NotConst { NotConst(10) }
/// #
/// context_var! {
///     /// A public documented context var.
///     pub static FOO_VAR: u8 = 10;
///
///     // A private context var.
///     static BAR_VAR: NotConst = init_val();
///
///     // A var that *inherits* from another.
///     pub static DERIVED_VAR: u8 = FOO_VAR;
/// }
/// ```
///
/// # Default Value
///
/// All context variable have a default fallback value that is used when the variable is not set in the context.
///
/// The default value is instantiated once per app and is the value of the variable when it is not set in the context,
/// any value [`IntoVar<T>`] is allowed, including other variables.
///
/// The default value can also be a [`Var::map`] to another context var, but note that mapping vars are contextualized,
/// meaning that they evaluate the mapping in each different context read, so a context var with mapping value
/// read in a thousand widgets will generate a thousand different mapping vars, but if the same var mapping is set
/// in the root widget, the thousand widgets will all use the same mapping var.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `_VAR` suffix.
///
/// # Context Local
///
/// If you are only interested in sharing a contextual value you can use the [`context_local!`] macro instead.
///
/// [`context_local!`]: crate::__context_var_local
#[macro_export]
macro_rules! context_var {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $NAME:ident: $Type:ty = $default:expr;
    )+) => {$(
        $(#[$attr])*
        $vis static $NAME: $crate::ContextVar<$Type> = {
            $crate::__context_var_local! {
                static CTX: $crate::AnyVar = $crate::context_var_init::<$Type>($default);
            }
            static VAR: std::sync::OnceLock<$crate::Var<$Type>> = std::sync::OnceLock::new();
            $crate::ContextVar::new(&CTX, &VAR)
        };
    )+}
}

#[doc(hidden)]
pub use zng_app_context::context_local as __context_var_local;

#[doc(hidden)]
pub fn context_var_init<T: VarValue>(init: impl IntoVar<T>) -> AnyVar {
    init.into_var().into()
}

impl<T: VarValue> ContextLocalKeyProvider for ContextVar<T> {
    fn context_local_key(&'static self) -> AppLocalId {
        self.ctx.context_local_key()
    }
}

/// Represents a named contextual variable.
///
/// This type dereferences to the actual context [`Var<T>`]. It also implements [`IntoVar<T>`]
/// that converts to the context var, you can assign it directly to properties.
///
/// See [`context_var!`] for more details about declaring and using context vars. See [`contextual_var`] for more details about
/// contextualizing variables.
///
/// [`contextual_var`]: crate::contextual_var
pub struct ContextVar<T: VarValue> {
    ctx: &'static ContextLocal<AnyVar>,
    var: &'static std::sync::OnceLock<Var<T>>,
}
impl<T: VarValue> Copy for ContextVar<T> {}
impl<T: VarValue> Clone for ContextVar<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: VarValue> ContextVar<T> {
    #[doc(hidden)]
    pub const fn new(ctx: &'static ContextLocal<AnyVar>, var: &'static std::sync::OnceLock<Var<T>>) -> Self {
        Self { ctx, var }
    }

    /// Reference the actual context var.
    ///
    /// The variable is [`CONTEXT`] capable, you can call [`current_context`]
    /// to get the current calling context actual variable. See [`contextual_var`] for more details about contextualizing variables.
    ///
    /// Note that `ContextVar<T>` also dereferences to this var.
    ///
    /// [`CONTEXT`]: VarCapability::CONTEXT
    /// [`current_context`]: crate::Var::current_context
    /// [`contextual_var`]: crate::contextual_var
    pub fn as_var(&self) -> &Var<T> {
        self.var
            .get_or_init(|| Var::new_any(AnyVar(DynAnyVar::Context(ContextVarImpl(self.ctx)))))
    }

    /// Runs `action` with this context var representing the other `var` in the current thread.
    ///
    /// The `var` must be `Some` and must be the [`current_context`], not another contextual variable. The `var`
    /// is moved to the context storage during the call and them returned to the `var`. The `var` value type must be `T`.
    ///
    /// Note that the `var` must be the same for subsequent calls in the same *context*, otherwise [contextualized]
    /// variables may not update their binding, in widgets you must re-init the descendants if you replace the `var`.
    ///
    /// [contextualized]: crate::contextual_var
    /// [`current_context`]: crate::Var::current_context
    pub fn with_context<R>(self, id: ContextInitHandle, var: &mut Option<Arc<AnyVar>>, action: impl FnOnce() -> R) -> R {
        #[cfg(debug_assertions)]
        {
            let var = var.as_ref().expect("context `var` not set");
            assert!(var.value_is::<T>(), "context `var` not of the expected value type `T`");
            assert!(!var.capabilities().is_contextual(), "context `var` must be current_context");
        }
        self.ctx.with_context_var(var, move || id.with_context(action))
    }

    /// Runs `action` with this context var representing the other `var` in the current thread.
    ///
    /// Note that the `var` must be the same for subsequent calls in the same *context*, otherwise [contextualized]
    /// variables may not update their binding, in widgets you must re-init the descendants if you replace the `var`.
    ///
    /// The `var` is converted into var, the actual var, boxed and placed in a new `Arc`, you can use the [`with_context`]
    /// method to avoid doing this in a hot path.
    ///
    /// [contextualized]: crate::contextual_var
    /// [`with_context`]: Self::with_context
    pub fn with_context_var<R>(self, id: ContextInitHandle, var: impl IntoVar<T>, action: impl FnOnce() -> R) -> R {
        let mut var = Some(Arc::new(var.into_var().as_any().current_context()));
        self.with_context(id, &mut var, action)
    }
}
impl<T: VarValue> ops::Deref for ContextVar<T> {
    type Target = Var<T>;

    fn deref(&self) -> &Self::Target {
        self.as_var()
    }
}
impl<T: VarValue> IntoVar<T> for ContextVar<T> {
    fn into_var(self) -> Var<T> {
        self.as_var().clone()
    }
}
impl<T: VarValue> From<ContextVar<T>> for Var<T> {
    fn from(v: ContextVar<T>) -> Self {
        v.as_var().clone()
    }
}
impl<T: VarValue> From<ContextVar<T>> for AnyVar {
    fn from(v: ContextVar<T>) -> Self {
        v.as_any().clone()
    }
}
pub(crate) struct ContextVarImpl(&'static ContextLocal<AnyVar>);
impl fmt::Debug for ContextVarImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ContextVar").finish_non_exhaustive()
    }
}
impl PartialEq for ContextVarImpl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl VarImpl for ContextVarImpl {
    fn clone_dyn(&self) -> DynAnyVar {
        DynAnyVar::Context(Self(self.0))
    }

    fn value_type(&self) -> TypeId {
        self.0.get_clone().0.value_type()
    }

    #[cfg(feature = "type_names")]
    fn value_type_name(&self) -> &'static str {
        self.0.get().0.value_type_name()
    }

    fn strong_count(&self) -> usize {
        1
    }

    fn var_eq(&self, other: &DynAnyVar) -> bool {
        match other {
            DynAnyVar::Context(b) => self == b,
            _ => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        self.0.get().0.var_instance_tag()
    }

    fn downgrade(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::Context(Self(self.0))
    }

    fn capabilities(&self) -> VarCapability {
        self.0.get().0.capabilities() | VarCapability::CONTEXT | VarCapability::MODIFY_CHANGES
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.0.get().0.with(visitor);
    }

    fn get(&self) -> BoxAnyVarValue {
        self.0.get().0.get()
    }

    fn set(&self, new_value: BoxAnyVarValue) -> bool {
        self.0.get().0.set(new_value)
    }

    fn update(&self) -> bool {
        self.0.get().0.update()
    }

    fn modify(&self, modify: SmallBox<dyn FnMut(&mut AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool {
        self.0.get().0.modify(modify)
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        self.0.get().0.hook(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.get().0.last_update()
    }

    fn modify_info(&self) -> crate::animation::ModifyInfo {
        self.0.get().0.modify_info()
    }

    fn modify_importance(&self) -> usize {
        self.0.get().0.modify_importance()
    }

    fn is_animating(&self) -> bool {
        self.0.get().0.is_animating()
    }

    fn hook_animation_stop(&self, handler: crate::animation::AnimationStopFn) -> VarHandle {
        self.0.get().0.hook_animation_stop(handler)
    }

    fn current_context(&self) -> DynAnyVar {
        // is already contextualized, but no downside calling current_context again, it just clones
        self.0.get().0.current_context()
    }
}
impl WeakVarImpl for ContextVarImpl {
    fn clone_dyn(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::Context(Self(self.0))
    }

    fn strong_count(&self) -> usize {
        1
    }

    fn upgrade(&self) -> Option<DynAnyVar> {
        Some(DynAnyVar::Context(Self(self.0)))
    }
}
