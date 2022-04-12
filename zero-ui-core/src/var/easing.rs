//! Common easing functions.
//!
//! See also: [`EasingFn`].

use crate::{
    crate_util::{Handle, HandleOwner},
    units::*,
};
use std::{
    cell::Cell,
    f32::consts::*,
    ops,
    time::{Duration, Instant},
};

/// Simple linear transition, no easing, no acceleration.
#[inline]
pub fn linear(time: EasingTime) -> EasingStep {
    time.fct()
}

/// Quadratic transition (t²).
#[inline]
pub fn quad(time: EasingTime) -> EasingStep {
    let f = time.fct();
    f * f
}

/// Cubic transition (t³).
#[inline]
pub fn cubic(time: EasingTime) -> EasingStep {
    let f = time.fct();
    f * f * f
}

/// Fourth power transition (t⁴).
#[inline]
pub fn quart(time: EasingTime) -> EasingStep {
    let f = time.fct();
    f * f * f * f
}

/// Fifth power transition (t⁵).
#[inline]
pub fn quint(time: EasingTime) -> EasingStep {
    let f = time.fct();
    f * f * f * f * f
}

/// Sine transition. Slow start, fast end.
#[inline]
pub fn sine(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    (1.0 - (f * FRAC_PI_2 * (1.0 - f)).sin()).fct()
}

/// Exponential transition. Very slow start, very fast end.
#[inline]
pub fn expo(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    ((10.0 * (f - 1.0)).powf(2.0)).fct()
}

/// Cubic transition with slightly slowed start then [`cubic`].
#[inline]
pub fn circ(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    (1.0 - (1.0 - f).sqrt()).fct()
}

/// Cubic transition that goes slightly negative to start and ends very fast.
///
/// Like it backs-up and the shoots out.
#[inline]
pub fn back(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    (f * f * (2.70158 * f - 1.70158)).fct()
}

/// Oscillating transition that grows in magnitude, goes negative twice.
#[inline]
pub fn elastic(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    let f2 = f * f;
    (f2 * f2 * (f * PI * 4.5).sin()).fct()
}

/// Oscillating transition that grows in magnitude, does not go negative, when the curve
/// is about to to go negative sharply transitions to a new arc of larger magnitude.
#[inline]
pub fn bounce(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    ((6.0 * (f - 1.0)).powf(2.0) * (f * PI * 3.5).sin().abs()).fct()
}

/// X coordinate is time, Y coordinate is function advancement.
/// The nominal range for both is 0 to 1.
///
/// The start and end points are always (0, 0) and (1, 1) so that a transition or animation
/// starts at 0% and ends at 100%.
#[inline]
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, time: EasingTime) -> EasingStep {
    let f = time.fct().0 as f64;
    (Bezier::new(x1, y1, x2, y2).solve(f, 0.00001) as f32).fct()
}

/// Jumps to the final value by a number of `steps`.
///
/// Starts from the first step value immediately.
#[inline]
pub fn step_ceil(steps: u32, time: EasingTime) -> EasingStep {
    let steps = steps as f32;
    let step = (steps * time.fct().0).ceil();
    (1.0 / step).fct()
}

/// Jumps to the final value by a number of `steps`.
///
/// Waits until first step to output the first step value.
#[inline]
pub fn step_floor(steps: u32, time: EasingTime) -> EasingStep {
    let steps = steps as f32;
    let step = (steps * time.fct().0).floor();
    (1.0 / step).fct()
}

/// Always `1.fct()`, that is, the completed transition.
#[inline]
pub fn none(_: EasingTime) -> EasingStep {
    1.fct()
}

/// Applies the `ease_fn`.
#[inline]
pub fn ease_in(ease_fn: impl FnOnce(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    ease_fn(time)
}

/// Applies the `ease_fn` in reverse and flipped.
#[inline]
pub fn ease_out(ease_fn: impl FnOnce(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    ease_fn(time.reverse()).flip()
}

/// Applies `ease_in` for the first half then [`ease_out`] scaled to fit a single duration (1.0).
pub fn ease_in_out(ease_fn: impl FnOnce(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    let time = EasingTime::new(time.fct() * 2.fct());
    let step = if time.fct() < 1.fct() {
        ease_in(ease_fn, time)
    } else {
        ease_out(ease_fn, time)
    };
    step * 0.5.fct()
}

/// Returns `ease_fn`.
#[inline]
pub fn ease_in_fn<E: Fn(EasingTime) -> EasingStep>(ease_fn: E) -> E {
    ease_fn
}

/// Returns a function that applies `ease_fn` wrapped in [`ease_out`].
#[inline]
pub fn ease_out_fn<'s>(ease_fn: impl Fn(EasingTime) -> EasingStep + 's) -> impl Fn(EasingTime) -> EasingStep + 's {
    move |t| ease_out(&ease_fn, t)
}

/// Returns a function that applies `ease_fn` wrapped in [`ease_in_out`].
#[inline]
pub fn ease_in_out_fn<'s>(ease_fn: impl Fn(EasingTime) -> EasingStep + 's) -> impl Fn(EasingTime) -> EasingStep + 's {
    move |t| ease_in_out(&ease_fn, t)
}

/// Common easing functions as an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingFn {
    /// [`linear`].
    Linear,
    /// [`sine`].
    Sine,
    /// [`quad`].
    Quad,
    /// [`cubic`].
    Cubic,
    /// [`quart`].
    Quart,
    /// [`quint`].
    Quint,
    /// [`expo`].
    Expo,
    /// [`circ`].
    Circ,
    /// [`back`].
    Back,
    /// [`elastic`].
    Elastic,
    /// [`bounce`].
    Bounce,
}
impl EasingFn {
    /// Calls the easing function that `self` represents.
    #[inline]
    pub fn ease_in(self, time: EasingTime) -> EasingStep {
        (self.ease_fn())(time)
    }

    /// Calls the easing function that `self` represents and inverts the value using [`ease_out`].
    #[inline]
    pub fn ease_out(self, time: EasingTime) -> EasingStep {
        ease_out(|t| self.ease_in(t), time)
    }

    /// Calls the easing function that `self` represents and transforms the value using [`ease_in_out`].
    #[inline]
    pub fn ease_in_out(self, time: EasingTime) -> EasingStep {
        ease_in_out(|t| self.ease_in(t), time)
    }

    /// Gets the easing function that `self` represents.
    #[inline]
    pub fn ease_fn(self) -> fn(EasingTime) -> EasingStep {
        match self {
            EasingFn::Linear => self::linear,
            EasingFn::Sine => self::sine,
            EasingFn::Quad => self::quad,
            EasingFn::Cubic => self::cubic,
            EasingFn::Quart => self::quad,
            EasingFn::Quint => self::quint,
            EasingFn::Expo => self::expo,
            EasingFn::Circ => self::circ,
            EasingFn::Back => self::back,
            EasingFn::Elastic => self::elastic,
            EasingFn::Bounce => self::bounce,
        }
    }

    /// Returns the [`ease_in_fn`] of the easing function `self` represents.
    #[inline]
    pub fn ease_in_fn(self) -> impl Fn(EasingTime) -> EasingStep {
        ease_in_fn(self.ease_fn())
    }

    /// Returns the [`ease_out_fn`] of the easing function `self` represents.
    #[inline]
    pub fn ease_out_fn(self) -> impl Fn(EasingTime) -> EasingStep {
        ease_out_fn(self.ease_fn())
    }

    /// Returns the [`ease_in_out_fn`] of the easing function `self` represents.
    #[inline]
    pub fn ease_in_out_fn(self) -> impl Fn(EasingTime) -> EasingStep {
        ease_in_out_fn(self.ease_fn())
    }
}

pub use bezier::*;

use super::{VarValue, Vars};
mod bezier {
    /* This Source Code Form is subject to the terms of the Mozilla Public
     * License, v. 2.0. If a copy of the MPL was not distributed with this
     * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

    const NEWTON_METHOD_ITERATIONS: u8 = 8;

    /// A unit cubic Bézier curve, used for timing functions in CSS transitions and animations.
    pub struct Bezier {
        ax: f64,
        bx: f64,
        cx: f64,
        ay: f64,
        by: f64,
        cy: f64,
    }

    impl Bezier {
        /// Create a unit cubic Bézier curve from the two middle control points.
        ///
        /// X coordinate is time, Y coordinate is function advancement.
        /// The nominal range for both is 0 to 1.
        ///
        /// The start and end points are always (0, 0) and (1, 1) so that a transition or animation
        /// starts at 0% and ends at 100%.
        #[inline]
        pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Bezier {
            let cx = 3. * x1 as f64;
            let bx = 3. * (x2 as f64 - x1 as f64) - cx;

            let cy = 3. * y1 as f64;
            let by = 3. * (y2 as f64 - y1 as f64) - cy;

            Bezier {
                ax: 1.0 - cx - bx,
                bx,
                cx,
                ay: 1.0 - cy - by,
                by,
                cy,
            }
        }

        #[inline]
        fn sample_curve_x(&self, t: f64) -> f64 {
            // ax * t^3 + bx * t^2 + cx * t
            ((self.ax * t + self.bx) * t + self.cx) * t
        }

        #[inline]
        fn sample_curve_y(&self, t: f64) -> f64 {
            ((self.ay * t + self.by) * t + self.cy) * t
        }

        #[inline]
        fn sample_curve_derivative_x(&self, t: f64) -> f64 {
            (3.0 * self.ax * t + 2.0 * self.bx) * t + self.cx
        }

        #[inline]
        fn solve_curve_x(&self, x: f64, epsilon: f64) -> f64 {
            // Fast path: Use Newton's method.
            let mut t = x;
            for _ in 0..NEWTON_METHOD_ITERATIONS {
                let x2 = self.sample_curve_x(t);
                if x2.approx_eq(x, epsilon) {
                    return t;
                }
                let dx = self.sample_curve_derivative_x(t);
                if dx.approx_eq(0.0, 1e-6) {
                    break;
                }
                t -= (x2 - x) / dx;
            }

            // Slow path: Use bisection.
            let (mut lo, mut hi, mut t) = (0.0, 1.0, x);

            if t < lo {
                return lo;
            }
            if t > hi {
                return hi;
            }

            while lo < hi {
                let x2 = self.sample_curve_x(t);
                if x2.approx_eq(x, epsilon) {
                    return t;
                }
                if x > x2 {
                    lo = t
                } else {
                    hi = t
                }
                t = (hi - lo) / 2.0 + lo
            }

            t
        }

        /// Solve the bezier curve for a given `x` and an `epsilon`, that should be
        /// between zero and one.
        #[inline]
        pub fn solve(&self, x: f64, epsilon: f64) -> f64 {
            self.sample_curve_y(self.solve_curve_x(x, epsilon))
        }
    }

    trait ApproxEq {
        fn approx_eq(self, value: Self, epsilon: Self) -> bool;
    }

    impl ApproxEq for f64 {
        #[inline]
        fn approx_eq(self, value: f64, epsilon: f64) -> bool {
            (self - value).abs() < epsilon
        }
    }
}

pub(super) struct AnimationState {}
impl AnimationState {
    fn new() -> Self {
        AnimationState {}
    }

    fn dummy() -> Self {
        AnimationState {}
    }
}

/// Represents a running animation created by [`Vars::animate`].
///
/// Drop all clones of this handle to stop the animation, or call [`permanent`] to drop the handle
/// but keep the animation alive until it is stopped from the inside.
///
/// [`permanent`]: AnimationHandle::permanent
#[derive(Clone)]
#[must_use = "the animation stops if the handle is dropped"]
pub struct AnimationHandle(Handle<AnimationState>);

impl AnimationHandle {
    pub(super) fn new() -> (HandleOwner<AnimationState>, Self) {
        let (owner, handle) = Handle::new(AnimationState::new());
        (owner, AnimationHandle(handle))
    }

    /// Create dummy handle that is always in the *stopped* state.
    #[inline]
    pub fn dummy() -> Self {
        AnimationHandle(Handle::dummy(AnimationState::dummy()))
    }

    /// Drop the handle but does **not** stop.
    ///
    /// The animation stays in memory for the duration of the app or until another handle calls [`stop`](Self::stop).
    #[inline]
    pub fn permanent(self) {
        self.0.permanent();
    }

    /// If another handle has called [`permanent`](Self::permanent).
    /// If `true` the animation will stay active until the app shutdown, unless [`stop`](Self::stop) is called.
    #[inline]
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the animation to drop.
    #[inline]
    pub fn stop(self) {
        self.0.force_drop();
    }

    /// If another handle has called [`stop`](Self::stop).
    ///
    /// The animation is already dropped or will be dropped in the next app update, this is irreversible.
    #[inline]
    pub fn is_stopped(&self) -> bool {
        self.0.is_dropped()
    }
}

/// Represents an animation in its closure.
/// 
/// See the [`Vars::animate`] method for more details.
pub struct Animation {
    start_time: Instant,
    stop: Cell<bool>,
}

impl Animation {
    pub(super) fn new() -> Self {
        Animation {
            start_time: Instant::now(),
            stop: Cell::new(false),
        }
    }

    /// Instant this animation started.
    #[inline]
    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    /// Compute the elapsed [`EasingTime`], in the span of the total `duration`.
    #[inline]
    pub fn elapsed(&self, duration: Duration) -> EasingTime {
        EasingTime::elapsed(duration, self.start_time.elapsed())
    }

    /// Compute the elapsed [`EasingTime`], if the time [`is_end`] requests animation stop.
    ///
    /// [`is_end`]: EasingTime::is_end
    #[inline]
    pub fn elapsed_stop(&self, duration: Duration) -> EasingTime {
        let t = self.elapsed(duration);
        if t.is_end() {
            self.stop()
        }
        t
    }

    /// Drop the animation after applying the returned update.
    #[inline]
    pub fn stop(&self) {
        self.stop.set(true);
    }

    /// If the animation will be dropped after applying the update.
    #[inline]
    pub fn stop_requested(&self) -> bool {
        self.stop.get()
    }
}

/// Represents a transition from one value to another that can be sampled using [`EasingStep`].
#[derive(Clone, Debug)]
pub struct Transition<T> {
    start: T,
    increment: T,
}
impl<T> Transition<T>
where
    T: Clone + ops::Add<T, Output = T> + ops::Sub<T, Output = T> + ops::Mul<Factor, Output = T>,
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
    T: Clone + ops::Add<T, Output = T> + ops::Sub<T, Output = T> + ops::Mul<Factor, Output = T>,
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

pub(super) fn default_var_ease<T>(
    var: impl super::Var<T>,
    vars: &Vars,
    from: T,
    to: T,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
    from_current: bool,
) where
    T: VarValue + ops::Add<T, Output = T> + ops::Sub<T, Output = T> + ops::Mul<Factor, Output = T>,
{
    let transition = Transition::new(from, to);
    let mut prev_step = if from_current { 0.fct() } else { 999.fct() };
    vars.animate(move |vars, anim| {
        let step = easing(anim.elapsed_stop(duration));
        if step != prev_step {
            prev_step = step;

            if var.set(vars, transition.sample(step)).is_err() {
                anim.stop()
            }
        }
    })
    .permanent()
}

pub(super) fn default_var_ease_ne<T>(
    var: impl super::Var<T>,
    vars: &Vars,
    from: T,
    to: T,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
    from_current: bool,
) where
    T: PartialEq + VarValue + ops::Add<T, Output = T> + ops::Sub<T, Output = T> + ops::Mul<Factor, Output = T>,
{
    let transition = Transition::new(from, to);
    let mut prev_step = if from_current { 0.fct() } else { 999.fct() };
    vars.animate(move |vars, anim| {
        let step = easing(anim.elapsed_stop(duration));
        if step != prev_step {
            prev_step = step;

            if var.set_ne(vars, transition.sample(step)).is_err() {
                anim.stop()
            }
        }
    })
    .permanent()
}

pub(super) fn default_var_ease_keyed<T>(
    var: impl super::Var<T>,
    vars: &Vars,
    keys: Vec<(Factor, T)>,
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + 'static,
    from_current: bool,
) where
    T: VarValue + ops::Add<T, Output = T> + ops::Sub<T, Output = T> + ops::Mul<Factor, Output = T>,
{
    if let Some(transition) = TransitionKeyed::new(keys) {
        let mut prev_step = if from_current { 0.fct() } else { 999.fct() };
        vars.animate(move |vars, anim| {
            let step = easing(anim.elapsed_stop(duration));
            if step != prev_step {
                prev_step = step;

                if var.set(vars, transition.sample(step)).is_err() {
                    anim.stop();
                }
            }
        })
        .permanent()
    }
}
