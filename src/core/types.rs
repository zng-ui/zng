//! Assorted small types.

pub use webrender::api::{BorderRadius, FontInstanceKey, GlyphInstance, GlyphOptions, GradientStop, LineOrientation};

use super::color::Rgba;

pub use glutin::event::{
    DeviceEvent, DeviceId, ElementState, KeyboardInput, ModifiersState, MouseButton, ScanCode, VirtualKeyCode, WindowEvent,
};

/// Id of a rendered or rendering window frame. Not unique across windows.
pub type FrameId = webrender::api::Epoch;

use crate::core::var::{IntoVar, OwnedVar};

impl IntoVar<Vec<GradientStop>> for Vec<(f32, Rgba)> {
    type Var = OwnedVar<Vec<GradientStop>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(
            self.into_iter()
                .map(|(offset, color)| GradientStop {
                    offset,
                    color: color.into(),
                })
                .collect(),
        )
    }
}

impl IntoVar<Vec<GradientStop>> for Vec<Rgba> {
    type Var = OwnedVar<Vec<GradientStop>>;

    fn into_var(self) -> Self::Var {
        let point = 1. / (self.len() as f32 - 1.);
        OwnedVar(
            self.into_iter()
                .enumerate()
                .map(|(i, color)| GradientStop {
                    offset: (i as f32) * point,
                    color: color.into(),
                })
                .collect(),
        )
    }
}

/// A type that can be a [`property`]((../zero_ui/attr.property.html)) argument for properties that can be used in when expressions.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait ArgWhenCompatible: Clone {}

impl<T: Clone> ArgWhenCompatible for T {}

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
