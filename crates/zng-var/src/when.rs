use std::sync::{Arc, Weak};

use crate::animation::AnimationHandle;

use super::*;

///<span data-del-macro-root></span> Initializes a new conditional var.
///
/// A condition var updates when the first `true` condition changes or the mapped var for the current condition changes.
///
/// # Syntax
///
/// The macro expects a list of `condition-var => condition-value-var`, the list is separated by comma.
/// The last condition must be the `_` token that maps to the value for when none of the conditions are `true`.
///
/// The `condition-var` must be an expression that evaluates to an `impl Var<bool>` type. The `condition-value-var` must
/// by any type that implements `IntoVar`. All condition values must be of the same [`VarValue`] type.
///
/// # Examples
///
/// ```
/// # use zng_var::*;
/// # use zng_txt::ToTxt;
/// # macro_rules! Text { ($($tt:tt)*) => { () } }
/// let condition = var(true);
/// let when_false = var("condition: false".to_txt());
///
/// let t = Text!(when_var! {
///     condition.clone() => "condition: true".to_txt(),
///     _ => when_false.clone(),
/// });
/// ```
///
/// In the example if `condition` or `when_false` are modified the text updates.
///
/// # `cfg`
///
/// Every condition can be annotated with attributes, including `#[cfg(..)]`.
///
/// ```
/// # use zng_var::*;
/// # use zng_txt::*;
/// # macro_rules! Text { ($($tt:tt)*) => { () } }
/// # let condition0 = var(true);
/// # let condition1 = var(true);
/// let t = Text!(when_var! {
///     #[cfg(some_flag)]
///     condition0 => "is condition 0".to_txt(),
///     #[cfg(not(some_flag))]
///     condition1 => "is condition 1".to_txt(),
///     _ => "is default".to_txt(),
/// });
/// ```
///
/// In the example above only one of the conditions will be compiled, the generated variable is the same
/// type as if you had written a single condition.
///
/// # Contextualized
///
/// The when var is contextualized when needed, meaning if any input [`is_contextual`] at the moment the var is created it
/// is also contextual. The full output type of this macro is a `BoxedVar<T>` that is either an `ArcWhenVar<T>` or
/// a `ContextualizedVar<T, ArcWhenVar<T>>`.
///
/// [`is_contextual`]: AnyVar::is_contextual
#[macro_export]
macro_rules! when_var {
    ($($tt:tt)*) => {
        $crate::types::__when_var! {
            $crate
            $($tt)*
        }
    }
}

use parking_lot::Mutex;
#[doc(hidden)]
pub use zng_var_proc_macros::when_var as __when_var;

#[doc(hidden)]
pub type ContextualizedArcWhenVar<T> = types::ContextualizedVar<T>;

/// Manually build a [`ArcWhenVar<T>`].
#[derive(Clone)]
pub struct WhenVarBuilder<T: VarValue> {
    default: BoxedAnyVar,
    conditions: Vec<(BoxedVar<bool>, BoxedAnyVar)>,
    _t: PhantomData<T>,
}
impl<T: VarValue> WhenVarBuilder<T> {
    /// Start building with the default value.
    pub fn new(default: impl IntoVar<T>) -> Self {
        Self {
            default: default.into_var().boxed_any(),
            conditions: vec![],
            _t: PhantomData,
        }
    }

    /// Push a condition and value.
    pub fn push(&mut self, condition: impl IntoVar<bool>, value: impl IntoVar<T>) {
        self.conditions.push((condition.into_var().boxed(), value.into_var().boxed_any()));
    }

    /// Finish the build.
    pub fn build(self) -> BoxedVar<T> {
        if self.default.is_contextual() || self.conditions.iter().any(|(c, v)| c.is_contextual() || v.is_contextual()) {
            types::ContextualizedVar::new(move || self.clone().build_impl()).boxed()
        } else {
            self.build_impl().boxed()
        }
    }
    fn build_impl(self) -> ArcWhenVar<T> {
        ArcWhenVar(
            build_impl_any(self.default, self.conditions, std::any::type_name::<T>()),
            PhantomData,
        )
    }
}

/// Manually build a [`ArcWhenVar<T>`] from type erased parts.
pub struct AnyWhenVarBuilder {
    default: BoxedAnyVar,
    conditions: Vec<(BoxedVar<bool>, BoxedAnyVar)>,
}
impl AnyWhenVarBuilder {
    /// Start building with only the default value.
    pub fn new<O: VarValue>(default: impl IntoVar<O>) -> Self {
        Self::new_any(default.into_var().boxed_any())
    }

    /// Start building with already boxed var.
    pub fn new_any(default: BoxedAnyVar) -> AnyWhenVarBuilder {
        Self {
            default,
            conditions: vec![],
        }
    }

    /// Create a builder from the parts of a formed [`when_var!`].
    ///
    /// # Panics
    ///
    /// Panics if called not called with a contextualized var produced by [`when_var!`].
    pub fn from_var<O: VarValue>(var: &types::ContextualizedVar<O>) -> Self {
        let g = var.borrow_init();
        let var = g
            .as_any()
            .downcast_ref::<ArcWhenVar<O>>()
            .expect("expected `when_var!` contextualized var");
        Self {
            default: var.0.default.clone_any(),
            conditions: var.0.conditions.iter().map(|(c, v)| (c.clone(), v.clone_any())).collect(),
        }
    }

    /// Returns the number of conditions set.
    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }

    /// Set/replace the default value.
    pub fn set_default<O: VarValue>(&mut self, default: impl IntoVar<O>) {
        self.set_default_any(default.into_var().boxed_any());
    }

    /// Set/replace the default value with an already typed erased var.
    pub fn set_default_any(&mut self, default: BoxedAnyVar) {
        self.default = default;
    }

    /// Push a when condition.
    pub fn push<C, O, V>(&mut self, condition: C, value: V)
    where
        C: Var<bool>,
        O: VarValue,
        V: IntoVar<O>,
    {
        self.push_any(condition.boxed(), value.into_var().boxed_any())
    }

    /// Push a when condition already boxed and type erased.
    pub fn push_any(&mut self, condition: BoxedVar<bool>, value: BoxedAnyVar) {
        self.conditions.push((condition, value));
    }

    /// Replace the default value if `other` has default and extend the conditions with clones of `other`.
    pub fn replace_extend(&mut self, other: &Self) {
        self.default = other.default.clone_any();
        self.extend(other);
    }

    /// Extend the conditions with clones of `other`.
    pub fn extend(&mut self, other: &Self) {
        for (c, v) in other.conditions.iter() {
            self.conditions.push((c.clone(), v.clone_any()));
        }
    }

    /// Build the when var if all value variables are of type [`BoxedVar<T>`].
    pub fn build<T: VarValue>(&self) -> Option<BoxedVar<T>> {
        let t = self.default.var_type_id();
        for (_, v) in &self.conditions {
            if v.var_type_id() != t {
                return None;
            }
        }
        let when = WhenVarBuilder {
            default: self.default.clone(),
            conditions: self.conditions.clone(),
            _t: PhantomData,
        };
        Some(when.build())
    }
}
impl fmt::Debug for AnyWhenVarBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyWhenVarBuilder")
            .field("condition_count", &self.condition_count())
            .finish_non_exhaustive()
    }
}
impl Clone for AnyWhenVarBuilder {
    fn clone(&self) -> Self {
        Self {
            default: self.default.clone_any(),
            conditions: self.conditions.iter().map(|(c, v)| (c.clone(), v.clone_any())).collect(),
        }
    }
}

struct WhenData {
    input_handles: Box<[VarHandle]>,
    hooks: Vec<VarHook>,
    last_update: VarUpdateId,
    active: usize,
}

struct Data {
    default: BoxedAnyVar,
    conditions: Vec<(BoxedVar<bool>, BoxedAnyVar)>,
    w: Mutex<WhenData>,
}

/// See [`when_var!`].
pub struct ArcWhenVar<T>(Arc<Data>, PhantomData<T>);

/// Weak reference to a [`ArcWhenVar<T>`].
pub struct WeakWhenVar<T>(Weak<Data>, PhantomData<T>);

fn build_impl_any(default: BoxedAnyVar, mut conditions: Vec<(BoxedVar<bool>, BoxedAnyVar)>, type_name: &'static str) -> Arc<Data> {
    conditions.shrink_to_fit();
    for (c, v) in conditions.iter_mut() {
        #[expect(unreachable_code)]
        fn panic_placeholder() -> BoxedVar<bool> {
            types::ContextualizedVar::<bool>::new(|| LocalVar(unreachable!())).boxed()
        }

        take_mut::take_or_recover(c, panic_placeholder, Var::actual_var);
        *v = v.actual_var_any();
    }

    let rc_when = Arc::new(Data {
        default: default.actual_var_any(),
        conditions,
        w: Mutex::new(WhenData {
            input_handles: Box::new([]),
            hooks: vec![],
            last_update: VarUpdateId::never(),
            active: usize::MAX,
        }),
    });
    let wk_when = Arc::downgrade(&rc_when);

    {
        let mut data = rc_when.w.lock();
        let data = &mut *data;

        // capacity can be n*2+1, but we only bet on conditions being `NEW`.
        let mut input_handles = Vec::with_capacity(rc_when.conditions.len());
        if rc_when.default.capabilities().contains(VarCapability::NEW) {
            input_handles.push(rc_when.default.hook_any(handle_value(wk_when.clone(), usize::MAX, type_name)));
        }
        for (i, (c, v)) in rc_when.conditions.iter().enumerate() {
            if c.get() && data.active > i {
                data.active = i;
            }

            if c.capabilities().contains(VarCapability::NEW) {
                input_handles.push(c.hook_any(handle_condition(wk_when.clone(), i, type_name)));
            }
            if v.capabilities().contains(VarCapability::NEW) {
                input_handles.push(v.hook_any(handle_value(wk_when.clone(), i, type_name)));
            }
        }

        data.input_handles = input_handles.into_boxed_slice();
    }
    rc_when
}

fn handle_condition(wk_when: Weak<Data>, i: usize, type_name: &'static str) -> Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync> {
    Box::new(move |args| {
        if let Some(rc_when) = wk_when.upgrade() {
            let data = rc_when.w.lock();
            let mut update = false;

            match data.active.cmp(&i) {
                std::cmp::Ordering::Equal => {
                    if let Some(&false) = args.downcast_value::<bool>() {
                        update = true;
                    }
                }
                std::cmp::Ordering::Greater => {
                    if let Some(&true) = args.downcast_value::<bool>() {
                        update = true;
                    }
                }
                std::cmp::Ordering::Less => {}
            }

            if update {
                drop(data);
                VARS.schedule_update(apply_update(rc_when, false, args.tags_vec()), type_name);
            }

            true
        } else {
            false
        }
    })
}

fn handle_value(wk_when: Weak<Data>, i: usize, type_name: &'static str) -> Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync> {
    Box::new(move |args| {
        if let Some(rc_when) = wk_when.upgrade() {
            let data = rc_when.w.lock();
            if data.active == i {
                drop(data);
                VARS.schedule_update(apply_update(rc_when, args.update(), args.tags_vec()), type_name);
            }
            true
        } else {
            false
        }
    })
}

fn apply_update(rc_merge: Arc<Data>, update: bool, tags: Vec<Box<dyn AnyVarValue>>) -> VarUpdateFn {
    Box::new(move || {
        let mut data = rc_merge.w.lock();
        let data = &mut *data;

        data.active = usize::MAX;
        for (i, (c, _)) in rc_merge.conditions.iter().enumerate() {
            if c.get() {
                data.active = i;
                break;
            }
        }
        data.last_update = VARS.update_id();

        let active = if data.active == usize::MAX {
            &rc_merge.default
        } else {
            &rc_merge.conditions[data.active].1
        };

        active.with_any(&mut |value| {
            let args = AnyVarHookArgs::new(value, update, &tags);
            data.hooks.retain(|h| h.call(&args));
        });
        VARS.wake_app();
    })
}

impl<T: VarValue> ArcWhenVar<T> {
    fn active(&self) -> &BoxedAnyVar {
        let active = self.0.w.lock().active;
        if active == usize::MAX {
            &self.0.default
        } else {
            &self.0.conditions[active].1
        }
    }

    /// Reference condition, value pairs.
    ///
    /// The active condition is the first `true`.
    pub fn conditions(&self) -> Vec<(BoxedVar<bool>, BoxedVar<T>)> {
        self.0
            .conditions
            .iter()
            .map(|(c, v)| (c.clone(), *v.clone().double_boxed_any().downcast::<BoxedVar<T>>().unwrap()))
            .collect()
    }

    /// The default value var.
    ///
    /// When no condition is active this is the backing var.
    pub fn default(&self) -> BoxedVar<T> {
        *self.0.default.clone().double_boxed_any().downcast::<BoxedVar<T>>().unwrap()
    }

    /// Create a variable similar to [`Var::easing`], but with different duration and easing functions for each condition.
    ///
    /// The `condition_easing` must contain one entry for each when condition, entries can be `None`, the easing used
    /// is the first entry that corresponds to a `true` condition, or falls back to the `default_easing`.
    pub fn easing_when(
        &self,
        condition_easing: Vec<Option<(Duration, Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>)>>,
        default_easing: (Duration, Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>),
    ) -> types::ContextualizedVar<T>
    where
        T: Transitionable,
    {
        let source = self.clone();
        types::ContextualizedVar::new(move || {
            debug_assert_eq!(source.0.conditions.len(), condition_easing.len());

            let source_wk = source.downgrade();
            let easing_var = super::var(source.get());

            let condition_easing = condition_easing.clone();
            let default_easing = default_easing.clone();
            let mut _anim_handle = AnimationHandle::dummy();
            var_bind(&source, &easing_var, move |value, _, easing_var| {
                let source = source_wk.upgrade().unwrap();
                for ((c, _), easing) in source.0.conditions.iter().zip(&condition_easing) {
                    if let Some((duration, func)) = easing {
                        if c.get() {
                            let func = func.clone();
                            _anim_handle = easing_var.ease(value.clone(), *duration, move |t| func(t));
                            return;
                        }
                    }
                }

                let (duration, func) = &default_easing;
                let func = func.clone();
                _anim_handle = easing_var.ease(value.clone(), *duration, move |t| func(t));
            })
            .perm();
            easing_var.read_only()
        })
    }
}

impl<T> Clone for ArcWhenVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<T> Clone for WeakWhenVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T: VarValue> crate::private::Sealed for ArcWhenVar<T> {}
impl<T: VarValue> crate::private::Sealed for WeakWhenVar<T> {}

impl<T: VarValue> AnyVar for ArcWhenVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
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
        self.active().set_any(value)
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.w.lock().last_update
    }

    fn is_contextual(&self) -> bool {
        if self.0.conditions.is_empty() {
            self.0.default.is_contextual()
        } else {
            self.active().is_contextual()
        }
    }

    fn capabilities(&self) -> VarCapability {
        if self.0.conditions.is_empty() {
            self.0.default.capabilities()
        } else {
            self.active().capabilities() | VarCapability::NEW | VarCapability::CAPS_CHANGE
        }
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        let (handle, hook) = VarHandle::new(pos_modify_action);
        self.0.w.lock().hooks.push(hook);
        handle
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.active().hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.clone_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakWhenVar(Arc::downgrade(&self.0), PhantomData::<T>))
    }

    fn is_animating(&self) -> bool {
        self.active().is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.active().modify_importance()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        VarPtr::new_arc(&self.0)
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

impl<T: VarValue> AnyWeakVar for WeakWhenVar<T> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.0.upgrade().map(|rc| Box::new(ArcWhenVar(rc, PhantomData::<T>)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue> IntoVar<T> for ArcWhenVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for ArcWhenVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakWhenVar<T>;

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
        let mut read = Some(read);
        let mut rsp = None;
        self.active().with_any(&mut |v| {
            let read = read.take().unwrap();
            let r = read(v.as_any().downcast_ref::<T>().unwrap());
            rsp = Some(r);
        });
        rsp.unwrap()
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        self.active()
            .clone()
            .double_boxed_any()
            .downcast::<BoxedVar<T>>()
            .unwrap()
            .modify(modify)
    }

    fn set<I>(&self, value: I) -> Result<(), VarIsReadOnlyError>
    where
        I: Into<T>,
    {
        self.active().set_any(Box::new(value.into()))
    }

    fn actual_var(self) -> Self {
        // inputs already actualized on ctor
        self
    }

    fn downgrade(&self) -> WeakWhenVar<T> {
        WeakWhenVar(Arc::downgrade(&self.0), PhantomData)
    }

    fn into_value(self) -> T {
        // need to clone the value anyway because of type erased internals
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
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

impl<T: VarValue> WeakVar<T> for WeakWhenVar<T> {
    type Upgrade = ArcWhenVar<T>;

    fn upgrade(&self) -> Option<ArcWhenVar<T>> {
        self.0.upgrade().map(|rc| ArcWhenVar(rc, PhantomData))
    }
}
