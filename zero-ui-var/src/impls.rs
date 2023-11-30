//! zero-ui-var depends on zero-ui-[units, txt] so we need to implement these traits here.

use std::{
    any::Any,
    borrow::Cow,
    path::PathBuf,
    time::{Duration, Instant},
};

use zero_ui_txt::Txt;
use zero_ui_units::{euclid, CornerRadius2D, Deadline, Dip, Factor, FactorPercent, FactorUnits, Px};

use crate::{animation::Transitionable, easing::EasingStep, impl_from_and_into_var};

impl Transitionable for f64 {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        self + (*to - self) * step.0 as f64
    }
}
impl Transitionable for f32 {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        self + (*to - self) * step.0
    }
}
macro_rules! impl_transitionable {
    ($FT:ident => $($T:ty,)+) => {$(
        impl Transitionable for $T {
            fn lerp(self, to: &Self, step: EasingStep) -> Self {
                $FT::lerp(self as $FT, &((*to) as $FT), step).round() as _
            }
        }
    )+}
}
impl_transitionable! {
    f32 => i8, u8, i16, u16, i32, u32,
}
impl_transitionable! {
    f64 => u64, i64, u128, i128, isize, usize,
}
impl Transitionable for Px {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        Px(self.0.lerp(&to.0, step))
    }
}
impl Transitionable for Dip {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        Dip::new_f32(self.to_f32().lerp(&to.to_f32(), step))
    }
}
impl<T, U> Transitionable for euclid::Point2D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::point2(self.x.lerp(&to.x, step), self.y.lerp(&to.y, step))
    }
}
impl<T, U> Transitionable for euclid::Box2D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::Box2D::new(self.min.lerp(&to.min, step), self.max.lerp(&to.max, step))
    }
}
impl<T, U> Transitionable for euclid::Point3D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::point3(self.x.lerp(&to.x, step), self.y.lerp(&to.y, step), self.z.lerp(&to.z, step))
    }
}
impl<T, U> Transitionable for euclid::Box3D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::Box3D::new(self.min.lerp(&to.min, step), self.max.lerp(&to.max, step))
    }
}
impl<T, U> Transitionable for euclid::Length<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::Length::new(self.get().lerp(&to.clone().get(), step))
    }
}
impl<T, U> Transitionable for euclid::Size2D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::size2(self.width.lerp(&to.width, step), self.height.lerp(&to.height, step))
    }
}
impl<T, U> Transitionable for euclid::Size3D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::size3(
            self.width.lerp(&to.width, step),
            self.height.lerp(&to.height, step),
            self.depth.lerp(&to.depth, step),
        )
    }
}
impl<T, U> Transitionable for euclid::Rect<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::Rect::new(self.origin.lerp(&to.origin, step), self.size.lerp(&to.size, step))
    }
}
impl<T, U> Transitionable for euclid::Vector2D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::vec2(self.x.lerp(&to.x, step), self.y.lerp(&to.y, step))
    }
}
impl<T, U> Transitionable for euclid::Vector3D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::vec3(self.x.lerp(&to.x, step), self.y.lerp(&to.y, step), self.z.lerp(&to.z, step))
    }
}
impl Transitionable for Factor {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        Factor(self.0.lerp(&to.0, step))
    }
}
impl<T, U> Transitionable for euclid::SideOffsets2D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        euclid::SideOffsets2D::new(
            self.top.lerp(&to.top, step),
            self.right.lerp(&to.right, step),
            self.bottom.lerp(&to.bottom, step),
            self.left.lerp(&to.left, step),
        )
    }
}
impl Transitionable for bool {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        if step >= 1.fct() {
            *to
        } else {
            self
        }
    }
}
impl<T, U> Transitionable for CornerRadius2D<T, U>
where
    T: Transitionable,
    U: Send + Sync + Any,
{
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        Self {
            top_left: self.top_left.lerp(&to.top_left, step),
            top_right: self.top_right.lerp(&to.top_right, step),
            bottom_right: self.bottom_right.lerp(&to.bottom_right, step),
            bottom_left: self.bottom_left.lerp(&to.bottom_left, step),
        }
    }
}

impl_from_and_into_var! {
    fn from(s: &'static str) -> Txt;
    fn from(s: String) -> Txt;
    fn from(s: Cow<'static, str>) -> Txt;
    fn from(c: char) -> Txt;
    fn from(t: Txt) -> PathBuf;
    fn from(t: Txt) -> String;
    fn from(t: Txt) -> Cow<'static, str>;

    fn from(f: f32) -> Factor;
    fn from(one_or_zero: bool) -> Factor;
    fn from(f: FactorPercent) -> Factor;
    fn from(f: Factor) -> FactorPercent;

    fn from(d: Instant) -> Deadline;
    fn from(d: Duration) -> Deadline;
}

macro_rules! impl_into_var_option {
    (
        $($T:ty),* $(,)?
    ) => {
        impl_from_and_into_var! { $(
            fn from(some: $T) -> Option<$T>;
        )* }
    }
}
impl_into_var_option! {
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64,
    char, bool,
    zero_ui_units::Orientation2D,
}
