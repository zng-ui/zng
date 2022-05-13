//! Gradient types.

use std::{fmt, ops::Range};

use crate::color::*;
use crate::context::*;
use crate::render::webrender_api::{self as wr, euclid};
use crate::units::*;
use crate::var::*;

/// Specifies how to draw the gradient outside the first and last stop.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtendMode {
    /// Default value. The color values at the ends of the gradient vector fill the remaining space.
    Clamp,
    /// The gradient is repeated until the space is filled.
    Repeat,
    /// The gradient is repeated alternating direction until the space is filled.
    Reflect,
}
impl fmt::Debug for ExtendMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ExtendMode::")?;
        }
        match self {
            ExtendMode::Clamp => write!(f, "Clamp"),
            ExtendMode::Repeat => write!(f, "Repeat"),
            ExtendMode::Reflect => write!(f, "Reflect"),
        }
    }
}
impl From<ExtendMode> for RenderExtendMode {
    /// `Reflect` is converted to `Repeat`, you need to prepare the color stops to repeat *reflecting*.
    fn from(mode: ExtendMode) -> Self {
        match mode {
            ExtendMode::Clamp => RenderExtendMode::Clamp,
            ExtendMode::Repeat => RenderExtendMode::Repeat,
            ExtendMode::Reflect => RenderExtendMode::Repeat,
        }
    }
}

/// Gradient extend mode supported by the render.
///
/// Note that [`ExtendMode::Reflect`](crate::gradient::ExtendMode::Reflect) is not supported
/// directly, you must duplicate and mirror the stops and use the `Repeat` render mode.
pub type RenderExtendMode = crate::render::webrender_api::ExtendMode;

/// The [angle](AngleUnits) or [line](crate::units::Line) that defines a linear gradient.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::units::*;
/// # use zero_ui_core::color::colors;
/// # use zero_ui_core::gradient::*;
/// # fn linear_gradient(axis: impl Into<LinearGradientAxis>, stops: impl Into<GradientStops>) { /* TODO move gradient nodes to core? */ }
/// let angle_gradient = linear_gradient(90.deg(), [colors::BLACK, colors::WHITE]);
/// let line_gradient = linear_gradient((0, 0).to(50, 30), [colors::BLACK, colors::WHITE]);
/// ```
#[derive(Clone, PartialEq)]
pub enum LinearGradientAxis {
    /// Line defined by an angle. 0º is a line from bottom-to-top, 90º is a line from left-to-right.
    ///
    /// The line end-points are calculated so that the full gradient is visible from corner-to-corner, this is
    /// sometimes called *magic corners*.
    Angle(AngleRadian),

    /// Line defined by two points. If the points are inside the fill area the gradient is extended-out in the
    /// same direction defined by the line, according to the extend mode.
    Line(Line),
}
impl fmt::Debug for LinearGradientAxis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                LinearGradientAxis::Angle(a) => f.debug_tuple("LinearGradientAxis::Angle").field(a).finish(),
                LinearGradientAxis::Line(l) => f.debug_tuple("LinearGradientAxis::Line").field(l).finish(),
            }
        } else {
            match self {
                LinearGradientAxis::Angle(a) => write!(f, "{}.deg()", AngleDegree::from(*a).0),
                LinearGradientAxis::Line(l) => write!(f, "{l:?}"),
            }
        }
    }
}
impl LinearGradientAxis {
    /// Compute a [`PxLine`].
    pub fn layout(&self, ctx: &LayoutMetrics) -> PxLine {
        match self {
            LinearGradientAxis::Angle(rad) => {
                let dir_x = rad.0.sin();
                let dir_y = -rad.0.cos();

                let av = ctx.constrains().fill_size();
                let av_width = av.width.0 as f32;
                let av_height = av.height.0 as f32;

                let line_length = (dir_x * av_width).abs() + (dir_y * av_height).abs();

                let inv_dir_length = 1.0 / (dir_x * dir_x + dir_y * dir_y).sqrt();

                let delta = euclid::Vector2D::<_, ()>::new(
                    dir_x * inv_dir_length * line_length / 2.0,
                    dir_y * inv_dir_length * line_length / 2.0,
                );

                let center = euclid::Point2D::new(av_width / 2.0, av_height / 2.0);

                let start = center - delta;
                let end = center + delta;
                PxLine::new(
                    PxPoint::new(Px(start.x as i32), Px(start.y as i32)),
                    PxPoint::new(Px(end.x as i32), Px(end.y as i32)),
                )
            }
            LinearGradientAxis::Line(line) => {
                let default_line = PxLine::new(PxPoint::new(Px(0), ctx.viewport_size().height), PxPoint::zero()); // 0º
                line.layout(ctx, default_line)
            }
        }
    }
}
impl_from_and_into_var! {
    fn from(angle: AngleRadian) -> LinearGradientAxis {
        LinearGradientAxis::Angle(angle)
    }
    fn from(angle: AngleDegree) -> LinearGradientAxis {
        LinearGradientAxis::Angle(angle.into())
    }
    fn from(angle: AngleTurn) -> LinearGradientAxis {
        LinearGradientAxis::Angle(angle.into())
    }
    fn from(angle: AngleGradian) -> LinearGradientAxis {
        LinearGradientAxis::Angle(angle.into())
    }
    fn from(line: Line) -> LinearGradientAxis {
        LinearGradientAxis::Line(line)
    }
}

/// A color stop in a gradient.
#[derive(Clone, PartialEq)]
pub struct ColorStop {
    /// The color.
    pub color: Rgba,
    /// Offset point where the [`color`] is fully visible.
    ///
    /// Relative lengths are calculated on the length of the gradient line. The [`Length::Default`] value
    /// indicates this color stop [is positional].
    ///
    /// [`color`]: ColorStop::color
    /// [is positional]: ColorStop::is_positional
    pub offset: Length,
}
impl fmt::Debug for ColorStop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("ColorStop")
                .field("color", &self.color)
                .field("offset", &self.offset)
                .finish()
        } else if self.is_positional() {
            write!(f, "{:?}", self.color)
        } else {
            write!(f, "({:?}, {:?})", self.color, self.offset)
        }
    }
}
impl ColorStop {
    /// New color stop with a defined offset.
    pub fn new(color: impl Into<Rgba>, offset: impl Into<Length>) -> Self {
        ColorStop {
            color: color.into(),
            offset: offset.into(),
        }
    }

    /// New color stop with a undefined offset.
    ///
    /// See [`is_positional`] for more details.
    ///
    /// [`is_positional`]: Self::is_positional
    pub fn new_positional(color: impl Into<Rgba>) -> Self {
        ColorStop {
            color: color.into(),
            offset: Length::Default,
        }
    }

    /// If this color stop offset is resolved relative to the position of the color stop in the stops list.
    ///
    /// A [`Length::Default`] offset indicates that the color stop is positional.
    ///
    /// # Resolution
    ///
    /// When a [`GradientStops`] calculates layout, positional stops are resolved like this:
    ///
    /// * If it is the first stop, the offset is 0%.
    /// * If it is the last stop, the offset is 100% or the previous stop offset whichever is greater.
    /// * If it is surrounded by two stops with known offsets it is the mid-point between the two stops.
    /// * If there is a sequence of positional stops, they split the available length that is defined by the two
    ///   stops with known length that define the sequence.
    ///
    /// # Note
    ///
    /// Use [`ColorStop::is_layout_positional`] if you already have the layout offset.
    pub fn is_positional(&self) -> bool {
        self.offset.is_default()
    }

    /// If a calculated layout offset is [positional].
    ///
    /// Positive infinity ([`f32::INFINITY`]) is used to indicate that the color stop is
    /// positional in webrender units.
    ///
    /// [positional]: Self::is_positional
    pub fn is_layout_positional(layout_offset: f32) -> bool {
        !f32::is_finite(layout_offset)
    }

    /// Compute a [`RenderGradientStop`].
    ///
    /// Note that if this color stop [is positional] the returned offset is [`f32::INFINITY`].
    /// You can use [`ColorStop::is_layout_positional`] to check a layout offset.
    ///
    /// [is positional]: Self::is_positional
    pub fn layout(&self, ctx: Layout1dMetrics) -> RenderGradientStop {
        RenderGradientStop {
            offset: if self.is_positional() {
                f32::INFINITY
            } else {
                self.offset.layout(ctx, Px(0)).to_wr().get()
            },
            color: self.color.into(),
        }
    }
}
impl_from_and_into_var! {
    fn from<C: Into<Rgba> + Clone, O: Into<Length> + Clone>((color, offset): (C, O)) -> ColorStop {
        ColorStop::new(color, offset)
    }

    fn from(positional_color: Rgba) -> ColorStop {
        ColorStop::new_positional(positional_color)
    }

    fn from(positional_color: Hsla) -> ColorStop {
        ColorStop::new_positional(positional_color)
    }

    fn from(positional_color: Hsva) -> ColorStop {
        ColorStop::new_positional(positional_color)
    }
}

/// Computed [`GradientStop`].
///
/// The color offset is in the 0..=1 range.
pub type RenderGradientStop = wr::GradientStop;

/// A stop in a gradient.
#[derive(Clone, PartialEq)]
pub enum GradientStop {
    /// Color stop.
    Color(ColorStop),
    /// Midway point between two colors.
    ColorHint(Length),
}
impl_from_and_into_var! {
    fn from<C: Into<Rgba> + Clone, O: Into<Length> + Clone>(color_offset: (C, O)) -> GradientStop {
        GradientStop::Color(color_offset.into())
    }

    fn from(color_stop: ColorStop) -> GradientStop {
        GradientStop::Color(color_stop)
    }

    fn from(color_hint: Length) -> GradientStop {
        GradientStop::ColorHint(color_hint)
    }

    /// Conversion to [`Length::Relative`] color hint.
    fn from(color_hint: FactorPercent) -> GradientStop {
        GradientStop::ColorHint(color_hint.into())
    }

    /// Conversion to [`Length::Relative`] color hint.
    fn from(color_hint: Factor) -> GradientStop {
        GradientStop::ColorHint(color_hint.into())
    }

    /// Conversion to [`Length::Dip`] color hint.
    fn from(color_hint: f32) -> GradientStop {
        GradientStop::ColorHint(color_hint.into())
    }

    /// Conversion to [`Length::Dip`] color hint.
    fn from(color_hint: i32) -> GradientStop {
        GradientStop::ColorHint(color_hint.into())
    }

    /// Conversion to positional color.
    fn from(positional_color: Rgba) -> GradientStop {
        GradientStop::Color(ColorStop::new_positional(positional_color))
    }

    /// Conversion to positional color.
    fn from(positional_color: Hsla) -> GradientStop {
        GradientStop::Color(ColorStop::new_positional(positional_color))
    }

    /// Conversion to positional color.
    fn from(positional_color: Hsva) -> GradientStop {
        GradientStop::Color(ColorStop::new_positional(positional_color))
    }
}
impl fmt::Debug for GradientStop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                GradientStop::Color(c) => f.debug_tuple("GradientStop::Color").field(c).finish(),
                GradientStop::ColorHint(l) => f.debug_tuple("GradientStop::ColorHint").field(l).finish(),
            }
        } else {
            match self {
                GradientStop::Color(c) => write!(f, "{c:?}"),
                GradientStop::ColorHint(l) => write!(f, "{l:?}"),
            }
        }
    }
}

/// Stops in a gradient.
///
/// Use [`stops!`] to create a new instance, you can convert from arrays for simpler gradients.
#[derive(Clone)]
pub struct GradientStops {
    /// First color stop.
    pub start: ColorStop,

    /// Optional stops between start and end.
    pub middle: Vec<GradientStop>,

    /// Last color stop.
    pub end: ColorStop,
}
impl fmt::Debug for GradientStops {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("GradientStops")
                .field("start", &self.start)
                .field("middle", &self.middle)
                .field("end", &self.end)
                .finish()
        } else {
            write!(f, "stops![{:?}, ", self.start)?;
            for stop in &self.middle {
                write!(f, "{stop:?}, ")?;
            }
            write!(f, "{:?}]", self.end)
        }
    }
}
#[allow(clippy::len_without_is_empty)] // cannot be empty
impl GradientStops {
    /// Gradients stops with two colors from `start` to `end`.
    pub fn new(start: impl Into<Rgba>, end: impl Into<Rgba>) -> Self {
        GradientStops {
            start: ColorStop {
                color: start.into(),
                offset: Length::zero(),
            },
            middle: vec![],
            end: ColorStop {
                color: end.into(),
                offset: 100.pct().into(),
            },
        }
    }

    fn start_missing() -> ColorStop {
        ColorStop {
            color: colors::BLACK.transparent(),
            offset: Length::zero(),
        }
    }

    fn end_missing(start_color: Rgba) -> ColorStop {
        ColorStop {
            color: start_color.transparent(),
            offset: 100.pct().into(),
        }
    }

    /// Gradient stops from colors spaced equally.
    ///
    /// The stops look like a sequence of positional only color stops but
    /// the proportional distribution is pre-calculated.
    ///
    /// If less than 2 colors are given, the missing stops are filled with transparent color.
    pub fn from_colors<C: Into<Rgba> + Copy>(colors: &[C]) -> Self {
        if colors.is_empty() {
            GradientStops {
                start: Self::start_missing(),
                middle: vec![],
                end: Self::end_missing(colors::BLACK),
            }
        } else if colors.len() == 1 {
            let color = colors[0].into();
            GradientStops {
                start: ColorStop {
                    color,
                    offset: Length::zero(),
                },
                middle: vec![],
                end: Self::end_missing(color),
            }
        } else {
            let last = colors.len() - 1;
            let mut offset = 1.0 / colors.len() as f32;
            let offset_step = offset;
            GradientStops {
                start: ColorStop {
                    color: colors[0].into(),
                    offset: Length::zero(),
                },
                middle: colors[1..last]
                    .iter()
                    .map(|&c| {
                        GradientStop::Color(ColorStop {
                            color: c.into(),
                            offset: {
                                let r = offset;
                                offset += offset_step;
                                r.fct().into()
                            },
                        })
                    })
                    .collect(),
                end: ColorStop {
                    color: colors[last].into(),
                    offset: 100.pct().into(),
                },
            }
        }
    }

    /// Gradient stops from colors forming stripes of same length.
    ///
    /// The `transition` parameter controls relative length of the transition between two stripes.
    /// `1.0` or `100.pct()` is the length of a stripe, set to `0.0` to get hard-lines.
    pub fn from_stripes<C: Into<Rgba> + Copy, T: Into<Factor>>(colors: &[C], transition: T) -> Self {
        let tran = transition.into().0;
        let tran = if tran.is_nan() || tran < 0.0 {
            0.0
        } else if tran > 1.0 {
            1.0
        } else {
            tran
        };

        if colors.is_empty() {
            GradientStops {
                start: Self::start_missing(),
                middle: vec![],
                end: Self::end_missing(colors::BLACK),
            }
        } else if colors.len() == 1 {
            let tran = 0.5 * tran;

            let color = colors[0].into();
            let end = Self::end_missing(color);
            GradientStops {
                start: ColorStop {
                    color,
                    offset: Length::zero(),
                },
                middle: vec![
                    GradientStop::Color(ColorStop {
                        color,
                        offset: Length::Relative(Factor(0.5 - tran)),
                    }),
                    GradientStop::Color(ColorStop {
                        color: end.color,
                        offset: Length::Relative(Factor(0.5 + tran)),
                    }),
                ],
                end,
            }
        } else {
            let last = colors.len() - 1;
            let mut offset = 1.0 / colors.len() as f32;
            let stripe_width = offset;
            let tran = stripe_width * tran;

            let start = ColorStop {
                color: colors[0].into(),
                offset: Length::zero(),
            };
            let mut middle = vec![ColorStop {
                color: start.color,
                offset: (offset - tran).fct().into(),
            }
            .into()];

            for &color in &colors[1..last] {
                let color = color.into();
                middle.push(
                    ColorStop {
                        color,
                        offset: (offset + tran).fct().into(),
                    }
                    .into(),
                );
                offset += stripe_width;
                middle.push(
                    ColorStop {
                        color,
                        offset: (offset - tran).fct().into(),
                    }
                    .into(),
                );
            }

            let end = ColorStop {
                color: colors[last].into(),
                offset: Length::Relative(Factor(1.0)),
            };
            middle.push(
                ColorStop {
                    color: end.color,
                    offset: offset.fct().into(),
                }
                .into(),
            );

            GradientStops { start, middle, end }
        }
    }

    /// Gradient stops from color stops.
    ///
    /// If less than 2 colors are given, the missing stops are filled with transparent color.
    pub fn from_stops<C: Into<ColorStop> + Copy>(stops: &[C]) -> Self {
        if stops.is_empty() {
            GradientStops {
                start: Self::start_missing(),
                middle: vec![],
                end: Self::end_missing(colors::BLACK),
            }
        } else if stops.len() == 1 {
            let start = stops[0].into();
            GradientStops {
                end: Self::end_missing(start.color),
                start,
                middle: vec![],
            }
        } else {
            let last = stops.len() - 1;
            GradientStops {
                start: stops[0].into(),
                middle: stops[1..last].iter().map(|&c| GradientStop::Color(c.into())).collect(),
                end: stops[last].into(),
            }
        }
    }

    /// Set the alpha of all colors in the gradient.
    pub fn set_alpha<A: Into<RgbaComponent>>(&mut self, alpha: A) {
        let alpha = alpha.into();
        self.start.color.set_alpha(alpha);
        for mid in &mut self.middle {
            if let GradientStop::Color(c) = mid {
                c.color.set_alpha(alpha);
            }
        }
        self.end.color.set_alpha(alpha);
    }

    /// Computes the layout for a linear gradient.
    ///
    /// The `render_stops` content is replaced with stops with offset in the `0..=1` range.
    ///
    /// The `start_pt` and `end_pt` points are moved to accommodate input offsets outside the line bounds.
    pub fn layout_linear(
        &self,
        ctx: Layout1dMetrics,
        extend_mode: ExtendMode,
        line: &mut PxLine,
        render_stops: &mut Vec<RenderGradientStop>,
    ) {
        let (start_offset, end_offset) = self.layout(ctx, extend_mode, render_stops);

        let mut l_start = line.start.to_wr();
        let mut l_end = line.end.to_wr();

        let v = l_end - l_start;
        let v = v / ctx.constrains().fill_length().to_wr().get();

        l_end = l_start + v * end_offset;
        l_start += v * start_offset;

        line.start = l_start.to_px();
        line.end = l_end.to_px();
    }

    /// Computes the actual color stops.
    ///
    /// Returns offsets of the first and last stop in the `length` line.
    fn layout(&self, ctx: Layout1dMetrics, extend_mode: ExtendMode, render_stops: &mut Vec<RenderGradientStop>) -> (f32, f32) {
        // In this method we need to:
        // 1 - Convert all Length values to LayoutLength.
        // 2 - Adjust offsets so they are always after or equal to the previous offset.
        // 3 - Convert GradientStop::ColorHint to RenderGradientStop.
        // 4 - Manually extend a reflection for ExtendMode::Reflect.
        // 5 - Normalize stop offsets to be all between 0.0..=1.0.
        // 6 - Return the first and last stop offset in pixels.

        fn is_positional(o: f32) -> bool {
            ColorStop::is_layout_positional(o)
        }

        render_stops.clear();

        if extend_mode == ExtendMode::Reflect {
            render_stops.reserve((self.middle.len() + 2) * 2);
        } else {
            render_stops.reserve(self.middle.len() + 2);
        }

        let mut start = self.start.layout(ctx); // 1
        if is_positional(start.offset) {
            start.offset = 0.0;
        }
        render_stops.push(start);

        let mut prev_offset = start.offset;
        let mut hints = vec![];
        let mut positional_start = None;

        for gs in self.middle.iter() {
            match gs {
                GradientStop::Color(s) => {
                    let mut stop = s.layout(ctx); // 1
                    if is_positional(stop.offset) {
                        if positional_start.is_none() {
                            positional_start = Some(render_stops.len());
                        }
                        render_stops.push(stop);
                    } else {
                        if stop.offset < prev_offset {
                            stop.offset = prev_offset; // 2
                        }
                        prev_offset = stop.offset;

                        render_stops.push(stop);

                        if let Some(start) = positional_start.take() {
                            // finished positional sequence.
                            // 1
                            Self::calculate_positional(start..render_stops.len(), render_stops, &hints);
                        }
                    }
                }
                GradientStop::ColorHint(_) => {
                    hints.push(render_stops.len());
                    render_stops.push(RenderGradientStop {
                        // offset and color will be calculated later.
                        offset: 0.0,
                        color: RenderColor::BLACK,
                    })
                }
            }
        }

        let mut stop = self.end.layout(ctx); // 1
        if is_positional(stop.offset) {
            stop.offset = ctx.constrains().fill_length().to_wr().get();
        }
        if stop.offset < prev_offset {
            stop.offset = prev_offset; // 2
        }
        render_stops.push(stop);

        if let Some(start) = positional_start.take() {
            // finished positional sequence.
            // 1
            Self::calculate_positional(start..render_stops.len(), render_stops, &hints);
        }

        // 3
        for &i in hints.iter() {
            let prev = render_stops[i - 1];
            let after = render_stops[i + 1];
            let length = after.offset - prev.offset;
            if length > 0.00001 {
                if let GradientStop::ColorHint(offset) = &self.middle[i - 1] {
                    let ctx = ctx.metrics.clone().with_constrains(|c| c.with_height_fill(Px(length as i32)));
                    let mut offset = offset.layout(ctx.for_y(), Px(0)).to_wr().get();
                    if is_positional(offset) {
                        offset = length / 2.0;
                    } else {
                        offset = offset.min(after.offset).max(prev.offset);
                    }
                    offset += prev.offset;

                    let color = lerp_render_color(prev.color, after.color, 100.0 / length / 2.0);

                    let stop = &mut render_stops[i];
                    stop.color = color;
                    stop.offset = offset;
                } else {
                    unreachable!()
                }
            } else {
                render_stops[i] = prev;
            }
        }

        // 4
        if extend_mode == ExtendMode::Reflect {
            let last_offset = render_stops[render_stops.len() - 1].offset;
            for i in (0..render_stops.len()).rev() {
                let mut stop = render_stops[i];
                stop.offset = last_offset + last_offset - stop.offset;
                render_stops.push(stop);
            }
        }

        let first = render_stops[0];
        let last = render_stops[render_stops.len() - 1];

        let actual_length = last.offset - first.offset;

        if actual_length >= 1.0 {
            // 5
            for stop in render_stops {
                stop.offset = (stop.offset - first.offset) / actual_length;
            }

            (first.offset, last.offset) // 5
        } else {
            // 5 - all stops are at the same offset (within 1px)
            match extend_mode {
                ExtendMode::Clamp => {
                    // we want the first and last color to fill their side
                    // any other middle colors can be removed.
                    render_stops.clear();
                    render_stops.push(first);
                    render_stops.push(first);
                    render_stops.push(last);
                    render_stops.push(last);
                    render_stops[0].offset = 0.0;
                    render_stops[1].offset = 0.48; // not exactly 0.5 to avoid aliasing.
                    render_stops[2].offset = 0.52;
                    render_stops[3].offset = 1.0;

                    // 6 - stretch the line a bit.
                    let offset = last.offset;
                    (offset - 10.0, offset + 10.0)
                }
                ExtendMode::Repeat | ExtendMode::Reflect => {
                    // fill with the average of all colors.
                    let len = render_stops.len() as f32;
                    let color = RenderColor {
                        r: render_stops.iter().map(|s| s.color.r).sum::<f32>() / len,
                        g: render_stops.iter().map(|s| s.color.g).sum::<f32>() / len,
                        b: render_stops.iter().map(|s| s.color.b).sum::<f32>() / len,
                        a: render_stops.iter().map(|s| s.color.a).sum::<f32>() / len,
                    };
                    render_stops.clear();
                    render_stops.push(RenderGradientStop { offset: 0.0, color });
                    render_stops.push(RenderGradientStop { offset: 1.0, color });

                    (0.0, 10.0) // 6
                }
            }
        }
    }

    fn calculate_positional(range: Range<usize>, render_stops: &mut [RenderGradientStop], hints: &[usize]) {
        // count of stops in the positional sequence that are not hints.
        let sequence_count = range.len() - hints.iter().filter(|i| range.contains(i)).count();
        debug_assert!(sequence_count > 1);

        // length that must be split between positional stops.
        let (start_offset, layout_length) = {
            // index of stop after the sequence that has a calculated offset.
            let sequence_ender = (range.end..render_stops.len()).find(|i| !hints.contains(i)).unwrap();
            // index of stop before the sequence that has a calculated offset.
            let sequence_starter = (0..range.start).rev().find(|i| !hints.contains(i)).unwrap();

            let start_offset = render_stops[sequence_starter].offset;
            let length = render_stops[sequence_ender].offset - start_offset;
            (start_offset, length)
        };

        let d = layout_length / (sequence_count + 1) as f32;
        let mut offset = start_offset;

        for i in range {
            if !hints.contains(&i) {
                offset += d;
                render_stops[i].offset = offset;
            }
        }
    }

    /// Number of stops.
    pub fn len(&self) -> usize {
        self.middle.len() + 2
    }
}
impl_from_and_into_var! {
    /// [`GradientStops::from_colors`]
    fn from<C: Into<Rgba> + Copy>(colors: &[C]) -> GradientStops {
        GradientStops::from_colors(colors)
    }

    /// [`GradientStops::from_stops`]
    fn from<C: Into<Rgba> + Copy, L: Into<Length> + Copy>(stops: &[(C, L)]) -> GradientStops {
        GradientStops::from_stops(stops)
    }

    /// [`GradientStops::from_colors`]
    fn from<C: Into<Rgba> + Copy, const N: usize>(colors: &[C; N]) -> GradientStops {
        GradientStops::from_colors(colors)
    }

    /// [`GradientStops::from_stops`]
    fn from<C: Into<Rgba> + Copy, L: Into<Length> + Copy, const N: usize>(stops: &[(C, L); N]) -> GradientStops {
        GradientStops::from_stops(stops)
    }

    /// [`GradientStops::from_colors`]
    fn from<C: Into<Rgba> + Copy, const N: usize>(colors: [C; N]) -> GradientStops {
        GradientStops::from_colors(&colors)
    }

    /// [`GradientStops::from_stops`]
    fn from<C: Into<Rgba> + Copy, L: Into<Length> + Copy, const N: usize>(stops: [(C, L); N]) -> GradientStops {
        GradientStops::from_stops(&stops)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __stops {
    // match single color stop at the $start, plus $color with 2 stops plus other stops, e.g.:
    // stops![colors::RED, (colors::GREEN, 14, 20), colors::BLUE]
    // OR
    // $next_middle that is a $color with 2 stops, plus other stops, e.g.:
    // .. (colors::GREEN, 14, 20), colors::BLUE]
    (
        start: $start:expr,
        middle: [$($middle:expr),*],
        tail: ($color:expr, $stop0:expr, $stop1:expr), $($stops:tt)+
    ) => {
        $crate::__stops! {
            start: $start,
            middle: [$($middle,)* ($color, $stop0), ($color, $stop1)],
            tail: $($stops)+
        }
    };
    // match single color stop at the $start, plus single color stop in the $next_middle, plus other stops, e.g.:
    // stops![colors::RED, colors::GREEN, colors::BLUE]
    // OR
    // $next_middle that is a single color stop, plus other stops, e.g.:
    // .. colors::GREEN, colors::BLUE]
    (
        start: $start:expr,
        middle: [$($middle:expr),*],
        tail: $next_middle:expr, $($stops:tt)+
    ) => {
        $crate::__stops! {
            start: $start,
            middle: [$($middle,)* $next_middle],
            tail: $($stops)+
        }
    };
    // match single color stop at the $start, plus single $color with 2 stops, e.g.:
    // stops![colors::RED, (colors::GREEN, 15, 30)]
    // OR
    // match last entry as single $color with 2 stops, e.g.:
    // .. (colors::BLUE, 20, 30)]
    (
        start: $start:expr,
        middle: [$($middle:expr),*],
        tail: ($color:expr, $stop0:expr, $stop1:expr) $(,)?
    ) => {
        $crate::__stops! {
            start: $start,
            middle: [$($middle,)* ($color, $stop0)],
            tail: ($color, $stop1)
        }
    };
    // match single color stop at the $start, plus single color stop at the $end, e.g.:
    // stops![colors::RED, colors::GREEN]
    // OR
    // match last entry as single color stop, at the $end, e.g.:
    // .. colors::GREEN]
    (
        start: $start:expr,
        middle: [$($middle:expr),*],
        tail: $end:expr $(,)?
    ) => {
        $crate::gradient::GradientStops {
            start: $crate::gradient::ColorStop::from($start),
            middle: std::vec![$($crate::gradient::GradientStop::from($middle)),*],
            end: $crate::gradient::ColorStop::from($end),
        }
    };
}
///<span data-del-macro-root></span> Creates a [`GradientStops`] containing the arguments.
///
/// A minimum of two arguments are required, the first and last argument must be expressions that convert to [`ColorStop`],
/// the middle arguments mut be expressions that convert to [`GradientStop`].
///
/// # Examples
///
/// ```
/// # use zero_ui_core::gradient::stops;
/// # use zero_ui_core::color::colors;
/// # use zero_ui_core::units::*;
/// // green 0%, red 30%, blue 100%.
/// let stops = stops![colors::GREEN, (colors::RED, 30.pct()), colors::BLUE];
///
/// // green to blue, the midway color is at 30%.
/// let stops = stops![colors::GREEN, 30.pct(), colors::BLUE];
/// ```
///
/// # Two Stops Per Color
///
/// The `stops!` macro also accepts a special 3 item *tuple* that represents a color followed by two offsets, this
/// expands to two color stops of the same color. The color type must implement `Into<Rgba> + Copy`. The offset types
/// must implement `Into<Length>`.
///
/// ## Examples
///
/// ```
/// # use zero_ui_core::gradient::stops;
/// # use zero_ui_core::color::colors;
/// # use zero_ui_core::units::*;
/// let zebra_stops = stops![(colors::WHITE, 0, 20), (colors::BLACK, 20, 40)];
/// ```
#[macro_export]
macro_rules! stops {
    // match single entry that is a single color with 2 stops, e.g.:
    // stops![(colors::RED, 0, 20)]
    (($color:expr, $stop0:expr, $stop1:expr) $(,)?) => {
        $crate::__stops! {
            start: ($color, $stop0),
            middle: [],
            tail: ($color, $stop1)
        }
    };
    // match first entry as as single color with 2 stops, plus other stops, e.g:
    // stops![(colors::RED, 0, 20), colors::WHITE]
    (($color:expr, $stop0:expr, $stop1:expr), $($stops:tt)+) => {
        $crate::__stops! {
            start: ($color, $stop0),
            middle: [($color, $stop1)],
            tail: $($stops)+
        }
    };
    ($start:expr, $($stops:tt)+) => {
        $crate::__stops! {
            start: $start,
            middle: [],
            tail: $($stops)+
        }
    };
}
#[doc(inline)]
pub use crate::stops;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stops_simple_2() {
        let stops = stops![colors::BLACK, colors::WHITE];

        assert!(stops.start.is_positional());
        assert_eq!(stops.start.color, colors::BLACK);

        assert!(stops.middle.is_empty());

        assert!(stops.end.is_positional());
        assert_eq!(stops.end.color, colors::WHITE);
    }

    fn test_layout_stops(stops: GradientStops) -> Vec<RenderGradientStop> {
        let mut render_stops = vec![];
        let mut ctx = TestWidgetContext::new();
        ctx.layout_context(
            Px(0),
            Px(0),
            PxSize::new(Px(100), Px(100)),
            1.0.fct(),
            96.0,
            LayoutMask::all(),
            |ctx| {
                stops.layout_linear(
                    ctx.for_x(),
                    ExtendMode::Clamp,
                    &mut PxLine::new(PxPoint::zero(), PxPoint::new(Px(100), Px(100))),
                    &mut render_stops,
                );
            },
        );

        render_stops
    }

    #[test]
    fn positional_end_stops() {
        let stops = test_layout_stops(stops![colors::BLACK, colors::WHITE]);
        assert_eq!(stops.len(), 2);

        assert_eq!(
            stops[0],
            RenderGradientStop {
                color: RenderColor::BLACK,
                offset: 0.0
            }
        );
        assert_eq!(
            stops[1],
            RenderGradientStop {
                color: RenderColor::WHITE,
                offset: 1.0
            }
        );
    }

    #[test]
    fn single_color_2_stops_only() {
        let stops = stops![(colors::BLACK, 0, 100.pct())];

        assert_eq!(stops.start, ColorStop::new(colors::BLACK, 0));
        assert!(stops.middle.is_empty());
        assert_eq!(stops.end, ColorStop::new(colors::BLACK, 100.pct()));
    }

    #[test]
    fn single_color_2_stops_at_start() {
        let stops = stops![(colors::BLACK, 0, 50.pct()), colors::WHITE];

        assert_eq!(stops.start, ColorStop::new(colors::BLACK, 0));
        assert_eq!(stops.middle.len(), 1);
        assert_eq!(stops.middle[0], GradientStop::Color(ColorStop::new(colors::BLACK, 50.pct())));
        assert_eq!(stops.end, ColorStop::new_positional(colors::WHITE));
    }

    #[test]
    fn single_color_2_stops_at_middle() {
        let stops = stops![colors::BLACK, (colors::RED, 10.pct(), 90.pct()), colors::WHITE];

        assert_eq!(stops.start, ColorStop::new_positional(colors::BLACK));
        assert_eq!(stops.middle.len(), 2);
        assert_eq!(stops.middle[0], GradientStop::Color(ColorStop::new(colors::RED, 10.pct())));
        assert_eq!(stops.middle[1], GradientStop::Color(ColorStop::new(colors::RED, 90.pct())));
        assert_eq!(stops.end, ColorStop::new_positional(colors::WHITE));
    }

    #[test]
    fn single_color_2_stops_at_end() {
        let stops = stops![colors::BLACK, (colors::WHITE, 10.pct(), 50.pct())];

        assert_eq!(stops.start, ColorStop::new_positional(colors::BLACK));
        assert_eq!(stops.middle.len(), 1);
        assert_eq!(stops.middle[0], GradientStop::Color(ColorStop::new(colors::WHITE, 10.pct())));
        assert_eq!(stops.end, ColorStop::new(colors::WHITE, 50.pct()));
    }

    #[test]
    fn color_hint() {
        let stops = stops![colors::BLACK, 30.pct(), colors::WHITE];
        assert_eq!(stops.middle.len(), 1);
        assert_eq!(stops.middle[0], GradientStop::ColorHint(30.pct().into()));
    }

    #[test]
    fn color_hint_layout() {
        let stops = test_layout_stops(stops![colors::BLACK, 30.pct(), colors::WHITE]);
        assert_eq!(stops.len(), 3);
        assert_eq!(
            stops[0],
            RenderGradientStop {
                color: RenderColor::BLACK,
                offset: 0.0
            }
        );
        assert_eq!(
            stops[1],
            RenderGradientStop {
                color: RenderColor {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0
                },
                offset: 30.0 / 100.0
            }
        );
        assert_eq!(
            stops[2],
            RenderGradientStop {
                color: RenderColor::WHITE,
                offset: 1.0
            }
        );
    }
}
