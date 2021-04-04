//! Animation API.

use crate::var::VarValue;
use std::fmt::Debug;
use std::{
    ops::{Add, Mul, Sub},
    time::Duration,
};

/// Easing function value. Mostly between 0.0 and 1.0 but can over/undershot.
#[derive(Debug, Clone, Copy)]
pub struct EasingStep(pub f32);

impl EasingStep {
    /// Gets the scale value.
    #[inline]
    pub fn get(self) -> f32 {
        self.0
    }

    /// Flipped step.
    #[inline]
    pub fn flip(self) -> EasingStep {
        EasingStep(1.0 - self.0)
    }
}

/// Easing function time input. Is a value in the `0.0..=1.0` range.
#[derive(Debug, Clone, Copy)]
pub struct EasingTime(f32);

impl EasingTime {
    /// New easing time from calculated value.
    pub fn from_raw(raw: f32) -> EasingTime {
        EasingTime(raw.max(0.0).min(1.0))
    }

    /// New easing time from total `duration` and `elapsed` time.
    #[inline]
    pub fn new(duration: Duration, elapsed: Duration) -> EasingTime {
        if elapsed < duration {
            EasingTime(duration.as_secs_f32() / elapsed.as_secs_f32())
        } else {
            EasingTime(1.0)
        }
    }

    /// Gets the scale value.
    #[inline]
    pub fn get(self) -> f32 {
        self.0
    }

    /// Inverted time.
    #[inline]
    pub fn reverse(self) -> EasingTime {
        EasingTime(1.0 - self.0)
    }
}

/// A [`VarValue`] that is the final value of a transition animation and
/// can be multiplied by a [`EasingStep`] to get an intermediary value of the
/// same type.
pub trait EasingVarValue: VarValue + Mul<EasingStep, Output = Self> {}

impl<T: VarValue + Mul<EasingStep, Output = Self>> EasingVarValue for T {}

impl Mul<EasingStep> for f32 {
    type Output = Self;

    fn mul(self, rhs: EasingStep) -> f32 {
        self * rhs.0
    }
}

impl Mul<EasingStep> for f64 {
    type Output = Self;

    fn mul(self, rhs: EasingStep) -> f64 {
        self * rhs.0 as f64
    }
}

macro_rules! mul_easing_step_for_small_int {
    ($($ty:ty),+) => {$(
        impl Mul<EasingStep> for $ty {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: EasingStep) -> Self {
                (self as f32 * rhs.0) as Self
            }
        }
    )+};
}

mul_easing_step_for_small_int!(u8, i8, u16, i16, u32, i32);

macro_rules! mul_easing_step_for_int  {
    ($($ty:ty),+) => {$(
        impl Mul<EasingStep> for $ty {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: EasingStep) -> Self {
                (self as f64 * rhs.0 as f64) as Self
            }
        }
    )+};
}

mul_easing_step_for_int!(usize, isize, u64, i64, u128, i128);

/// Common easing functions.
///
/// See also: [`EasingFn`].
pub mod easing {
    use super::Bezier;
    use super::{EasingStep, EasingTime};
    use std::f32::consts::*;

    /// Simple linear transition, no easing, no acceleration.
    #[inline]
    pub fn linear(time: EasingTime) -> EasingStep {
        EasingStep(time.get())
    }

    /// Quadratic transition (t²).
    #[inline]
    pub fn quad(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t)
    }

    /// Cubic transition (t³).
    #[inline]
    pub fn cubic(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t)
    }

    /// Fourth power transition (t⁴).
    #[inline]
    pub fn quart(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t * t)
    }

    /// Fifth power transition (t⁵).
    #[inline]
    pub fn quint(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t * t * t)
    }

    /// Sine transition. Slow start, fast end.
    #[inline]
    pub fn sine(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(1.0 - (t * FRAC_PI_2 * (1.0 - t)).sin())
    }

    /// Exponential transition. Very slow start, very fast end.
    #[inline]
    pub fn expo(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((10.0 * (t - 1.0)).powf(2.0))
    }

    /// Cubic transition with slightly slowed start then [`cubic`].
    #[inline]
    pub fn circ(time: EasingTime) -> EasingStep {
        EasingStep(1.0 - (1.0 - time.get()).sqrt())
    }

    /// Cubic transition that goes slightly negative to start and ends very fast.
    ///
    /// Like it backs-up and the shoots out.
    #[inline]
    pub fn back(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * (2.70158 * t - 1.70158))
    }

    /// Oscillating transition that grows in magnitude, goes negative twice.
    #[inline]
    pub fn elastic(time: EasingTime) -> EasingStep {
        let t = time.get();
        let t2 = t * t;
        EasingStep(t2 * t2 * (t * PI * 4.5).sin())
    }

    /// Oscillating transition that grows in magnitude, does not goes negative, when the curve
    /// is about to to go negative sharply transitions to a new arc of larger magnitude.
    #[inline]
    pub fn bounce(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((6.0 * (t - 1.0)).powf(2.0) * (t * PI * 3.5).sin().abs())
    }

    /// X coordinate is time, Y coordinate is function advancement.
    /// The nominal range for both is 0 to 1.
    ///
    /// The start and end points are always (0, 0) and (1, 1) so that a transition or animation
    /// starts at 0% and ends at 100%.
    #[inline]
    pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, time: EasingTime) -> EasingStep {
        let t = time.get() as f64;
        EasingStep(Bezier::new(x1, y1, x2, y2).solve(t, 0.00001) as f32)
    }

    /// Always `1.0`, that is, the completed transition.
    #[inline]
    pub fn none(_: EasingTime) -> EasingStep {
        EasingStep(1.0)
    }
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
    let time = EasingTime(time.get() * 2.0);
    let step = if time.get() < 1.0 {
        ease_in(ease_fn, time)
    } else {
        ease_out(ease_fn, time)
    };
    EasingStep(step.get() * 0.5)
}

/// Returns `ease_fn`.
#[inline]
pub fn ease_in_fn<E: Fn(EasingTime) -> EasingStep>(ease_fn: E) -> E {
    ease_fn
}

/// Returns a function that applies `ease_fn` wrapped in [`ease_out`].
#[inline]
pub fn ease_out_fn<'s>(ease_fn: impl Fn(EasingTime) -> EasingStep + 's) -> impl Fn(EasingTime) -> EasingStep + 's {
    move |t| ease_out(|t| ease_fn(t), t)
}

/// Returns a function that applies `ease_fn` wrapped in [`ease_in_out`].
#[inline]
pub fn ease_in_out_fn<'s>(ease_fn: impl Fn(EasingTime) -> EasingStep + 's) -> impl Fn(EasingTime) -> EasingStep + 's {
    move |t| ease_in_out(|t| ease_fn(t), t)
}

/// Common [easing functions](easing) as an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingFn {
    /// [`linear`](easing::linear).
    Linear,
    /// [`sine`](easing::sine).
    Sine,
    /// [`quad`](easing::quad).
    Quad,
    /// [`cubic`](easing::cubic).
    Cubic,
    /// [`quart`](easing::quart).
    Quart,
    /// [`quint`](easing::quint).
    Quint,
    /// [`expo`](easing::expo).
    Expo,
    /// [`circ`](easing::circ).
    Circ,
    /// [`back`](easing::back).
    Back,
    /// [`elastic`](easing::elastic).
    Elastic,
    /// [`bounce`](easing::bounce).
    Bounce,
}
impl EasingFn {
    /// Calls the easing function that `self` matches too.
    #[inline]
    pub fn ease_in(self, time: EasingTime) -> EasingStep {
        (self.ease_fn())(time)
    }

    /// Calls the easing function that `self` matches too and inverts the value using
    /// [`ease_out`](ease_out).
    #[inline]
    pub fn ease_out(self, time: EasingTime) -> EasingStep {
        ease_out(|t| self.ease_in(t), time)
    }

    /// Calls the easing function that `self` matches too and transforms the value using
    /// [`ease_in_out`](ease_in_out).
    #[inline]
    pub fn ease_in_out(self, time: EasingTime) -> EasingStep {
        ease_in_out(|t| self.ease_in(t), time)
    }

    /// Gets the [`easing`] function that `self` matches too.
    #[inline]
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
        }
    }
}

/// A value that can be animated using [`Transition`].
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait TransitionValue: Mul<EasingStep, Output = Self> + Sub<Output = Self> + Add<Output = Self> + Clone + Sized {}

impl<T: Mul<EasingStep, Output = Self> + Sub<Output = Self> + Add<Output = Self> + Clone + Sized> TransitionValue for T {}

/// An animated transition from one value to another.
pub struct Transition<T: TransitionValue, E: Fn(EasingTime) -> EasingStep> {
    from: T,
    plus: T,
    easing: E,
}

impl<T: TransitionValue, E: Fn(EasingTime) -> EasingStep> Transition<T, E> {
    /// Transition `from` value, `to` value, with a custom `easing` function.
    pub fn new(from: T, to: T, easing: E) -> Self {
        Self {
            plus: to - from.clone(),
            from,
            easing,
        }
    }

    /// Calculates the transition at the `time` offset.
    pub fn step(&self, time: EasingTime) -> T {
        self.from.clone() + self.plus.clone() * (self.easing)(time)
    }
}

pub use bezier::*;

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
