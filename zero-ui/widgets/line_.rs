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

        /// Line stroke thickness.
        width(impl IntoVar<Length>) = 1;

        /// Line length.
        length(impl IntoVar<Length>) = 100.pct();

        /// Line style.
        style(impl IntoVar<LineStyle>) = LineStyle::Solid;
    }

    fn new_child(
        orientation: impl IntoVar<LineOrientation>,
        length: impl IntoVar<Length>,
        width: impl IntoVar<Length>,
        color: impl IntoVar<Rgba>,
        style: impl IntoVar<LineStyle>,
    ) -> impl UiNode {
        LineNode {
            bounds: LayoutSize::zero(),
            orientation: orientation.into_var(),
            length: length.into_var(),
            width: width.into_var(),
            color: color.into_var(),
            style: style.into_var(),
        }
    }

    struct LineNode<W, L, O, C, S> {
        width: W,
        length: L,
        orientation: O,
        color: C,
        style: S,

        bounds: LayoutSize,
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
            if self.width.is_new(ctx) || self.length.is_new(ctx) || self.orientation.is_new(ctx) {
                ctx.updates.layout();
            }
            if self.color.is_new(ctx) || self.style.is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_space: LayoutSize) -> LayoutSize {
            let (width, height) = match *self.orientation.get(ctx) {
                LineOrientation::Horizontal => (self.length.get(ctx), self.width.get(ctx)),
                LineOrientation::Vertical => (self.width.get(ctx), self.length.get(ctx)),
            };

            let width = width.to_layout(LayoutLength::new(available_space.width), ctx);
            let height = height.to_layout(LayoutLength::new(available_space.height), ctx);

            LayoutSize::new(width.0, height.0)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            self.bounds = self.measure(ctx, final_size);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let bounds = LayoutRect::from_size(self.bounds);
            let orientation = *self.orientation.get(ctx);
            let color = *self.color.get(ctx);
            let style = *self.style.get(ctx);
            frame.push_line(bounds, orientation, color.into(), style);
        }
    }
}

/// Draws a horizontal or vertical line.
pub fn line_w(
    orientation: impl IntoVar<LineOrientation> + 'static,
    length: impl IntoVar<Length> + 'static,
    width: impl IntoVar<Length> + 'static,
    color: impl IntoVar<Rgba> + 'static,
    style: impl IntoVar<LineStyle> + 'static,
) -> impl Widget {
    line_w! { orientation; length; width; color; style; }
}
