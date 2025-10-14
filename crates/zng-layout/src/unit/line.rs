use std::{fmt, ops};

use zng_var::{animation::Transitionable, impl_from_and_into_var};

use crate::unit::ParseCompositeError;

use super::{Factor2d, LayoutMask, Length, Point, Px, PxPoint, PxRect};

/// 2D line in [`Length`] units.
#[derive(Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Transitionable)]
pub struct Line {
    /// Start point in length units.
    pub start: Point,
    /// End point in length units.
    pub end: Point,
}
impl fmt::Debug for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Line").field("start", &self.start).field("end", &self.end).finish()
        } else {
            write!(f, "{:.p$?}.to{:.p$?}", self.start, self.end, p = f.precision().unwrap_or(0))
        }
    }
}
impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.p$} to {:.p$}", self.start, self.end, p = f.precision().unwrap_or(0))
    }
}
impl std::str::FromStr for Line {
    type Err = ParseCompositeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((a, b)) = s.split_once(".to") {
            Ok(Self::new(Point::from_str(a)?, Point::from_str(b)?))
        } else if let Some((a, b)) = s.split_once(" to ") {
            Ok(Self::new(Point::from_str(a.trim())?, Point::from_str(b.trim())?))
        } else {
            Err(ParseCompositeError::UnknownFormat)
        }
    }
}
impl Line {
    /// New line defined by two points of any type that converts to [`Point`].
    ///
    /// Also see [`LineFromTuplesBuilder`] for another way of initializing a line value.
    pub fn new<S: Into<Point>, E: Into<Point>>(start: S, end: E) -> Self {
        Line {
            start: start.into(),
            end: end.into(),
        }
    }

    /// Line from [zero](Point::zero) to [zero](Point::zero).
    pub fn zero() -> Line {
        Line {
            start: Point::zero(),
            end: Point::zero(),
        }
    }

    /// Line that fills the available length from [bottom](Point::bottom) to [top](Point::top).
    pub fn to_top() -> Line {
        Line {
            start: Point::bottom(),
            end: Point::top(),
        }
    }

    /// Line that traces the length from [top](Point::top) to [bottom](Point::bottom).
    pub fn to_bottom() -> Line {
        Line {
            start: Point::top(),
            end: Point::bottom(),
        }
    }

    /// Line that traces the length from [left](Point::left) to [right](Point::right).
    pub fn to_right() -> Line {
        Line {
            start: Point::left(),
            end: Point::right(),
        }
    }

    /// Line that traces the length from [right](Point::right) to [left](Point::left).
    pub fn to_left() -> Line {
        Line {
            start: Point::right(),
            end: Point::left(),
        }
    }

    /// Line that traces the length from [bottom-right](Point::bottom_right) to [top-left](Point::top_left).
    pub fn to_top_left() -> Line {
        Line {
            start: Point::bottom_right(),
            end: Point::top_left(),
        }
    }

    /// Line that traces the length from [bottom-left](Point::bottom_left) to [top-right](Point::top_right).
    pub fn to_top_right() -> Line {
        Line {
            start: Point::bottom_left(),
            end: Point::top_right(),
        }
    }

    /// Line that traces the length from [top-right](Point::top_right) to [bottom-left](Point::bottom_left).
    pub fn to_bottom_left() -> Line {
        Line {
            start: Point::top_right(),
            end: Point::bottom_left(),
        }
    }

    /// Line that traces the length from [top-left](Point::top_left) to [bottom-right](Point::bottom_right).
    pub fn to_bottom_right() -> Line {
        Line {
            start: Point::top_left(),
            end: Point::bottom_right(),
        }
    }
}
impl super::Layout2d for Line {
    type Px = PxLine;

    fn layout_dft(&self, default: Self::Px) -> Self::Px {
        PxLine {
            start: self.start.layout_dft(default.start),
            end: self.end.layout_dft(default.end),
        }
    }

    fn affect_mask(&self) -> LayoutMask {
        self.start.affect_mask() | self.end.affect_mask()
    }
}
impl_from_and_into_var! {
    /// From exact lengths.
    fn from(line: PxLine) -> Line {
        Line::new(line.start, line.end)
    }
}

/// Computed [`Line`].
#[derive(Clone, Default, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PxLine {
    /// Start point in layout units.
    pub start: PxPoint,
    /// End point in layout units.
    pub end: PxPoint,
}
impl PxLine {
    /// New layout line defined by two layout points.
    pub fn new(start: PxPoint, end: PxPoint) -> Self {
        Self { start, end }
    }

    /// Line from (0, 0) to (0, 0).
    pub fn zero() -> Self {
        Self::new(PxPoint::zero(), PxPoint::zero())
    }

    /// Line length in rounded pixels.
    pub fn length(self) -> Px {
        let s = self.start.cast::<f32>();
        let e = self.end.cast::<f32>();
        Px(s.distance_to(e).round() as i32)
    }

    /// Bounding box that fits the line points, in layout units.
    pub fn bounds(self) -> PxRect {
        PxRect::from_points([self.start, self.end])
    }

    /// Returns a line that starts from the left-top most point and ends at the bottom-right most point.
    pub fn normalize(self) -> PxLine {
        let start = self.start.min(self.end);
        let end = self.start.max(self.end);
        PxLine { start, end }
    }
}

/// Build a [`Line`] using the syntax `(x1, y1).to(x2, y2)`.
///
/// # Examples
///
/// ```
/// # use zng_layout::unit::*;
/// let line = (10, 20).to(100, 120);
/// assert_eq!(Line::new(Point::new(10, 20), Point::new(100, 120)), line);
/// ```
pub trait LineFromTuplesBuilder {
    /// New [`Line`] from `self` as a start point to `x2, y2` end point.
    fn to<X2: Into<Length>, Y2: Into<Length>>(self, x2: X2, y2: Y2) -> Line;
}
impl<X1: Into<Length>, Y1: Into<Length>> LineFromTuplesBuilder for (X1, Y1) {
    fn to<X2: Into<Length>, Y2: Into<Length>>(self, x2: X2, y2: Y2) -> Line {
        Line::new(self, (x2, y2))
    }
}

impl<S: Into<Factor2d>> ops::Mul<S> for Line {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Mul<S> for &Line {
    type Output = Line;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}
impl<S: Into<Factor2d>> ops::MulAssign<S> for Line {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.start *= rhs;
        self.end *= rhs;
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for Line {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}
impl<S: Into<Factor2d>> ops::Div<S> for &Line {
    type Output = Line;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}
impl<S: Into<Factor2d>> ops::DivAssign<S> for Line {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.start /= rhs;
        self.end /= rhs;
    }
}

impl ops::Add for Line {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}
impl ops::AddAssign for Line {
    fn add_assign(&mut self, rhs: Self) {
        self.start += rhs.start;
        self.end += rhs.end;
    }
}
impl ops::Sub for Line {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}
impl ops::SubAssign for Line {
    fn sub_assign(&mut self, rhs: Self) {
        self.start -= rhs.start;
        self.end -= rhs.end;
    }
}
