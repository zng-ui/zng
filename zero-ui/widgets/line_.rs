use crate::prelude::new_widget::*;

/// Draws a horizontal or vertical line.
#[widget($crate::widgets::line_w)]
pub mod line_w {
    use super::*;

    properties! {
        /// Line orientation.
        orientation(impl IntoVar<LineOrientation>) = LineOrientation::Horizontal;

        /// Line color.
        color(impl IntoVar<Rgba>) = rgb(0, 0, 0);

        /// Line stroke width.
        stroke_width(impl IntoVar<Length>) = 1;

        /// Line length.
        ///
        /// Set to [`Default`] to fill available length without requesting any length.
        ///
        /// [`Default`]: Length::Default
        length(impl IntoVar<Length>) = Length::Default;

        /// Line style.
        style(impl IntoVar<LineStyle>) = LineStyle::Solid;
    }

    fn new_child(
        orientation: impl IntoVar<LineOrientation>,
        length: impl IntoVar<Length>,
        stroke_width: impl IntoVar<Length>,
        color: impl IntoVar<Rgba>,
        style: impl IntoVar<LineStyle>,
    ) -> impl UiNode {
        LineNode {
            bounds: PxSize::zero(),
            orientation: orientation.into_var(),
            length: length.into_var(),
            stroke_width: stroke_width.into_var(),
            color: color.into_var(),
            style: style.into_var(),
        }
    }

    struct LineNode<W, L, O, C, S> {
        stroke_width: W,
        length: L,
        orientation: O,
        color: C,
        style: S,

        bounds: PxSize,
    }
    #[impl_ui_node(none)]
    impl<W, L, O, C, S> UiNode for LineNode<W, L, O, C, S>
    where
        W: Var<Length>,
        L: Var<Length>,
        O: Var<LineOrientation>,
        C: Var<Rgba>,
        S: Var<LineStyle>,
    {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.stroke_width.is_new(ctx) || self.length.is_new(ctx) || self.orientation.is_new(ctx) {
                ctx.updates.layout();
            }
            if self.color.is_new(ctx) || self.style.is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_space: AvailableSize) -> PxSize {
            let default_stroke = Dip::new(1).to_px(ctx.scale_factor.0);

            match *self.orientation.get(ctx) {
                LineOrientation::Horizontal => PxSize::new(
                    self.length.get(ctx).to_layout(ctx, available_space.width, Px(0)),
                    self.stroke_width.get(ctx).to_layout(ctx, available_space.height, default_stroke),
                ),
                LineOrientation::Vertical => PxSize::new(
                    self.stroke_width.get(ctx).to_layout(ctx, available_space.height, default_stroke),
                    self.length.get(ctx).to_layout(ctx, available_space.width, Px(0)),
                ),
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            let bounds = if self.length.get(ctx).is_default() {
                final_size
            } else {
                self.measure(ctx, AvailableSize::finite(final_size)).max(final_size)
            };

            if bounds != self.bounds {
                self.bounds = bounds;
                ctx.updates.render();
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let bounds = PxRect::from_size(self.bounds);
            let orientation = self.orientation.copy(ctx);
            let color = self.color.copy(ctx);
            let style = self.style.copy(ctx);
            frame.push_line(bounds, orientation, color.into(), style);
        }
    }
}

/// Draws a horizontal or vertical line.
pub fn line_w(
    orientation: impl IntoVar<LineOrientation> + 'static,
    color: impl IntoVar<Rgba> + 'static,
    style: impl IntoVar<LineStyle> + 'static,
) -> impl Widget {
    line_w! { orientation; color; style; }
}
