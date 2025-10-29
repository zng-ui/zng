use std::{fmt, mem, ops};
use zng_layout::context::LayoutMask;
use zng_var::animation::{Transitionable, easing::EasingStep};
use zng_wgt::prelude::*;

/// Defines a placement point in the previous item and the origin point of the next.
///
/// Defining stack direction like this allows expressing the traditional stack directions along an axis, as well as
/// intermediary for transition animations or diagonal directions.
///
/// Note that collapsed items (layout size zero) are skipped, so the previous and next items are both non-empty in layout.
///
/// # Alignment & Spacing
///
/// The direction type can express non-fill alignment and spacing by it self, but prefer using the [`stack::children_align`] and
/// [`stack::spacing`] properties as they are more readable and include fill alignment.
///
/// The [`Stack!`] widget implements alignment along the axis that does not change, so if the computed layout vector
/// is zero in a dimension the items can fill in that dimension.
///
/// The [`Stack!`] widget adds the spacing along non-zero axis for each item offset after the first, so the spacing is not
/// added for a perfect straight column or row, but it is added even for a single pixel shift *diagonal* stack.
///
/// [`stack::children_align`]: fn@crate::children_align
/// [`stack::spacing`]: fn@crate::spacing
/// [`Stack!`]: struct@crate::Stack
#[derive(Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StackDirection {
    /// Point on the previous item where the next item is placed.
    pub place: Point,
    /// Point on the next item that is offset to match `place`.
    pub origin: Point,

    /// If `place.x` and `origin.x` are swapped in [`LayoutDirection::RTL`] contexts.
    ///
    /// [`LayoutDirection::RTL`]: zng_wgt::prelude::LayoutDirection::RTL
    pub is_rtl_aware: bool,
}

impl fmt::Debug for StackDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("StackDirection")
                .field("place", &self.place)
                .field("origin", &self.origin)
                .field("is_rtl_aware", &self.is_rtl_aware)
                .finish()
        } else if self.is_rtl_aware {
            write!(f, "({:?}, {:?}, {:?})", self.place, self.origin, self.is_rtl_aware)
        } else {
            write!(f, "({:?}, {:?})", self.place, self.origin)
        }
    }
}

impl fmt::Display for StackDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.p$}, {:.p$}", self.place, self.origin, p = p)?;
        } else {
            write!(f, "({}, {}", self.place, self.origin)?;
        }

        if self.is_rtl_aware {
            write!(f, ", {})", self.is_rtl_aware)
        } else {
            write!(f, ")")
        }
    }
}
impl StackDirection {
    /// New custom direction.
    pub fn new<P: Into<Point>, O: Into<Point>>(place: P, origin: O, is_rtl_aware: bool) -> Self {
        Self {
            place: place.into(),
            origin: origin.into(),
            is_rtl_aware,
        }
    }

    /// `((100.pct(), 0), (0, 0))`, items are placed in a row from left to right.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    ///
    /// The shorthand unit `left_to_right!` converts into this.
    pub fn left_to_right() -> Self {
        Self {
            place: (100.pct(), 0).into(),
            origin: (0, 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((0, 0), (100.pct(), 0))`, items are placed in a row from right to left.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    ///
    /// The shorthand unit `right_to_left!` converts into this.
    pub fn right_to_left() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (100.pct(), 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((100.pct(), 0), (0, 0), true)`, items are placed in a row from left to right or from right to left in RTL contexts.
    ///
    /// In [`LayoutDirection::RTL`] contexts the `place.x` and `origin.x` values are swapped before they are computed.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    ///
    /// [`LayoutDirection::RTL`]: zng_wgt::prelude::LayoutDirection::RTL
    ///
    /// The shorthand unit `start_to_end!` converts into this.
    pub fn start_to_end() -> Self {
        Self {
            place: (100.pct(), 0).into(),
            origin: (0, 0).into(),
            is_rtl_aware: true,
        }
    }

    /// `((0, 0), (100.pct(), 0)), true)`, items are placed in a row from right to left or from left to right in RTL contexts.
    ///
    /// In [`LayoutDirection::RTL`] contexts the `place.x` and `origin.x` values are swapped before they are computed.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    ///
    /// [`LayoutDirection::RTL`]: zng_wgt::prelude::LayoutDirection::RTL
    ///
    /// The shorthand unit `end_to_start!` converts into this.
    pub fn end_to_start() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (100.pct(), 0).into(),
            is_rtl_aware: true,
        }
    }

    /// `((0, 100.pct()), (0, 0))`, items are placed in a column from top to bottom.
    ///  
    /// Alignment works on the `x` direction because it is not affected.
    ///
    /// The shorthand unit `top_to_bottom!` converts into this.
    pub fn top_to_bottom() -> Self {
        Self {
            place: (0, 100.pct()).into(),
            origin: (0, 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `(0, 0), (0, 100.pct())`, items are placed in a column from bottom to top.
    ///  
    /// Alignment works on the `x` direction because it is not affected.
    ///
    /// The shorthand unit `bottom_to_top!` converts into this.
    pub fn bottom_to_top() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (0, 100.pct()).into(),
            is_rtl_aware: false,
        }
    }

    /// `(0, 0)`, items are just stacked in the Z order.
    ///
    /// Fill alignment works in both dimensions because they don't change.
    ///
    /// Note that items are always rendered in the order defined by the [`z_index`] property.
    ///
    /// [`z_index`]: fn@zng_wgt::z_index
    ///
    /// The shorthand unit `none!` converts into this.
    pub fn none() -> Self {
        Self {
            place: Point::zero(),
            origin: Point::zero(),
            is_rtl_aware: false,
        }
    }

    /// Compute offset of the next item in the current [`LAYOUT`] context.
    ///
    /// [`LAYOUT`]: zng_wgt::prelude::LAYOUT
    pub fn layout(&self, prev_item: PxRect, next_item: PxSize) -> PxVector {
        if self.is_rtl_aware && LAYOUT.direction().is_rtl() {
            let mut d = self.clone();
            mem::swap(&mut d.place.x, &mut d.origin.x);
            d.is_rtl_aware = false;
            return d.layout_resolved_rtl(prev_item, next_item);
        }

        self.layout_resolved_rtl(prev_item, next_item)
    }
    pub(crate) fn layout_resolved_rtl(&self, prev_item: PxRect, next_item: PxSize) -> PxVector {
        let c = LAYOUT.constraints();
        let place = LAYOUT.with_constraints(c.with_exact_size(prev_item.size), || self.place.layout());
        let origin = LAYOUT.with_constraints(c.with_exact_size(next_item), || self.origin.layout());
        prev_item.origin.to_vector() + place.to_vector() - origin.to_vector()
    }

    /// Factor that defines the proportional direction.
    ///
    /// Values are in the range of `-1.0..=1.0`.
    pub fn direction_factor(&self, direction: LayoutDirection) -> Factor2d {
        let size = PxSize::new(Px(1000), Px(1000));
        let metrics = LayoutMetrics::new(1.fct(), size, Px(1000)).with_direction(direction);
        let p = LAYOUT.with_context(metrics, || self.layout(PxRect::from_size(size), size));

        fn v(px: Px) -> Factor {
            (px.0 as f32 / 1000.0).fct()
        }
        (v(p.x), v(p.y)).into()
    }

    /// Scale proportional to how one dimensional the direction is.
    ///
    /// Values are in the `0.0..=1.0` range where 0 is 20º or more from a single direction and 1 is 0º or 90º.
    pub fn direction_scale(&self) -> Factor2d {
        let scale = self.direction_factor(LayoutDirection::LTR).abs().yx();
        if scale.x == 0.fct() && scale.y == 0.fct() {
            return Factor2d::new(1.0, 1.0);
        }
        let angle = scale.y.0.atan2(scale.x.0).to_degrees();
        if angle <= 20.0 {
            Factor2d::new(1.0 - angle / 20.0, 0.0)
        } else if angle >= 70.0 {
            Factor2d::new(0.0, (angle - 70.0) / 20.0)
        } else {
            Factor2d::new(0.0, 0.0)
        }
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`layout`].
    ///
    /// [`layout`]: Self::layout
    /// [`LayoutMask`]: zng_layout::context::LayoutMask
    pub fn affect_mask(&self) -> LayoutMask {
        self.place.affect_mask() | self.origin.affect_mask()
    }

    /// Returns `true` if all values are [`Length::Default`].
    ///
    /// [`Length::Default`]: zng_wgt::prelude::Length::Default
    pub fn is_default(&self) -> bool {
        self.place.is_default() && self.origin.is_default()
    }
}

impl_from_and_into_var! {
    /// New from place and origin, not RTL aware.
    fn from<P: Into<Point>, O: Into<Point>>((origin, size): (P, O)) -> StackDirection {
        (origin, size, false).into()
    }

    /// New from place, origin, and RTL aware flag.
    fn from<P: Into<Point>, O: Into<Point>>((origin, size, rtl_aware): (P, O, bool)) -> StackDirection {
        StackDirection::new(origin, size, rtl_aware)
    }

    fn from(_: ShorthandUnit![left_to_right]) -> StackDirection {
        StackDirection::left_to_right()
    }
    fn from(_: ShorthandUnit![right_to_left]) -> StackDirection {
        StackDirection::right_to_left()
    }
    fn from(_: ShorthandUnit![start_to_end]) -> StackDirection {
        StackDirection::start_to_end()
    }
    fn from(_: ShorthandUnit![end_to_start]) -> StackDirection {
        StackDirection::end_to_start()
    }
    fn from(_: ShorthandUnit![top_to_bottom]) -> StackDirection {
        StackDirection::top_to_bottom()
    }
    fn from(_: ShorthandUnit![bottom_to_top]) -> StackDirection {
        StackDirection::bottom_to_top()
    }
    fn from(_: ShorthandUnit![none]) -> StackDirection {
        StackDirection::none()
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for StackDirection {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for &StackDirection {
    type Output = StackDirection;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}
impl<S: Into<Factor2d>> ops::MulAssign<S> for StackDirection {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.place *= rhs;
        self.origin *= rhs;
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for StackDirection {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for &StackDirection {
    type Output = StackDirection;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}
impl<S: Into<Factor2d>> ops::DivAssign<S> for StackDirection {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.place /= rhs;
        self.origin /= rhs;
    }
}
impl ops::Add for StackDirection {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}
impl ops::AddAssign for StackDirection {
    fn add_assign(&mut self, rhs: Self) {
        self.place += rhs.place;
        self.origin += rhs.origin;
    }
}
impl ops::Sub for StackDirection {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}
impl ops::SubAssign for StackDirection {
    fn sub_assign(&mut self, rhs: Self) {
        self.place -= rhs.place;
        self.origin -= rhs.origin;
    }
}

impl Transitionable for StackDirection {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        Self {
            place: self.place.lerp(&to.place, step),
            origin: self.origin.lerp(&to.origin, step),
            is_rtl_aware: if step < 1.fct() { self.is_rtl_aware } else { to.is_rtl_aware },
        }
    }
}
