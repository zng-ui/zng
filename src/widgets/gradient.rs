use crate::prelude::new_widget::*;

/// Gradient extend mode.
///
/// # Clamp
///
/// The first or last color is used to fill the rest of the widget area.
///
/// # Repeat
///
/// The gradient is repeated to fill the rest of the widget area.
pub type ExtendMode = webrender::api::ExtendMode;

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
    LinearGradientNode {
        angle: angle.into_local(),
        stops: stops.into_local(),
        extend_mode: extend_mode.into_local(),
        render_start: LayoutPoint::zero(),
        render_end: LayoutPoint::zero(),
        render_stops: vec![],
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
    LinearGradientPointsNode {
        start: start.into_local(),
        end: end.into_local(),
        stops: stops.into_local(),
        extend_mode: extend_mode.into_local(),
        render_start: LayoutPoint::zero(),
        render_end: LayoutPoint::zero(),
        render_stops: vec![],
        final_size: LayoutSize::zero(),
    }
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
    LinearGradientTileNode {
        angle: angle.into_local(),
        stops: stops.into_local(),
        extend_mode: extend_mode.into_local(),
        tile_size: tile_size.into_local(),
        tile_spacing: tile_spacing.into_local(),
        render_start: LayoutPoint::zero(),
        render_end: LayoutPoint::zero(),
        final_size: LayoutSize::zero(),
        render_stops: vec![],
        render_tile_size: LayoutSize::zero(),
        render_tile_spacing: LayoutSize::zero(),
    }
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
    LinearGradientPointsTileNode {
        start: start.into_local(),
        end: end.into_local(),
        stops: stops.into_local(),
        extend_mode: extend_mode.into_local(),
        tile_size: tile_size.into_local(),
        tile_spacing: tile_spacing.into_local(),
        render_start: LayoutPoint::zero(),
        render_end: LayoutPoint::zero(),
        final_size: LayoutSize::zero(),
        render_stops: vec![],
        render_tile_size: LayoutSize::zero(),
        render_tile_spacing: LayoutSize::zero(),
    }
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

struct LinearGradientNode<A: VarLocal<AngleRadian>, S: VarLocal<GradientStops>, E: VarLocal<ExtendMode>> {
    angle: A,
    stops: S,
    extend_mode: E,
    render_start: LayoutPoint,
    render_end: LayoutPoint,
    render_stops: Vec<RenderColorStop>,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A: VarLocal<AngleRadian>, S: VarLocal<GradientStops>, E: VarLocal<ExtendMode>> UiNode for LinearGradientNode<A, S, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.angle.init_local(ctx.vars);
        self.extend_mode.init_local(ctx.vars);
        let stops = self.stops.init_local(ctx.vars);
        self.render_stops.reserve(stops.len());
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.angle.update_local(ctx.vars).is_some() {
            // angle changes the line length, so we need to update the stops.
            ctx.updates.layout();
        }
        if self.stops.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.extend_mode.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;
        let (start, end, length) = gradient_ends_from_rad(*self.angle.get_local(), self.final_size);

        self.render_start = start;
        self.render_end = end;
        self.stops.get_local().layout_linear(
            length,
            ctx,
            *self.extend_mode.get_local(),
            &mut self.render_start,
            &mut self.render_end,
            &mut self.render_stops,
        );
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            LayoutRect::from_size(self.final_size),
            self.render_start,
            self.render_end,
            &self.render_stops,
            *self.extend_mode.get_local(),
        );
    }
}

struct LinearGradientPointsNode<A: VarLocal<Point>, B: VarLocal<Point>, S: VarLocal<GradientStops>, E: VarLocal<ExtendMode>> {
    start: A,
    end: B,
    stops: S,
    extend_mode: E,
    render_start: LayoutPoint,
    render_end: LayoutPoint,
    render_stops: Vec<RenderColorStop>,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A: VarLocal<Point>, B: VarLocal<Point>, S: VarLocal<GradientStops>, E: VarLocal<ExtendMode>> UiNode
    for LinearGradientPointsNode<A, B, S, E>
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.start.init_local(ctx.vars);
        self.end.init_local(ctx.vars);
        self.extend_mode.init_local(ctx.vars);
        let stops = self.stops.init_local(ctx.vars);
        self.render_stops.reserve(stops.len());
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.start.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.end.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.stops.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.extend_mode.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;

        self.render_start = self.start.get_local().to_layout(final_size, ctx);
        self.render_end = self.end.get_local().to_layout(final_size, ctx);

        let length = LayoutLength::new(self.render_start.distance_to(self.render_end));

        self.stops.get_local().layout_linear(
            length,
            ctx,
            *self.extend_mode.get_local(),
            &mut self.render_start,
            &mut self.render_end,
            &mut self.render_stops,
        );
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            LayoutRect::from_size(self.final_size),
            self.render_start,
            self.render_end,
            &self.render_stops,
            *self.extend_mode.get_local(),
        );
    }
}

struct LinearGradientTileNode<
    A: VarLocal<AngleRadian>,
    S: VarLocal<GradientStops>,
    E: VarLocal<ExtendMode>,
    T: VarLocal<Size>,
    TS: VarLocal<Size>,
> {
    angle: A,
    stops: S,
    extend_mode: E,
    tile_size: T,
    tile_spacing: TS,

    render_start: LayoutPoint,
    render_end: LayoutPoint,
    render_stops: Vec<RenderColorStop>,

    render_tile_size: LayoutSize,
    render_tile_spacing: LayoutSize,

    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A: VarLocal<AngleRadian>, S: VarLocal<GradientStops>, E: VarLocal<ExtendMode>, T: VarLocal<Size>, TS: VarLocal<Size>> UiNode
    for LinearGradientTileNode<A, S, E, T, TS>
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.angle.init_local(ctx.vars);
        self.tile_size.init_local(ctx.vars);
        self.tile_spacing.init_local(ctx.vars);
        let stops = self.stops.init_local(ctx.vars);
        self.render_stops.reserve(stops.len());
        self.extend_mode.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.stops.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.tile_size.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.tile_spacing.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.angle.update_local(ctx.vars).is_some() {
            // angle changes the line length, so we need to update the stops.
            ctx.updates.layout();
        }
        if self.extend_mode.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;
        self.render_tile_spacing = self.tile_spacing.get_local().to_layout(final_size, ctx);
        self.render_tile_size = self.tile_size.get_local().to_layout(final_size, ctx);

        let (start, end, length) = gradient_ends_from_rad(*self.angle.get_local(), self.render_tile_size);

        self.render_start = start;
        self.render_end = end;
        self.stops.get_local().layout_linear(
            length,
            ctx,
            *self.extend_mode.get_local(),
            &mut self.render_start,
            &mut self.render_end,
            &mut self.render_stops,
        );
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient_tile(
            LayoutRect::from_size(self.final_size),
            self.render_start,
            self.render_end,
            &self.render_stops,
            *self.extend_mode.get_local(),
            self.render_tile_size,
            self.render_tile_spacing,
        );
    }
}

struct LinearGradientPointsTileNode<
    A: VarLocal<Point>,
    B: VarLocal<Point>,
    S: VarLocal<GradientStops>,
    E: VarLocal<ExtendMode>,
    T: VarLocal<Size>,
    TS: VarLocal<Size>,
> {
    start: A,
    end: B,
    stops: S,
    extend_mode: E,
    tile_size: T,
    tile_spacing: TS,

    render_start: LayoutPoint,
    render_end: LayoutPoint,
    render_stops: Vec<RenderColorStop>,

    render_tile_size: LayoutSize,
    render_tile_spacing: LayoutSize,

    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<
        A: VarLocal<Point>,
        B: VarLocal<Point>,
        S: VarLocal<GradientStops>,
        E: VarLocal<ExtendMode>,
        T: VarLocal<Size>,
        TS: VarLocal<Size>,
    > UiNode for LinearGradientPointsTileNode<A, B, S, E, T, TS>
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.start.init_local(ctx.vars);
        self.end.init_local(ctx.vars);
        self.tile_size.init_local(ctx.vars);
        self.tile_spacing.init_local(ctx.vars);
        let stops = self.stops.init_local(ctx.vars);
        self.render_stops.reserve(stops.len());
        self.extend_mode.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.stops.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.tile_size.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.tile_spacing.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.start.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.end.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.extend_mode.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;
        self.render_tile_spacing = self.tile_spacing.get_local().to_layout(final_size, ctx);
        self.render_tile_size = self.tile_size.get_local().to_layout(final_size, ctx);

        self.render_start = self.start.get_local().to_layout(final_size, ctx);
        self.render_end = self.end.get_local().to_layout(final_size, ctx);

        let length = LayoutLength::new(self.render_start.distance_to(self.render_end));

        self.stops.get_local().layout_linear(
            length,
            ctx,
            *self.extend_mode.get_local(),
            &mut self.render_start,
            &mut self.render_end,
            &mut self.render_stops,
        );
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient_tile(
            LayoutRect::from_size(self.final_size),
            self.render_start,
            self.render_end,
            &self.render_stops,
            *self.extend_mode.get_local(),
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
#[derive(Clone, Copy, Debug)]
pub struct ColorStop {
    pub color: Rgba,
    pub offset: Length,
}
impl ColorStop {
    #[inline]
    pub fn to_layout(self, length: LayoutLength, ctx: &LayoutContext) -> RenderColorStop {
        RenderColorStop {
            offset: self.offset.to_layout(length, ctx).get(),
            color: self.color.into(),
        }
    }
}
impl<C: Into<Rgba>, O: Into<Length>> From<(C, O)> for ColorStop {
    fn from((c, o): (C, O)) -> Self {
        ColorStop {
            color: c.into(),
            offset: o.into(),
        }
    }
}

/// A stop in a gradient.
#[derive(Clone, Copy, Debug)]
pub enum GradientStop {
    /// Color stop.
    Color(ColorStop),
    /// Midway point between two colors.
    Mid(Length),
}

/// Stops in a gradient.
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
    /// Start a color gradient builder with the first color stop.
    pub fn start(color: impl Into<Rgba>, offset: impl Into<Length>) -> GradientStopsBuilder {
        GradientStopsBuilder {
            start: ColorStop {
                color: color.into(),
                offset: offset.into(),
            },
            middle: vec![],
        }
    }

    /// Gradients stops with two colors from `start` to `end`.
    pub fn start_end(start: impl Into<Rgba>, end: impl Into<Rgba>) -> Self {
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

    /// Gradients stops with two colors from `start` to `end` and with custom midway point.
    pub fn start_mid_end(start: impl Into<Rgba>, mid: impl Into<Length>, end: impl Into<Rgba>) -> Self {
        GradientStops {
            start: ColorStop {
                color: start.into(),
                offset: Length::zero(),
            },
            middle: vec![GradientStop::Mid(mid.into())],
            end: ColorStop {
                color: end.into(),
                offset: 100.pct().into(),
            },
        }
    }

    fn start_missing() -> ColorStop {
        ColorStop {
            color: colors::TRANSPARENT,
            offset: Length::zero(),
        }
    }

    fn end_missing() -> ColorStop {
        ColorStop {
            color: colors::TRANSPARENT,
            offset: 100.pct().into(),
        }
    }

    /// Gradient stops from colors spaced equally.
    pub fn from_colors<C: Into<Rgba> + Copy>(colors: &[C]) -> Self {
        if colors.is_empty() {
            GradientStops {
                start: Self::start_missing(),
                middle: vec![],
                end: Self::end_missing(),
            }
        } else if colors.len() == 1 {
            GradientStops {
                start: ColorStop {
                    color: colors[0].into(),
                    offset: Length::zero(),
                },
                middle: vec![],
                end: Self::end_missing(),
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

    /// Gradient stops from color stops.
    pub fn from_stops<C: Into<ColorStop> + Copy>(stops: &[C]) -> Self {
        if stops.is_empty() {
            GradientStops {
                start: Self::start_missing(),
                middle: vec![],
                end: Self::end_missing(),
            }
        } else if stops.len() == 1 {
            GradientStops {
                start: stops[0].into(),
                middle: vec![],
                end: Self::end_missing(),
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

    pub fn layout_linear(
        &self,
        length: LayoutLength,
        ctx: &LayoutContext,
        extend_mode: ExtendMode,
        start_pt: &mut LayoutPoint,
        end_pt: &mut LayoutPoint,
        render_stops: &mut Vec<RenderColorStop>,
    ) {
        let (start_offset, end_offset) = self.layout(length, ctx, extend_mode, render_stops);

        let v = *end_pt - *start_pt;
        let v = v / length.get();

        *end_pt = *start_pt + v * end_offset;
        *start_pt += v * start_offset;
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
        // 3 - Convert GradientStop::Mid to RenderColorStop.
        // 4 - Normalize stop offsets to be all between 0.0..=1.0.
        // 5 - Return the first and last stop offset.

        render_stops.clear();
        let mut prev_stop = self.start.to_layout(length, ctx); // 1
        let mut pending_mid = None;

        render_stops.push(prev_stop);
        for gs in self.middle.iter() {
            match gs {
                GradientStop::Color(s) => {
                    let mut stop = s.to_layout(length, ctx); // 1

                    if let Some(mid) = pending_mid.take() {
                        if stop.offset < mid {
                            stop.offset = mid; // 2
                        }

                        render_stops.push(Self::mid_to_color_stop(prev_stop, mid, stop));
                    // 3
                    } else if stop.offset < prev_stop.offset {
                        stop.offset = prev_stop.offset; // 2
                    }

                    render_stops.push(stop);
                    prev_stop = stop;
                }
                GradientStop::Mid(l) => {
                    // TODO do we care if pending_mid is some here?

                    let mut l = l.to_layout(length, ctx).0; // 1
                    if l > prev_stop.offset {
                        l = prev_stop.offset; // 2
                    }
                    pending_mid = Some(l);
                }
            }
        }

        let mut stop = self.end.to_layout(length, ctx); // 1
        if let Some(mid) = pending_mid.take() {
            if stop.offset < mid {
                stop.offset = mid; // 2
            }

            render_stops.push(Self::mid_to_color_stop(prev_stop, mid, stop)); // 3
        } else if stop.offset < prev_stop.offset {
            stop.offset = prev_stop.offset; // 2
        }

        render_stops.push(stop);

        let first = render_stops[0];
        let last = render_stops[render_stops.len() - 1];

        let actual_length = last.offset - first.offset;

        if actual_length > 0.00001 {
            // 4
            for stop in render_stops {
                stop.offset = (stop.offset - first.offset) / actual_length;
            }

            (first.offset, last.offset) // 5
        } else {
            // 4 - all stops are at the same offset
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

                    // 5 - line starts and ends at the offset point.
                    let offset = last.offset;
                    (offset - 0.5, offset + 0.5)
                }
                ExtendMode::Repeat => {
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

                    (0.0, 1.0) // 5
                }
            }
        }
    }

    fn mid_to_color_stop(prev: RenderColorStop, mid: f32, next: RenderColorStop) -> RenderColorStop {
        let lerp_mid = (next.offset - prev.offset) / (mid - prev.offset);
        RenderColorStop {
            color: lerp_render_color(prev.color, next.color, lerp_mid),
            offset: mid,
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

/// A [`GradientStops`] builder.
pub struct GradientStopsBuilder {
    start: ColorStop,
    middle: Vec<GradientStop>,
}
impl GradientStopsBuilder {
    /// Add a color stop.
    pub fn color(mut self, color: impl Into<Rgba>, offset: impl Into<Length>) -> GradientStopsBuilderWithMid {
        self.middle.push(GradientStop::Color(ColorStop {
            color: color.into(),
            offset: offset.into(),
        }));
        GradientStopsBuilderWithMid(self)
    }

    fn mid(mut self, offset: impl Into<Length>) -> Self {
        self.middle.push(GradientStop::Mid(offset.into()));
        self
    }

    /// Finishes the gradient with the last color stop.
    pub fn end(self, color: impl Into<Rgba>, offset: impl Into<Length>) -> GradientStops {
        GradientStops {
            start: self.start,
            middle: self.middle,
            end: ColorStop {
                color: color.into(),
                offset: offset.into(),
            },
        }
    }
}

/// [`GradientStopsBuilder`] in a state that allows adding a midway point.
pub struct GradientStopsBuilderWithMid(GradientStopsBuilder);
impl GradientStopsBuilderWithMid {
    /// Add a color stop.
    pub fn color(self, color: impl Into<Rgba>, offset: impl Into<Length>) -> GradientStopsBuilderWithMid {
        self.0.color(color, offset)
    }

    /// Add the midway points between the previous color stop and the next.
    pub fn mid(self, offset: impl Into<Length>) -> GradientStopsBuilder {
        self.0.mid(offset)
    }

    /// Finishes the gradient with the last color stop.
    pub fn end(self, color: impl Into<Rgba>, offset: impl Into<Length>) -> GradientStops {
        self.0.end(color, offset)
    }
}

fn gradient_ends_from_rad(rad: AngleRadian, size: LayoutSize) -> (LayoutPoint, LayoutPoint, LayoutLength) {
    let dir = LayoutPoint::new(rad.0.sin(), -rad.0.cos());

    let line_length = (dir.x * size.width).abs() + (dir.y * size.height).abs();

    let inv_dir_length = 1.0 / (dir.x * dir.x + dir.y * dir.y).sqrt();

    let delta = euclid::Vector2D::new(
        dir.x * inv_dir_length * line_length / 2.0,
        dir.y * inv_dir_length * line_length / 2.0,
    );

    let length = LayoutLength::new((delta.x * 2.0).hypot(delta.y * 2.0));

    let center = LayoutPoint::new(size.width / 2.0, size.height / 2.0);

    (center - delta, center + delta, length)
}
