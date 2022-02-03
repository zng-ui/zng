use std::{fmt, mem, ops};

use crate::{context::LayoutMetrics, impl_from_and_into_var};

use super::{impl_length_comp_conversions, AvailableSize, DipSize, Factor, FactorPercent, LayoutMask, Length, PxSize, Scale2d, Vector};

/// 2D size in [`Length`] units.
#[derive(Clone, Default, PartialEq)]
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
    #[inline]
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// Size that fills the available space.
    ///
    /// ***width:*** [`Length::fill`], ***height:*** [`Length::fill`]
    #[inline]
    pub fn fill() -> Self {
        Self::new(Length::fill(), Length::fill())
    }

    /// Returns `(width, height)`.
    #[inline]
    pub fn as_tuple(self) -> (Length, Length) {
        (self.width, self.height)
    }

    /// Returns `[width, height]`.
    #[inline]
    pub fn as_array(self) -> [Length; 2] {
        [self.width, self.height]
    }

    /// Compute the size in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: AvailableSize, default_value: PxSize) -> PxSize {
        PxSize::new(
            self.width.to_layout(ctx, available_size.width, default_value.width),
            self.height.to_layout(ctx, available_size.height, default_value.height),
        )
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`to_layout`].
    ///
    /// [`to_layout`]: Self::to_layout
    #[inline]
    pub fn affect_mask(&self) -> LayoutMask {
        self.width.affect_mask() | self.height.affect_mask()
    }

    /// Returns `true` if all values are [`Length::Default`].
    pub fn is_default(&self) -> bool {
        self.width.is_default() && self.height.is_default()
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
}
impl<S: Into<Scale2d>> ops::Mul<S> for Size {
    type Output = Self;

    fn mul(self, rhs: S) -> Self {
        let fct = rhs.into();

        Size {
            width: self.width * fct.x,
            height: self.height * fct.y,
        }
    }
}
impl<S: Into<Scale2d>> ops::MulAssign<S> for Size {
    fn mul_assign(&mut self, rhs: S) {
        let width = mem::take(&mut self.width);
        let height = mem::take(&mut self.height);
        let fct = rhs.into();

        self.width = width * fct.x;
        self.height = height * fct.y;
    }
}
impl<S: Into<Scale2d>> ops::Div<S> for Size {
    type Output = Self;

    fn div(self, rhs: S) -> Self {
        let fct = rhs.into();

        Size {
            width: self.width / fct.x,
            height: self.height / fct.y,
        }
    }
}
impl<S: Into<Scale2d>> ops::DivAssign<S> for Size {
    fn div_assign(&mut self, rhs: S) {
        let width = mem::take(&mut self.width);
        let height = mem::take(&mut self.height);
        let fct = rhs.into();

        self.width = width / fct.x;
        self.height = height / fct.y;
    }
}
