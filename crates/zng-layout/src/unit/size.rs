use std::{fmt, ops};

use zng_var::{animation::Transitionable, impl_from_and_into_var};

use crate::unit::{LengthCompositeParser, ParseCompositeError};

use super::{DipSize, Factor, Factor2d, FactorPercent, Layout1d, LayoutMask, Length, PxSize, Rect, Vector, impl_length_comp_conversions};

/// 2D size in [`Length`] units.
#[derive(Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct Size {
    /// *width* in length units.
    pub width: Length,
    /// *height* in length units.
    pub height: Length,
}
impl fmt::Debug for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Size")
                .field("width", &self.width)
                .field("height", &self.height)
                .finish()
        } else {
            write!(f, "({:?}, {:?})", self.width, self.height)
        }
    }
}
impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "{:.p$} × {:.p$}", self.width, self.height, p = p)
        } else {
            write!(f, "{} × {}", self.width, self.height)
        }
    }
}
impl std::str::FromStr for Size {
    type Err = ParseCompositeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parser = LengthCompositeParser::new_sep(s, &[',', '×'])?;
        let a = parser.next()?;
        if parser.has_ended() {
            return Ok(Self::splat(a));
        }
        let b = parser.expect_last()?;
        Ok(Self::new(a, b))
    }
}
impl Size {
    /// New width, height from any [`Length`] unit.
    pub fn new<W: Into<Length>, H: Into<Length>>(width: W, height: H) -> Self {
        Size {
            width: width.into(),
            height: height.into(),
        }
    }

    /// New width, height from single value of any [`Length`] unit.
    pub fn splat(wh: impl Into<Length>) -> Self {
        let wh = wh.into();
        Size {
            width: wh.clone(),
            height: wh,
        }
    }

    /// ***width:*** [`Length::zero`], ***height:*** [`Length::zero`]
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// Size that fills the available space.
    ///
    /// ***width:*** [`Length::fill`], ***height:*** [`Length::fill`]
    pub fn fill() -> Self {
        Self::new(Length::fill(), Length::fill())
    }

    /// Returns `(width, height)`.
    pub fn as_tuple(self) -> (Length, Length) {
        (self.width, self.height)
    }

    /// Returns `[width, height]`.
    pub fn as_array(self) -> [Length; 2] {
        [self.width, self.height]
    }

    /// Returns `true` if all values are [`Length::Default`].
    pub fn is_default(&self) -> bool {
        self.width.is_default() && self.height.is_default()
    }

    /// Returns `true` if any value is [`Length::Default`].
    pub fn has_default(&self) -> bool {
        self.width.has_default() || self.height.has_default()
    }

    /// Replaces [`Length::Default`] values with `overwrite` values.
    pub fn replace_default(&mut self, overwrite: &Size) {
        self.width.replace_default(&overwrite.width);
        self.height.replace_default(&overwrite.height);
    }

    /// Returns a vector of x: width and y: height.
    pub fn as_vector(self) -> Vector {
        Vector {
            x: self.width,
            y: self.height,
        }
    }
}
impl super::Layout2d for Size {
    type Px = PxSize;

    fn layout_dft(&self, default: Self::Px) -> Self::Px {
        PxSize::new(self.width.layout_dft_x(default.width), self.height.layout_dft_y(default.height))
    }

    fn affect_mask(&self) -> LayoutMask {
        self.width.affect_mask() | self.height.affect_mask()
    }
}
impl_length_comp_conversions! {
    fn from(width: W, height: H) -> Size {
        Size::new(width, height)
    }
}
impl_from_and_into_var! {
    /// Splat.
    fn from(all: Length) -> Size {
        Size::splat(all)
    }

    /// Splat relative length.
    fn from(percent: FactorPercent) -> Size {
        Size::splat(percent)
    }
    /// Splat relative length.
    fn from(norm: Factor) -> Size {
        Size::splat(norm)
    }

    /// Splat exact length.
    fn from(f: f32) -> Size {
        Size::splat(f)
    }
    /// Splat exact length.
    fn from(i: i32) -> Size {
        Size::splat(i)
    }
    fn from(size: PxSize) -> Size {
        Size::new(size.width, size.height)
    }
    fn from(size: DipSize) -> Size {
        Size::new(size.width, size.height)
    }
    fn from(v: Vector) -> Size {
        v.as_size()
    }
    fn from(r: Rect) -> Size {
        r.size
    }
}
impl<S: Into<Size>> ops::Add<S> for Size {
    type Output = Size;

    fn add(self, rhs: S) -> Self::Output {
        let rhs = rhs.into();

        Size {
            width: self.width + rhs.width,
            height: self.height + rhs.height,
        }
    }
}
impl<S: Into<Size>> ops::AddAssign<S> for Size {
    fn add_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.width += rhs.width;
        self.height += rhs.height;
    }
}
impl<S: Into<Size>> ops::Sub<S> for Size {
    type Output = Size;

    fn sub(self, rhs: S) -> Self::Output {
        let rhs = rhs.into();

        Size {
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}
impl<S: Into<Size>> ops::SubAssign<S> for Size {
    fn sub_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.width -= rhs.width;
        self.height -= rhs.height;
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for Size {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for &Size {
    type Output = Size;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}
impl<S: Into<Factor2d>> ops::MulAssign<S> for Size {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.width *= rhs.x;
        self.height *= rhs.y;
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for Size {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for &Size {
    type Output = Size;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}
impl<S: Into<Factor2d>> ops::DivAssign<S> for Size {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.width /= rhs.x;
        self.height /= rhs.y;
    }
}
