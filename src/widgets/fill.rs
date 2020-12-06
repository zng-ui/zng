use crate::prelude::new_widget::*;

pub use webrender::api::ExtendMode;

/// Computed [`GradientStop`].
pub type LayoutGradientStop = webrender::api::GradientStop;

/// A color stop in a linear or radial gradient.
#[derive(Clone, Copy, Debug)]
pub struct GradientStop {
    pub offset: Length,
    pub color: Rgba,
}
impl GradientStop {
    #[inline]
    pub fn to_layout(self, available_length: LayoutLength, ctx: &LayoutContext) -> LayoutGradientStop {
        LayoutGradientStop {
            offset: self.offset.to_layout(available_length, ctx).get(),
            color: self.color.into(),
        }
    }
}

struct FillGradientNode<A: VarLocal<AngleRadian>, S: VarLocal<GradientStops>> {
    angle: A,
    stops: S,
    render_start: LayoutPoint,
    render_end: LayoutPoint,
    render_stops: Vec<LayoutGradientStop>,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A: VarLocal<AngleRadian>, S: VarLocal<GradientStops>> UiNode for FillGradientNode<A, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.angle.init_local(ctx.vars);
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
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;
        let (start, end, length) = gradient_ends_from_rad(*self.angle.get_local(), self.final_size);
        self.render_start = start;
        self.render_end = end;
        self.render_stops.clear();
        self.render_stops
            .extend(self.stops.get_local().iter().map(|&s| s.to_layout(length, ctx)));
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            LayoutRect::from_size(self.final_size),
            self.render_start,
            self.render_end,
            &self.render_stops,
            ExtendMode::Clamp,
        );
    }
}

struct FillGradientPointsNode<A: VarLocal<Point>, B: VarLocal<Point>, S: VarLocal<GradientStops>, E: VarLocal<ExtendMode>> {
    start: A,
    end: B,
    stops: S,
    extend_mode: E,
    render_start: LayoutPoint,
    render_end: LayoutPoint,
    render_stops: Vec<LayoutGradientStop>,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A: VarLocal<Point>, B: VarLocal<Point>, S: VarLocal<GradientStops>, E: VarLocal<ExtendMode>> UiNode
    for FillGradientPointsNode<A, B, S, E>
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

        self.render_stops.clear();
        self.render_stops
            .extend(self.stops.get_local().iter().map(|&s| s.to_layout(length, ctx)));
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

struct FillGradientTileNode<A: VarLocal<AngleRadian>, S: VarLocal<GradientStops>, T: VarLocal<Size>, TS: VarLocal<Size>> {
    angle: A,
    stops: S,
    tile_size: T,
    tile_spacing: TS,

    render_start: LayoutPoint,
    render_end: LayoutPoint,
    render_stops: Vec<LayoutGradientStop>,

    render_tile_size: LayoutSize,
    render_tile_spacing: LayoutSize,

    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A: VarLocal<AngleRadian>, S: VarLocal<GradientStops>, T: VarLocal<Size>, TS: VarLocal<Size>> UiNode
    for FillGradientTileNode<A, S, T, TS>
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.angle.init_local(ctx.vars);
        self.tile_size.init_local(ctx.vars);
        self.tile_spacing.init_local(ctx.vars);
        let stops = self.stops.init_local(ctx.vars);
        self.render_stops.reserve(stops.len());
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
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;
        self.render_tile_spacing = self.tile_spacing.get_local().to_layout(final_size, ctx);
        self.render_tile_size = self.tile_size.get_local().to_layout(final_size, ctx);

        let (start, end, length) = gradient_ends_from_rad(*self.angle.get_local(), self.render_tile_size);
        self.render_start = start;
        self.render_end = end;
        self.render_stops
            .extend(self.stops.get_local().iter().map(|s| s.to_layout(length, ctx)));
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient_tile(
            LayoutRect::from_size(self.final_size),
            self.render_start,
            self.render_end,
            &self.render_stops,
            self.render_tile_size,
            self.render_tile_spacing,
        );
    }
}

/// Fill the widget area with a linear gradient.
pub fn fill_gradient(angle: impl IntoVar<AngleRadian>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    FillGradientNode {
        angle: angle.into_local(),
        stops: stops.into_local(),
        render_start: LayoutPoint::zero(),
        render_end: LayoutPoint::zero(),
        render_stops: vec![],
        final_size: LayoutSize::zero(),
    }
}

/// Fill the widget area with a linear gradient defined by two points relative to the widget top-left corner.
pub fn fill_gradient_points(
    start: impl IntoVar<Point>,
    end: impl IntoVar<Point>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    FillGradientPointsNode {
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

/// Fill the widget area with a linear gradient, from left to right.
pub fn fill_gradient_to_right(stops: impl IntoVar<GradientStops>) -> impl UiNode {
    fill_gradient_points(Point::zero(), Point::new(1.0.normal(), 0), stops, ExtendMode::Clamp)
}

/// Fill the widget area with a linear gradient, from right to left.
pub fn fill_gradient_to_left(stops: impl IntoVar<GradientStops>) -> impl UiNode {
    fill_gradient_points(Point::new(1.0.normal(), 0), Point::zero(), stops, ExtendMode::Clamp)
}

/// Fill the widget area with a linear gradient, from top to bottom.
pub fn fill_gradient_to_bottom(stops: impl IntoVar<GradientStops>) -> impl UiNode {
    fill_gradient_points(Point::zero(), Point::new(0, 1.0.normal()), stops, ExtendMode::Clamp)
}

/// Fill the widget area with a linear gradient, from bottom to top.
pub fn fill_gradient_to_top(stops: impl IntoVar<GradientStops>) -> impl UiNode {
    fill_gradient_points(Point::new(0, 1.0.normal()), Point::zero(), stops, ExtendMode::Clamp)
}

/// Fill the widget area with a linear gradient.
pub fn fill_gradient_tile(
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    FillGradientTileNode {
        angle: angle.into_local(),
        stops: stops.into_local(),
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

struct FillColorNode<C: VarLocal<Rgba>> {
    color: C,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<C: VarLocal<Rgba>> UiNode for FillColorNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.color.init_local(ctx.vars);
    }
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.color.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
    }
    fn arrange(&mut self, final_size: LayoutSize, _: &mut LayoutContext) {
        self.final_size = final_size;
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_color(LayoutRect::from_size(self.final_size), (*self.color.get_local()).into());
    }
}

/// Fill the widget area with a color.
pub fn fill_color(color: impl IntoVar<Rgba>) -> impl UiNode {
    FillColorNode {
        color: color.into_local(),
        final_size: LayoutSize::default(),
    }
}

/// Gradient stops for linear or radial gradients.
#[derive(Debug, Clone)]
pub struct GradientStops(pub Vec<GradientStop>);
impl std::ops::Deref for GradientStops {
    type Target = [GradientStop];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl_from_and_into_var! {
    fn from(stops: Vec<(Rgba, Length)>) -> GradientStops {
        GradientStops(stops.into_iter()
        .map(|(color, offset)| GradientStop {
            color,
            offset,
        })
        .collect())
    }

    /// Each item contains two color stops, with the same color.
    fn from(stops: Vec<(Rgba, Length, Length)>) -> GradientStops {{
        let mut r = Vec::with_capacity(stops.len() * 2);
        for (color, offset0, offset1) in stops {
            r.push(GradientStop {
                color,
                offset: offset0
            });
            r.push(GradientStop {
                color,
                offset: offset1
            });
        }
        GradientStops(r)
    }}

    /// Gradient stops that are all evenly spaced.
    fn from(stops: Vec<Rgba>) -> GradientStops {{
        let point = 1. / (stops.len() as f32 - 1.);
        GradientStops(stops.into_iter()
        .enumerate()
        .map(|(i, color)| GradientStop {
            offset: ((i as f32) * point).normal().into(),
            color,
        })
        .collect())
    }}

    /// A single two color gradient stops. The first color is at offset `0.0`,
    /// the second color is at offset `1.0`.
    fn from((stop0, stop1): (Rgba, Rgba)) -> GradientStops {
        GradientStops(vec![
            GradientStop { offset: 0.0.normal().into(), color: stop0 },
            GradientStop { offset: 1.0.normal().into(), color: stop1 },
        ])
    }

    fn from(stops: Vec<GradientStop>) -> GradientStops {
        GradientStops(stops)
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
