//! Common easing functions.

use std::{
    f32::consts::{FRAC_PI_2, TAU},
    fmt, ops,
};

use crate::impl_from_and_into_var;

use super::*;

/// Easing function output.
///
/// Usually in the [0..=1] range, but can overshoot. An easing function converts a [`EasingTime`]
/// into this factor.
///
/// # Examples
///
/// ```
/// use zng_unit::*;
/// use zng_var::animation::easing::{EasingStep, EasingTime};
///
/// /// Cubic animation curve.
/// fn cubic(time: EasingTime) -> EasingStep {
///     let f = time.fct();
///     f * f * f
/// }
/// ```
///
/// Note that all the common easing functions are implemented in [`easing`].
pub type EasingStep = Factor;

/// Easing function input.
///
/// An easing function converts this time into a [`EasingStep`] factor.
///
/// The time is always in the [0..=1] range, factors are clamped to this range on creation.
#[derive(Debug, PartialEq, Copy, Clone, Hash, PartialOrd)]
pub struct EasingTime(Factor);
impl_from_and_into_var! {
    fn from(factor: Factor) -> EasingTime {
        EasingTime::new(factor)
    }
}
impl EasingTime {
    /// New from [`Factor`].
    ///
    /// The `factor` is clamped to the [0..=1] range.
    ///
    /// [`Factor`]: zng_unit::Factor
    pub fn new(factor: Factor) -> Self {
        EasingTime(factor.clamp_range())
    }

    /// New easing time from total `duration`, `elapsed` time and `time_scale`.
    ///
    /// If `elapsed >= duration` the time is 1.
    pub fn elapsed(duration: Duration, elapsed: Duration, time_scale: Factor) -> Self {
        EasingTime::new(elapsed.as_secs_f32().fct() / duration.as_secs_f32().fct() * time_scale)
    }

    /// Gets the start time, zero.
    pub fn start() -> Self {
        EasingTime(0.fct())
    }

    /// Gets the end time, one.
    pub fn end() -> Self {
        EasingTime(1.fct())
    }

    /// If the time represents the start of the animation.
    pub fn is_start(self) -> bool {
        self == Self::start()
    }

    /// If the time represents the end of the animation.
    pub fn is_end(self) -> bool {
        self == Self::end()
    }

    /// Get the time as a [`Factor`].
    ///
    /// [`Factor`]: zng_unit::Factor
    pub fn fct(self) -> Factor {
        self.0
    }

    /// Get the time as a [`FactorPercent`].
    ///
    /// [`FactorPercent`]: zng_unit::FactorPercent
    pub fn pct(self) -> FactorPercent {
        self.0.0.pct()
    }

    /// Flip the time.
    ///
    /// Returns `1 - self`.
    pub fn reverse(self) -> Self {
        EasingTime(self.0.flip())
    }
}
impl ops::Add for EasingTime {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl ops::AddAssign for EasingTime {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}
impl ops::Sub for EasingTime {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl ops::SubAssign for EasingTime {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

/// Easing functions as a value.
#[derive(Clone)]
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
    ///Custom function.
    Custom(Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>),
}
impl PartialEq for EasingFn {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(l0), Self::Custom(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl Eq for EasingFn {}
impl fmt::Debug for EasingFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Linear => write!(f, "linear"),
            Self::Sine => write!(f, "sine"),
            Self::Quad => write!(f, "quad"),
            Self::Cubic => write!(f, "cubic"),
            Self::Quart => write!(f, "quart"),
            Self::Quint => write!(f, "quint"),
            Self::Expo => write!(f, "expo"),
            Self::Circ => write!(f, "circ"),
            Self::Back => write!(f, "back"),
            Self::Elastic => write!(f, "elastic"),
            Self::Bounce => write!(f, "bounce"),
            Self::None => write!(f, "none"),
            Self::Custom(_) => f.debug_tuple("Custom").finish(),
        }
    }
}
impl EasingFn {
    /// Create a closure that calls the easing function.
    pub fn ease_fn(&self) -> impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static {
        let me = self.clone();
        move |t| me(t)
    }

    /// New custom function.
    pub fn custom(f: impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static) -> Self {
        Self::Custom(Arc::new(f))
    }

    /// Creates a custom function that is `self` modified by `modifier`
    pub fn modified(self, modifier: impl Fn(&dyn Fn(EasingTime) -> EasingStep, EasingTime) -> EasingStep + Send + Sync + 'static) -> Self {
        Self::custom(move |t| modifier(&*self, t))
    }

    /// Creates a custom function that is `self` modified by [`easing::ease_out`].
    pub fn ease_out(self) -> Self {
        self.modified(|f, t| easing::ease_out(f, t))
    }

    /// Creates a custom function that is `self` modified by [`easing::ease_in_out`].
    pub fn ease_in_out(self) -> Self {
        self.modified(|f, t| easing::ease_in_out(f, t))
    }

    /// Creates a custom function that is `self` modified by [`easing::ease_out_in`].
    pub fn ease_out_in(self) -> Self {
        self.modified(|f, t| easing::ease_out_in(f, t))
    }

    /// Creates a custom function that is `self` modified by [`easing::reverse`].
    pub fn reverse(self) -> Self {
        self.modified(|f, t| easing::reverse(f, t))
    }

    /// Creates a custom function that is `self` modified by [`easing::reverse_out`].
    pub fn reverse_out(self) -> Self {
        self.modified(|f, t| easing::reverse_out(f, t))
    }
}
impl ops::Deref for EasingFn {
    type Target = dyn Fn(EasingTime) -> EasingStep + Send + Sync;

    fn deref(&self) -> &Self::Target {
        match self {
            EasingFn::Linear => &easing::linear,
            EasingFn::Sine => &easing::sine,
            EasingFn::Quad => &easing::quad,
            EasingFn::Cubic => &easing::cubic,
            EasingFn::Quart => &easing::quad,
            EasingFn::Quint => &easing::quint,
            EasingFn::Expo => &easing::expo,
            EasingFn::Circ => &easing::circ,
            EasingFn::Back => &easing::back,
            EasingFn::Elastic => &easing::elastic,
            EasingFn::Bounce => &easing::bounce,
            EasingFn::None => &easing::none,
            EasingFn::Custom(c) => &**c,
        }
    }
}

/// Simple linear transition, no easing, no acceleration.
pub fn linear(time: EasingTime) -> EasingStep {
    time.fct()
}

/// Quadratic transition (t²).
pub fn quad(time: EasingTime) -> EasingStep {
    let f = time.fct();
    f * f
}

/// Cubic transition (t³).
pub fn cubic(time: EasingTime) -> EasingStep {
    let f = time.fct();
    f * f * f
}

/// Fourth power transition (t⁴).
pub fn quart(time: EasingTime) -> EasingStep {
    let f = time.fct();
    f * f * f * f
}

/// Fifth power transition (t⁵).
pub fn quint(time: EasingTime) -> EasingStep {
    let f = time.fct();
    f * f * f * f * f
}

/// Sine transition. Slow start, fast end.
pub fn sine(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    (1.0 - (f * FRAC_PI_2).cos()).fct()
}

/// Exponential transition. Very slow start, very fast end.
pub fn expo(time: EasingTime) -> EasingStep {
    let f = time.fct();
    if f == 0.fct() {
        0.fct()
    } else {
        2.0_f32.powf(10.0 * f.0 - 10.0).fct()
    }
}

/// Cubic transition with slightly slowed start then [`cubic`].
pub fn circ(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    (1.0 - (1.0 - f.powf(2.0)).sqrt()).fct()
}

/// Cubic transition that goes slightly negative to start and ends very fast.
///
/// Like it backs-up and the shoots out.
pub fn back(time: EasingTime) -> EasingStep {
    let f = time.fct().0;
    (f * f * (2.70158 * f - 1.70158)).fct()
}

/// Oscillating transition that grows in magnitude, goes negative twice.
pub fn elastic(time: EasingTime) -> EasingStep {
    let t = time.fct();

    const C: f32 = TAU / 3.0;

    if t == 0.fct() || t == 1.fct() {
        t
    } else {
        let t = t.0;
        let s = -(2.0_f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * C).sin();
        s.fct()
    }
}

/// Oscillating transition that grows in magnitude, does not go negative, when the curve
/// is about to go negative it sharply transitions to a new arc of larger magnitude.
pub fn bounce(time: EasingTime) -> EasingStep {
    const N: f32 = 7.5625;
    const D: f32 = 2.75;

    let mut t = 1.0 - time.fct().0;

    let f = if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        t -= 1.5 / D;
        N * t * t + 0.75
    } else if t < 2.5 / D {
        t -= 2.25 / D;
        N * t * t + 0.9375
    } else {
        t -= 2.625 / D;
        N * t * t + 0.984375
    };

    (1.0 - f).fct()
}

/// X coordinate is time, Y coordinate is function advancement.
/// The nominal range for both is 0 to 1.
///
/// The start and end points are always (0, 0) and (1, 1) so that a transition or animation
/// starts at 0% and ends at 100%.
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, time: EasingTime) -> EasingStep {
    let f = time.fct().0 as f64;
    (Bezier::new(x1, y1, x2, y2).solve(f, 0.00001) as f32).fct()
}

/// Jumps to the final value by a number of `steps`.
///
/// Starts from the first step value immediately.
pub fn step_ceil(steps: u32, time: EasingTime) -> EasingStep {
    let steps = steps as f32;
    let step = (steps * time.fct().0).ceil();
    (step / steps).fct()
}

/// Jumps to the final value by a number of `steps`.
///
/// Waits until first step to output the first step value.
pub fn step_floor(steps: u32, time: EasingTime) -> EasingStep {
    let steps = steps as f32;
    let step = (steps * time.fct().0).floor();
    (step / steps).fct()
}

/// Always `1.fct()`, that is, the completed transition.
pub fn none(_: EasingTime) -> EasingStep {
    1.fct()
}

/// Applies the `ease_fn`.
pub fn ease_in(ease_fn: impl Fn(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    ease_fn(time)
}

/// Applies the `ease_fn` in reverse and flipped.
pub fn ease_out(ease_fn: impl Fn(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    ease_fn(time.reverse()).flip()
}

/// Applies [`ease_in`] for the first half then [`ease_out`] scaled to fit a single duration (1.0).
pub fn ease_in_out(ease_fn: impl Fn(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    let t = time.fct();
    if t <= 0.5.fct() {
        ease_in(&ease_fn, EasingTime::new(t * 2.fct())) / 2.fct()
    } else {
        ease_out(ease_fn, EasingTime::new((t - 0.5.fct()) * 2.fct())) / 2.fct() + 0.5.fct()
    }
}

/// Applies [`ease_out`] for the first half then [`ease_in`] scaled to fit a single duration (1.0).
pub fn ease_out_in(ease_fn: impl Fn(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    let t = time.fct();
    if t <= 0.5.fct() {
        ease_out(&ease_fn, EasingTime::new(t * 2.fct())) / 2.fct()
    } else {
        ease_in(ease_fn, EasingTime::new((t - 0.5.fct()) * 2.fct())) / 2.fct() + 0.5.fct()
    }
}

/// Applies the `ease_fn` in reverse.
pub fn reverse(ease_fn: impl Fn(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    ease_fn(time.reverse())
}

/// Applies the `ease_fn` flipped.
pub fn reverse_out(ease_fn: impl Fn(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
    ease_fn(time).flip()
}

pub use bezier::*;
use zng_unit::{FactorPercent, FactorUnits as _};

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

        fn sample_curve_x(&self, t: f64) -> f64 {
            // ax * t^3 + bx * t^2 + cx * t
            ((self.ax * t + self.bx) * t + self.cx) * t
        }

        fn sample_curve_y(&self, t: f64) -> f64 {
            ((self.ay * t + self.by) * t + self.cy) * t
        }

        fn sample_curve_derivative_x(&self, t: f64) -> f64 {
            (3.0 * self.ax * t + 2.0 * self.bx) * t + self.cx
        }

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
        pub fn solve(&self, x: f64, epsilon: f64) -> f64 {
            self.sample_curve_y(self.solve_curve_x(x, epsilon))
        }
    }

    trait ApproxEq {
        fn approx_eq(self, value: Self, epsilon: Self) -> bool;
    }

    impl ApproxEq for f64 {
        fn approx_eq(self, value: f64, epsilon: f64) -> bool {
            (self - value).abs() < epsilon
        }
    }
}
