use std::fmt;

use crate::{context::LayoutMetrics, impl_from_and_into_var};

use super::{LayoutMask, Length, Point, Px, PxPoint, PxRect, PxToWr};

/// 2D line in [`Length`] units.
#[derive(Clone, Default, PartialEq)]
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
            write!(f, "{:?}.to{:?}", self.start, self.end)
        }
    }
}
impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "{:.p$} to {:.p$}", self.start, self.end, p = p)
        } else {
            write!(f, "{} to {}", self.start, self.end)
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

    /// Compute the line in a layout context.
    pub fn layout(&self, ctx: &LayoutMetrics, default_value: PxLine) -> PxLine {
        PxLine {
            start: self.start.layout(ctx, default_value.start),
            end: self.end.layout(ctx, default_value.end),
        }
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`to_layout`].
    ///
    /// [`layout`]: Self::layout
    pub fn affect_mask(&self) -> LayoutMask {
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
#[derive(Clone, Default, Copy, Debug, PartialEq, Eq)]
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
        let s = self.start.to_wr();
        let e = self.end.to_wr();
        Px(s.distance_to(e).round() as i32)
    }

    /// Bounding box that fits the line points, in layout units.
    pub fn bounds(self) -> PxRect {
        PxRect::from_points(&[self.start, self.end])
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
/// # use zero_ui_core::units::*;
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
