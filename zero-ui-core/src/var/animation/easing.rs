//! Common easing functions.
//!
//! See also: [`EasingFn`] and [`EasingModifierFn`].

use super::*;

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
/// is about to to go negative sharply transitions to a new arc of larger magnitude.
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
