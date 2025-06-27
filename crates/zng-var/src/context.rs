use zng_app_context::{AppLocalId, ContextLocal, ContextLocalKeyProvider, context_local};

use super::*;

///<span data-del-macro-root></span> Declares new [`ContextVar`] static items.
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
/// [`context_local!`]: crate::context::context_local
#[macro_export]
macro_rules! context_var {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $NAME:ident: $Type:ty = $default:expr;
    )+) => {$(
        $(#[$attr])*
        $vis static $NAME: $crate::ContextVar<$Type> = {
            $crate::types::context_local! {
                static VAR: $crate::BoxedVar<$Type> = $crate::types::context_var_init::<$Type>($default);
            }
            $crate::ContextVar::new(&VAR)
        };
    )+}
}

#[doc(hidden)]
pub fn context_var_init<T: VarValue>(init: impl IntoVar<T>) -> BoxedVar<T> {
    init.into_var().boxed()
}

impl<T: VarValue> ContextLocalKeyProvider for ContextVar<T> {
    fn context_local_key(&'static self) -> AppLocalId {
        self.0.context_local_key()
    }
}

/// Represents another variable in a context.
///
/// Context variables are [`Var<T>`] implementers that represent a contextual value, unlike other variables it does not own
/// the value it represents.
///
/// See [`context_var!`] for more details.
#[derive(Clone)]
pub struct ContextVar<T: VarValue>(&'static ContextLocal<BoxedVar<T>>);
impl<T: VarValue> ContextVar<T> {
    #[doc(hidden)]
    pub const fn new(var: &'static ContextLocal<BoxedVar<T>>) -> Self {
        Self(var)
    }

    /// Runs `action` with this context var representing the other `var` in the current thread.
    ///
    /// The `var` must be `Some` and must be the `actual_var`, it is moved to the context storage during the call.
    ///
    /// Note that the `var` must be the same for subsequent calls in the same *context*, otherwise [contextualized]
    /// variables may not update their binding, in widgets you must re-init the descendants if you replace the `var`.
    ///
    /// [contextualized]: types::ContextualizedVar
    pub fn with_context<R>(self, id: ContextInitHandle, var: &mut Option<Arc<BoxedVar<T>>>, action: impl FnOnce() -> R) -> R {
        self.0.with_context_var(var, move || id.with_context(action))
    }

    /// Runs `action` with this context var representing the other `var` in the current thread.
    ///
    /// Note that the `var` must be the same for subsequent calls in the same *context*, otherwise [contextualized]
    /// variables may not update their binding, in widgets you must re-init the descendants if you replace the `var`.
    ///
    /// The `var` is converted into var, the actual var, boxed and placed in a new `Arc`, you can use the [`with_context`]
    /// method to avoid doing this in a hot path.
    ///
    /// [contextualized]: types::ContextualizedVar
    /// [`with_context`]: Self::with_context
    pub fn with_context_var<R>(self, id: ContextInitHandle, var: impl IntoVar<T>, action: impl FnOnce() -> R) -> R {
        let mut var = Some(Arc::new(var.into_var().actual_var().boxed()));
        self.with_context(id, &mut var, action)
    }
}
impl<T: VarValue> Copy for ContextVar<T> {}

impl<T: VarValue> crate::private::Sealed for ContextVar<T> {}

impl<T: VarValue> AnyVar for ContextVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(*self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_unboxed_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<T> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn with_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.with(|v| read(v))
    }

    fn with_new_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) -> bool {
        self.with_new(|v| read(v)).is_some()
    }

    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.get().last_update()
    }

    fn is_contextual(&self) -> bool {
        true
    }

    fn capabilities(&self) -> VarCapability {
        self.0.get().capabilities() | VarCapability::CAPS_CHANGE
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        self.0.get().hook_any(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.0.get().hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        self.0.get().strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.get().weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.0.get().actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        self.0.get().downgrade_any()
    }

    fn is_animating(&self) -> bool {
        self.0.get().is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.0.get().modify_importance()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        VarPtr::new_ctx_local(self.0)
    }

    fn get_debug(&self) -> Txt {
        self.with(var_debug)
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        Var::modify(self, var_update)
    }

    fn map_debug(&self) -> BoxedVar<Txt> {
        Var::map(self, var_debug).boxed()
    }
}

impl<T: VarValue> IntoVar<T> for ContextVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for ContextVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = BoxedVar<T>;

    type Downgrade = BoxedWeakVar<T>;

    type Map<O: VarValue> = contextualized::ContextualizedVar<O>;
    type MapBidi<O: VarValue> = contextualized::ContextualizedVar<O>;

    type FlatMap<O: VarValue, V: Var<O>> = contextualized::ContextualizedVar<O>;

    type FilterMap<O: VarValue> = contextualized::ContextualizedVar<O>;
    type FilterMapBidi<O: VarValue> = contextualized::ContextualizedVar<O>;

    type MapRef<O: VarValue> = types::MapRef<T, O, Self>;
    type MapRefBidi<O: VarValue> = types::MapRefBidi<T, O, Self>;

    type Easing = types::ContextualizedVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.get().with(read)
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        self.0.get().modify(modify)
    }

    fn actual_var(self) -> BoxedVar<T> {
        self.0.get_clone().actual_var()
    }

    fn downgrade(&self) -> BoxedWeakVar<T> {
        self.0.get().downgrade()
    }

    fn into_value(self) -> T {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(*self)
    }

    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
    {
        var_map_ctx(self, map)
    }

    fn map_bidi<O, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static,
    {
        var_map_bidi_ctx(self, map, map_back)
    }

    fn flat_map<O, V, M>(&self, map: M) -> Self::FlatMap<O, V>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + Send + 'static,
    {
        var_flat_map_ctx(self, map)
    }

    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> Self::FilterMap<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        var_filter_map_ctx(self, map, fallback)
    }

    fn filter_map_bidi<O, M, B, I>(&self, map: M, map_back: B, fallback: I) -> Self::FilterMapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        var_filter_map_bidi_ctx(self, map, map_back, fallback)
    }

    fn map_ref<O, M>(&self, map: M) -> Self::MapRef<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
    {
        var_map_ref(self, map)
    }

    fn map_ref_bidi<O, M, B>(&self, map: M, map_mut: B) -> Self::MapRefBidi<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static,
    {
        var_map_ref_bidi(self, map, map_mut)
    }

    fn easing<F>(&self, duration: Duration, easing: F) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        var_easing_ctx(self, duration, easing)
    }

    fn easing_with<F, S>(&self, duration: Duration, easing: F, sampler: S) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
    {
        var_easing_with_ctx(self, duration, easing, sampler)
    }
}

/// Context var that is always read-only, even if it is representing a read-write var.
pub type ReadOnlyContextVar<T> = types::ReadOnlyVar<T, ContextVar<T>>;

#[derive(Default)]
struct ContextInitHandleMarker;

/// Identifies the unique context a [`ContextualizedVar`] is in.
///
/// Each node that sets context-vars have an unique ID, it is different after each (re)init. The [`ContextualizedVar`]
/// records this ID, and rebuilds when it has changed. The contextualized inner vars are retained when the ID has at least one
/// clone.
///
/// [`ContextualizedVar`]: crate::types::ContextualizedVar
#[derive(Clone, Default)]
pub struct ContextInitHandle(Arc<ContextInitHandleMarker>);
context_local! {
    static CONTEXT_INIT_ID: ContextInitHandleMarker = ContextInitHandleMarker;
}
impl ContextInitHandle {
    /// Generates a new unique handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the current context handle.
    pub fn current() -> Self {
        Self(CONTEXT_INIT_ID.get())
    }

    /// Runs `action` with `self` as the current context ID.
    ///
    /// Note that [`ContextVar::with_context`] already calls this method.
    pub fn with_context<R>(&self, action: impl FnOnce() -> R) -> R {
        let mut opt = Some(self.0.clone());
        CONTEXT_INIT_ID.with_context(&mut opt, action)
    }

    /// Create a weak handle that can be used to monitor `self`, but does not hold it.
    pub fn downgrade(&self) -> WeakContextInitHandle {
        WeakContextInitHandle(Arc::downgrade(&self.0))
    }
}
impl fmt::Debug for ContextInitHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ContextInitHandle").field(&Arc::as_ptr(&self.0)).finish()
    }
}
impl PartialEq for ContextInitHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ContextInitHandle {}
impl std::hash::Hash for ContextInitHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let i = Arc::as_ptr(&self.0) as usize;
        std::hash::Hash::hash(&i, state)
    }
}

/// Weak [`ContextInitHandle`].
#[derive(Clone, Default)]
pub struct WeakContextInitHandle(std::sync::Weak<ContextInitHandleMarker>);
impl WeakContextInitHandle {
    /// Returns `true` if the strong handle still exists.
    pub fn is_alive(&self) -> bool {
        self.0.strong_count() > 0
    }
}
impl fmt::Debug for WeakContextInitHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakContextInitHandle")
            .field(&std::sync::Weak::as_ptr(&self.0))
            .finish()
    }
}
impl PartialEq for WeakContextInitHandle {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Weak::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WeakContextInitHandle {}
impl std::hash::Hash for WeakContextInitHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let i = std::sync::Weak::as_ptr(&self.0) as usize;
        std::hash::Hash::hash(&i, state)
    }
}
