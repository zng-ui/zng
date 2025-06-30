//! Color filter effect.

use std::fmt;

use zng_layout::{
    context::LayoutMask,
    unit::{AngleDegree, EQ_GRANULARITY, Factor, FactorUnits, Layout1d, Layout2d, Length, Point, about_eq, about_eq_hash},
};
use zng_var::{
    animation::{Transitionable, easing::EasingStep},
    impl_from_and_into_var,
};
use zng_view_api::display_list::{FilterOp, FrameValue};

use crate::{Rgba, lerp_rgba};

/// A color filter or combination of filters.
///
/// # Examples
///
/// ```
/// use zng_color::filter::Filter;
/// use zng_layout::unit::*;
///
/// let filter = Filter::new_opacity(50.pct()).blur(3);
/// ```
///
/// The example above creates a filter that lowers the opacity to `50%` and blurs by `3px`.
#[derive(Clone, Default, PartialEq)]
pub struct Filter {
    filters: Vec<FilterData>,
    needs_layout: bool,
}

impl Filter {
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
    ///
    /// [`Length`]: zng_layout::unit::Length
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
    /// [`hue`]: crate::Hsla::hue
    pub fn hue_rotate<A: Into<AngleDegree>>(self, angle: A) -> Self {
        self.op(FilterOp::HueRotate(angle.into().0))
    }

    /// Add a filter that fills the pixel space with `color`.
    pub fn flood<C: Into<Rgba>>(self, color: C) -> Self {
        self.op(FilterOp::Flood(color.into()))
    }

    /// Custom filter.
    ///
    /// The color matrix is in the format of SVG color matrix, [0..5] is the first matrix row.
    pub fn color_matrix<M: Into<ColorMatrix>>(self, matrix: M) -> Self {
        self.op(FilterOp::ColorMatrix(matrix.into().0))
    }
}

impl Filter {
    /// New default empty.
    pub const fn new() -> Filter {
        Self {
            filters: vec![],
            needs_layout: false,
        }
    }

    /// New [`Filter::opacity`].
    pub fn new_opacity<A: Into<Factor>>(alpha: A) -> Filter {
        Filter::default().opacity(alpha)
    }
    /// New [`Filter::invert`].
    pub fn new_invert<A: Into<Factor>>(amount: A) -> Filter {
        Filter::default().invert(amount)
    }
    /// New [`Filter::blur`].
    pub fn new_blur<R: Into<Length>>(radius: R) -> Filter {
        Filter::default().blur(radius)
    }
    /// New [`Filter::sepia`].
    pub fn new_sepia<A: Into<Factor>>(amount: A) -> Filter {
        Filter::default().sepia(amount)
    }
    /// New [`Filter::grayscale`].
    pub fn new_grayscale<A: Into<Factor>>(amount: A) -> Filter {
        Filter::default().grayscale(amount)
    }
    /// New [`Filter::drop_shadow`].
    pub fn new_drop_shadow<O: Into<Point>, R: Into<Length>, C: Into<Rgba>>(offset: O, blur_radius: R, color: C) -> Filter {
        Filter::default().drop_shadow(offset, blur_radius, color)
    }
    /// New [`Filter::brightness`].
    pub fn new_brightness<A: Into<Factor>>(amount: A) -> Filter {
        Filter::default().brightness(amount)
    }
    /// New [`Filter::contrast`].
    pub fn new_contrast<A: Into<Factor>>(amount: A) -> Filter {
        Filter::default().contrast(amount)
    }
    /// New [`Filter::saturate`].
    pub fn new_saturate<A: Into<Factor>>(amount: A) -> Filter {
        Filter::default().saturate(amount)
    }
    /// New [`Filter::hue_rotate`].
    pub fn new_hue_rotate<A: Into<AngleDegree>>(angle: A) -> Filter {
        Filter::default().hue_rotate(angle)
    }
    /// New [`Filter::flood`].
    pub fn new_flood<C: Into<Rgba>>(color: C) -> Filter {
        Filter::default().flood(color)
    }
    /// New [`Filter::color_matrix`].
    pub fn new_color_matrix<M: Into<ColorMatrix>>(matrix: M) -> Filter {
        Filter::default().color_matrix(matrix)
    }
}

impl Filter {
    fn op(mut self, op: FilterOp) -> Self {
        self.filters.push(FilterData::Op(op));
        self
    }

    /// Compute a [`RenderFilter`] in the current [`LAYOUT`] context.
    ///
    /// Most filters convert one-to-one, effects that have a [`Length`] value use the
    /// layout context to calculate relative values.
    ///
    /// Relative blur radius lengths are calculated using the `constraints().fill_size().width` value.
    ///
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    /// [`Length`]: zng_layout::unit::Length
    pub fn layout(&self) -> RenderFilter {
        self.filters
            .iter()
            .map(|f| match f {
                FilterData::Op(op) => *op,
                FilterData::Blur(l) => {
                    let l = l.layout_f32_x();
                    FilterOp::Blur(l, l)
                }
                FilterData::DropShadow {
                    offset,
                    blur_radius,
                    color,
                } => FilterOp::DropShadow {
                    offset: offset.layout().to_vector().cast(),
                    color: *color,
                    blur_radius: blur_radius.layout_f32_x(),
                },
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
impl Layout2d for Filter {
    type Px = RenderFilter;

    fn layout_dft(&self, _: Self::Px) -> Self::Px {
        self.layout()
    }

    fn affect_mask(&self) -> LayoutMask {
        let mut mask = LayoutMask::empty();
        for f in &self.filters {
            match f {
                FilterData::Op(_) => {}
                FilterData::Blur(l) => mask |= l.affect_mask(),
                FilterData::DropShadow { offset, blur_radius, .. } => {
                    mask |= offset.affect_mask();
                    mask |= blur_radius.affect_mask();
                }
            }
        }
        mask
    }
}

/// A computed [`Filter`], ready for Webrender.
pub type RenderFilter = Vec<FilterOp>;

#[derive(Clone, PartialEq)]
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
                    FilterOp::DropShadow {
                        offset,
                        color,
                        blur_radius,
                    } => write!(f, "drop_shadow(({}, {}), {}, {})", offset.x, offset.y, blur_radius, *color),
                    FilterOp::ColorMatrix(m) => write!(f, "color_matrix({:?})", ColorMatrix(*m)),
                    FilterOp::Flood(c) => write!(f, "flood({})", *c),
                    w => write!(f, "{w:?}"),
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

fn lerp_frame_value<T: Transitionable>(s: FrameValue<T>, to: &FrameValue<T>, step: EasingStep) -> FrameValue<T> {
    let mut bind_data = None;
    let mut value = match s {
        FrameValue::Bind { id, value, animating } => {
            bind_data = Some((id, animating));
            value
        }
        FrameValue::Value(v) => v,
        _ => return to.clone(),
    };

    value = value.lerp(to.value(), step);

    if step < 1.fct() {
        if let Some((id, animating)) = bind_data {
            FrameValue::Bind { id, value, animating }
        } else {
            FrameValue::Value(value)
        }
    } else {
        match to {
            FrameValue::Bind { id, animating, .. } => FrameValue::Bind {
                id: *id,
                value,
                animating: *animating,
            },
            FrameValue::Value(_) => FrameValue::Value(value),
            _ => to.clone(),
        }
    }
}

fn lerp_filter_op(mut s: FilterOp, to: &FilterOp, step: EasingStep) -> FilterOp {
    match (&mut s, to) {
        (FilterOp::Blur(x, y), FilterOp::Blur(xb, yb)) => {
            *x = x.lerp(xb, step);
            *y = y.lerp(yb, step);
        }
        (FilterOp::Brightness(a), FilterOp::Brightness(b)) => {
            *a = a.lerp(b, step);
        }
        (FilterOp::Contrast(a), FilterOp::Contrast(b)) => {
            *a = a.lerp(b, step);
        }
        (FilterOp::Grayscale(a), FilterOp::Grayscale(b)) => {
            *a = a.lerp(b, step);
        }
        (FilterOp::HueRotate(a), FilterOp::HueRotate(b)) => {
            *a = a.lerp(b, step);
        }
        (FilterOp::Invert(a), FilterOp::Invert(b)) => {
            *a = a.lerp(b, step);
        }
        (FilterOp::Opacity(a), FilterOp::Opacity(b)) => {
            *a = lerp_frame_value(*a, b, step);
        }
        (FilterOp::Saturate(a), FilterOp::Saturate(b)) => {
            *a = a.lerp(b, step);
        }
        (FilterOp::Sepia(a), FilterOp::Sepia(b)) => {
            *a = a.lerp(b, step);
        }
        (
            FilterOp::DropShadow {
                offset,
                color,
                blur_radius,
            },
            FilterOp::DropShadow {
                offset: to_offset,
                color: to_color,
                blur_radius: to_blur,
            },
        ) => {
            *offset = Transitionable::lerp(*offset, to_offset, step);
            *color = lerp_rgba(*color, *to_color, step);
            *blur_radius = blur_radius.lerp(to_blur, step);
        }
        (FilterOp::ColorMatrix(a), FilterOp::ColorMatrix(b)) => {
            for (a, b) in a.iter_mut().zip(b) {
                *a = a.lerp(b, step);
            }
        }
        (FilterOp::Flood(a), FilterOp::Flood(b)) => {
            *a = lerp_rgba(*a, *b, step);
        }
        (a, b) => {
            if step >= 1.fct() {
                *a = *b
            }
        }
    }
    s
}

impl Transitionable for Filter {
    fn lerp(mut self, to: &Self, step: EasingStep) -> Self {
        let end = step >= 1.fct();

        for z in self.filters.iter_mut().zip(&to.filters) {
            match z {
                (FilterData::Op(a), FilterData::Op(b)) => *a = lerp_filter_op(*a, b, step),
                (FilterData::Blur(a), FilterData::Blur(b)) => {
                    *a = a.clone().lerp(b, step);
                }
                (
                    FilterData::DropShadow {
                        offset,
                        blur_radius,
                        color,
                    },
                    FilterData::DropShadow {
                        offset: offset_b,
                        blur_radius: blur_b,
                        color: color_b,
                    },
                ) => {
                    *offset = offset.clone().lerp(offset_b, step);
                    *blur_radius = blur_radius.clone().lerp(blur_b, step);
                    *color = color.lerp(color_b, step);
                }
                (a, b) => {
                    if end {
                        *a = b.clone();
                    }
                }
            }
        }

        if end {
            match self.filters.len().cmp(&to.filters.len()) {
                std::cmp::Ordering::Less => self.filters.truncate(to.filters.len()),
                std::cmp::Ordering::Greater => self.filters.extend(to.filters[self.filters.len()..].iter().cloned()),
                std::cmp::Ordering::Equal => {}
            }

            self.needs_layout = to.needs_layout;
        }

        self
    }
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
            1.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0,
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
                    write!(f, "{v:.3}, ")?;
                } else {
                    write!(f, " {v:.3}, ")?;
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
        self.0.iter().zip(&other.0).all(|(&a, &b)| about_eq(a, b, EQ_GRANULARITY))
    }
}
impl Eq for ColorMatrix { }
impl std::hash::Hash for ColorMatrix {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for f in self.0 {
            about_eq_hash(f, EQ_GRANULARITY, state);
        }
    }
}
impl Transitionable for ColorMatrix {
    fn lerp(mut self, to: &Self, step: EasingStep) -> Self {
        for (a, b) in self.0.iter_mut().zip(&to.0) {
            *a = a.lerp(b, step);
        }
        self
    }
}
