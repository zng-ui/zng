//! Var animation types and functions.

use std::{any::Any, fmt, sync::Arc, time::Duration};

use parking_lot::Mutex;
use smallbox::SmallBox;
use zng_app_context::context_local;
use zng_handle::{Handle, HandleOwner, WeakHandle};
use zng_time::{DInstant, Deadline};
use zng_unit::Factor;

use crate::{
    Var, VarValue,
    animation::easing::{EasingStep, EasingTime},
};

pub mod easing;
pub use zng_var_proc_macros::Transitionable;

/// View on an app loop timer.
pub trait AnimationTimer {
    /// Returns `true` if the `deadline` has elapsed, `false` if the `deadline` was
    /// registered for future waking.
    fn elapsed(&mut self, deadline: Deadline) -> bool;

    /// Register the future `deadline` for waking.
    fn register(&mut self, deadline: Deadline);

    /// Frame timestamp.
    fn now(&self) -> DInstant;
}

/// Animations controller.
///
/// See [`VARS.with_animation_controller`] for more details.
///
/// [`VARS.with_animation_controller`]: crate::VARS::with_animation_controller
pub trait AnimationController: Send + Sync + Any {
    /// Called for each `animation` that starts in the controller context.
    ///
    /// Note that this handler itself is not called inside the controller context.
    fn on_start(&self, animation: &Animation) {
        let _ = animation;
    }

    /// Called for each `animation` that ends in the controller context.
    ///
    /// Note that this handler itself is not called inside the controller context.
    fn on_stop(&self, animation: &Animation) {
        let _ = animation;
    }
}

impl AnimationController for () {}

/// An [`AnimationController`] that forces animations to run even if animations are not enabled.
pub struct ForceAnimationController;
impl AnimationController for ForceAnimationController {
    fn on_start(&self, animation: &Animation) {
        animation.force_enable();
    }
}

context_local! {
    pub(crate) static VARS_ANIMATION_CTRL_CTX: Box<dyn AnimationController> = {
        let r: Box<dyn AnimationController> = Box::new(());
        r
    };
}

/// Represents an animation in its closure.
///
/// See the [`VARS.animate`] method for more details.
///
/// [`VARS.animate`]: crate::VARS::animate
#[derive(Clone)]
pub struct Animation(Arc<Mutex<AnimationData>>);
struct AnimationData {
    start_time: DInstant,
    restart_count: usize,
    stop: bool,
    sleep: Option<Deadline>,
    animations_enabled: bool,
    force_enabled: bool,
    now: DInstant,
    time_scale: Factor,
}

impl Animation {
    pub(super) fn new(animations_enabled: bool, now: DInstant, time_scale: Factor) -> Self {
        Animation(Arc::new(Mutex::new(AnimationData {
            start_time: now,
            restart_count: 0,
            stop: false,
            now,
            sleep: None,
            animations_enabled,
            force_enabled: false,
            time_scale,
        })))
    }

    /// The instant this animation (re)started.
    pub fn start_time(&self) -> DInstant {
        self.0.lock().start_time
    }

    /// The instant the current animation update started.
    ///
    /// Use this value instead of [`INSTANT.now`], animations update sequentially, but should behave as if
    /// they are updating exactly in parallel, using this timestamp ensures that.
    ///
    /// [`INSTANT.now`]: zng_time::INSTANT::now
    pub fn now(&self) -> DInstant {
        self.0.lock().now
    }

    /// Global time scale for animations.
    pub fn time_scale(&self) -> Factor {
        self.0.lock().time_scale
    }

    pub(crate) fn reset_state(&self, enabled: bool, now: DInstant, time_scale: Factor) {
        let mut m = self.0.lock();
        if !m.force_enabled {
            m.animations_enabled = enabled;
        }
        m.now = now;
        m.time_scale = time_scale;
        m.sleep = None;
    }

    pub(crate) fn reset_sleep(&self) {
        self.0.lock().sleep = None;
    }

    /// Set the duration to the next animation update. The animation will *sleep* until `duration` elapses.
    ///
    /// The animation awakes in the next [`VARS.frame_duration`] after the `duration` elapses. The minimum
    /// possible `duration` is the frame duration, shorter durations behave the same as if not set.
    ///
    /// [`VARS.frame_duration`]: crate::VARS::frame_duration
    pub fn sleep(&self, duration: Duration) {
        let mut me = self.0.lock();
        me.sleep = Some(Deadline(me.now + duration));
    }

    pub(crate) fn sleep_deadline(&self) -> Option<Deadline> {
        self.0.lock().sleep
    }

    /// Returns a value that indicates if animations are enabled in the operating system.
    ///
    /// If `false` all animations must be skipped to the end, users with photo-sensitive epilepsy disable animations system wide.
    pub fn animations_enabled(&self) -> bool {
        self.0.lock().animations_enabled
    }

    /// Set [`animations_enabled`] to `true`.
    ///
    /// This should only be used for animations that are component of an app feature, cosmetic animations must not force enable.
    ///
    /// [`animations_enabled`]: crate::VARS::animations_enabled
    pub fn force_enable(&self) {
        let mut me = self.0.lock();
        me.force_enabled = true;
        me.animations_enabled = true;
    }

    /// Compute the time elapsed from [`start_time`] to [`now`].
    ///
    /// [`start_time`]: Self::start_time
    /// [`now`]: Self::now
    pub fn elapsed_dur(&self) -> Duration {
        let me = self.0.lock();
        me.now - me.start_time
    }

    /// Compute the elapsed [`EasingTime`], in the span of the total `duration`, if [`animations_enabled`].
    ///
    /// If animations are disabled, returns [`EasingTime::end`], the returned time is scaled.
    ///
    /// [`animations_enabled`]: Self::animations_enabled
    pub fn elapsed(&self, duration: Duration) -> EasingTime {
        let me = self.0.lock();
        if me.animations_enabled {
            EasingTime::elapsed(duration, me.now - me.start_time, me.time_scale)
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
        self.0.lock().stop = true;
    }

    /// If the animation will be dropped after applying the update.
    pub fn stop_requested(&self) -> bool {
        self.0.lock().stop
    }

    /// Set the animation start time to now.
    pub fn restart(&self) {
        let now = self.0.lock().now;
        self.set_start_time(now);
        let mut me = self.0.lock();
        me.restart_count += 1;
    }

    /// Number of times the animation restarted.
    pub fn restart_count(&self) -> usize {
        self.0.lock().restart_count
    }

    /// Change the start time to an arbitrary value.
    ///
    /// Note that this does not affect the restart count.
    pub fn set_start_time(&self, instant: DInstant) {
        self.0.lock().start_time = instant;
    }

    /// Change the start to an instant that computes the `elapsed` for the `duration` at the moment
    /// this method is called.
    ///
    /// Note that this does not affect the restart count.
    pub fn set_elapsed(&self, elapsed: EasingTime, duration: Duration) {
        let now = self.0.lock().now;
        self.set_start_time(now.checked_sub(duration * elapsed.fct()).unwrap());
    }
}

/// Represents the current *modify* operation when it is applying.
#[derive(Clone)]
pub struct ModifyInfo {
    pub(crate) handle: Option<WeakAnimationHandle>,
    pub(crate) importance: usize,
}
impl ModifyInfo {
    /// Initial value, is always of lowest importance.
    pub fn never() -> Self {
        ModifyInfo {
            handle: None,
            importance: 0,
        }
    }

    /// Indicates the *override* importance of the operation, when two animations target
    /// a variable only the newer one must apply, and all running animations are *overridden* by
    /// a later modify/set operation.
    ///
    /// Variables ignore modify requests from lower importance closures.
    pub fn importance(&self) -> usize {
        self.importance
    }

    /// Indicates if the *modify* request was made from inside an animation, if `true` the [`importance`]
    /// is for that animation, even if the modify request is from the current frame.
    ///
    /// You can clone this info to track this animation, when it stops or is dropped this returns `false`. Note
    /// that sleeping animations still count as animating.
    ///
    /// [`importance`]: Self::importance
    pub fn is_animating(&self) -> bool {
        self.handle.as_ref().map(|h| h.upgrade().is_some()).unwrap_or(false)
    }

    /// Returns `true` if `self` and `other` have the same animation or are both not animating.
    pub fn animation_eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }

    /// Register a `handler` to be called once when the current animation stops.
    ///
    /// [`importance`]: Self::importance
    pub fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn> {
        if let Some(h) = &self.handle
            && let Some(h) = h.upgrade()
        {
            return h.hook_animation_stop(handler);
        }
        Err(handler)
    }
}
impl fmt::Debug for ModifyInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifyInfo")
            .field("is_animating()", &self.is_animating())
            .field("importance()", &self.importance)
            .finish()
    }
}

pub(crate) type AnimationStopFn = SmallBox<dyn FnMut() + Send + 'static, smallbox::space::S4>;

#[derive(Default)]
pub(super) struct AnimationHandleData {
    on_drop: Mutex<Vec<AnimationStopFn>>,
}
impl Drop for AnimationHandleData {
    fn drop(&mut self) {
        for mut f in self.on_drop.get_mut().drain(..) {
            f()
        }
    }
}
/// Represents a running animation.
///
/// Drop all clones of this handle to stop the animation, or call [`perm`] to drop the handle
/// but keep the animation alive until it is stopped from the inside.
///
/// [`perm`]: AnimationHandle::perm
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
#[must_use = "the animation stops if the handle is dropped"]
pub struct AnimationHandle(Handle<AnimationHandleData>);
impl Default for AnimationHandle {
    /// `dummy`.
    fn default() -> Self {
        Self::dummy()
    }
}
impl AnimationHandle {
    pub(super) fn new() -> (HandleOwner<AnimationHandleData>, Self) {
        let (owner, handle) = Handle::new(AnimationHandleData::default());
        (owner, AnimationHandle(handle))
    }

    /// Create dummy handle that is always in the *stopped* state.
    ///
    /// Note that `Option<AnimationHandle>` takes up the same space as `AnimationHandle` and avoids an allocation.
    pub fn dummy() -> Self {
        AnimationHandle(Handle::dummy(AnimationHandleData::default()))
    }

    /// Drops the handle but does **not** stop.
    ///
    /// The animation stays in memory for the duration of the app or until another handle calls [`stop`](Self::stop).
    pub fn perm(self) {
        self.0.perm();
    }

    /// If another handle has called [`perm`](Self::perm).
    ///
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

    /// Register a `handler` to be called once when the animation stops.
    ///
    /// Returns the `handler` if the animation has already stopped.
    ///
    /// [`importance`]: ModifyInfo::importance
    pub fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn> {
        if !self.is_stopped() {
            self.0.data().on_drop.lock().push(handler);
            Ok(())
        } else {
            Err(handler)
        }
    }
}

/// Weak [`AnimationHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakAnimationHandle(pub(super) WeakHandle<AnimationHandleData>);
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

/// Represents a type that can be animated between two values.
///
/// This trait is auto-implemented for all [`Copy`] types that can add, subtract and multiply by [`Factor`], [`Clone`]
/// only types must implement this trait manually.
///
/// [`Factor`]: zng_unit::Factor
pub trait Transitionable: VarValue {
    /// Sample the linear interpolation from `self` -> `to` by `step`.  
    fn lerp(self, to: &Self, step: EasingStep) -> Self;
}

/// Represents a simple transition between two values.
#[non_exhaustive]
pub struct Transition<T> {
    /// Value sampled at the `0.fct()` step.
    pub from: T,
    ///
    /// Value sampled at the `1.fct()` step.
    pub to: T,
}
impl<T> Transition<T>
where
    T: Transitionable,
{
    /// New transition.
    pub fn new(from: T, to: T) -> Self {
        Self { from, to }
    }

    /// Compute the transition value at the `step`.
    pub fn sample(&self, step: EasingStep) -> T {
        self.from.clone().lerp(&self.to, step)
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

    /// Keyed values.
    pub fn keys(&self) -> &[(Factor, T)] {
        &self.keys
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

                    let (_, to_value) = &self.keys[i];
                    let step = step - from_step;

                    from_value.lerp(to_value, step)
                }
            }
        } else {
            // step is after last
            self.keys[self.keys.len() - 1].1.clone()
        }
    }
}

/// Represents the editable final value of a [`Var::chase`] animation.
pub struct ChaseAnimation<T: VarValue + Transitionable> {
    pub(super) target: T,
    pub(super) var: Var<T>,
    pub(super) handle: AnimationHandle,
}
impl<T> fmt::Debug for ChaseAnimation<T>
where
    T: VarValue + Transitionable,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChaseAnimation")
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}
impl<T> ChaseAnimation<T>
where
    T: VarValue + Transitionable,
{
    /// Current animation target.
    pub fn target(&self) -> &T {
        &self.target
    }

    /// Modify the chase target, replaces the animation with a new one from the current value to the modified target.
    pub fn modify(&mut self, modify: impl FnOnce(&mut T), duration: Duration, easing: impl Fn(EasingTime) -> EasingStep + Send + 'static) {
        if self.handle.is_stopped() {
            // re-sync target
            self.target = self.var.get();
        }
        modify(&mut self.target);
        self.handle = self.var.ease(self.target.clone(), duration, easing);
    }

    /// Replace the chase target, replaces the animation with a new one from the current value to the modified target.
    pub fn set(&mut self, value: impl Into<T>, duration: Duration, easing: impl Fn(EasingTime) -> EasingStep + Send + 'static) {
        self.target = value.into();
        self.handle = self.var.ease(self.target.clone(), duration, easing);
    }
}

/// Spherical linear interpolation sampler.
///
/// Animates rotations over the shortest change between angles by modulo wrapping.
/// A transition from 358º to 1º goes directly to 361º (modulo normalized to 1º).
///
/// Types that support this use the [`is_slerp_enabled`] function inside [`Transitionable::lerp`] to change
/// mode, types that don't support this use the normal linear interpolation. All angle and transform units
/// implement this.
///
/// Samplers can be set in animations using the `Var::easing_with` method.
pub fn slerp_sampler<T: Transitionable>(t: &Transition<T>, step: EasingStep) -> T {
    slerp_enabled(true, || t.sample(step))
}

/// Gets if slerp mode is enabled in the context.
///
/// See [`slerp_sampler`] for more details.
pub fn is_slerp_enabled() -> bool {
    SLERP_ENABLED.get_clone()
}

/// Calls `f` with [`is_slerp_enabled`] set to `enabled`.
///
/// See [`slerp_sampler`] for a way to enable in animations.
pub fn slerp_enabled<R>(enabled: bool, f: impl FnOnce() -> R) -> R {
    SLERP_ENABLED.with_context(&mut Some(Arc::new(enabled)), f)
}

context_local! {
    static SLERP_ENABLED: bool = false;
}

/// API for app implementers to replace the transitionable implementation for foreign types.
#[expect(non_camel_case_types)]
pub struct TRANSITIONABLE_APP;
