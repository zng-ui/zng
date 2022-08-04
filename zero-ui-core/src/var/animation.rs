//! Animation types and functions.

use crate::{
    crate_util::{Handle, HandleOwner, WeakHandle},
    units::*,
};
use std::{
    cell::{Cell, RefCell},
    f32::consts::*,
    fmt,
    marker::PhantomData,
    ops,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use super::{any, AnyWeakVar, IntoVar, Var, VarValue, Vars, WeakVar};

mod vars;
pub(crate) use vars::*;

pub mod easing;

/// Common easing modifier functions as an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingModifierFn {
    /// [`easing::ease_in`].
    EaseIn,
    /// [`easing::ease_out`].
    EaseOut,
    /// [`easing::ease_in_out`].
    EaseInOut,
    /// [`easing::ease_out_in`].
    EaseOutIn,
}
impl EasingModifierFn {
    /// Calls the easing function with the modifier `self` represents.
    pub fn modify(self, easing: impl Fn(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
        match self {
            EasingModifierFn::EaseIn => easing::ease_in(easing, time),
            EasingModifierFn::EaseOut => easing::ease_out(easing, time),
            EasingModifierFn::EaseInOut => easing::ease_in_out(easing, time),
            EasingModifierFn::EaseOutIn => easing::ease_out_in(easing, time),
        }
    }

    /// Create a closure that applies the `easing` with the modifier `self` represents.
    pub fn modify_fn(self, easing: impl Fn(EasingTime) -> EasingStep) -> impl Fn(EasingTime) -> EasingStep {
        move |t| self.modify(&easing, t)
    }
}
impl fmt::Display for EasingModifierFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EasingModifierFn::EaseIn => write!(f, "ease_in"),
            EasingModifierFn::EaseOut => write!(f, "ease_out"),
            EasingModifierFn::EaseInOut => write!(f, "ease_in_out"),
            EasingModifierFn::EaseOutIn => write!(f, "ease_out_in"),
        }
    }
}

/// Common easing functions as an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingFn {
    /// [`easing::linear`].
    Linear,
    /// [`easing::sine`].
    Sine,
    /// [`easing::quad`].
    Quad,
    /// [`easing::cubic`].
    Cubic,
    /// [`easing::quart`].
    Quart,
    /// [`easing::quint`].
    Quint,
    /// [`easing::expo`].
    Expo,
    /// [`easing::circ`].
    Circ,
    /// [`easing::back`].
    Back,
    /// [`easing::elastic`].
    Elastic,
    /// [`easing::bounce`].
    Bounce,
    /// [`easing::none`].
    None,
}
impl EasingFn {
    /// Calls the easing function that `self` represents.
    pub fn ease_in(self, time: EasingTime) -> EasingStep {
        (self.ease_fn())(time)
    }

    /// Calls the easing function that `self` represents and inverts the value using [`easing::ease_out`].
    pub fn ease_out(self, time: EasingTime) -> EasingStep {
        easing::ease_out(|t| self.ease_in(t), time)
    }

    /// Calls the easing function that `self` represents and transforms the value using [`easing::ease_in_out`].
    pub fn ease_in_out(self, time: EasingTime) -> EasingStep {
        easing::ease_in_out(|t| self.ease_in(t), time)
    }

    /// Gets the easing function that `self` represents.
    pub fn ease_fn(self) -> fn(EasingTime) -> EasingStep {
        match self {
            EasingFn::Linear => easing::linear,
            EasingFn::Sine => easing::sine,
            EasingFn::Quad => easing::quad,
            EasingFn::Cubic => easing::cubic,
            EasingFn::Quart => easing::quad,
            EasingFn::Quint => easing::quint,
            EasingFn::Expo => easing::expo,
            EasingFn::Circ => easing::circ,
            EasingFn::Back => easing::back,
            EasingFn::Elastic => easing::elastic,
            EasingFn::Bounce => easing::bounce,
            EasingFn::None => easing::none,
        }
    }
}
impl fmt::Display for EasingFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EasingFn::Linear => write!(f, "linear"),
            EasingFn::Sine => write!(f, "sine"),
            EasingFn::Quad => write!(f, "quad"),
            EasingFn::Cubic => write!(f, "cubic"),
            EasingFn::Quart => write!(f, "quart"),
            EasingFn::Quint => write!(f, "quint"),
            EasingFn::Expo => write!(f, "expo"),
            EasingFn::Circ => write!(f, "circ"),
            EasingFn::Back => write!(f, "back"),
            EasingFn::Elastic => write!(f, "elastic"),
            EasingFn::Bounce => write!(f, "bounce"),
            EasingFn::None => write!(f, "none"),
        }
    }
}

/// Represents a running animation created by [`Vars::animate`].
///
/// Drop all clones of this handle to stop the animation, or call [`perm`] to drop the handle
/// but keep the animation alive until it is stopped from the inside.
///
/// [`perm`]: AnimationHandle::perm
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
#[must_use = "the animation stops if the handle is dropped"]
pub struct AnimationHandle(Handle<()>);
impl AnimationHandle {
    pub(super) fn new() -> (HandleOwner<()>, Self) {
        let (owner, handle) = Handle::new(());
        (owner, AnimationHandle(handle))
    }

    /// Create dummy handle that is always in the *stopped* state.
    ///
    /// Note that `Option<AnimationHandle>` takes up the same space as `AnimationHandle` and avoids an allocation.
    pub fn dummy() -> Self {
        assert_non_null!(AnimationHandle);
        AnimationHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** stop.
    ///
    /// The animation stays in memory for the duration of the app or until another handle calls [`stop`](Self::stop).
    pub fn perm(self) {
        self.0.perm();
    }

    /// If another handle has called [`perm`](Self::perm).
    /// If `true` the animation will stay active until the app exits, unless [`stop`](Self::stop) is called.
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the animation to drop.
    pub fn stop(self) {
        self.0.force_drop();
    }

    /// If another handle has called [`stop`](Self::stop).
    ///
    /// The animation is already dropped or will be dropped in the next app update, this is irreversible.
    pub fn is_stopped(&self) -> bool {
        self.0.is_dropped()
    }

    /// Create a weak handle.
    pub fn downgrade(&self) -> WeakAnimationHandle {
        WeakAnimationHandle(self.0.downgrade())
    }
}

/// Weak [`AnimationHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakAnimationHandle(pub(super) WeakHandle<()>);
impl WeakAnimationHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Get the animation handle if it is still animating.
    pub fn upgrade(&self) -> Option<AnimationHandle> {
        self.0.upgrade().map(AnimationHandle)
    }
}

/// Represents a running chase animation created by [`Var::chase`] or other *chase* animation methods.
#[derive(Clone, Debug)]
pub struct ChaseAnimation<T> {
    /// Underlying animation handle.
    pub handle: AnimationHandle,
    pub(super) next_target: Rc<RefCell<ChaseMsg<T>>>,
}
impl<T: VarValue> ChaseAnimation<T> {
    /// Sets a new target value for the easing animation and restarts the time.
    ///
    /// The animation will update to lerp between the current variable value to the `new_target`.
    pub fn reset(&self, new_target: T) {
        *self.next_target.borrow_mut() = ChaseMsg::Replace(new_target);
    }

    /// Adds `increment` to the current target value for the easing animation and restarts the time.
    pub fn add(&self, increment: T) {
        *self.next_target.borrow_mut() = ChaseMsg::Add(increment);
    }
}
#[derive(Debug)]
pub(super) enum ChaseMsg<T> {
    None,
    Replace(T),
    Add(T),
}
impl<T> Default for ChaseMsg<T> {
    fn default() -> Self {
        Self::None
    }
}

/// Represents an animation in its closure.
///
/// See the [`Vars::animate`] method for more details.
pub struct AnimationArgs {
    start_time: Cell<Instant>,
    restart_count: Cell<usize>,
    stop: Cell<bool>,
    sleep: Cell<Option<Instant>>,
    animations_enabled: bool,
    now: Instant,
    time_scale: Factor,
}
impl AnimationArgs {
    pub(super) fn new(animations_enabled: bool, now: Instant, time_scale: Factor) -> Self {
        AnimationArgs {
            start_time: Cell::new(now),
            restart_count: Cell::new(0),
            stop: Cell::new(false),
            now,
            sleep: Cell::new(None),
            animations_enabled,
            time_scale,
        }
    }

    /// Instant this animation (re)started.
    pub fn start_time(&self) -> Instant {
        self.start_time.get()
    }

    /// Instant the current animation update started.
    ///
    /// Use this value instead of [`Instant::now`], animations update sequentially, but should behave as if
    /// they are updating exactly in parallel, using this timestamp ensures that.
    pub fn now(&self) -> Instant {
        self.now
    }

    /// Global time scale for animations.
    pub fn time_scale(&self) -> Factor {
        self.time_scale
    }

    pub(crate) fn reset_state(&mut self, enabled: bool, now: Instant, time_scale: Factor) {
        self.animations_enabled = enabled;
        self.now = now;
        self.time_scale = time_scale;
        *self.sleep.get_mut() = None;
    }

    pub(crate) fn reset_sleep(&mut self) {
        *self.sleep.get_mut() = None;
    }

    /// Set the duration to the next animation update. The animation will *sleep* until `duration` elapses.
    ///
    /// The animation awakes in the next [`Vars::frame_duration`] after the `duration` elapses, if the sleep duration is not
    /// a multiple of the frame duration it will delay an extra `frame_duration - 1ns` in the worst case. The minimum
    /// possible `duration` is the frame duration, shorter durations behave the same as if not set.
    pub fn sleep(&self, duration: Duration) {
        self.sleep.set(Some(self.now + duration));
    }

    pub(crate) fn sleep_deadline(&self) -> Option<Instant> {
        self.sleep.get()
    }

    /// Returns a value that indicates if animations are enabled in the operating system.
    ///
    /// If `false` all animations must be skipped to the end, users with photo-sensitive epilepsy disable animations system wide.
    pub fn animations_enabled(&self) -> bool {
        self.animations_enabled
    }

    /// Compute the time elapsed from [`start_time`] to [`now`].
    ///
    /// [`start_time`]: Self::start_time
    /// [`now`]: Self::now
    pub fn elapsed_dur(&self) -> Duration {
        self.now - self.start_time.get()
    }

    /// Compute the elapsed [`EasingTime`], in the span of the total `duration`, if [`animations_enabled`].
    ///
    /// If animations are disabled, returns [`EasingTime::end`], the returned time is scaled.
    ///
    /// [`animations_enabled`]: Self::animations_enabled
    pub fn elapsed(&self, duration: Duration) -> EasingTime {
        if self.animations_enabled {
            EasingTime::elapsed(duration, self.elapsed_dur(), self.time_scale)
        } else {
            EasingTime::end()
        }
    }

    /// Compute the elapsed [`EasingTime`], if the time [`is_end`] requests animation stop.
    ///
    /// [`is_end`]: EasingTime::is_end
    pub fn elapsed_stop(&self, duration: Duration) -> EasingTime {
        let t = self.elapsed(duration);
        if t.is_end() {
            self.stop()
        }
        t
    }

    /// Compute the elapsed [`EasingTime`], if the time [`is_end`] restarts the animation.
    ///
    /// [`is_end`]: EasingTime::is_end
    pub fn elapsed_restart(&self, duration: Duration) -> EasingTime {
        let t = self.elapsed(duration);
        if t.is_end() {
            self.restart()
        }
        t
    }

    /// Compute the elapsed [`EasingTime`], if the time [`is_end`] restarts the animation, repeats until has
    /// restarted `max_restarts` inclusive, then stops the animation.
    ///
    /// [`is_end`]: EasingTime::is_end
    pub fn elapsed_restart_stop(&self, duration: Duration, max_restarts: usize) -> EasingTime {
        let t = self.elapsed(duration);
        if t.is_end() {
            if self.restart_count() < max_restarts {
                self.restart();
            } else {
                self.stop();
            }
        }
        t
    }

    /// Drop the animation after applying the current update.
    pub fn stop(&self) {
        self.stop.set(true);
    }

    /// If the animation will be dropped after applying the update.
    pub fn stop_requested(&self) -> bool {
        self.stop.get()
    }

    /// Set the animation start time to now.
    pub fn restart(&self) {
        self.set_start_time(self.now);
        self.restart_count.set(self.restart_count.get() + 1);
    }

    /// Number of times the animation restarted.
    pub fn restart_count(&self) -> usize {
        self.restart_count.get()
    }

    /// Change the start time to an arbitrary value.
    ///
    /// Note that this does not affect the restart count.
    pub fn set_start_time(&self, instant: Instant) {
        self.start_time.set(instant)
    }

    /// Change the start to an instant that computes the `elapsed` for the `duration` at the moment
    /// this method is called.
    ///
    /// Note that this does not affect the restart count.
    pub fn set_elapsed(&self, elapsed: EasingTime, duration: Duration) {
        self.set_start_time(self.now - (duration * elapsed.fct()));
    }
}

/// A type that can animated by a transition.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to. To be transitionable a type must add and subtract to it self
/// and be multipliable by [`Factor`].
pub trait Transitionable:
    Clone + ops::Add<Self, Output = Self> + std::ops::AddAssign + ops::Sub<Self, Output = Self> + ops::Mul<Factor, Output = Self>
{
}
impl<T> Transitionable for T where
    T: Clone + ops::Add<T, Output = T> + std::ops::AddAssign + ops::Sub<T, Output = T> + ops::Mul<Factor, Output = T>
{
}

/// Represents a transition from one value to another that can be sampled using [`EasingStep`].
#[derive(Clone, Debug)]
pub struct Transition<T> {
    /// Value sampled at the `0.fct()` step.
    pub start: T,
    ///
    /// Value plus start is sampled at the `1.fct()` step.
    pub increment: T,
}
impl<T> Transition<T>
where
    T: Transitionable,
{
    /// New transition.
    pub fn new(from: T, to: T) -> Self {
        let increment = to - from.clone();
        Transition { start: from, increment }
    }

    /// Compute the transition value at the `step`.
    pub fn sample(&self, step: EasingStep) -> T {
        self.start.clone() + self.increment.clone() * step
    }
}

/// Represents a transition across multiple keyed values that can be sampled using [`EasingStep`].
#[derive(Clone, Debug)]
pub struct TransitionKeyed<T> {
    keys: Vec<(Factor, T)>,
}
impl<T> TransitionKeyed<T>
where
    T: Transitionable,
{
    /// New transition.
    ///
    /// Returns `None` if `keys` is empty.
    pub fn new(mut keys: Vec<(Factor, T)>) -> Option<Self> {
        if keys.is_empty() {
            return None;
        }

        // correct backtracking keyframes.
        for i in 1..keys.len() {
            if keys[i].0 < keys[i - 1].0 {
                keys[i].0 = keys[i - 1].0;
            }
        }

        Some(TransitionKeyed { keys })
    }

    /// Compute the transition value at the `step`.
    pub fn sample(&self, step: EasingStep) -> T {
        if let Some(i) = self.keys.iter().position(|(f, _)| *f > step) {
            if i == 0 {
                // step before first
                self.keys[0].1.clone()
            } else {
                let (from_step, from_value) = self.keys[i - 1].clone();
                if from_step == step {
                    // step exact key
                    from_value
                } else {
                    // linear interpolate between steps

                    let (_, to_value) = self.keys[i].clone();
                    let step = step - from_step;

                    from_value.clone() + (to_value - from_value) * step
                }
            }
        } else {
            // step is after last
            self.keys[self.keys.len() - 1].1.clone()
        }
    }
}

struct EasingVarData<T, V, F> {
    _t: PhantomData<T>,
    var: V,
    duration: Duration,
    easing: Rc<F>,
}

/// A weak reference to a [`EasingVar`].
pub struct WeakEasingVar<T, V, F>(Weak<EasingVarData<T, V, F>>);
impl<T, V, F> crate::private::Sealed for WeakEasingVar<T, V, F>
where
    T: VarValue + Transitionable,
    V: Var<T>,
    F: Fn(EasingTime) -> EasingStep + 'static,
{
}
impl<T, V, F> Clone for WeakEasingVar<T, V, F>
where
    T: VarValue + Transitionable,
    V: Var<T>,
    F: Fn(EasingTime) -> EasingStep + 'static,
{
    fn clone(&self) -> Self {
        WeakEasingVar(self.0.clone())
    }
}
impl<T, V, F> AnyWeakVar for WeakEasingVar<T, V, F>
where
    T: VarValue + Transitionable,
    V: Var<T>,
    F: Fn(EasingTime) -> EasingStep + 'static,
{
    fn into_any(self) -> Box<dyn any::AnyWeakVar> {
        Box::new(self)
    }

    any_var_impls!(WeakVar);
}
impl<T, V, F> WeakVar<T> for WeakEasingVar<T, V, F>
where
    T: VarValue + Transitionable,
    V: Var<T>,
    F: Fn(EasingTime) -> EasingStep + 'static,
{
    type Strong = EasingVar<T, V, F>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(EasingVar)
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr() as *const ()
    }
}

/// Wraps another variable and turns assigns into transition animations.
///
/// Redirects calls to [`Var::set`] to [`Var::ease`] and [`Var::set_ne`] to [`Var::ease_ne`], calls to the
/// methods that create animations by default are not affected by the var easing.
///
/// Use [`Var::easing`] to create.
pub struct EasingVar<T, V, F>(Rc<EasingVarData<T, V, F>>);
impl<T, V, F> EasingVar<T, V, F>
where
    T: VarValue + Transitionable,
    V: Var<T>,
    F: Fn(EasingTime) -> EasingStep + 'static,
{
    /// New easing var.
    pub fn new(var: V, duration: impl Into<Duration>, easing: F) -> Self {
        EasingVar(Rc::new(EasingVarData {
            _t: PhantomData,
            var,
            duration: duration.into(),
            easing: Rc::new(easing),
        }))
    }

    /// Create a weak reference to the var.
    pub fn downgrade(&self) -> WeakEasingVar<T, V, F> {
        WeakEasingVar(Rc::downgrade(&self.0))
    }
}
impl<T, V, F> crate::private::Sealed for EasingVar<T, V, F> {}
impl<T, V: Clone, F> Clone for EasingVar<T, V, F> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T, V, F> Var<T> for EasingVar<T, V, F>
where
    T: VarValue + Transitionable,
    V: Var<T>,
    F: Fn(EasingTime) -> EasingStep + 'static,
{
    type AsReadOnly = V::AsReadOnly;

    fn get<'a, Vr: AsRef<super::VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        self.0.var.get(vars)
    }

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        self.0.var.get_new(vars)
    }

    fn is_new<Vw: super::WithVars>(&self, vars: &Vw) -> bool {
        self.0.var.is_new(vars)
    }

    fn version<Vr: super::WithVarsRead>(&self, vars: &Vr) -> super::VarVersion {
        self.0.var.version(vars)
    }

    fn is_read_only<Vw: super::WithVars>(&self, vars: &Vw) -> bool {
        self.0.var.is_read_only(vars)
    }

    fn is_animating<Vr: super::WithVarsRead>(&self, vars: &Vr) -> bool {
        self.0.var.is_animating(vars)
    }

    fn always_read_only(&self) -> bool {
        self.0.var.always_read_only()
    }

    fn is_contextual(&self) -> bool {
        self.0.var.is_contextual()
    }

    fn actual_var<Vw: super::WithVars>(&self, vars: &Vw) -> super::BoxedVar<T> {
        if self.is_contextual() {
            let var = EasingVar(Rc::new(EasingVarData {
                _t: PhantomData,
                var: self.0.var.actual_var(vars),
                duration: self.0.duration,
                easing: self.0.easing.clone(),
            }));
            var.boxed()
        } else {
            self.clone().boxed()
        }
    }

    fn can_update(&self) -> bool {
        self.0.var.can_update()
    }

    fn into_value<Vr: super::WithVarsRead>(self, vars: &Vr) -> T {
        match Rc::try_unwrap(self.0) {
            Ok(v) => v.var.into_value(vars),
            Err(v) => v.var.get_clone(vars),
        }
    }

    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), super::VarIsReadOnly>
    where
        Vw: super::WithVars,
        M: FnOnce(super::VarModify<T>) + 'static,
    {
        self.0.var.modify(vars, modify)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        match Rc::try_unwrap(self.0) {
            Ok(v) => v.var.into_read_only(),
            Err(v) => v.var.clone().into_read_only(),
        }
    }

    fn update_mask<Vr: super::WithVarsRead>(&self, vars: &Vr) -> crate::widget_info::UpdateMask {
        self.0.var.update_mask(vars)
    }

    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), super::VarIsReadOnly>
    where
        Vw: super::WithVars,
        N: Into<T>,
    {
        vars.with_vars(|vars| {
            if self.is_read_only(vars) {
                Err(super::VarIsReadOnly)
            } else {
                let easing = self.0.easing.clone();
                self.0.var.ease(vars, new_value, self.0.duration, move |t| easing(t)).perm();
                Ok(())
            }
        })
    }

    fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, super::VarIsReadOnly>
    where
        Vw: super::WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        vars.with_vars(|vars| {
            if self.is_read_only(vars) {
                Err(super::VarIsReadOnly)
            } else {
                let new_value = new_value.into();
                if self.0.var.get(vars) != &new_value {
                    let easing = self.0.easing.clone();
                    self.0.var.ease_ne(vars, new_value, self.0.duration, move |t| easing(t)).perm();
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        })
    }

    fn ease<Vw, N, F2>(&self, vars: &Vw, new_value: N, duration: Duration, easing: F2) -> AnimationHandle
    where
        Vw: super::WithVars,
        N: Into<T>,
        F2: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.0.var.ease(vars, new_value, duration, easing)
    }

    fn ease_ne<Vw, N, F2>(&self, vars: &Vw, new_value: N, duration: Duration, easing: F2) -> AnimationHandle
    where
        Vw: super::WithVars,
        N: Into<T>,
        F2: Fn(EasingTime) -> EasingStep + 'static,

        T: PartialEq,
    {
        self.0.var.ease_ne(vars, new_value, duration, easing)
    }

    fn ease_keyed<Vw, F2>(&self, vars: &Vw, keys: Vec<(Factor, T)>, duration: Duration, easing: F2) -> AnimationHandle
    where
        Vw: super::WithVars,
        F2: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.0.var.ease_keyed(vars, keys, duration, easing)
    }

    fn set_ease<Vw, N, Th, F2>(&self, vars: &Vw, new_value: N, then: Th, duration: Duration, easing: F2) -> AnimationHandle
    where
        Vw: super::WithVars,
        N: Into<T>,
        Th: Into<T>,
        F2: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.0.var.set_ease(vars, new_value, then, duration, easing)
    }

    fn set_ease_ne<Vw, N, Th, F2>(&self, vars: &Vw, new_value: N, then: Th, duration: Duration, easing: F2) -> AnimationHandle
    where
        Vw: super::WithVars,
        N: Into<T>,
        Th: Into<T>,
        F2: Fn(EasingTime) -> EasingStep + 'static,

        T: PartialEq,
    {
        self.0.var.set_ease_ne(vars, new_value, then, duration, easing)
    }

    fn set_ease_keyed<Vw, F2>(&self, vars: &Vw, keys: Vec<(Factor, T)>, duration: Duration, easing: F2) -> AnimationHandle
    where
        Vw: super::WithVars,
        F2: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.0.var.set_ease_keyed(vars, keys, duration, easing)
    }

    fn step<Vw, N>(&self, vars: &Vw, new_value: N, delay: Duration) -> AnimationHandle
    where
        Vw: super::WithVars,
        N: Into<T>,
    {
        self.0.var.step(vars, new_value, delay)
    }

    fn step_ne<Vw, N>(&self, vars: &Vw, new_value: N, delay: Duration) -> AnimationHandle
    where
        Vw: super::WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        self.0.var.step_ne(vars, new_value, delay)
    }

    fn steps<Vw, F2>(&self, vars: &Vw, steps: Vec<(Factor, T)>, duration: Duration, easing: F2) -> AnimationHandle
    where
        Vw: super::WithVars,
        F2: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.0.var.steps(vars, steps, duration, easing)
    }

    fn steps_ne<Vw, F2>(&self, vars: &Vw, steps: Vec<(Factor, T)>, duration: Duration, easing: F2) -> AnimationHandle
    where
        Vw: super::WithVars,
        F2: Fn(EasingTime) -> EasingStep + 'static,
        T: PartialEq,
    {
        self.0.var.steps_ne(vars, steps, duration, easing)
    }

    type Weak = WeakEasingVar<T, V, F>;

    fn is_rc(&self) -> bool {
        true
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn downgrade(&self) -> Option<Self::Weak> {
        Some(self.downgrade())
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    fn as_ptr(&self) -> *const () {
        Rc::as_ptr(&self.0) as _
    }
}
impl<T, V, F> IntoVar<T> for EasingVar<T, V, F>
where
    T: VarValue + Transitionable,
    V: Var<T>,
    F: Fn(EasingTime) -> EasingStep + 'static,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
impl<T, V, F> any::AnyVar for EasingVar<T, V, F>
where
    T: VarValue + Transitionable,
    V: Var<T>,
    F: Fn(EasingTime) -> EasingStep + 'static,
{
    fn into_any(self) -> Box<dyn any::AnyVar> {
        Box::new(self)
    }

    any_var_impls!(Var);
}
