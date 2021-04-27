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
            orientation: orientation.into_local(),
            length: length.into_local(),
            width: width.into_local(),
            color: color.into_local(),
            style: style.into_local(),
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
        W: VarLocal<Length>,
        L: VarLocal<Length>,
        O: VarLocal<LineOrientation>,
        C: VarLocal<Rgba>,
        S: VarLocal<LineStyle>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.width.init_local(ctx.vars);
            self.length.init_local(ctx.vars);
            self.color.init_local(ctx.vars);
            self.orientation.init_local(ctx.vars);
            self.style.init_local(ctx.vars);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.width.update_local(ctx.vars).is_some()
                | self.length.update_local(ctx.vars).is_some()
                | self.orientation.update_local(ctx.vars).is_some()
            {
                ctx.updates.layout();
            }
            if self.color.update_local(ctx.vars).is_some() {
                ctx.updates.render();
            }
            if self.style.update_local(ctx.vars).is_some() {
                ctx.updates.render();
            }
        }

        fn measure(&mut self, available_space: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let (width, height) = match *self.orientation.get_local() {
                LineOrientation::Horizontal => (self.length.get_local(), self.width.get_local()),
                LineOrientation::Vertical => (self.width.get_local(), self.length.get_local()),
            };

            let width = width.to_layout(LayoutLength::new(available_space.width), ctx);
            let height = height.to_layout(LayoutLength::new(available_space.height), ctx);

            LayoutSize::new(width.0, height.0)
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.bounds = self.measure(final_size, ctx);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            let bounds = LayoutRect::from_size(self.bounds);
            let orientation = *self.orientation.get_local();
            let color = *self.color.get_local();
            let style = *self.style.get_local();
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
