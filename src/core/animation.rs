use crate::core::context::Vars;
use crate::core::var::VarValue;
use std::fmt::Debug;
use std::ops::Mul;
use std::time::Duration;

/// A [variable](crate::core::var::Var) animation.
pub trait VarAnimation<T: VarValue> {
    /// Current animating value.
    fn get_step(&self, vars: &Vars) -> &T;

    /// Update animating value.
    fn update_step(&self, vars: &Vars) -> Option<&T>;

    /// Borrow end value.
    fn get_end(&self) -> &T;

    /// Stops the animation and returns  the end value.
    fn end(self) -> T;
}

/// Easing function value. Mostly between 0.0 and 1.0 but can over/undershot.
#[derive(Debug, Clone, Copy)]
pub struct EasingStep(pub f32);

impl EasingStep {
    /// Gets the scale value.
    #[inline]
    pub fn get(self) -> f32 {
        self.0
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
}

/// A [VarValue] that is the final value of a transition animation and
/// can be multiplied by a [EasingStep] to get an intermediary value of the
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
                let r = f32::from(self) * rhs.0;

                let min = <$ty>::min_value();
                let min_f: f32 = min.into();
                if r < min_f {
                    return min;
                }

                let max = <$ty>::max_value();
                let max_f: f32 = max.into();
                if r > max_f {
                    return max;
                }

                r.round() as Self
            }
        }
    )+};
}

mul_easing_step_for_small_int!(u8, i8, u16, i16);

macro_rules! mul_easing_step_for_int  {
    ($($ty:ty),+) => {$(
        impl Mul<EasingStep> for $ty {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: EasingStep) -> Self {
                let r = f64::from(self) * rhs.0 as f64;

                let min = <$ty>::min_value();
                let min_f: f64 = min.into();
                if r < min_f {
                    return min;
                }

                let max = <$ty>::max_value();
                let max_f: f64 = max.into();
                if r > max_f {
                    return max;
                }

                r.round() as Self
            }
        }
    )+};
}

mul_easing_step_for_int!(u32, i32);

impl Mul<EasingStep> for u64 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: EasingStep) -> Self {
        todo!()
    }
}

// math source: https://referencesource.microsoft.com/#PresentationCore/Core/CSharp/System/Windows/Media/Animation/EasingFunctionBase.cs,7ee062c60623f179,references
pub trait EasingFunction: Debug + Clone {
    fn ease_in(&self, time: EasingTime) -> EasingStep;

    /// Inverse of [ease_in].
    #[inline]
    fn ease_out(&self, time: EasingTime) -> EasingStep {
        EasingStep(1.0 - self.ease_in(EasingTime(1.0 - time.get())))
    }

    /// Combination of [ease_in] and [ease_out].
    #[inline]
    fn ease_in_out(&self, time: EasingTime) -> EasingStep {
        if time.get() < 0.5 {
            EasingStep(self.ease_in(EasingTime(time.get() * 2.0)) * 0.5)
        } else {
            EasingStep(1.0 - self.ease_in((1.0 - time.get()) * 2.0)) * 0.5 + 0.5
        }
    }
}

/// Common easing functions.
pub mod easing {
    use super::{EasingStep, EasingTime};
    use std::f32::consts::*;
    // math source: http://gizma.com/easing/

    /// Simple linear transition, no easing, no acceleration.
    #[inline]
    pub fn linear(time: EasingTime) -> EasingStep {
        EasingStep(time.get())
    }

    /// Quadratic accelerating from zero velocity.
    #[inline]
    pub fn quad_in(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t)
    }
    /// Quadratic decelerating to zero velocity.
    #[inline]
    pub fn quad_out(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * (2.0 - t))
    }
    /// Quadratic in/out.
    #[inline]
    pub fn quad(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(if t < 0.5 {
            2.0 * t * t
        } else {
            -1.0 + (4.0 - 2.0 * t) * t
        })
    }

    /// Cubic accelerating from zero velocity.
    #[inline]
    pub fn cubic_in(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t)
    }
    /// Cubic decelerating to zero velocity.
    #[inline]
    pub fn cubic_out(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((t - 1.0) * t * t + 1.0)
    }
    /// Cubic in/out.
    #[inline]
    pub fn cubic(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(if t < 0.5 {
            4.0 * t * t * t
        } else {
            (t - 1.0) * (2.0 * t - 2.0) * (2.0 * t - 2.0) + 1.0
        })
    }

    /// Quartic accelerating from zero velocity.
    #[inline]
    pub fn quart_in(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t * t)
    }
    /// Quartic decelerating to zero velocity.
    #[inline]
    pub fn quart_out(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(1.0 - (t - 1.0) * t * t * t)
    }
    /// Quartic in/out.
    #[inline]
    pub fn quart(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            1.0 - 8.0 * (t - 1.0) * t * t * t
        })
    }

    /// Quintic accelerating from zero velocity.
    #[inline]
    pub fn quint_in(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t * t * t)
    }
    /// Quintic decelerating to zero velocity.
    #[inline]
    pub fn quint_out(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(1.0 + (t * 1.0) * t * t * t * t)
    }
    /// Quintic in/out.
    #[inline]
    pub fn quint(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(if t < 0.5 {
            16.0 * t * t * t * t * t
        } else {
            1.0 + 16.0 * (t - 1.0) * t * t * t * t
        })
    }

    #[inline]
    pub fn sine_in(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((t * FRAC_PI_2).cos() + 1.0)
    }
    #[inline]
    pub fn sine_out(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((t * FRAC_PI_2).sin())
    }
    /// Sinusoidal in/out.
    #[inline]
    pub fn sine(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(-0.5 * ((PI * t / 1.0).cos() - 1.0))
    }

    #[inline]
    pub fn expo_in(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((10.0 * (t - 1.0)).powf(2.0))
    }
    #[inline]
    pub fn expo_out(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((-10.0 * t).powf(2.0) - 1.0)
    }
    /// Exponential in/out.
    #[inline]
    pub fn expo(time: EasingTime) -> EasingStep {
        // t: current time
        // b: start value
        // c: change in value
        // d: duration
        // t/d: `time`
        // b: 0.0
        // c: 1.0
        /*

        t /= d/2;
        if (t < 1) return c/2 * Math.pow( 2, 10 * (t - 1) ) + b;
        t--;
        return c/2 * ( -Math.pow( 2, -10 * t) + 2 ) + b;

        */
        let t = time.get();
        EasingStep()
    }

    #[inline]
    pub fn circ_in(time: EasingTime) -> EasingStep {
        // return -c * (Math.sqrt(1 - t*t) - 1) + b;
        todo!()
    }
    #[inline]
    pub fn circ_out(time: EasingTime) -> EasingStep {
        //	t--;
        //	return c * Math.sqrt(1 - t*t) + b;
        todo!()
    }
    /// Circular in/out.
    #[inline]
    pub fn circ(time: EasingTime) -> EasingStep {
        // t /= d/2;
        //if (t < 1) return -c/2 * (Math.sqrt(1 - t*t) - 1) + b;
        //t -= 2;
        //return c/2 * (Math.sqrt(1 - t*t) + 1) + b;
        todo!()
    }

    #[inline]
    pub fn back_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn back_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Back in/out.
    #[inline]
    pub fn back(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn elastic_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn elastic_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Elastic in/out.
    #[inline]
    pub fn elastic(time: EasingTime) -> EasingStep {
        todo!()
    }

    #[inline]
    pub fn bounce_in(time: EasingTime) -> EasingStep {
        todo!()
    }
    #[inline]
    pub fn bounce_out(time: EasingTime) -> EasingStep {
        todo!()
    }
    /// Bounce in/out.
    #[inline]
    pub fn bounce(time: EasingTime) -> EasingStep {
        todo!()
    }
}
