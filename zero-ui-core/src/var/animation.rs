//! Var animation types and functions.

use std::{
    mem, ops,
    time::{Duration, Instant},
};

use crate::{app::LoopTimer, clone_move, crate_util};

use super::*;

pub mod easing;

/// Represents a running animation created by [`Animations.animate`].
///
/// Drop all clones of this handle to stop the animation, or call [`perm`] to drop the handle
/// but keep the animation alive until it is stopped from the inside.
///
/// [`perm`]: AnimationHandle::perm
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
#[must_use = "the animation stops if the handle is dropped"]
pub struct AnimationHandle(crate_util::Handle<()>);
impl AnimationHandle {
    pub(super) fn new() -> (crate_util::HandleOwner<()>, Self) {
        let (owner, handle) = crate_util::Handle::new(());
        (owner, AnimationHandle(handle))
    }

    /// Create dummy handle that is always in the *stopped* state.
    ///
    /// Note that `Option<AnimationHandle>` takes up the same space as `AnimationHandle` and avoids an allocation.
    pub fn dummy() -> Self {
        assert_non_null!(AnimationHandle);
        AnimationHandle(crate_util::Handle::dummy(()))
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
pub struct WeakAnimationHandle(pub(super) crate_util::WeakHandle<()>);
impl WeakAnimationHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(crate_util::WeakHandle::new())
    }

    /// Get the animation handle if it is still animating.
    pub fn upgrade(&self) -> Option<AnimationHandle> {
        self.0.upgrade().map(AnimationHandle)
    }
}

struct AnimationData {
    start_time: Instant,
    restart_count: usize,
    stop: bool,
    sleep: Option<Deadline>,
    animations_enabled: bool,
    now: Instant,
    time_scale: Factor,
}

/// Represents an animation in its closure.
///
/// See the [`Vars.animate`] method for more details.
#[derive(Clone)]
pub struct Animation(Arc<Mutex<AnimationData>>);
impl Animation {
    pub(super) fn new(animations_enabled: bool, now: Instant, time_scale: Factor) -> Self {
        Animation(Arc::new(Mutex::new(AnimationData {
            start_time: now,
            restart_count: 0,
            stop: false,
            now,
            sleep: None,
            animations_enabled,
            time_scale,
        })))
    }

    /// Instant this animation (re)started.
    pub fn start_time(&self) -> Instant {
        self.0.lock().start_time
    }

    /// Instant the current animation update started.
    ///
    /// Use this value instead of [`Instant::now`], animations update sequentially, but should behave as if
    /// they are updating exactly in parallel, using this timestamp ensures that.
    pub fn now(&self) -> Instant {
        self.0.lock().now
    }

    /// Global time scale for animations.
    pub fn time_scale(&self) -> Factor {
        self.0.lock().time_scale
    }

    pub(crate) fn reset_state(&self, enabled: bool, now: Instant, time_scale: Factor) {
        let mut m = self.0.lock();
        m.animations_enabled = enabled;
        m.now = now;
        m.time_scale = time_scale;
        m.sleep = None;
    }

    pub(crate) fn reset_sleep(&self) {
        self.0.lock().sleep = None;
    }

    /// Set the duration to the next animation update. The animation will *sleep* until `duration` elapses.
    ///
    /// The animation awakes in the next [`Vars::frame_duration`] after the `duration` elapses, if the sleep duration is not
    /// a multiple of the frame duration it will delay an extra `frame_duration - 1ns` in the worst case. The minimum
    /// possible `duration` is the frame duration, shorter durations behave the same as if not set.
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
    pub fn set_start_time(&self, instant: Instant) {
        self.0.lock().start_time = instant;
    }

    /// Change the start to an instant that computes the `elapsed` for the `duration` at the moment
    /// this method is called.
    ///
    /// Note that this does not affect the restart count.
    pub fn set_elapsed(&self, elapsed: EasingTime, duration: Duration) {
        let now = self.0.lock().now;
        self.set_start_time(now - (duration * elapsed.fct()));
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

pub(super) struct Animations {
    animations: RefCell<Vec<AnimationFn>>,
    animation_id: Cell<usize>,
    pub(super) current_modify: RefCell<ModifyInfo>,
    pub(super) animation_start_time: Cell<Option<Instant>>,
    next_frame: Cell<Option<Deadline>>,
    pub(super) animations_enabled: RcVar<bool>,
    pub(super) frame_duration: RcVar<Duration>,
    pub(super) animation_time_scale: RcVar<Factor>,
}
impl Animations {
    pub(crate) fn new() -> Self {
        Self {
            animations: RefCell::default(),
            animation_id: Cell::new(1),
            current_modify: RefCell::new(ModifyInfo {
                handle: None,
                importance: 1,
            }),
            animation_start_time: Cell::new(None),
            next_frame: Cell::new(None),
            animations_enabled: var(true),
            frame_duration: var((1.0 / 60.0).secs()),
            animation_time_scale: var(1.fct()),
        }
    }

    pub(super) fn update_animations(vars: &mut Vars, timer: &mut LoopTimer) {
        if let Some(next_frame) = vars.ans.next_frame.get() {
            if timer.elapsed(next_frame) {
                let mut animations = mem::take(&mut *vars.ans.animations.borrow_mut());
                debug_assert!(!animations.is_empty());

                let info = AnimationUpdateInfo {
                    animations_enabled: vars.ans.animations_enabled.get(),
                    time_scale: vars.ans.animation_time_scale.get(),
                    now: timer.now(),
                    next_frame: next_frame + vars.ans.frame_duration.get(),
                };

                let mut min_sleep = Deadline(info.now + Duration::from_secs(60 * 60));

                animations.retain_mut(|animate| {
                    if let Some(sleep) = animate(vars, info) {
                        min_sleep = min_sleep.min(sleep);
                        true
                    } else {
                        false
                    }
                });

                let mut self_animations = vars.ans.animations.borrow_mut();
                animations.extend(self_animations.drain(..));
                *self_animations = animations;

                if !self_animations.is_empty() {
                    vars.ans.next_frame.set(Some(min_sleep));
                    timer.register(min_sleep);
                } else {
                    vars.ans.next_frame.set(None);
                }
            }
        }
    }

    pub(super) fn next_deadline(vars: &mut Vars, timer: &mut LoopTimer) {
        if let Some(next_frame) = vars.ans.next_frame.get() {
            timer.register(next_frame);
        }
    }

    pub(crate) fn animate<A>(vars: &Vars, mut animation: A) -> AnimationHandle
    where
        A: FnMut(&Vars, &Animation) + 'static,
    {
        // # Animation ID
        //
        // Variables only accept modifications from an animation ID >= the previous animation ID that modified it.
        //
        // Direct modifications always overwrite previous animations, so we advance the ID for each call to
        // this method **and then** advance the ID again for all subsequent direct modifications.
        //
        // Example sequence of events:
        //
        // |ID| Modification  | Accepted
        // |--|---------------|----------
        // | 1| Var::set      | YES
        // | 2| Var::ease     | YES
        // | 2| ease update   | YES
        // | 3| Var::set      | YES
        // | 3| Var::set      | YES
        // | 2| ease update   | NO
        // | 4| Var::ease     | YES
        // | 2| ease update   | NO
        // | 4| ease update   | YES
        // | 5| Var::set      | YES
        // | 2| ease update   | NO
        // | 4| ease update   | NO

        // ensure that all animations started in this update have the same exact time, we update then with the same `now`
        // timestamp also, this ensures that synchronized animations match perfectly.
        let start_time = if let Some(t) = vars.ans.animation_start_time.get() {
            t
        } else {
            let t = Instant::now();
            vars.ans.animation_start_time.set(Some(t));
            t
        };

        let mut id = vars.ans.animation_id.get().wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        let mut next_set_id = id.wrapping_add(1);
        if next_set_id == 0 {
            next_set_id = 1;
        }
        vars.ans.animation_id.set(next_set_id);
        vars.ans.current_modify.borrow_mut().importance = next_set_id;

        let handle_owner;
        let handle;
        let weak_handle;

        if let Some(parent_handle) = vars.ans.current_modify.borrow().handle.clone() {
            // is `animate` request inside other animate closure,
            // in this case we give it the same animation handle as the *parent*
            // animation, that holds the actual handle owner.
            handle_owner = None;

            if let Some(h) = parent_handle.upgrade() {
                handle = h;
            } else {
                // attempt to create new animation from inside dropping animation, ignore
                return AnimationHandle::dummy();
            }

            weak_handle = parent_handle;
        } else {
            let (o, h) = AnimationHandle::new();
            handle_owner = Some(o);
            weak_handle = h.downgrade();
            handle = h;
        };

        let anim = Animation::new(vars.ans.animations_enabled.get(), start_time, vars.ans.animation_time_scale.get());
        vars.ans.animations.borrow_mut().push(Box::new(move |vars, info| {
            let _handle_owner = &handle_owner; // capture and own the handle owner.

            if weak_handle.upgrade().is_some() {
                if anim.stop_requested() {
                    // drop
                    return None;
                }

                if let Some(sleep) = anim.sleep_deadline() {
                    if sleep > info.next_frame {
                        // retain sleep
                        return Some(sleep);
                    } else if sleep.0 > info.now {
                        // sync-up to frame rate after sleep
                        anim.reset_sleep();
                        return Some(info.next_frame);
                    }
                }

                anim.reset_state(info.animations_enabled, info.now, info.time_scale);

                let prev = mem::replace(
                    &mut *vars.ans.current_modify.borrow_mut(),
                    ModifyInfo {
                        handle: Some(weak_handle.clone()),
                        importance: id,
                    },
                );
                let _cleanup = crate_util::RunOnDrop::new(|| *vars.ans.current_modify.borrow_mut() = prev);

                animation(vars, &anim);

                // retain until next frame
                //
                // stop or sleep may be requested after this (during modify apply),
                // these updates are applied on the next frame.
                Some(info.next_frame)
            } else {
                // drop
                None
            }
        }));

        if vars.ans.next_frame.get().is_none() {
            vars.ans.next_frame.set(Some(Deadline(Instant::now())));
        }

        handle
    }
}

type AnimationFn = Box<dyn FnMut(&Vars, AnimationUpdateInfo) -> Option<Deadline>>;

#[derive(Clone, Copy)]
struct AnimationUpdateInfo {
    animations_enabled: bool,
    now: Instant,
    time_scale: Factor,
    next_frame: Deadline,
}

pub(super) fn var_animate<T: VarValue>(
    vars: &Vars,
    target: &impl Var<T>,
    animate: impl FnMut(&Animation, &mut Cow<T>) + 'static,
) -> AnimationHandle {
    if !target.capabilities().is_always_read_only() {
        let target = target.clone().actual_var();
        if !target.capabilities().is_always_read_only() {
            let wk_target = target.downgrade();
            let animate = Rc::new(RefCell::new(animate));
            return vars.animate(move |vars, args| {
                if let Some(target) = wk_target.upgrade() {
                    let r = target.modify(
                        vars,
                        clone_move!(animate, args, |value| {
                            (animate.borrow_mut())(&args, value);
                        }),
                    );
                    if let Err(VarIsReadOnlyError { .. }) = r {
                        // var can maybe change to allow write again, but we wipe all animations anyway.
                        args.stop();
                    }
                } else {
                    // target dropped.
                    args.stop();
                }
            });
        }
    }
    AnimationHandle::dummy()
}

pub(super) fn var_set_ease<T>(
    start_value: T,
    end_value: T,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
    init_step: EasingStep, // set to 0 skips first frame, set to 999 includes first frame.
) -> impl FnMut(&Animation, &mut Cow<T>)
where
    T: VarValue + Transitionable,
{
    let transition = Transition::new(start_value, end_value);
    let mut prev_step = init_step;
    move |args, value| {
        let step = easing(args.elapsed_stop(duration));

        if prev_step != step {
            *value = Cow::Owned(transition.sample(step));
            prev_step = step;
        }
    }
}

pub(super) fn var_set_ease_ne<T>(
    start_value: T,
    end_value: T,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
    init_step: EasingStep, // set to 0 skips first frame, set to 999 includes first frame.
) -> impl FnMut(&Animation, &mut Cow<T>)
where
    T: VarValue + Transitionable + PartialEq,
{
    let transition = Transition::new(start_value, end_value);
    let mut prev_step = init_step;
    move |args, value| {
        let step = easing(args.elapsed_stop(duration));

        if prev_step != step {
            let val = transition.sample(step);
            if value.as_ref() != &val {
                *value = Cow::Owned(val);
            }
            prev_step = step;
        }
    }
}

pub(super) fn var_set_ease_keyed<T>(
    transition: TransitionKeyed<T>,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
    init_step: EasingStep,
) -> impl FnMut(&Animation, &mut Cow<T>)
where
    T: VarValue + Transitionable,
{
    let mut prev_step = init_step;
    move |args, value| {
        let step = easing(args.elapsed_stop(duration));

        if prev_step != step {
            *value = Cow::Owned(transition.sample(step));
            prev_step = step;
        }
    }
}

pub(super) fn var_set_ease_keyed_ne<T>(
    transition: TransitionKeyed<T>,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
    init_step: EasingStep,
) -> impl FnMut(&Animation, &mut Cow<T>)
where
    T: VarValue + Transitionable + PartialEq,
{
    let mut prev_step = init_step;
    move |args, value| {
        let step = easing(args.elapsed_stop(duration));

        if prev_step != step {
            let val = transition.sample(step);
            if value.as_ref() != &val {
                *value = Cow::Owned(val);
            }
            prev_step = step;
        }
    }
}

pub(super) fn var_step<T>(new_value: T, delay: Duration) -> impl FnMut(&Animation, &mut Cow<T>)
where
    T: VarValue,
{
    let mut new_value = Some(new_value);
    move |args, value| {
        if !args.animations_enabled() || args.elapsed_dur() >= delay {
            args.stop();
            if let Some(nv) = new_value.take() {
                *value = Cow::Owned(nv);
            }
        } else {
            args.sleep(delay);
        }
    }
}

pub(super) fn var_step_ne<T>(new_value: T, delay: Duration) -> impl FnMut(&Animation, &mut Cow<T>)
where
    T: VarValue + PartialEq,
{
    let mut new_value = Some(new_value);
    move |args, value| {
        if !args.animations_enabled() || args.elapsed_dur() >= delay {
            args.stop();
            if let Some(nv) = new_value.take() {
                if value.as_ref() != &nv {
                    *value = Cow::Owned(nv);
                }
            }
        } else {
            args.sleep(delay);
        }
    }
}

pub(super) fn var_steps<T: VarValue>(
    steps: Vec<(Factor, T)>,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
) -> impl FnMut(&Animation, &mut Cow<T>) {
    let mut prev_step = 999.fct();
    move |args, value| {
        let step = easing(args.elapsed_stop(duration));
        if step != prev_step {
            prev_step = step;
            if let Some(val) = steps.iter().find(|(f, _)| *f >= step).map(|(_, step)| step.clone()) {
                *value = Cow::Owned(val);
            }
        }
    }
}

pub(super) fn var_steps_ne<T>(
    steps: Vec<(Factor, T)>,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
) -> impl FnMut(&Animation, &mut Cow<T>)
where
    T: VarValue + PartialEq,
{
    let mut prev_step = 999.fct();
    move |args, value| {
        let step = easing(args.elapsed_stop(duration));
        if step != prev_step {
            prev_step = step;
            if let Some(val) = steps.iter().find(|(f, _)| *f >= step).map(|(_, step)| step.clone()) {
                if value.as_ref() != &val {
                    *value = Cow::Owned(val);
                }
            }
        }
    }
}

/// Represents a running chase animation created by [`Var::chase`] or other *chase* animation methods.
#[derive(Clone, Debug)]
pub struct ChaseAnimation<T> {
    /// Underlying animation handle.
    pub handle: AnimationHandle,
    pub(super) next_target: Arc<Mutex<ChaseMsg<T>>>,
}
impl<T: VarValue> ChaseAnimation<T> {
    /// Sets a new target value for the easing animation and restarts the time.
    ///
    /// The animation will update to lerp between the current variable value to the `new_target`.
    pub fn reset(&self, new_target: T) {
        *self.next_target.lock() = ChaseMsg::Replace(new_target);
    }

    /// Adds `increment` to the current target value for the easing animation and restarts the time.
    pub fn add(&self, increment: T) {
        *self.next_target.lock() = ChaseMsg::Add(increment);
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

pub(super) fn var_chase<T>(
    from: T,
    first_target: T,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
) -> (impl FnMut(&Animation, &mut Cow<T>) + 'static, Arc<Mutex<ChaseMsg<T>>>)
where
    T: VarValue + animation::Transitionable,
{
    let mut prev_step = 0.fct();
    let next_target = Arc::new(Mutex::new(ChaseMsg::None));
    let mut transition = Transition::new(from, first_target);

    let anim = clone_move!(next_target, |args: &Animation, value: &mut Cow<T>| {
        let step = easing(args.elapsed_stop(duration));
        match mem::take(&mut *next_target.lock()) {
            ChaseMsg::Add(inc) => {
                args.restart();
                let from = transition.sample(step);
                transition.start = from.clone();
                transition.increment += inc;
                if step != prev_step {
                    prev_step = step;
                    *value = Cow::Owned(from);
                }
            }
            ChaseMsg::Replace(new_target) => {
                args.restart();
                let from = transition.sample(step);
                transition = Transition::new(from.clone(), new_target);
                if step != prev_step {
                    prev_step = step;
                    *value = Cow::Owned(from);
                }
            }
            ChaseMsg::None => {
                if step != prev_step {
                    prev_step = step;
                    *value = Cow::Owned(transition.sample(step));
                }
            }
        }
    });

    (anim, next_target)
}

pub(super) fn var_chase_bounded<T>(
    from: T,
    first_target: T,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
    bounds: ops::RangeInclusive<T>,
) -> (impl FnMut(&Animation, &mut Cow<T>) + 'static, Arc<Mutex<ChaseMsg<T>>>)
where
    T: VarValue + animation::Transitionable + std::cmp::PartialOrd<T>,
{
    let mut prev_step = 0.fct();
    let mut check_linear = !bounds.contains(&first_target);
    let mut transition = Transition::new(from, first_target);

    let next_target = Arc::new(Mutex::new(ChaseMsg::None));

    let anim = clone_move!(next_target, |args: &Animation, value: &mut Cow<T>| {
        let mut time = args.elapsed_stop(duration);
        let mut step = easing(time);
        match mem::take(&mut *next_target.lock()) {
            // to > bounds
            // stop animation when linear sampling > bounds
            ChaseMsg::Add(inc) => {
                args.restart();

                let partial_inc = transition.increment.clone() * step;
                let from = transition.start.clone() + partial_inc.clone();
                let to = from.clone() + transition.increment.clone() - partial_inc + inc;

                check_linear = !bounds.contains(&to);

                transition = Transition::new(from, to);

                step = 0.fct();
                prev_step = 1.fct();
                time = EasingTime::start();
            }
            ChaseMsg::Replace(new_target) => {
                args.restart();
                let from = transition.sample(step);

                check_linear = !bounds.contains(&new_target);

                transition = Transition::new(from, new_target);

                step = 0.fct();
                prev_step = 1.fct();
                time = EasingTime::start();
            }
            ChaseMsg::None => {
                // normal execution
            }
        }

        if step != prev_step {
            prev_step = step;

            if check_linear {
                let linear_sample = transition.sample(time.fct());
                if &linear_sample > bounds.end() {
                    args.stop();
                    *value = Cow::Owned(bounds.end().clone());
                    return;
                } else if &linear_sample < bounds.start() {
                    args.stop();
                    *value = Cow::Owned(bounds.start().clone());
                    return;
                }
            }
            *value = Cow::Owned(transition.sample(step));
        }
    });

    (anim, next_target)
}

/// Represents the current *modify* operation when it is applying.
#[derive(Clone)]
pub struct ModifyInfo {
    handle: Option<WeakAnimationHandle>,
    importance: usize,
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
    /// that *paused* or sleeping animations still count as animating.
    ///
    /// [`importance`]: Self::importance
    pub fn is_animating(&self) -> bool {
        self.handle.as_ref().map(|h| h.upgrade().is_some()).unwrap_or(false)
    }
}
