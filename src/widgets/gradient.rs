use std::{marker::PhantomData, ops::Range};

use crate::core::render::RenderExtendMode;
use crate::prelude::new_widget::*;

/// Specifies how to draw the gradient outside the first and last stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtendMode {
    /// Default value. The color values at the ends of the gradient vector fill the remaining space.
    Clamp,
    /// The gradient is repeated until the space is filled.
    Repeat,
    /// The gradient is repeated alternating direction until the space is filled.
    Reflect,
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

/// Paints a linear gradient with a line defined by angle.
///
/// The line is centered in the widget, the start and end points are defined so that
/// color stops at 0% and 100% are always visible at opposite corners.
///
/// If the first color stop is greater then 0% or the last color stop is less then 100% the gradient
/// is extended using the `extend_mode`.
pub fn linear_gradient(
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    linear_gradient_full(angle, stops, extend_mode, Size::fill(), Size::zero())
}

pub fn linear_gradient_full<A: LinearGradientAxis>(
    axis: impl IntoVar<A>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    LinearGradientFullNode {
        _axis_type: PhantomData,
        axis: axis.into_local(),
        stops: stops.into_local(),
        extend_mode: extend_mode.into_local(),
        tile_size: tile_size.into_local(),
        tile_spacing: tile_spacing.into_local(),
        render_line: LayoutLine::zero(),
        render_stops: vec![],
        render_tile_size: LayoutSize::zero(),
        render_tile_spacing: LayoutSize::zero(),
        final_size: LayoutSize::zero(),
    }
}

/// Paints a linear gradient with a line defined by two points.
///
/// The points are relative to the widget area (top-left origin), a color stop at 0% is at the `start` point,
/// a color stop at 100% is at the `end` point.
///
/// The line is logically infinite, the two points define the angle, center and 0-100% range, color stops
/// can be set outside the range.
///
/// If no color stop outside the range fully covers the visible gradient in the widget
/// area the gradient is extended using the `extend_mode`.
pub fn linear_gradient_pt(
    start: impl IntoVar<Point>,
    end: impl IntoVar<Point>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    linear_gradient_full(
        merge_var!(start.into_var(), end.into_var(), |a, b| Line::new(*a, *b)),
        stops,
        extend_mode,
        Size::fill(),
        Size::zero(),
    )
}

/// Paints a [`linear_gradient`] to fill the `tile_size` area, the tile is then repeated to fill
/// the widget area. Space can be added between tiles using `tile_spacing`.
pub fn linear_gradient_tile(
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    linear_gradient_full(angle, stops, extend_mode, tile_size, tile_spacing)
}

/// Paints a [`linear_gradient_pt`] to fill the `tile_size` area, the tile is then repeated to fill
/// the widget area. Space can be added between tiles using `tile_spacing`.
pub fn linear_gradient_pt_tile(
    start: impl IntoVar<Point>,
    end: impl IntoVar<Point>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    linear_gradient_full(
        merge_var!(start.into_var(), end.into_var(), |a, b| Line::new(*a, *b)),
        stops,
        extend_mode,
        tile_size,
        tile_spacing,
    )
}

/// Linear gradient from bottom to top.
///
/// This is equivalent to angle `0.deg()` or points `(0, 100.pct()) to (0, 0)`.
pub fn linear_gradient_to_top(stops: impl IntoVar<GradientStops>, extend_mode: impl IntoVar<ExtendMode>) -> impl UiNode {
    linear_gradient_pt((0, 100.pct()), (0, 0), stops, extend_mode)
}

/// Linear gradient from top to bottom.
///
/// This is equivalent to angle `180.deg()` or points `(0, 0), (0, 100.pct())`.
pub fn linear_gradient_to_bottom(stops: impl IntoVar<GradientStops>, extend_mode: impl IntoVar<ExtendMode>) -> impl UiNode {
    linear_gradient_pt((0, 0), (0, 100.pct()), stops, extend_mode)
}

/// Linear gradient from right to left.
///
/// This is equivalent to angle `270.deg()` or points `(100.pct(), 0), (0, 0)`.
pub fn linear_gradient_to_left(stops: impl IntoVar<GradientStops>, extend_mode: impl IntoVar<ExtendMode>) -> impl UiNode {
    linear_gradient_pt((100.pct(), 0), (0, 0), stops, extend_mode)
}

/// Linear gradient from left to right.
///
/// This is equivalent to angle `90.deg()` or points `(0, 0), (100.pct(), 0)`.
pub fn linear_gradient_to_right(stops: impl IntoVar<GradientStops>, extend_mode: impl IntoVar<ExtendMode>) -> impl UiNode {
    linear_gradient_pt((0, 0), (100.pct(), 0), stops, extend_mode)
}

/// Linear gradient from bottom-left to top-right.
///
/// This is equivalent to points `(0, 100.pct()), (100.pct(), 0)`. There is no angle equivalent.
pub fn linear_gradient_to_top_right(stops: impl IntoVar<GradientStops>, extend_mode: impl IntoVar<ExtendMode>) -> impl UiNode {
    linear_gradient_pt((0, 100.pct()), (100.pct(), 0), stops, extend_mode)
}

/// Linear gradient from top-left to bottom-right.
///
/// This is equivalent to points `(0, 0), (100.pct(), 100.pct())`. There is no angle equivalent.
pub fn linear_gradient_to_bottom_right(stops: impl IntoVar<GradientStops>, extend_mode: impl IntoVar<ExtendMode>) -> impl UiNode {
    linear_gradient_pt((0, 0), (100.pct(), 100.pct()), stops, extend_mode)
}

/// Linear gradient from bottom-right to top-left.
///
/// This is equivalent to points `(100.pct(), 100.pct()), (0, 0)`. There is no angle equivalent.
pub fn linear_gradient_to_top_left(stops: impl IntoVar<GradientStops>, extend_mode: impl IntoVar<ExtendMode>) -> impl UiNode {
    linear_gradient_pt((100.pct(), 100.pct()), (0, 0), stops, extend_mode)
}

/// Linear gradient from top-right to bottom-left.
///
/// This is equivalent to points `(100.pct(), 0), (0, 100.pct())`. There is no angle equivalent.
pub fn linear_gradient_to_bottom_left(stops: impl IntoVar<GradientStops>, extend_mode: impl IntoVar<ExtendMode>) -> impl UiNode {
    linear_gradient_pt((100.pct(), 0), (0, 100.pct()), stops, extend_mode)
}

/// The [angle](AngleUnits) or [line](crate::core::units::Line) that defines a linear gradient.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// # use zero_ui::widgets::LinearGradientAxis;
/// fn angle() -> impl LinearGradientAxis {
///     90.deg() // all angle units supported.
/// }
///
/// fn line() -> impl LinearGradientAxis {
///     (0, 0).to(100.pct(), 100.pct())
/// }
/// ```
pub trait LinearGradientAxis: VarValue {
    fn layout(&self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutLine;
}
impl<A: Into<AngleRadian> + Copy + VarValue> LinearGradientAxis for A {
    fn layout(&self, size: LayoutSize, ctx: &LayoutContext) -> LayoutLine {
        let rad = (*self).into();

        let dir = LayoutPoint::new(rad.0.sin(), -rad.0.cos());

        let line_length = (dir.x * size.width).abs() + (dir.y * size.height).abs();

        let inv_dir_length = 1.0 / (dir.x * dir.x + dir.y * dir.y).sqrt();

        let delta = euclid::Vector2D::new(
            dir.x * inv_dir_length * line_length / 2.0,
            dir.y * inv_dir_length * line_length / 2.0,
        );

        let center = LayoutPoint::new(size.width / 2.0, size.height / 2.0);

        let line = LayoutLine::new(center - delta, center + delta);

        line.snap_to(ctx.pixel_grid())
    }
}
impl LinearGradientAxis for Line {
    fn layout(&self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutLine {
        self.to_layout(available_size, ctx)
    }
}

struct LinearGradientFullNode<
    A: LinearGradientAxis,
    VA: VarLocal<A>,
    S: VarLocal<GradientStops>,
    E: VarLocal<ExtendMode>,
    T: VarLocal<Size>,
    TS: VarLocal<Size>,
> {
    _axis_type: PhantomData<A>,

    axis: VA,
    stops: S,
    extend_mode: E,
    tile_size: T,
    tile_spacing: TS,

    render_line: LayoutLine,
    render_stops: Vec<RenderColorStop>,

    render_tile_size: LayoutSize,
    render_tile_spacing: LayoutSize,

    final_size: LayoutSize,
}

#[impl_ui_node(none)]
impl<
        A: LinearGradientAxis,
        VA: VarLocal<A>,
        S: VarLocal<GradientStops>,
        E: VarLocal<ExtendMode>,
        T: VarLocal<Size>,
        TS: VarLocal<Size>,
    > UiNode for LinearGradientFullNode<A, VA, S, E, T, TS>
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.axis.init_local(ctx.vars);
        self.extend_mode.init_local(ctx.vars);
        self.stops.init_local(ctx.vars);
        self.tile_size.init_local(ctx.vars);
        self.tile_spacing.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.axis.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.stops.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.extend_mode.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.tile_size.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.tile_spacing.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;
        self.render_tile_size = self.tile_size.get_local().to_layout(final_size, ctx);
        self.render_tile_spacing = self.tile_spacing.get_local().to_layout(final_size, ctx);
        self.render_line = self.axis.get_local().layout(self.render_tile_size, ctx);

        let length = self.render_line.length();

        self.stops.get_local().layout_linear(
            length,
            ctx,
            *self.extend_mode.get_local(),
            &mut self.render_line,
            &mut self.render_stops,
        );
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            LayoutRect::from_size(self.final_size),
            self.render_line,
            &self.render_stops,
            (*self.extend_mode.get_local()).into(),
            self.render_tile_size,
            self.render_tile_spacing,
        );
    }
}

/// Computed [`GradientStop`].
///
/// The color offset is in the 0..=1 range.
pub type RenderColorStop = webrender::api::GradientStop;

/// A color stop in a gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorStop {
    pub color: Rgba,
    pub offset: Length,
}
impl ColorStop {
    #[inline]
    pub fn new(color: impl Into<Rgba>, offset: impl Into<Length>) -> Self {
        ColorStop {
            color: color.into(),
            offset: offset.into(),
        }
    }

    /// New color stop with a undefined offset.
    ///
    /// See [`is_positional`](Self::is_positional) for more details.
    #[inline]
    pub fn new_positional(color: impl Into<Rgba>) -> Self {
        ColorStop {
            color: color.into(),
            offset: Length::Relative(FactorNormal(f32::INFINITY)),
        }
    }

    /// If this color stop offset is resolved relative to the position of the color stop in the stops list.
    ///
    /// Any offset that does not resolve to a finite layout offset is positional.
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
    /// Use [`ColorStop::is_layout_positional`] is you already have the layout offset, it is faster then calling
    /// this method and then converting to layout.
    pub fn is_positional(&self) -> bool {
        let l = self.offset.to_layout(
            LayoutLength::new(100.0),
            &LayoutContext::new(20.0, LayoutSize::new(100.0, 100.0), PixelGrid::new(1.0)),
        );
        Self::is_layout_positional(l.get())
    }

    /// If a calculated layout offset is [positional](Self::is_positional).
    #[inline]
    pub fn is_layout_positional(layout_offset: f32) -> bool {
        !f32::is_finite(layout_offset)
    }

    #[inline]
    pub fn to_layout(self, length: LayoutLength, ctx: &LayoutContext) -> RenderColorStop {
        RenderColorStop {
            offset: self.offset.to_layout(length, ctx).get(),
            color: self.color.into(),
        }
    }
}
impl_from_and_into_var! {
    fn from<C: Into<Rgba>, O: Into<Length>>(color_offset: (C, O)) -> ColorStop {
        ColorStop::new(color_offset.0, color_offset.1)
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

/// A stop in a gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientStop {
    /// Color stop.
    Color(ColorStop),
    /// Midway point between two colors.
    ColorHint(Length),
}
impl_from_and_into_var! {
    fn from<C: Into<Rgba>, O: Into<Length>>(color_offset: (C, O)) -> GradientStop {
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
    fn from(color_hint: FactorNormal) -> GradientStop {
        GradientStop::ColorHint(color_hint.into())
    }

    /// Conversion to [`Length::Exact`] color hint.
    fn from(color_hint: f32) -> GradientStop {
        GradientStop::ColorHint(color_hint.into())
    }

    /// Conversion to [`Length::Exact`] color hint.
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

/// Stops in a gradient.
///
/// Use [`stops!`] to create a new instance, you can convert from arrays for simpler gradients.
#[derive(Debug, Clone)]
pub struct GradientStops {
    /// First color stop.
    pub start: ColorStop,

    /// Optional stops between start and end.
    pub middle: Vec<GradientStop>,

    /// Last color stop.
    pub end: ColorStop,
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
    /// If less then 2 colors are given, the missing stops are filled with transparent color.
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
                                r.normal().into()
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

    /// Gradient stops from colors forming hardline stripes of same length.
    pub fn from_stripes<C: Into<Rgba> + Copy>(colors: &[C]) -> Self {
        if colors.is_empty() {
            GradientStops {
                start: Self::start_missing(),
                middle: vec![],
                end: Self::end_missing(colors::BLACK),
            }
        } else if colors.len() == 1 {
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
                        offset: Length::Relative(FactorNormal(0.5)),
                    }),
                    GradientStop::Color(ColorStop {
                        color: end.color,
                        offset: Length::Relative(FactorNormal(0.5)),
                    }),
                ],
                end,
            }
        } else {
            let last = colors.len() - 1;
            let mut offset = 1.0 / colors.len() as f32;
            let stripe_width = offset;

            let start = ColorStop {
                color: colors[0].into(),
                offset: Length::zero(),
            };
            let mut middle = vec![ColorStop {
                color: start.color,
                offset: offset.normal().into(),
            }
            .into()];

            for &color in &colors[1..last] {
                let color = color.into();
                middle.push(
                    ColorStop {
                        color,
                        offset: offset.normal().into(),
                    }
                    .into(),
                );
                offset += stripe_width;
                middle.push(
                    ColorStop {
                        color,
                        offset: offset.normal().into(),
                    }
                    .into(),
                );
            }

            let end = ColorStop {
                color: colors[last].into(),
                offset: Length::Relative(FactorNormal(1.0)),
            };
            middle.push(
                ColorStop {
                    color: end.color,
                    offset: offset.normal().into(),
                }
                .into(),
            );

            GradientStops { start, middle, end }
        }
    }

    /// Gradient stops from color stops.
    ///
    /// If less then 2 colors are given, the missing stops are filled with transparent color.
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
                start,
                middle: vec![],
                end: Self::end_missing(start.color),
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

    /// Computes the layout for a linear gradient.
    ///
    /// The `render_stops` content is replaced with stops with offset in the `0..=1` range.
    ///
    /// The `start_pt` and `end_pt` points are moved to accommodate input offsets outside the line bounds.
    pub fn layout_linear(
        &self,
        length: LayoutLength,
        ctx: &LayoutContext,
        extend_mode: ExtendMode,
        line: &mut LayoutLine,
        render_stops: &mut Vec<RenderColorStop>,
    ) {
        let (start_offset, end_offset) = self.layout(length, ctx, extend_mode, render_stops);

        let v = line.end - line.start;
        let v = v / length.get();

        line.end = line.start + v * end_offset;
        line.start += v * start_offset;
    }

    /// Computes the actual color stops.
    ///
    /// Returns offsets of the first and last stop in the `length` line.
    fn layout(
        &self,
        length: LayoutLength,
        ctx: &LayoutContext,
        extend_mode: ExtendMode,
        render_stops: &mut Vec<RenderColorStop>,
    ) -> (f32, f32) {
        // In this method we need to:
        // 1 - Convert all Length values to LayoutLength.
        // 2 - Adjust offsets so they are always after or equal to the previous offset.
        // 3 - Convert GradientStop::ColorHint to RenderColorStop.
        // 4 - Manually extend a reflection for ExtendMode::Reflect.
        // 5 - Normalize stop offsets to be all between 0.0..=1.0.
        // 6 - Return the first and last stop offset in layout units.

        fn is_positional(o: f32) -> bool {
            ColorStop::is_layout_positional(o)
        }

        render_stops.clear();

        if extend_mode == ExtendMode::Reflect {
            render_stops.reserve((self.middle.len() + 2) * 2);
        } else {
            render_stops.reserve(self.middle.len() + 2);
        }

        let mut start = self.start.to_layout(length, ctx); // 1
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
                    let mut stop = s.to_layout(length, ctx); // 1
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
                    render_stops.push(RenderColorStop {
                        // offset and color will be calculated later.
                        offset: 0.0,
                        color: RenderColor::BLACK,
                    })
                }
            }
        }

        let mut stop = self.end.to_layout(length, ctx); // 1
        if is_positional(stop.offset) {
            stop.offset = length.get();
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
                if let GradientStop::ColorHint(offset) = self.middle[i - 1] {
                    let mut offset = offset.to_layout(LayoutLength::new(length), ctx).get();
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

        if actual_length > 0.00001 {
            // 5
            for stop in render_stops {
                stop.offset = (stop.offset - first.offset) / actual_length;
            }

            (first.offset, last.offset) // 5
        } else {
            // 5 - all stops are at the same offset
            match extend_mode {
                ExtendMode::Clamp => {
                    // we want the first and last color to fill their side
                    // any other middle colors can be removed.
                    // TODO: can we make this happen with just two stops?
                    render_stops.clear();
                    render_stops.push(first);
                    render_stops.push(first);
                    render_stops.push(last);
                    render_stops.push(last);
                    render_stops[0].offset = 0.0;
                    render_stops[1].offset = 0.5;
                    render_stops[2].offset = 0.5;
                    render_stops[3].offset = 1.0;

                    // 6 - line starts and ends at the offset point.
                    let offset = last.offset;
                    (offset - 0.5, offset + 0.5)
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
                    render_stops.push(RenderColorStop { offset: 0.0, color });
                    render_stops.push(RenderColorStop { offset: 1.0, color });

                    (0.0, 1.0) // 6
                }
            }
        }
    }

    fn calculate_positional(range: Range<usize>, render_stops: &mut [RenderColorStop], hints: &[usize]) {
        // count of stops in the positional sequence that are not hints.
        let sequence_count = range.len() - hints.iter().filter(|i| range.contains(i)).count();
        debug_assert!(sequence_count > 1);

        // length that must be split between positional stops.
        let (start_offset, layout_length) = {
            // index of stop after the sequence that has a calculated offset.
            let sequence_ender = (range.end..render_stops.len()).find(|i| !hints.contains(&i)).unwrap();
            // index of stop before the sequence that has a calculated offset.
            let sequence_starter = (0..range.start).rev().find(|i| !hints.contains(&i)).unwrap();

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
    #[inline]
    pub fn len(&self) -> usize {
        self.middle.len() + 2
    }
}
impl<C: Into<Rgba> + Copy + 'static> From<&[C]> for GradientStops {
    fn from(a: &[C]) -> Self {
        GradientStops::from_colors(a)
    }
}
impl<C: Into<Rgba> + Copy + 'static> IntoVar<GradientStops> for &[C] {
    type Var = OwnedVar<GradientStops>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}
impl<C: Into<Rgba> + Copy + 'static, L: Into<Length> + Copy + 'static> From<&[(C, L)]> for GradientStops {
    fn from(a: &[(C, L)]) -> Self {
        GradientStops::from_stops(a)
    }
}
impl<C: Into<Rgba> + Copy + 'static, L: Into<Length> + Copy + 'static> IntoVar<GradientStops> for &[(C, L)] {
    type Var = OwnedVar<GradientStops>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}
macro_rules! impl_from_color_arrays {
    ($($N:tt),+ $(,)?) => {$(
        impl<C: Into<Rgba> + Copy + 'static> From<[C; $N]> for GradientStops {
            fn from(a: [C; $N]) -> Self {
                GradientStops::from_colors(&a)
            }
        }
        impl<C: Into<Rgba> + Copy + 'static> IntoVar<GradientStops> for [C; $N] {
            type Var = OwnedVar<GradientStops>;

            fn into_var(self) -> Self::Var {
                OwnedVar(self.into())
            }
        }

        impl<C: Into<Rgba> + Copy + 'static, L: Into<Length> + Copy + 'static> From<[(C, L); $N]> for GradientStops {
            fn from(a: [(C, L); $N]) -> Self {
                GradientStops::from_stops(&a)
            }
        }
        impl<C: Into<Rgba> + Copy + 'static, L: Into<Length> + Copy + 'static> IntoVar<GradientStops> for [(C, L); $N] {
            type Var = OwnedVar<GradientStops>;

            fn into_var(self) -> Self::Var {
                OwnedVar(self.into())
            }
        }
    )+};
}
impl_from_color_arrays!(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);

/// Creates a [`GradientStops`] containing the arguments.
///
/// A minimum of two arguments are required, the first and last argument must be expressions that convert to [`ColorStop`],
/// the middle arguments mut be expressions that convert to [`GradientStop`].
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// # use zero_ui::widgets::stops;
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
/// ## Example
/// ```
/// # use zero_ui::prelude::*;
/// # use zero_ui::widgets::stops;
/// let zebra_stops = stops![(colors::WHITE, 0, 20), (colors::BLACK, 20, 40)];
/// ```
pub use zero_ui_macros::stops;

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

    fn test_layout_stops(stops: GradientStops) -> Vec<RenderColorStop> {
        let mut render_stops = vec![];
        stops.layout_linear(
            LayoutLength::new(100.0),
            &LayoutContext::new(20.0, LayoutSize::new(100.0, 100.0), PixelGrid::new(1.0)),
            ExtendMode::Clamp,
            &mut LayoutPoint::zero(),
            &mut LayoutPoint::new(100.0, 100.0),
            &mut render_stops,
        );
        render_stops
    }

    #[test]
    fn positional_end_stops() {
        let stops = test_layout_stops(stops![colors::BLACK, colors::WHITE]);

        assert_eq!(stops.len(), 2);

        assert_eq!(
            stops[0],
            RenderColorStop {
                color: RenderColor::BLACK,
                offset: 0.0
            }
        );
        assert_eq!(
            stops[1],
            RenderColorStop {
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
            RenderColorStop {
                color: RenderColor::BLACK,
                offset: 0.0
            }
        );
        assert_eq!(
            stops[1],
            RenderColorStop {
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
            RenderColorStop {
                color: RenderColor::WHITE,
                offset: 1.0
            }
        );
    }
}
