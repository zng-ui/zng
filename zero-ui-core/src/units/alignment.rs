use std::{fmt::{self, Write}, ops};

use crate::{impl_from_and_into_var, widget_info::WidgetLayoutTranslation};

use super::{Factor, FactorPercent, Point, Px, PxConstrains2d, PxSize, PxVector, FactorUnits, Factor2d};

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
/// The [`f32::INFINITY`] value can be used in ***x*** or ***y*** to indicate that the content should *fill* the available space.
///
/// The [`f32::NEG_INFINITY`] value can be used in ***y*** to indicate that a panel widget should align its items by each *baseline*,
/// for most widgets this is the same as `BOTTOM`, but for texts this aligns to the baseline of the texts.
///
/// You can use the [`is_fill_x`], [`is_fill_y`] and [`is_baseline`] methods to probe for this special values.
///
/// [`is_fill_x`]: Align::is_fill_x
/// [`is_fill_y`]: Align::is_fill_y
/// [`is_baseline`]: Align::is_baseline
/// [`as_self_align`]: Align::as_self_align
#[derive(Clone, Copy)]
pub struct Align {
    /// *x* alignment in a `[0.0..=1.0]` range.
    pub x: Factor,
    /// *y* alignment in a `[0.0..=1.0]` range.
    pub y: Factor,
}
impl PartialEq for Align {
    fn eq(&self, other: &Self) -> bool {
        self.is_fill_x() == other.is_fill_x() && self.is_fill_y() == other.is_fill_y() && self.x == other.x && self.y == other.y
    }
}
impl Align {
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
    /// If this is `true` the *y* alignment should be `BOTTOM` plus the baseline offset.
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

    /// Constrains that must be used to layout a child node with the alignment.
    pub fn child_constrains(self, parent_constrains: PxConstrains2d) -> PxConstrains2d {
        parent_constrains
            .with_new_min(Px(0), Px(0))
            .with_fill_and(self.is_fill_x(), self.is_fill_y())
    }

    /// Computes the size returned by [`layout`] for the given child size and constrains.
    ///
    /// [`layout`]: Self::layout
    pub fn measure(self, child_size: PxSize, parent_constrains: PxConstrains2d) -> PxSize {
        let size = parent_constrains.fill_size().max(child_size);
        parent_constrains.clamp_size(size)
    }

    /// Applies the alignment transform to `wl` and returns the size of the parent align node.
    pub fn layout(self, child_size: PxSize, parent_constrains: PxConstrains2d, wl: &mut WidgetLayoutTranslation) -> PxSize {
        let size = parent_constrains.fill_size().max(child_size);
        let size = parent_constrains.clamp_size(size);

        let mut offset = PxVector::zero();
        if !self.is_fill_x() {
            offset.x = (size.width - child_size.width) * self.x.0;
        }

        let baseline = self.is_baseline();

        if !self.is_fill_y() {
            let y = if baseline { 1.0 } else { self.y.0 };

            offset.y = (size.height - child_size.height) * y;
        }

        wl.translate(offset);

        if baseline {
            wl.translate_baseline(true);
        }

        size
    }
}
impl_from_and_into_var! {
    fn from<X: Into<Factor>, Y: Into<Factor>>((x, y): (X, Y)) -> Align {
        Align { x: x.into(), y: y.into() }
    }

    fn from(xy: Factor) -> Align {
        Align { x: xy, y: xy }
    }

    fn from(xy: FactorPercent) -> Align {
        xy.fct().into()
    }
}
macro_rules! named_aligns {
    ( $($NAME:ident = ($x:expr, $y:expr);)+ ) => {named_aligns!{$(
        [stringify!(($x, $y))] $NAME = ($x, $y);
    )+}};

    ( $([$doc:expr] $NAME:ident = ($x:expr, $y:expr);)+ ) => {
        $(
        #[doc=$doc]
        pub const $NAME: Align = Align { x: Factor($x), y: Factor($y) };
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
impl fmt::Debug for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            if f.alternate() {
                write!(f, "Align::{name}")
            } else {
                f.write_str(name)
            }
        } else {
            f.debug_struct("Align").field("x", &self.x).field("y", &self.y).finish()
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
                f.write_str("<fill>")?;
            } else {
                write!(f, "{}", FactorPercent::from(self.x))?;
            }
            f.write_str(", ")?;
            if self.is_fill_y() {
                f.write_str("<fill>")?;
            } else if self.is_baseline() {
                f.write_str("<baseline>")?;
            } else {
                write!(f, "{}", FactorPercent::from(self.x))?;
            }
            f.write_char(')')
        }
    }
}
impl Align {
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

        FILL_TOP = (f32::INFINITY, 0.0);
        FILL_BOTTOM = (f32::INFINITY, 1.0);
        FILL_RIGHT = (1.0, f32::INFINITY);
        FILL_LEFT = (0.0, f32::INFINITY);

        FILL_X = (f32::INFINITY, 0.5);
        FILL_Y = (0.5, f32::INFINITY);

        FILL = (f32::INFINITY, f32::INFINITY);

        BASELINE_LEFT = (0.0, f32::NEG_INFINITY);
        BASELINE_CENTER = (0.5, f32::NEG_INFINITY);
        BASELINE_RIGHT = (1.0, f32::NEG_INFINITY);

        BASELINE = (f32::INFINITY, f32::NEG_INFINITY);
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
            y: factor2d.y,
        }
    }
}

impl ops::Add for Align {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}
impl ops::AddAssign for Align {
    fn add_assign(&mut self, rhs: Self) {
        if self.is_fill_x() || rhs.is_fill_x() {
            self.x = f32::INFINITY.fct();
        } else {
            self.x += rhs.x;
        }

        if self.is_baseline() || rhs.is_baseline() {
            self.y = f32::NEG_INFINITY.fct();
        } else {
            self.y += rhs.y;
        }
    }
}
impl ops::Sub for Align {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}
impl ops::SubAssign for Align {
    fn sub_assign(&mut self, rhs: Self) {
        if self.is_fill_x() {
            if rhs.is_fill_x() {
                self.x = 0.fct();
            }
            // else stays infinite.
        } else if rhs.is_fill_x() {
            self.x = 0.fct();
        } else {
            self.x -= rhs.x;
        }

        if self.is_baseline() || rhs.is_baseline() {
            // baseline is an special value.
            self.y = f32::NEG_INFINITY.fct();
        } else {
            self.y -= rhs.y;
        }
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
        let rhs : Self = rhs.into().into();
        
        if self.is_fill_x() {
            if rhs.is_fill_x() {
                self.x = 0.fct();
            }
            // else stays infinite.
        } else if rhs.is_fill_x() {
            self.x = 0.fct();
        } else {
            self.x *= rhs.x;
        }

        if self.is_baseline() || rhs.is_baseline() {
            // baseline is an special value.
            self.y = f32::NEG_INFINITY.fct();
        } else {
            self.y *= rhs.y;
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
        let rhs : Self = rhs.into().into();
        
        if self.is_fill_x() {
            if rhs.is_fill_x() {
                self.x = 0.fct();
            }
            // else stays infinite.
        } else if rhs.is_fill_x() {
            self.x = 0.fct();
        } else {
            self.x /= rhs.x;
        }

        if self.is_baseline() || rhs.is_baseline() {
            // baseline is an special value.
            self.y = f32::NEG_INFINITY.fct();
        } else {
            self.y /= rhs.y;
        }
    }
}