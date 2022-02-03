use std::fmt::{self, Write};

use crate::impl_from_and_into_var;

use super::{Factor, FactorPercent, Point, PxRect, PxSize, PxVector};

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
/// Values outside of the `[0.0..=1.0]` range places the content outside of the container bounds. A **non-finite
/// value** means the content stretches to fill the container bounds.
#[derive(Clone, Copy)]
pub struct Alignment {
    /// *x* alignment in a `[0.0..=1.0]` range.
    pub x: Factor,
    /// *y* alignment in a `[0.0..=1.0]` range.
    pub y: Factor,
}
impl PartialEq for Alignment {
    fn eq(&self, other: &Self) -> bool {
        self.fill_width() == other.fill_width() && self.fill_height() == other.fill_height() && self.x == other.x && self.y == other.y
    }
}
impl Alignment {
    /// Returns `true` if [`x`] is a special value that indicates the content width must be the container width.
    ///
    /// [`x`]: Alignment::x
    pub fn fill_width(self) -> bool {
        !self.x.0.is_finite()
    }

    /// Returns `true` if [`y`] is a special value that indicates the content height must be the container height.
    ///
    /// [`y`]: Alignment::y
    pub fn fill_height(self) -> bool {
        !self.y.0.is_finite()
    }
}
impl_from_and_into_var! {
    fn from<X: Into<Factor> + Clone, Y: Into<Factor> + Clone>((x, y): (X, Y)) -> Alignment {
        Alignment { x: x.into(), y: y.into() }
    }

    fn from(xy: Factor) -> Alignment {
        Alignment { x: xy, y: xy }
    }

    fn from(xy: FactorPercent) -> Alignment {
        xy.as_normal().into()
    }
}
macro_rules! named_aligns {
    ( $($NAME:ident = ($x:expr, $y:expr);)+ ) => {named_aligns!{$(
        [stringify!(($x, $y))] $NAME = ($x, $y);
    )+}};

    ( $([$doc:expr] $NAME:ident = ($x:expr, $y:expr);)+ ) => {
        $(
        #[doc=$doc]
        pub const $NAME: Alignment = Alignment { x: Factor($x), y: Factor($y) };
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
    };
}
impl fmt::Debug for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            if f.alternate() {
                write!(f, "Alignment::{name}")
            } else {
                f.write_str(name)
            }
        } else {
            f.debug_struct("Alignment").field("x", &self.x).field("y", &self.y).finish()
        }
    }
}
impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            f.write_str(name)
        } else {
            f.write_char('(')?;
            if self.fill_width() {
                f.write_str("<fill>")?;
            } else {
                write!(f, "{}", FactorPercent::from(self.x))?;
            }
            f.write_str(", ")?;
            if self.fill_height() {
                f.write_str("<fill>")?;
            } else {
                write!(f, "{}", FactorPercent::from(self.x))?;
            }
            f.write_char(')')
        }
    }
}
impl Alignment {
    named_aligns! {
        TOP_LEFT = (0.0, 0.0);
        BOTTOM_LEFT = (0.0, 1.0);

        TOP_RIGHT = (1.0, 0.0);
        BOTTOM_RIGHT = (1.0, 1.0);

        LEFT = (0.0, 0.5);
        RIGHT = (1.0, 0.5);
        TOP = (0.5, 0.0);
        BOTTOM = (0.5, 1.0);

        CENTER = (0.5, 0.5);

        FILL_TOP = (f32::NAN, 0.0);
        FILL_BOTTOM = (f32::NAN, 1.0);
        FILL_RIGHT = (1.0, f32::NAN);
        FILL_LEFT = (0.0, f32::NAN);

        FILL = (f32::NAN, f32::NAN);
    }
}
impl_from_and_into_var! {
     /// To relative length x and y.
    fn from(alignment: Alignment) -> Point {
        Point {
            x: alignment.x.into(),
            y: alignment.y.into(),
        }
    }
}
impl Alignment {
    /// Compute a content rectangle given this alignment, the content size and the available size.
    ///
    /// To implement alignment, the `content_size` should be measured and recorded in [`UiNode::measure`]
    /// and then this method called in the [`UiNode::arrange`] with the final container size to get the
    /// content rectangle that must be recorded and used in [`UiNode::render`] to size and position the content
    /// in the space of the container.
    ///
    /// [`UiNode::measure`]: crate::UiNode::measure
    /// [`UiNode::arrange`]: crate::UiNode::arrange
    /// [`UiNode::render`]: crate::UiNode::render
    pub fn solve(self, content_size: PxSize, container_size: PxSize) -> PxRect {
        let mut r = PxRect::zero();

        if self.fill_width() {
            r.size.width = container_size.width;
        } else {
            r.size.width = container_size.width.min(content_size.width);
            r.origin.x = (container_size.width - r.size.width) * self.x.0;
        }
        if self.fill_height() {
            r.size.height = container_size.height;
        } else {
            r.size.height = container_size.height.min(content_size.height);
            r.origin.y = (container_size.height - r.size.height) * self.y.0;
        }

        r
    }

    /// Compute an offset to apply to the content given the available size.
    ///
    /// [`FILL`] align resolves like [`TOP_LEFT`] align.
    ///
    /// Unlike [`solve`] the content does not change size, it must be clipped if larger than the container.
    ///
    /// [`FILL`]: Alignment::FILL
    /// [`TOP_LEFT`]: Alignment::TOP_LEFT
    /// [`solve`]: Alignment::solve
    pub fn solve_offset(self, content_size: PxSize, container_size: PxSize) -> PxVector {
        let mut r = PxVector::zero();

        if !self.fill_width() {
            r.x = (container_size.width - content_size.width) * self.x.0;
        }

        if !self.fill_height() {
            r.y = (container_size.height - content_size.height) * self.y.0;
        }

        r
    }
}
