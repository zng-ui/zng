use std::{fmt, mem, ops};

use zng_var::{animation::Transitionable, impl_from_and_into_var};

use crate::unit::{LengthCompositeParser, ParseCompositeError};

use super::{Factor, Factor2d, FactorPercent, Layout1d, LayoutMask, Length, Px, PxVector, impl_length_comp_conversions};

/// Spacing in-between grid cells in [`Length`] units.
#[derive(Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
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
}
impl super::Layout2d for GridSpacing {
    type Px = PxGridSpacing;

    fn layout_dft(&self, default: Self::Px) -> Self::Px {
        PxGridSpacing {
            column: self.column.layout_dft_x(default.column),
            row: self.row.layout_dft_y(default.row),
        }
    }

    fn affect_mask(&self) -> LayoutMask {
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
            write!(f, "{:.p$?}", self.column, p = f.precision().unwrap_or(0))
        } else {
            write!(f, "({:.p$?}, {:.p$?})", self.column, self.row, p = f.precision().unwrap_or(0))
        }
    }
}
impl fmt::Display for GridSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.column == self.row {
            write!(f, "{:.p$}", self.column, p = f.precision().unwrap_or(0))
        } else {
            write!(f, "({:.p$}, {:.p$})", self.column, self.row, p = f.precision().unwrap_or(0))
        }
    }
}
impl std::str::FromStr for GridSpacing {
    type Err = ParseCompositeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parser = LengthCompositeParser::new(s)?;
        let a = parser.next()?;
        if parser.has_ended() {
            return Ok(Self::new_all(a));
        }
        let b = parser.expect_last()?;
        Ok(Self::new(a, b))
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
impl<S: Into<Factor2d>> ops::Mul<S> for &GridSpacing {
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
impl<S: Into<Factor2d>> ops::Div<S> for &GridSpacing {
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
#[derive(Clone, Default, Copy, Debug, PartialEq, Eq, Hash)]
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

    /// Convert to vector.
    pub fn to_vector(self) -> PxVector {
        PxVector::new(self.column, self.row)
    }
}
impl ops::Add for GridSpacing {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}
impl ops::AddAssign for GridSpacing {
    fn add_assign(&mut self, rhs: Self) {
        self.column += rhs.column;
        self.row += rhs.row;
    }
}
impl ops::Sub for GridSpacing {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}
impl ops::SubAssign for GridSpacing {
    fn sub_assign(&mut self, rhs: Self) {
        self.column -= rhs.column;
        self.row -= rhs.row;
    }
}
impl From<PxGridSpacing> for PxVector {
    fn from(s: PxGridSpacing) -> Self {
        s.to_vector()
    }
}
impl From<PxVector> for PxGridSpacing {
    fn from(s: PxVector) -> Self {
        PxGridSpacing { column: s.x, row: s.y }
    }
}
