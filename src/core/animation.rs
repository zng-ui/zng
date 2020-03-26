use crate::core::context::Vars;
use crate::core::var::VarValue;
use std::fmt::Debug;
use std::{ops::Mul, time::Duration};

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
/// See also: [`EasingFn`](EasingFn).
pub mod easing {
    use super::{EasingStep, EasingTime};
    use std::f32::consts::*;

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

    /// Applies `ease_in` for the first half then `[ease_out]` scaled to fit a single duration (1.0).
    pub fn ease_in_out(ease_fn: impl FnOnce(EasingTime) -> EasingStep, time: EasingTime) -> EasingStep {
        let time = EasingTime(time.get() * 2.0);
        let step = if time.get() < 1.0 {
            ease_in(ease_fn, time)
        } else {
            ease_out(ease_fn, time)
        };
        EasingStep(step.get() * 0.5)
    }

    /// Simple linear transition, no easing, no acceleration.
    #[inline]
    pub fn linear(time: EasingTime) -> EasingStep {
        EasingStep(time.get())
    }

    #[inline]
    pub fn quad(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t)
    }

    #[inline]
    pub fn cubic(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t)
    }

    #[inline]
    pub fn quart(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t * t)
    }

    #[inline]
    pub fn quint(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * t * t * t)
    }

    #[inline]
    pub fn sine(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(1.0 - (t * FRAC_PI_2 * (1.0 - t)).sin())
    }

    #[inline]
    pub fn expo(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((10.0 * (t - 1.0)).powf(2.0))
    }

    #[inline]
    pub fn circ(time: EasingTime) -> EasingStep {
        EasingStep(1.0 - (1.0 - time.get()).sqrt())
    }

    #[inline]
    pub fn back(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep(t * t * (2.70158 * t - 1.70158))
    }

    #[inline]
    pub fn elastic(time: EasingTime) -> EasingStep {
        let t = time.get();
        let t2 = t * t;
        EasingStep(t2 * t2 * (t * PI * 4.5).sin())
    }

    #[inline]
    pub fn bounce(time: EasingTime) -> EasingStep {
        let t = time.get();
        EasingStep((6.0 * (t - 1.0)).powf(2.0) * (t * PI * 3.5).sin().abs())
    }
}

/// Common easing functions as an enum.
#[derive(Debug, Clone, Copy)]
pub enum EasingFn {
    Linear,
    Sine,
    Quad,
    Cubic,
    Quart,
    Quint,
    Expo,
    Circ,
    Back,
    Elastic,
    Bounce,
}

impl EasingFn {
    #[inline]
    pub fn ease_in(self, time: EasingTime) -> EasingStep {
        match self {
            EasingFn::Linear => easing::linear(time),
            EasingFn::Sine => easing::sine(time),
            EasingFn::Quad => easing::quad(time),
            EasingFn::Cubic => easing::cubic(time),
            EasingFn::Quart => easing::quad(time),
            EasingFn::Quint => easing::quint(time),
            EasingFn::Expo => easing::expo(time),
            EasingFn::Circ => easing::circ(time),
            EasingFn::Back => easing::back(time),
            EasingFn::Elastic => easing::elastic(time),
            EasingFn::Bounce => easing::bounce(time),
        }
    }

    #[inline]
    pub fn ease_out(self, time: EasingTime) -> EasingStep {
        easing::ease_out(|t| self.ease_in(t), time)
    }

    #[inline]
    pub fn ease_in_out(self, time: EasingTime) -> EasingStep {
        easing::ease_in_out(|t| self.ease_in(t), time)
    }
}
