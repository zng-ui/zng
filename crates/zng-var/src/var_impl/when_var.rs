//! Conditional var proxy.

use std::{
    sync::{
        Arc, Weak,
        atomic::{AtomicU32, AtomicUsize, Ordering},
    },
    time::Duration,
};

use crate::{
    AnyVar, VARS, Var,
    animation::{
        AnimationHandle, Transitionable,
        easing::{EasingStep, EasingTime},
    },
    contextual_var::{ContextInitFnImpl, ContextualVar, any_contextual_var_impl},
    shared_var::MutexHooks,
};

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
/// # Capabilities
///
/// The when var is contextualized when needed, meaning if any input is [`CONTEXT`] at the moment the var is created it
/// is also contextual. The full output type of this macro is a `Var<T>`.
///
/// [`CONTEXT`]: crate::VarCapability::CONTEXT
#[macro_export]
macro_rules! when_var {
    ($($tt:tt)*) => {
        $crate::__when_var! {
            $crate
            $($tt)*
        }
    }
}

use zng_clone_move::clmv;
#[doc(hidden)]
pub use zng_var_proc_macros::when_var as __when_var;

/// Type erased [`when_var!`] manual builder.
///
/// See [`WhenVarBuilder`] for more details.
#[derive(Clone)]
pub struct AnyWhenVarBuilder {
    conditions: Vec<(Var<bool>, AnyVar)>,
    default: AnyVar,
}
impl AnyWhenVarBuilder {
    /// New with value variable used when no other conditions are `true`.
    pub fn new(default: AnyVar) -> Self {
        AnyWhenVarBuilder {
            conditions: Vec::with_capacity(2),
            default,
        }
    }

    /// Push a conditional value.
    ///
    /// When the `condition` is `true` and all previous pushed conditions
    /// are `false` the when variable represents the `value` variable.
    pub fn push(&mut self, condition: Var<bool>, value: AnyVar) -> &mut Self {
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
    ///
    /// The `value_type` is the when var output value type.
    pub fn build(self, value_type: TypeId) -> AnyVar {
        when_var(self, value_type)
    }

    /// Convert to typed builder.
    ///
    /// Note that the type is not checked.
    pub fn into_typed<O: VarValue>(self) -> WhenVarBuilder<O> {
        WhenVarBuilder {
            builder: self,
            _t: PhantomData,
        }
    }

    /// If the `var` was built by [`build`] clones the internal conditions, values and default variables into a new builder.
    ///
    /// [`build`]: Self::build
    pub fn try_from_built(var: &AnyVar) -> Option<Self> {
        let any: &dyn Any = &*var.0;
        if let Some(built) = any.downcast_ref::<WhenVar>() {
            Some(Self {
                conditions: built.0.conditions.to_vec(),
                default: built.0.default.clone(),
            })
        } else if let Some(built) = any.downcast_ref::<ContextualVar>() {
            let init = built.init.lock();
            let init: &dyn Any = &**init;
            init.downcast_ref::<Self>().cloned()
        } else {
            None
        }
    }

    fn is_contextual(&self) -> bool {
        self.default.capabilities().is_contextual()
            || self
                .conditions
                .iter()
                .any(|(c, v)| c.capabilities().is_contextual() || v.capabilities().is_contextual())
    }

    fn current_context(&self) -> Self {
        AnyWhenVarBuilder {
            conditions: self
                .conditions
                .iter()
                .map(|(c, v)| (c.current_context(), v.current_context()))
                .collect(),
            default: self.default.current_context(),
        }
    }
}
impl AnyWhenVarBuilder {
    /// Returns the number of conditions set.
    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }
}

/// Manual [`when_var!`] builder.
#[derive(Clone)]
pub struct WhenVarBuilder<O: VarValue> {
    builder: AnyWhenVarBuilder,
    _t: PhantomData<O>,
}
impl<O: VarValue> WhenVarBuilder<O> {
    /// New with value variable used when no other conditions are `true`.
    pub fn new(default: impl IntoVar<O>) -> Self {
        Self {
            builder: AnyWhenVarBuilder::new(default.into_var().into()),
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
        Var::new_any(self.builder.build(TypeId::of::<O>()))
    }

    /// Reference the type erased when builder.
    pub fn as_any(&mut self) -> &mut AnyWhenVarBuilder {
        &mut self.builder
    }

    /// If the `var` was built by [`build`] clones the internal conditions, values and default variables into a new builder.
    ///
    /// [`build`]: Self::build
    pub fn try_from_built(var: &Var<O>) -> Option<Self> {
        // this is used by #[easing(_)] in PropertyBuildAction to modify widget properties

        let builder = AnyWhenVarBuilder::try_from_built(var)?;
        Some(Self { builder, _t: PhantomData })
    }
}

fn when_var(builder: AnyWhenVarBuilder, value_type: TypeId) -> AnyVar {
    if builder.is_contextual() {
        return any_contextual_var_impl(smallbox!(builder), value_type);
    }
    when_var_tail(builder)
}
impl ContextInitFnImpl for AnyWhenVarBuilder {
    fn init(&mut self) -> AnyVar {
        let builder = self.current_context();
        when_var_tail(builder)
    }
}
fn when_var_tail(builder: AnyWhenVarBuilder) -> AnyVar {
    AnyVar(smallbox!(when_var_tail_impl(builder)))
}
fn when_var_tail_impl(builder: AnyWhenVarBuilder) -> WhenVar {
    let data = Arc::new(WhenVarData {
        active_condition: AtomicUsize::new(builder.conditions.iter().position(|(c, _)| c.get()).unwrap_or(usize::MAX)),
        conditions: builder.conditions.into_boxed_slice(),
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
                        changed = true;
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
                        data.hooks.notify(&AnyVarHookArgs {
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

    WhenVar(data)
}

struct WhenVarData {
    conditions: Box<[(Var<bool>, AnyVar)]>,
    default: AnyVar,
    active_condition: AtomicUsize,
    hooks: MutexHooks,
    // Atomic<VarUpdateId>
    last_active_change: AtomicU32,
}
struct WhenVar(Arc<WhenVarData>);
impl WhenVar {
    fn active(&self) -> &AnyVar {
        let i = self.0.active_condition.load(Ordering::Relaxed);
        if i < self.0.conditions.len() {
            &self.0.conditions[i].1
        } else {
            &self.0.default
        }
    }
}
impl fmt::Debug for WhenVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut b = f.debug_struct("MergeVar");
        b.field("var_instance_tag()", &self.var_instance_tag());
        b.field("inputs", &self.0.conditions);
        b.field("default", &self.0.default);
        let n = self.0.active_condition.load(Ordering::Relaxed);
        b.field("active_condition", if n < self.0.conditions.len() { &n } else { &None::<usize> });
        b.field(
            "last_active_change",
            &VarUpdateId(self.0.last_active_change.load(Ordering::Relaxed)),
        );
        b.field("hooks", &self.0.hooks);
        b.finish()
    }
}
impl VarImpl for WhenVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(Self(self.0.clone()))
    }

    fn value_type(&self) -> TypeId {
        self.0.default.0.value_type()
    }

    #[cfg(feature = "type_names")]
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
        fn cap_changes(caps: VarCapability) -> VarCapability {
            let mut out = VarCapability::NEW;
            if caps.contains(VarCapability::MODIFY) || caps.contains(VarCapability::MODIFY_CHANGES) {
                out |= VarCapability::MODIFY_CHANGES;
            }
            // this is never true, already contextualized
            // if caps.contains(VarCapability::CONTEXT) || caps.contains(VarCapability::CONTEXT_CHANGES) {
            //     out |= VarCapability::CONTEXT_CHANGES;
            // }
            out
        }
        self.active().0.capabilities()
            | cap_changes(self.0.default.capabilities())
            | self
                .0
                .conditions
                .iter()
                .map(|(_, v)| cap_changes(v.capabilities()))
                .fold(VarCapability::empty(), |a, b| a | b)
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.active().0.with(visitor)
    }

    fn get(&self) -> BoxAnyVarValue {
        self.active().0.get()
    }

    fn set(&self, new_value: BoxAnyVarValue) -> bool {
        self.active().0.set(new_value)
    }

    fn update(&self) -> bool {
        self.active().0.update()
    }

    fn modify(&self, modify: SmallBox<dyn FnMut(&mut AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool {
        self.active().0.modify(modify)
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        self.0.hooks.push(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        // can be active update, or any of the conditions that updated and caused this update.
        VarUpdateId(self.0.last_active_change.load(Ordering::Relaxed)).max(self.active().0.last_update())
    }

    fn modify_info(&self) -> ModifyInfo {
        self.active().0.modify_info()
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
impl fmt::Debug for WeakWhenVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakWhenVar").field(&self.0.as_ptr()).finish()
    }
}
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

fn when_var_easing<O: VarValue + Transitionable>(builder: AnimatingWhenVarBuilder<O>) -> Var<O> {
    if builder.builder.is_contextual() {
        let any = any_contextual_var_impl(smallbox!(builder), TypeId::of::<O>());
        return Var::new_any(any);
    }
    when_var_easing_tail(builder)
}
struct AnimatingWhenVarBuilder<O: VarValue + Transitionable> {
    builder: AnyWhenVarBuilder,
    condition_easing: Vec<Option<EasingData>>,
    default_easing: EasingData,
    _t: PhantomData<fn() -> O>,
}
impl<O: VarValue + Transitionable> ContextInitFnImpl for AnimatingWhenVarBuilder<O> {
    fn init(&mut self) -> AnyVar {
        let builder = AnimatingWhenVarBuilder {
            builder: self.builder.current_context(),
            condition_easing: self.condition_easing.clone(),
            default_easing: self.default_easing.clone(),
            _t: self._t,
        };
        when_var_easing_tail(builder).any
    }
}
fn when_var_easing_tail<O: VarValue + Transitionable>(builder: AnimatingWhenVarBuilder<O>) -> Var<O> {
    let source = when_var_tail_impl(builder.builder);
    let weak_source = Arc::downgrade(&source.0);
    let output = var(source.get().downcast::<O>().unwrap());
    let weak_output = output.downgrade();
    let condition_easing = builder.condition_easing.into_boxed_slice();
    let default_easing = builder.default_easing;
    let mut _animation_handle = AnimationHandle::dummy();
    source
        .hook(smallbox!(move |args: &AnyVarHookArgs| {
            if let Some(output) = weak_output.upgrade() {
                let source = weak_source.upgrade().unwrap();
                for ((c, _), easing) in source.conditions.iter().zip(&condition_easing) {
                    if let Some((duration, func)) = easing {
                        if c.get() {
                            _animation_handle =
                                output.ease(args.downcast_value::<O>().unwrap().clone(), *duration, clmv!(func, |t| func(t)));
                            return true;
                        }
                    }
                }
                // else default
                let (duration, func) = &default_easing;
                _animation_handle = output.ease(args.downcast_value::<O>().unwrap().clone(), *duration, clmv!(func, |t| func(t)));
                true
            } else {
                false
            }
        }))
        .perm();
    output.hold(source).perm();
    output
}

type EasingData = (Duration, Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>);

impl<O: VarValue + Transitionable> WhenVarBuilder<O> {
    /// Build a variable similar to [`Var::easing`], but with different duration and easing functions for each condition.
    ///
    /// The `condition_easing` must contain one entry for each when condition, entries can be `None`, the easing used
    /// is the first entry that corresponds to a `true` condition, or falls back to the `default_easing`.
    pub fn build_easing(self, condition_easing: Vec<Option<EasingData>>, default_easing: EasingData) -> Var<O> {
        when_var_easing(AnimatingWhenVarBuilder {
            builder: self.builder,
            condition_easing,
            default_easing,
            _t: PhantomData,
        })
    }
}
