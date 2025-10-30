use std::{
    borrow::Cow,
    fmt::{self, Write},
    ops,
};

use crate::{context::LayoutDirection, unit::ParseFloatCompositeError};
use zng_var::{
    animation::{Transitionable, easing::EasingStep},
    impl_from_and_into_var,
};

use super::{Factor, Factor2d, FactorPercent, FactorUnits, Point, Px, PxConstraints, PxConstraints2d, PxSize, PxVector};

/// `x` and `y` alignment.
///
/// The values indicate how much to the right and bottom the content is moved within
/// a larger available space. An `x` value of `0.0` means the content left border touches
/// the container left border, a value of `1.0` means the content right border touches the
/// container right border.
///
/// There is a constant for each of the usual alignment values, the alignment is defined as two factors like this
/// primarily for animating transition between alignments.
///
/// Values outside of the `[0.0..=1.0]` range places the content outside of the container bounds.
///
/// ## Special Values
///
/// The [`f32::INFINITY`] value can be used in ***x*** or ***y*** to indicate that the content must *fill* the available space.
///
/// The [`f32::NEG_INFINITY`] value can be used in ***y*** to indicate that a panel widget must align its items by each *baseline*,
/// for most widgets this is the same as `BOTTOM`, but for texts this aligns to the baseline of the texts (bottom + baseline).
///
/// You can use the [`is_fill_x`], [`is_fill_y`] and [`is_baseline`] methods to probe for these special values.
///
/// ## Right-to-Left
///
/// The `x` alignment can be flagged as `x_rtl_aware`, in widgets that implement right-to-left the `x` value is flipped around `0.5.fct()`.
/// The named `const` values that contain `START` and `END` are `x_rtl_aware`, the others are not. The `x_rtl_aware` flag is sticky, all
/// arithmetic operations between aligns output an `x_rtl_aware` align if any of the inputs is flagged. The flag is only resolved explicitly,
/// arithmetic operations apply on the
///
/// [`is_fill_x`]: Align::is_fill_x
/// [`is_fill_y`]: Align::is_fill_y
/// [`is_baseline`]: Align::is_baseline
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Align {
    /// *x* alignment in a `[0.0..=1.0]` range.
    pub x: Factor,
    /// If `x` is flipped (around `0.5`) in right-to-left contexts.
    pub x_rtl_aware: bool,

    /// *y* alignment in a `[0.0..=1.0]` range.
    pub y: Factor,
}
impl Default for Align {
    /// [`Align::START`].
    fn default() -> Self {
        Align::START
    }
}
impl Align {
    /// Gets the best finite [`x`] align value.
    ///
    /// Replaces `FILL` with `START`, flips `x` for right-to-left if applicable.
    ///
    /// [`x`]: Self::x
    pub fn x(self, direction: LayoutDirection) -> Factor {
        let x = if self.x.0.is_finite() { self.x } else { 0.fct() };

        if self.x_rtl_aware && direction.is_rtl() { x.flip() } else { x }
    }

    /// Gets the best finite [`y`] align value.
    ///
    /// Returns `1.fct()` for [`is_baseline`], implementers must add the baseline offset to that.
    ///
    /// [`y`]: Self::y
    /// [`is_baseline`]: Self::is_baseline
    pub fn y(self) -> Factor {
        if self.y.0.is_finite() {
            self.y
        } else if self.is_baseline() {
            1.fct()
        } else {
            0.fct()
        }
    }

    /// Gets the best finite [`x`] and [`y`] align values.
    ///
    /// [`x`]: fn@Self::x
    /// [`y`]: fn@Self::y
    pub fn xy(self, direction: LayoutDirection) -> Factor2d {
        Factor2d::new(self.x(direction), self.y())
    }

    /// Returns `true` if [`x`] is a special value that indicates the content width must be the container width.
    ///
    /// [`x`]: Align::x
    pub fn is_fill_x(self) -> bool {
        self.x.0.is_infinite() && self.x.0.is_sign_positive()
    }

    /// Returns `true` if [`y`] is a special value that indicates the content height must be the container height.
    ///
    /// [`y`]: Align::y
    pub fn is_fill_y(self) -> bool {
        self.y.0.is_infinite() && self.y.0.is_sign_positive()
    }

    /// Returns `true` if [`y`] is a special value that indicates the contents must be aligned by their baseline.
    ///
    /// If this is `true` the *y* alignment must be `BOTTOM` plus the baseline offset.
    ///
    /// [`y`]: Align::y
    pub fn is_baseline(self) -> bool {
        self.y.0.is_infinite() && self.y.0.is_sign_negative()
    }

    /// Returns a boolean vector of the fill values.
    pub fn fill_vector(self) -> super::euclid::BoolVector2D {
        super::euclid::BoolVector2D {
            x: self.is_fill_x(),
            y: self.is_fill_y(),
        }
    }

    /// Constraints that must be used to layout a child node with the alignment.
    ///
    /// Note that these constraints define the child inner bounds (the visual size) only,
    /// using the child size call [`layout`] to get the child outer bounds size.
    ///
    /// [`layout`]: Self::layout
    pub fn child_constraints(self, parent_constraints: PxConstraints2d) -> PxConstraints2d {
        // FILL is the *default* property value, so it must behave the same way as if the alignment was not applied.
        parent_constraints
            .with_new_min(
                if self.is_fill_x() { parent_constraints.x.min() } else { Px(0) },
                if self.is_fill_y() { parent_constraints.y.min() } else { Px(0) },
            )
            .with_fill_and(self.is_fill_x(), self.is_fill_y())
    }

    /// Compute the offset for a given child size, parent size and layout direction.
    ///
    /// Note that this does not flag baseline offset, you can use [`layout`] to cover all corner cases.
    ///
    /// [`layout`]: Self::layout
    pub fn child_offset(self, child_size: PxSize, parent_size: PxSize, direction: LayoutDirection) -> PxVector {
        let mut offset = PxVector::zero();
        if !self.is_fill_x() {
            let x = if self.x_rtl_aware && direction.is_rtl() {
                self.x.flip().0
            } else {
                self.x.0
            };

            offset.x = (parent_size.width - child_size.width) * x;
        }

        let baseline = self.is_baseline();

        if !self.is_fill_y() {
            let y = if baseline { 1.0 } else { self.y.0 };

            offset.y = (parent_size.height - child_size.height) * y;
        }
        offset
    }

    /// Computes the size returned by [`layout`] for the given child size and constraints.
    ///
    /// Note that the child must be measured using [`child_constraints`], the `child_size` is the size the child
    /// will be rendered at, this method computes the child outer bounds.
    ///
    /// [`layout`]: Self::layout
    /// [`child_constraints`]: Self::child_constraints
    pub fn measure(self, child_size: PxSize, parent_constraints: PxConstraints2d) -> PxSize {
        PxSize::new(
            self.measure_x(child_size.width, parent_constraints.x),
            self.measure_y(child_size.height, parent_constraints.y),
        )
    }

    /// Computes the width returned by layout for the given child width and ***x*** constraints.
    ///
    /// See [`measure`] for more details.
    ///
    /// [`measure`]: Self::measure
    pub fn measure_x(self, child_width: Px, parent_constraints_x: PxConstraints) -> Px {
        if parent_constraints_x.is_inner() {
            child_width
        } else {
            let width = parent_constraints_x.fill().max(child_width);
            parent_constraints_x.clamp(width)
        }
    }

    /// Computes the height returned by layout for the given child height and ***y*** constraints.
    ///
    /// See [`measure`] for more details.
    ///
    /// [`measure`]: Self::measure
    pub fn measure_y(self, child_height: Px, parent_constraints_y: PxConstraints) -> Px {
        if parent_constraints_y.is_inner() {
            child_height
        } else {
            let height = parent_constraints_y.fill().max(child_height);
            parent_constraints_y.clamp(height)
        }
    }

    /// Compute the outer size and inner offset.
    ///
    /// Note that the child must be layout using the [`child_constraints`], the `child_size` is the size the child
    /// will be rendered at, this method computes the child outer bounds.
    ///
    /// Returns the outer size, inner offset and [`is_baseline`]
    ///
    /// [`child_constraints`]: Self::child_constraints
    /// [`is_baseline`]: Self::is_baseline
    pub fn layout(self, child_size: PxSize, parent_constraints: PxConstraints2d, direction: LayoutDirection) -> (PxSize, PxVector, bool) {
        let size = self.measure(child_size, parent_constraints);
        let offset = self.child_offset(child_size, size, direction);
        (size, offset, self.is_baseline())
    }
}
impl_from_and_into_var! {
    fn from<X: Into<Factor>, Y: Into<Factor>>((x, y): (X, Y)) -> Align {
        Align {
            x: x.into(),
            x_rtl_aware: false,
            y: y.into(),
        }
    }

    fn from<X: Into<Factor>, Y: Into<Factor>>((x, rtl, y): (X, bool, Y)) -> Align {
        Align {
            x: x.into(),
            x_rtl_aware: rtl,
            y: y.into(),
        }
    }

    fn from(xy: Factor) -> Align {
        Align {
            x: xy,
            x_rtl_aware: false,
            y: xy,
        }
    }

    fn from(xy: FactorPercent) -> Align {
        xy.fct().into()
    }
}
macro_rules! named_aligns {

    ( $($(#[$doc:meta])* $NAME:ident = ($x:expr, $rtl:expr, $y:expr);)+ ) => {
        $(
        $(#[$doc])*
        pub const $NAME: Align = Align { x: Factor($x), x_rtl_aware: $rtl, y: Factor($y) };
        )+

        /// Returns the alignment `const` name if `self` is equal to one of then.
        pub fn name(self) -> Option<&'static str> {
            $(
                if self == Self::$NAME {
                    Some(stringify!($NAME))
                }
            )else+
            else {
                None
            }
        }

        /// Returns the named alignment.
        pub fn from_name(name: &str) -> Option<Self> {
            $(
                if name == stringify!($NAME) {
                    Some(Self::$NAME)
                }
            )else+
            else {
                None
            }
        }
    };
}
impl fmt::Debug for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            if f.alternate() {
                write!(f, "Align::{name}")
            } else {
                f.write_str(name)
            }
        } else {
            f.debug_struct("Align")
                .field("x", &self.x)
                .field("x_rtl_aware", &self.x_rtl_aware)
                .field("y", &self.y)
                .finish()
        }
    }
}
impl fmt::Display for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            f.write_str(name)
        } else {
            f.write_char('(')?;
            if self.is_fill_x() {
                f.write_str("FILL")?;
            } else {
                write!(f, "{}", FactorPercent::from(self.x))?;
            }
            f.write_str(", ")?;
            if self.is_fill_y() {
                f.write_str("FILL")?;
            } else if self.is_baseline() {
                f.write_str("BASELINE")?;
            } else {
                write!(f, "{}", FactorPercent::from(self.y))?;
            }
            f.write_char(')')
        }
    }
}
/// Parse a named align or `"(x, y)"` were x `Factor` or `"FILL"` and y is `factor`, `"FILL"` or `"BASELINE"`.
impl std::str::FromStr for Align {
    type Err = ParseFloatCompositeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(named) = Align::from_name(s) {
            Ok(named)
        } else if let Some(s) = s.strip_prefix('(')
            && let Some(s) = s.strip_prefix(')')
        {
            let mut parser = ComponentParser { iter: s.split(',') };
            let r = Self {
                x: parser.next(false)?,
                x_rtl_aware: false,
                y: parser.next(true)?,
            };
            parser.end()?;
            Ok(r)
        } else {
            Err(ParseFloatCompositeError::UnknownFormat)
        }
    }
}
struct ComponentParser<'a> {
    iter: std::str::Split<'a, char>,
}
impl<'a> ComponentParser<'a> {
    fn next(&mut self, y: bool) -> Result<Factor, ParseFloatCompositeError> {
        let fct = match self.iter.next().ok_or(ParseFloatCompositeError::MissingComponent)?.trim() {
            "FILL" => f32::INFINITY.fct(),
            "BASELINE" if y => f32::NEG_INFINITY.fct(),
            s => s.parse()?,
        };
        Ok(fct)
    }
    fn end(mut self) -> Result<(), ParseFloatCompositeError> {
        if self.iter.next().is_some() {
            Err(ParseFloatCompositeError::ExtraComponent)
        } else {
            Ok(())
        }
    }
}

#[zng_var::impl_property_value]
impl Align {
    named_aligns! {
        /// x: 0, y: 0, RTL aware.
        ///
        /// In left-to-right contexts this is `TOP_LEFT`, in right-to-left contexts this is `TOP_RIGHT`.
        TOP_START = (0.0, true, 0.0);
        /// x: 0, y: 0
        TOP_LEFT = (0.0, false, 0.0);
        /// x: 0, y: 1, RTL aware.
        ///
        /// In left-to-right contexts this is `BOTTOM_LEFT`, in right-to-left contexts this is `BOTTOM_RIGHT`.
        BOTTOM_START = (0.0, true, 1.0);
        /// x: 0, y: 1
        BOTTOM_LEFT = (0.0, false, 1.0);

        /// x: 1, y: 0, RTL aware.
        ///
        /// In left-to-right contexts this is `TOP_RIGHT`, in right-to-left contexts this is `TOP_LEFT`.
        TOP_END = (1.0, true, 0.0);
        /// x: 1, y: 0
        TOP_RIGHT = (1.0, false, 0.0);
        /// x: 1, y: 1
        ///
        /// In left-to-right contexts this is `BOTTOM_RIGHT`, in right-to-left contexts this is `BOTTOM_LEFT`.
        BOTTOM_END = (1.0, true, 1.0);
        /// x: 1, y: 1
        BOTTOM_RIGHT = (1.0, false, 1.0);

        /// x: 0, y: 0.5, RTL aware.
        ///
        /// In left-to-right contexts this is `LEFT`, in right-to-left contexts this is `RIGHT`.
        START = (0.0, true, 0.5);
        /// x: 0, y: 0.5
        LEFT = (0.0, false, 0.5);
        /// x: 1, y: 0.5, RTL aware.
        ///
        /// In left-to-right contexts this is `RIGHT`, in right-to-left contexts this is `LEFT`.
        END = (1.0, true, 0.5);
        /// x: 1, y: 0.5
        RIGHT = (1.0, false, 0.5);
        /// x: 0.5, y: 0
        TOP = (0.5, false, 0.0);
        /// x: 0.5, y: 1
        BOTTOM = (0.5, false, 1.0);

        /// x: 0.5, y: 0.5
        CENTER = (0.5, false, 0.5);

        /// x: +inf, y: 0
        FILL_TOP = (f32::INFINITY, false, 0.0);
        /// x: +inf, y: 1
        FILL_BOTTOM = (f32::INFINITY, false, 1.0);
        /// x: 0, y: +inf, RTL aware.
        ///
        /// In left-to-right contexts this is `FILL_LEFT`, in right-to-left contexts this is `FILL_RIGHT`.
        FILL_START = (0.0, true, f32::INFINITY);
        /// x: 0, y: +inf
        FILL_LEFT = (0.0, false, f32::INFINITY);
        /// x: 1, y: +inf
        FILL_RIGHT = (1.0, false, f32::INFINITY);
        /// x: 1, y: +inf, RTL aware.
        ///
        /// In left-to-right contexts this is `FILL_RIGHT`, in right-to-left contexts this is `FILL_LEFT`.
        FILL_END = (1.0, true, f32::INFINITY);

        /// x: +inf, y: 0.5
        FILL_X = (f32::INFINITY, false, 0.5);
        /// x: 0.5, y: +inf
        FILL_Y = (0.5, false, f32::INFINITY);

        /// x: +inf, y: +inf
        FILL = (f32::INFINITY, false, f32::INFINITY);

        /// x: 0, y: -inf, RTL aware.
        ///
        /// In left-to-right contexts this is `BASELINE_LEFT`, in right-to-left contexts this is `BASELINE_RIGHT`.
        BASELINE_START = (0.0, true, f32::NEG_INFINITY);
        /// x: 0, y: -inf
        BASELINE_LEFT = (0.0, false, f32::NEG_INFINITY);
        /// x: 0.5, y: -inf
        BASELINE_CENTER = (0.5, false, f32::NEG_INFINITY);
        /// x: 1, y: -inf, RTL aware.
        ///
        /// In left-to-right contexts this is `BASELINE_RIGHT`, in right-to-left contexts this is `BASELINE_LEFT`.
        BASELINE_END = (1.0, true, f32::NEG_INFINITY);
        /// x: 1, y: -inf
        BASELINE_RIGHT = (1.0, false, f32::NEG_INFINITY);

        /// x: +inf, y: -inf
        BASELINE = (f32::INFINITY, false, f32::NEG_INFINITY);
    }
}
impl_from_and_into_var! {
    /// To relative length x and y.
    fn from(alignment: Align) -> Point {
        Point {
            x: alignment.x.into(),
            y: alignment.y.into(),
        }
    }

    fn from(factor2d: Factor2d) -> Align {
        Align {
            x: factor2d.x,
            x_rtl_aware: false,
            y: factor2d.y,
        }
    }
}

impl Transitionable for Align {
    fn lerp(mut self, to: &Self, step: EasingStep) -> Self {
        let end = step >= 1.fct();

        if end {
            self.x_rtl_aware = to.x_rtl_aware;
        }

        if self.x.0.is_finite() && to.x.0.is_finite() {
            self.x = self.x.lerp(&to.x, step);
        } else if end {
            self.x = to.x;
        }

        if self.y.0.is_finite() && to.y.0.is_finite() {
            self.y = self.y.lerp(&to.y, step);
        } else if end {
            self.y = to.y;
        }

        self
    }
}

impl<S: Into<Factor2d>> ops::Mul<S> for Align {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::MulAssign<S> for Align {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();

        if self.x.0.is_finite() {
            self.x *= rhs.x;
        } else if rhs.x == 0.fct() {
            self.x = 0.fct();
        }
        if self.y.0.is_finite() {
            self.y *= rhs.y;
        } else if rhs.y == 0.fct() {
            self.y = 0.fct()
        }
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for Align {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::DivAssign<S> for Align {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();

        if self.x.0.is_finite() {
            self.x /= rhs.x;
        }
        if self.y.0.is_finite() {
            self.y /= rhs.y;
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum AlignSerde<'s> {
    Named(Cow<'s, str>),
    Unnamed { x: Factor, x_rtl_aware: bool, y: Factor },
}
impl serde::Serialize for Align {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable()
            && let Some(name) = self.name()
        {
            return AlignSerde::Named(Cow::Borrowed(name)).serialize(serializer);
        }

        AlignSerde::Unnamed {
            x: self.x,
            x_rtl_aware: self.x_rtl_aware,
            y: self.y,
        }
        .serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for Align {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        match AlignSerde::deserialize(deserializer)? {
            AlignSerde::Named(n) => match Align::from_name(&n) {
                Some(a) => Ok(a),
                None => Err(D::Error::custom("unknown align name")),
            },
            AlignSerde::Unnamed { x, x_rtl_aware, y } => Ok(Align { x, x_rtl_aware, y }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_named() {
        let value = serde_json::to_value(Align::TOP_START).unwrap();
        assert_eq!(value, serde_json::Value::String("TOP_START".to_owned()));

        let align: Align = serde_json::from_value(value).unwrap();
        assert_eq!(align, Align::TOP_START);
    }
}
