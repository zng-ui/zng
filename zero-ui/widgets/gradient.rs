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
    LinearGradientNode::new(axis.into_var(), stops.into_var(), extend_mode.into_var())
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
        g: LinearGradientNode::new(axis.into_var(), stops.into_var(), extend_mode.into_var()),
        tile_size: tile_size.into_var(),
        tile_spacing: tile_spacing.into_var(),
        render_tile_size: PxSize::zero(),
        render_tile_spacing: PxSize::zero(),
    }
}

struct LinearGradientNode<A, S, E> {
    axis: A,
    stops: S,
    extend_mode: E,

    render_line: PxLine,
    render_stops: Vec<RenderGradientStop>,

    final_size: PxSize,
}
#[impl_ui_node(none)]
impl<A, S, E> LinearGradientNode<A, S, E>
where
    A: Var<LinearGradientAxis>,
    S: Var<GradientStops>,
    E: Var<ExtendMode>,
{
    fn new(axis: A, stops: S, extend_mode: E) -> Self {
        Self {
            axis,
            stops,
            extend_mode,
            render_line: PxLine::zero(),
            render_stops: vec![],
            final_size: PxSize::zero(),
        }
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.axis.is_new(ctx) || self.stops.is_new(ctx) || self.extend_mode.is_new(ctx) {
            ctx.updates.layout();
        }
    }
    #[UiNode]
    fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
        self.final_size = final_size;
        self.render_line = self.axis.get(ctx).layout(ctx, AvailableSize::finite(final_size));

        let length = self.render_line.length();

        self.stops.get(ctx).layout_linear(
            AvailablePx::Finite(length),
            ctx,
            *self.extend_mode.get(ctx),
            &mut self.render_line,
            &mut self.render_stops,
        );
    }
    #[UiNode]
    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            PxRect::from_size(self.final_size),
            self.render_line,
            &self.render_stops,
            (*self.extend_mode.get(ctx)).into(),
            self.final_size,
            PxSize::zero(),
        );
    }
}

struct LinearGradientFullNode<A, S, E, T, TS> {
    g: LinearGradientNode<A, S, E>,

    tile_size: T,
    tile_spacing: TS,

    render_tile_size: PxSize,
    render_tile_spacing: PxSize,
}

#[impl_ui_node(none)]
impl<A, S, E, T, TS> UiNode for LinearGradientFullNode<A, S, E, T, TS>
where
    A: Var<LinearGradientAxis>,
    S: Var<GradientStops>,
    E: Var<ExtendMode>,
    T: Var<Size>,
    TS: Var<Size>,
{
    fn update(&mut self, ctx: &mut WidgetContext) {
        self.g.update(ctx);

        if self.tile_size.is_new(ctx) || self.tile_spacing.is_new(ctx) {
            ctx.updates.layout();
        }
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
        let available_size = AvailableSize::finite(final_size);
        self.render_tile_size = self.tile_size.get(ctx).to_layout(ctx, available_size);
        self.render_tile_spacing = self.tile_spacing.get(ctx).to_layout(ctx, available_size);
        self.g.arrange(ctx, self.render_tile_size);
        self.g.final_size = final_size;
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            PxRect::from_size(self.g.final_size),
            self.g.render_line,
            &self.g.render_stops,
            self.g.extend_mode.copy(ctx).into(),
            self.render_tile_size,
            self.render_tile_spacing,
        );
    }
}
