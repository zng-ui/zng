//! Color filter effect.

use std::fmt;

use crate::{
    color::{RenderColor, Rgba},
    context::LayoutMetrics,
    impl_from_and_into_var,
    render::{webrender_api as wr, FilterOp, FrameValue},
    units::*,
};

/// A color filter or combination of filters.
///
/// You can start a filter from one of the standalone filter functions, and then combine more filters using
/// the builder call style.
///
/// The standalone filter functions are all in the [`color`](crate::color) module and have the same name
/// as methods of this type.
///
/// # Examples
///
/// ```
/// use zero_ui_core::color::filters;
/// use zero_ui_core::units::*;
///
/// let filter = filters::opacity(50.pct()).blur(3);
/// ```
///
/// The example above creates a filter that lowers the opacity to `50%` and blurs by `3px`.
#[derive(Clone, Default)]
pub struct Filter {
    filters: Vec<FilterData>,
    needs_layout: bool,
}
impl Filter {
    fn op(mut self, op: FilterOp) -> Self {
        self.filters.push(FilterData::Op(op));
        self
    }

    /// Compute a [`RenderFilter`].
    ///
    /// Most filters convert one-to-one, effects that have a [`Length`] value use the
    /// layout context to calculate relative values.
    ///
    /// Relative blur radius lengths are calculated using the `constrains().fill_size().width` value.
    pub fn layout(&self, ctx: &LayoutMetrics) -> RenderFilter {
        self.filters
            .iter()
            .map(|f| match f {
                FilterData::Op(op) => *op,
                FilterData::Blur(l) => {
                    let l = l.layout(ctx.for_x(), |_| Px(0)).0 as f32;
                    FilterOp::Blur(l, l)
                }
                FilterData::DropShadow {
                    offset,
                    blur_radius,
                    color,
                } => FilterOp::DropShadow(wr::Shadow {
                    offset: offset.layout(ctx, |_| PxPoint::zero()).to_wr().to_vector(),
                    color: RenderColor::from(*color),
                    blur_radius: blur_radius.layout(ctx.for_x(), |_| Px(0)).0 as f32,
                }),
            })
            .collect()
    }

    /// Compute a [`RenderFilter`] if the filter is not affected by layout.
    pub fn try_render(&self) -> Option<RenderFilter> {
        if self.needs_layout {
            return None;
        }

        let mut r = Vec::with_capacity(self.filters.len());

        for f in &self.filters {
            match f {
                FilterData::Op(op) => r.push(*op),
                FilterData::Blur(_) | FilterData::DropShadow { .. } => unreachable!(),
            }
        }

        Some(r)
    }

    /// Returns `true` if this filter is affected by the layout context where it is evaluated.
    pub fn needs_layout(&self) -> bool {
        self.needs_layout
    }

    /// Add an opacity adjustment to the filter, zero is fully transparent, one is the input transparency.
    pub fn opacity<A: Into<Factor>>(self, alpha: A) -> Self {
        let alpha_value = alpha.into().0;
        self.op(FilterOp::Opacity(FrameValue::Value(alpha_value)))
    }

    /// Add a color inversion filter, zero does not invert, one fully inverts.
    pub fn invert<A: Into<Factor>>(self, amount: A) -> Self {
        self.op(FilterOp::Invert(amount.into().0))
    }

    /// Add a blue effect to the filter, the blue `radius` is defined by a [`Length`].
    ///
    /// Relative lengths are calculated by the width of the available space.
    pub fn blur<R: Into<Length>>(mut self, radius: R) -> Self {
        self.needs_layout = true;
        self.filters.push(FilterData::Blur(radius.into()));
        self
    }

    /// Add a sepia color effect to the filter, zero is the input color, one is the full desaturated brown look.
    pub fn sepia<A: Into<Factor>>(self, amount: A) -> Self {
        self.op(FilterOp::Sepia(amount.into().0))
    }

    /// Add a grayscale color effect to the filter, zero is the input color, one if the full grayscale.
    pub fn grayscale<A: Into<Factor>>(self, amount: A) -> Self {
        self.op(FilterOp::Grayscale(amount.into().0))
    }

    /// Add a drop-shadow to the effect.
    pub fn drop_shadow<O: Into<Point>, R: Into<Length>, C: Into<Rgba>>(mut self, offset: O, blur_radius: R, color: C) -> Self {
        self.needs_layout = true;
        self.filters.push(FilterData::DropShadow {
            offset: offset.into(),
            blur_radius: blur_radius.into(),
            color: color.into(),
        });
        self
    }

    /// Add a brightness adjustment to the filter, zero removes all brightness, one is the input brightness.
    pub fn brightness<A: Into<Factor>>(self, amount: A) -> Self {
        self.op(FilterOp::Brightness(amount.into().0))
    }

    /// Add a contrast adjustment to the filter, zero removes all contrast, one is the input contrast.
    pub fn contrast<A: Into<Factor>>(self, amount: A) -> Self {
        self.op(FilterOp::Contrast(amount.into().0))
    }

    /// Add a saturation adjustment to the filter, zero fully desaturates, one is the input saturation.
    pub fn saturate<A: Into<Factor>>(self, amount: A) -> Self {
        self.op(FilterOp::Saturate(amount.into().0))
    }

    /// Add a filter that adds the `angle` to each color [`hue`] value.
    ///
    /// [`hue`]: crate::color::Hsla::hue
    pub fn hue_rotate<A: Into<AngleDegree>>(self, angle: A) -> Self {
        self.op(FilterOp::HueRotate(angle.into().0))
    }

    /// Add a filter that fills the pixel space with `color`.
    pub fn flood<C: Into<Rgba>>(self, color: C) -> Self {
        self.op(FilterOp::Flood(color.into().into()))
    }

    /// Custom filter.
    ///
    /// The color matrix is in the format of SVG color matrix, [0..5] is the first matrix row.
    pub fn color_matrix<M: Into<ColorMatrix>>(self, matrix: M) -> Self {
        self.op(FilterOp::ColorMatrix(matrix.into().0))
    }
}
impl fmt::Debug for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("Filter").field(&self.filters).finish()
        } else if self.filters.is_empty() {
            write!(f, "[]")
        } else {
            write!(f, "{:?}", self.filters[0])?;
            for filter in &self.filters[1..] {
                write!(f, ".{filter:?}")?;
            }
            Ok(())
        }
    }
}

/// A computed [`Filter`], ready for Webrender.
pub type RenderFilter = Vec<FilterOp>;

#[derive(Clone)]
enum FilterData {
    Op(FilterOp),
    Blur(Length),
    DropShadow { offset: Point, blur_radius: Length, color: Rgba },
}
impl fmt::Debug for FilterData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "FilterData::")?;
            match self {
                FilterData::Op(op) => f.debug_tuple("Op").field(op).finish(),
                FilterData::Blur(l) => f.debug_tuple("Blur").field(l).finish(),
                FilterData::DropShadow {
                    offset,
                    blur_radius,
                    color,
                } => f
                    .debug_struct("DropShadow")
                    .field("offset", offset)
                    .field("blur_radius", blur_radius)
                    .field("color", color)
                    .finish(),
            }
        } else {
            fn bool_or_pct(func: &'static str, value: f32, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if value.abs() < 0.0001 {
                    write!(f, "{func}(false)")
                } else if (value - 1.0).abs() < 0.0001 {
                    write!(f, "{func}(true)")
                } else {
                    write!(f, "{func}({}.pct())", value * 100.0)
                }
            }
            match self {
                FilterData::Op(op) => match op {
                    FilterOp::Blur(w, _) => write!(f, "blur({w})"),
                    FilterOp::Brightness(b) => write!(f, "brightness({}.pct())", b * 100.0),
                    FilterOp::Contrast(c) => write!(f, "brightness({}.pct())", c * 100.0),
                    FilterOp::Grayscale(c) => bool_or_pct("grayscale", *c, f),
                    FilterOp::HueRotate(d) => write!(f, "hue_rotate({d}.deg())"),
                    FilterOp::Invert(i) => bool_or_pct("invert", *i, f),
                    FilterOp::Opacity(o) => write!(f, "opacity({}.pct())", *o.value() * 100.0),
                    FilterOp::Saturate(s) => write!(f, "saturate({}.pct())", s * 100.0),
                    FilterOp::Sepia(s) => bool_or_pct("sepia", *s, f),
                    FilterOp::DropShadow(s) => write!(
                        f,
                        "drop_shadow(({}, {}), {}, {})",
                        s.offset.x,
                        s.offset.y,
                        s.blur_radius,
                        Rgba::from(s.color)
                    ),
                    FilterOp::ColorMatrix(m) => write!(f, "color_matrix({:?})", ColorMatrix(*m)),
                    FilterOp::Flood(c) => write!(f, "flood({})", Rgba::from(*c)),
                },
                FilterData::Blur(l) => write!(f, "blur({l:?})"),
                FilterData::DropShadow {
                    offset,
                    blur_radius,
                    color,
                } => {
                    write!(f, "drop_shadow({offset:?}, {blur_radius:?}, {color:?})")
                }
            }
        }
    }
}

/// New [`Filter::opacity`].
pub fn opacity<A: Into<Factor>>(alpha: A) -> Filter {
    Filter::default().opacity(alpha)
}
/// New [`Filter::invert`].
pub fn invert<A: Into<Factor>>(amount: A) -> Filter {
    Filter::default().invert(amount)
}
/// New [`Filter::blur`].
pub fn blur<R: Into<Length>>(radius: R) -> Filter {
    Filter::default().blur(radius)
}
/// New [`Filter::sepia`].
pub fn sepia<A: Into<Factor>>(amount: A) -> Filter {
    Filter::default().sepia(amount)
}
/// New [`Filter::grayscale`].
pub fn grayscale<A: Into<Factor>>(amount: A) -> Filter {
    Filter::default().grayscale(amount)
}
/// New [`Filter::drop_shadow`].
pub fn drop_shadow<O: Into<Point>, R: Into<Length>, C: Into<Rgba>>(offset: O, blur_radius: R, color: C) -> Filter {
    Filter::default().drop_shadow(offset, blur_radius, color)
}
/// New [`Filter::brightness`].
pub fn brightness<A: Into<Factor>>(amount: A) -> Filter {
    Filter::default().brightness(amount)
}
/// New [`Filter::contrast`].
pub fn contrast<A: Into<Factor>>(amount: A) -> Filter {
    Filter::default().contrast(amount)
}
/// New [`Filter::saturate`].
pub fn saturate<A: Into<Factor>>(amount: A) -> Filter {
    Filter::default().saturate(amount)
}
/// New [`Filter::hue_rotate`].
pub fn hue_rotate<A: Into<AngleDegree>>(angle: A) -> Filter {
    Filter::default().hue_rotate(angle)
}
/// New [`Filter::flood`].
pub fn flood<C: Into<Rgba>>(color: C) -> Filter {
    Filter::default().flood(color)
}
/// New [`Filter::color_matrix`].
pub fn color_matrix<M: Into<ColorMatrix>>(matrix: M) -> Filter {
    Filter::default().color_matrix(matrix)
}

/// Represents a custom color filter.
///
/// The color matrix is in the format of SVG color matrix, [0..5] is the first matrix row.
#[derive(Clone, Copy)]
pub struct ColorMatrix(pub [f32; 20]);
impl ColorMatrix {
    /// Matrix that does not alter any color.
    #[rustfmt::skip]
    pub const fn identity() -> Self {
        ColorMatrix([
            1.0,  0.0,  0.0,  0.0,  0.0,
            0.0,  1.0,  0.0,  0.0,  0.0,
            0.0,  0.0,  1.0,  0.0,  0.0,
            0.0,  0.0,  0.0,  1.0,  0.0,
        ])
    }
}
impl fmt::Debug for ColorMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self == &ColorMatrix::identity() {
            if f.alternate() {
                write!(f, "ColorMatrix::")?;
            }
            return write!(f, "identity()");
        }

        if f.alternate() {
            write!(f, "ColorMatrix(")?;
        }
        writeln!(f, "[")?;
        for row in 0..4 {
            write!(f, "    ")?;
            for column in 0..5 {
                let v = self.0[row * column];
                if v < 0.0 {
                    write!(f, "{:.3}, ", v)?;
                } else {
                    write!(f, " {:.3}, ", v)?;
                }
            }
            writeln!(f)?;
        }
        write!(f, "]")?;
        if f.alternate() {
            write!(f, ")")?;
        }
        Ok(())
    }
}
impl_from_and_into_var! {
    fn from(matrix: [f32; 20]) -> ColorMatrix {
        ColorMatrix(matrix)
    }
}
impl Default for ColorMatrix {
    fn default() -> Self {
        Self::identity()
    }
}
impl PartialEq for ColorMatrix {
    fn eq(&self, other: &Self) -> bool {
        self.0.iter().zip(&other.0).all(|(&a, &b)| about_eq(a, b, 0.00001))
    }
}
impl std::hash::Hash for ColorMatrix {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for f in self.0 {
            about_eq_hash(f, 0.00001, state);
        }
    }
}
