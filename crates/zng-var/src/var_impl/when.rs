//! Conditional var proxy.

use std::sync::{
    Arc, Weak,
    atomic::{AtomicU32, AtomicUsize, Ordering},
};

use crate::{VARS, Var, VarAny, shared::MutexHooks};

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
/// The `condition-var` must be an expression that evaluates to a `Var<bool>` type. The `condition-value-var` must
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
/// let t = Text!(var_when! {
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
/// let t = Text!(var_when! {
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
/// # Capabilities
///
/// The when var is contextualized when needed, meaning if any input is [`CONTEXT`] at the moment the var is created it
/// is also contextual. The full output type of this macro is a `Var<T>`.
///
/// [`CONTEXT`]: crate::VarCapability::CONTEXT
#[macro_export]
macro_rules! var_when {
    ($($tt:tt)*) => {
        $crate::__var_when! {
            $crate
            $($tt)*
        }
    }
}

use zng_clone_move::clmv;
#[doc(hidden)]
pub use zng_var_proc_macros::var_when as __var_when;

/// Type erased [`var_when!`] manual builder.
///
/// See [`VarWhenBuilder`] for more details.
#[derive(Clone)]
pub struct VarWhenAnyBuilder {
    conditions: Vec<(Var<bool>, VarAny)>,
    default: VarAny,
}
impl VarWhenAnyBuilder {
    /// New with value variable used when no other conditions are `true`.
    pub fn new(default: VarAny) -> Self {
        VarWhenAnyBuilder {
            conditions: Vec::with_capacity(2),
            default,
        }
    }

    /// Push a conditional value.
    ///
    /// When the `condition` is `true` and all previous pushed conditions
    /// are `false` the when variable represents the `value` variable.
    pub fn push(&mut self, condition: Var<bool>, value: VarAny) -> &mut Self {
        self.conditions.push((condition, value));
        self
    }

    /// Replace the default value if `other` has default and extend the conditions with clones of `other`.
    pub fn replace_extend(&mut self, other: &Self) {
        self.default = other.default.clone();
        self.extend(other);
    }

    /// Extend the conditions with clones of `other`.
    pub fn extend(&mut self, other: &Self) {
        for (c, v) in other.conditions.iter() {
            self.conditions.push((c.clone(), v.clone()));
        }
    }

    /// Build the when var.
    pub fn build(self) -> VarAny {
        var_when(self)
    }

    /// Convert to typed builder.
    pub fn into_typed<O: VarValue>(self) -> VarWhenBuilder<O> {
        VarWhenBuilder {
            builder: self,
            _t: PhantomData,
        }
    }

    /// If the `var` was built by [`build`] clones the internal conditions, values and default variables into a new builder.
    ///
    /// [`build`]: Self::build
    pub fn try_from_built(var: &VarAny) -> Option<Self> {
        let any: &dyn Any = &*var.0;
        let built = any.downcast_ref::<WhenVar>()?;
        Some(Self {
            conditions: built.0.conditions.clone(),
            default: built.0.default.clone(),
        })
    }
}
impl VarWhenAnyBuilder {
    /// Returns the number of conditions set.
    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }
}

/// Manual [`var_when!`] builder.
#[derive(Clone)]
pub struct VarWhenBuilder<O: VarValue> {
    builder: VarWhenAnyBuilder,
    _t: PhantomData<O>,
}
impl<O: VarValue> VarWhenBuilder<O> {
    /// New with value variable used when no other conditions are `true`.
    pub fn new(default: impl IntoVar<O>) -> Self {
        Self {
            builder: VarWhenAnyBuilder::new(default.into_var().into()),
            _t: PhantomData,
        }
    }

    /// Push a conditional value.
    ///
    /// When the `condition` is `true` and all previous pushed conditions
    /// are `false` the when variable represents the `value` variable.
    pub fn push(&mut self, condition: impl IntoVar<bool>, value: impl IntoVar<O>) -> &mut Self {
        self.builder.conditions.push((condition.into_var(), value.into_var().into()));
        self
    }

    /// Build the when var.
    pub fn build(self) -> Var<O> {
        Var::new_any(self.builder.build())
    }

    /// Reference the type erased when builder.
    pub fn as_any(&mut self) -> &mut VarWhenAnyBuilder {
        &mut self.builder
    }

    /// If the `var` was built by [`build`] clones the internal conditions, values and default variables into a new builder.
    ///
    /// [`build`]: Self::build
    pub fn try_from_built(var: &Var<O>) -> Option<Self> {
        // this is used by #[easing(_)] in PropertyBuildAction to modify widget properties

        let builder = VarWhenAnyBuilder::try_from_built(var)?;
        Some(Self { builder, _t: PhantomData })
    }
}

fn var_when(builder: VarWhenAnyBuilder) -> VarAny {
    // !!: TODO contextualize? And in a way that can recover builder in `try_from_built`

    let data = Arc::new(WhenVarData {
        active_condition: AtomicUsize::new(builder.conditions.iter().position(|(c, _)| c.get()).unwrap_or(usize::MAX)),
        conditions: builder.conditions,
        default: builder.default,
        hooks: MutexHooks::default(),
        last_active_change: AtomicU32::new(VarUpdateId::never().0),
    });

    for (i, (c, v)) in data.conditions.iter().enumerate() {
        let weak = Arc::downgrade(&data);
        c.hook(clmv!(weak, |args| {
            if let Some(data) = weak.upgrade() {
                let mut changed = false;
                let mut active = data.active_condition.load(Ordering::Relaxed);
                if active == i {
                    if !*args.value() {
                        // deactivated active
                        active = data.conditions.iter().position(|(c, _)| c.get()).unwrap_or(usize::MAX);
                    }
                } else if active > i && *args.value() {
                    // activated higher priority
                    changed = true;
                    active = i;
                }

                if changed {
                    data.active_condition.store(active, Ordering::Relaxed);
                    data.last_active_change.store(VARS.update_id().0, Ordering::Relaxed);

                    let active = if active < data.conditions.len() {
                        &data.conditions[active].1
                    } else {
                        &data.default
                    };

                    active.0.with(&mut |v| {
                        data.hooks.notify(&VarAnyHookArgs {
                            value: v,
                            update: args.update,
                            tags: args.tags,
                        });
                    });
                }

                true
            } else {
                false
            }
        }))
        .perm();

        v.hook(move |args| {
            if let Some(data) = weak.upgrade() {
                if data.active_condition.load(Ordering::Relaxed) == i {
                    data.hooks.notify(args);
                }
                true
            } else {
                false
            }
        })
        .perm();
    }
    let weak = Arc::downgrade(&data);
    data.default
        .hook(move |args| {
            if let Some(data) = weak.upgrade() {
                if data.active_condition.load(Ordering::Relaxed) >= data.conditions.len() {
                    data.hooks.notify(args);
                }
                true
            } else {
                false
            }
        })
        .perm();

    VarAny(smallbox!(WhenVar(data)))
}

struct WhenVarData {
    conditions: Vec<(Var<bool>, VarAny)>,
    default: VarAny,
    active_condition: AtomicUsize,
    hooks: MutexHooks,
    // Atomic<VarUpdateId>
    last_active_change: AtomicU32,
}
struct WhenVar(Arc<WhenVarData>);
impl WhenVar {
    fn active(&self) -> &VarAny {
        let i = self.0.active_condition.load(Ordering::Relaxed);
        if i < self.0.conditions.len() {
            &self.0.conditions[i].1
        } else {
            &self.0.default
        }
    }
}
impl VarImpl for WhenVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(Self(self.0.clone()))
    }

    fn value_type(&self) -> TypeId {
        self.0.default.0.value_type()
    }

    #[cfg(feature = "value_type_name")]
    fn value_type_name(&self) -> &'static str {
        self.0.default.0.value_type_name()
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn var_eq(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<Self>() {
            Some(o) => Arc::ptr_eq(&self.0, &o.0),
            None => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        VarInstanceTag(Arc::as_ptr(&self.0) as _)
    }

    fn downgrade(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakWhenVar(Arc::downgrade(&self.0)))
    }

    fn capabilities(&self) -> VarCapability {
        self.active().0.capabilities() | VarCapability::CAPS_CHANGE
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn VarValueAny)) {
        self.active().0.with(visitor)
    }

    fn get(&self) -> BoxedVarValueAny {
        self.active().0.get()
    }

    fn set(&self, new_value: BoxedVarValueAny) -> bool {
        self.active().0.set(new_value)
    }

    fn update(&self) -> bool {
        self.active().0.update()
    }

    fn modify(&self, modify: SmallBox<dyn FnMut(&mut VarModifyAny) + Send + 'static, smallbox::space::S4>) -> bool {
        self.active().0.modify(modify)
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&VarAnyHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        self.0.hooks.push(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        // can be active update, or any of the conditions that updated and caused this update.
        VarUpdateId(self.0.last_active_change.load(Ordering::Relaxed)).max(self.active().0.last_update())
    }

    fn modify_importance(&self) -> usize {
        self.active().0.modify_importance()
    }

    // the conditions could be animating too, but this was not handled in the previous impl either

    fn is_animating(&self) -> bool {
        self.active().0.is_animating()
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn> {
        self.active().0.hook_animation_stop(handler)
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.clone_boxed()
    }
}

struct WeakWhenVar(Weak<WhenVarData>);
impl WeakVarImpl for WeakWhenVar {
    fn clone_boxed(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(Self(self.0.clone()))
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<SmallBox<dyn VarImpl, smallbox::space::S2>> {
        let s = WhenVar(self.0.upgrade()?);
        Some(smallbox!(s))
    }
}

/*
!!:TODO This was on the ArcWhenVar, used by #[easing(_)] in properties


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

*/
