//! Common easing functions.
//!
//! See also: [`EasingFn`].

use crate::units::*;
use std::f32::consts::*;

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
