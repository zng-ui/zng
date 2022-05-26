use std::{fmt, mem, ops};

use crate::{context::LayoutMetrics, impl_from_and_into_var};

use super::{impl_length_comp_conversions, Factor, Factor2d, FactorPercent, LayoutMask, Length, Px};

/// Spacing in-between grid cells in [`Length`] units.
#[derive(Clone, Default, PartialEq)]
pub struct GridSpacing {
    /// Spacing in-between columns, in length units.
    pub column: Length,
    /// Spacing in-between rows, in length units.
    pub row: Length,
}
impl GridSpacing {
    /// New column, row from any [`Length`] unit..
    pub fn new<C: Into<Length>, R: Into<Length>>(column: C, row: R) -> Self {
        GridSpacing {
            column: column.into(),
            row: row.into(),
        }
    }

    /// Same spacing for both columns and rows.
    pub fn new_all<S: Into<Length>>(same: S) -> Self {
        let same = same.into();
        GridSpacing {
            column: same.clone(),
            row: same,
        }
    }

    /// Compute the spacing in a layout context.
    pub fn layout(&self, ctx: &LayoutMetrics, mut default_value: impl FnMut(&LayoutMetrics) -> PxGridSpacing) -> PxGridSpacing {
        PxGridSpacing {
            column: self.column.layout(ctx.for_x(), |ctx| default_value(ctx.metrics).column),
            row: self.row.layout(ctx.for_y(), |ctx| default_value(ctx.metrics).row),
        }
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`layout`].
    ///
    /// [`layout`]: Self::layout
    pub fn affect_mask(&self) -> LayoutMask {
        self.column.affect_mask() | self.row.affect_mask()
    }
}
impl_length_comp_conversions! {
    fn from(column: C, row: R) -> GridSpacing {
        GridSpacing::new(column, row)
    }
}
impl_from_and_into_var! {
    /// Same spacing for both columns and rows.
    fn from(all: Length) -> GridSpacing {
        GridSpacing::new_all(all)
    }

    /// Column and row equal relative length.
    fn from(percent: FactorPercent) -> GridSpacing {
        GridSpacing::new_all(percent)
    }
    /// Column and row equal relative length.
    fn from(norm: Factor) -> GridSpacing {
        GridSpacing::new_all(norm)
    }

    /// Column and row equal exact length.
    fn from(f: f32) -> GridSpacing {
        GridSpacing::new_all(f)
    }
    /// Column and row equal exact length.
    fn from(i: i32) -> GridSpacing {
        GridSpacing::new_all(i)
    }

    /// Column and row in device pixel length.
    fn from(spacing: PxGridSpacing) -> GridSpacing {
        GridSpacing::new(spacing.column, spacing.row)
    }
}
impl fmt::Debug for GridSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("GridSpacing")
                .field("column", &self.column)
                .field("row", &self.row)
                .finish()
        } else if self.column == self.row {
            write!(f, "{:?}", self.column)
        } else {
            write!(f, "({:?}, {:?})", self.column, self.row)
        }
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for GridSpacing {
    type Output = Self;

    fn mul(self, rhs: S) -> Self {
        let fct = rhs.into();

        GridSpacing {
            column: self.column * fct.x,
            row: self.row * fct.y,
        }
    }
}
impl<'a, S: Into<Factor2d>> ops::Mul<S> for &'a GridSpacing {
    type Output = GridSpacing;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}
impl<S: Into<Factor2d>> ops::MulAssign<S> for GridSpacing {
    fn mul_assign(&mut self, rhs: S) {
        let column = mem::take(&mut self.column);
        let row = mem::take(&mut self.row);
        let fct = rhs.into();

        self.column = column * fct.x;
        self.row = row * fct.y;
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for GridSpacing {
    type Output = Self;

    fn div(self, rhs: S) -> Self {
        let fct = rhs.into();

        GridSpacing {
            column: self.column / fct.x,
            row: self.row / fct.y,
        }
    }
}
impl<'a, S: Into<Factor2d>> ops::Div<S> for &'a GridSpacing {
    type Output = GridSpacing;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}
impl<S: Into<Factor2d>> ops::DivAssign<S> for GridSpacing {
    fn div_assign(&mut self, rhs: S) {
        let column = mem::take(&mut self.column);
        let row = mem::take(&mut self.row);
        let fct = rhs.into();

        self.column = column / fct.x;
        self.row = row / fct.y;
    }
}

/// Computed [`GridSpacing`].
#[derive(Clone, Default, Copy, Debug)]
pub struct PxGridSpacing {
    /// Spacing in-between columns, in layout pixels.
    pub column: Px,
    /// Spacing in-between rows, in layout pixels.
    pub row: Px,
}
impl PxGridSpacing {
    /// New grid spacing
    pub fn new(column: Px, row: Px) -> Self {
        Self { column, row }
    }
    /// Zero spacing.
    pub fn zero() -> Self {
        PxGridSpacing { column: Px(0), row: Px(0) }
    }
}
