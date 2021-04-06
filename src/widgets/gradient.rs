use crate::core::gradient::*;
use crate::prelude::new_widget::*;

/// Linear gradient with a line defined by angle or points.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp).
pub fn linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Clamp)
}

/// Linear gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Repeat)
}

/// Linear gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Reflect)
}

/// Linear gradient with extend mode configurable.
pub fn linear_gradient_ext(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    LinearGradientNode::new(axis.into_local(), stops.into_local(), extend_mode.into_local())
}

/// Linear gradient with all features configurable.
pub fn linear_gradient_full(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    LinearGradientFullNode {
        g: LinearGradientNode::new(axis.into_local(), stops.into_local(), extend_mode.into_local()),
        tile_size: tile_size.into_local(),
        tile_spacing: tile_spacing.into_local(),
        render_tile_size: LayoutSize::zero(),
        render_tile_spacing: LayoutSize::zero(),
    }
}

struct LinearGradientNode<A, S, E> {
    axis: A,
    stops: S,
    extend_mode: E,

    render_line: LayoutLine,
    render_stops: Vec<RenderGradientStop>,

    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A, S, E> LinearGradientNode<A, S, E>
where
    A: VarLocal<LinearGradientAxis>,
    S: VarLocal<GradientStops>,
    E: VarLocal<ExtendMode>,
{
    fn new(axis: A, stops: S, extend_mode: E) -> Self {
        Self {
            axis,
            stops,
            extend_mode,
            render_line: LayoutLine::zero(),
            render_stops: vec![],
            final_size: LayoutSize::zero(),
        }
    }

    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.axis.init_local(ctx.vars);
        self.extend_mode.init_local(ctx.vars);
        self.stops.init_local(ctx.vars);
    }
    #[UiNode]
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
    }
    #[UiNode]
    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;
        self.render_line = self.axis.get_local().layout(final_size, ctx);

        let length = self.render_line.length();

        self.stops.get_local().layout_linear(
            length,
            ctx,
            *self.extend_mode.get_local(),
            &mut self.render_line,
            &mut self.render_stops,
        );
    }
    #[UiNode]
    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            LayoutRect::from_size(self.final_size),
            self.render_line,
            &self.render_stops,
            (*self.extend_mode.get_local()).into(),
            self.final_size,
            LayoutSize::zero(),
        );
    }
}

struct LinearGradientFullNode<A, S, E, T, TS> {
    g: LinearGradientNode<A, S, E>,

    tile_size: T,
    tile_spacing: TS,

    render_tile_size: LayoutSize,
    render_tile_spacing: LayoutSize,
}

#[impl_ui_node(none)]
impl<A, S, E, T, TS> UiNode for LinearGradientFullNode<A, S, E, T, TS>
where
    A: VarLocal<LinearGradientAxis>,
    S: VarLocal<GradientStops>,
    E: VarLocal<ExtendMode>,
    T: VarLocal<Size>,
    TS: VarLocal<Size>,
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.g.init(ctx);
        self.tile_size.init_local(ctx.vars);
        self.tile_spacing.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.g.update(ctx);

        if self.tile_size.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.tile_spacing.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.render_tile_size = self.tile_size.get_local().to_layout(final_size, ctx);
        self.render_tile_spacing = self.tile_spacing.get_local().to_layout(final_size, ctx);
        self.g.arrange(self.render_tile_size, ctx);
        self.g.final_size = final_size;
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            LayoutRect::from_size(self.g.final_size),
            self.g.render_line,
            &self.g.render_stops,
            (*self.g.extend_mode.get_local()).into(),
            self.render_tile_size,
            self.render_tile_spacing,
        );
    }
}
