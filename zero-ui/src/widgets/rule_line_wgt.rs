use crate::prelude::new_widget::*;

/// Draws a horizontal or vertical rule line.
#[widget($crate::widgets::rule_line)]
pub mod rule_line {
    use super::*;

    properties! {
        /// Line orientation.
        orientation(impl IntoVar<LineOrientation>) = LineOrientation::Horizontal;

        /// Line color.
        color(impl IntoVar<Rgba>) = rgb(0, 0, 0);

        /// Line stroke thickness.
        stroke_thickness(impl IntoVar<Length>) = 1;

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
        stroke_thickness: impl IntoVar<Length>,
        color: impl IntoVar<Rgba>,
        style: impl IntoVar<LineStyle>,
    ) -> impl UiNode {
        let node = LineNode {
            bounds: PxSize::zero(),
            orientation: orientation.into_var(),
            length: length.into_var(),
            stroke_thickness: stroke_thickness.into_var(),
            color: color.into_var(),
            style: style.into_var(),
        };
        node
    }

    struct LineNode<W, L, O, C, S> {
        stroke_thickness: W,
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
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .vars(ctx)
                .var(&self.stroke_thickness)
                .var(&self.length)
                .var(&self.orientation)
                .var(&self.color)
                .var(&self.style);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.stroke_thickness.is_new(ctx) || self.length.is_new(ctx) || self.orientation.is_new(ctx) {
                ctx.updates.layout();
            }
            if self.color.is_new(ctx) || self.style.is_new(ctx) {
                ctx.updates.render(); // TODO !!: use render_update for color.
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let default_stroke = Dip::new(1).to_px(ctx.scale_factor().0);

            let bounds = match *self.orientation.get(ctx) {
                LineOrientation::Horizontal => PxSize::new(
                    self.length.get(ctx).layout(ctx.for_x(), |_| Px(0)),
                    self.stroke_thickness.get(ctx).layout(ctx.for_y(), |_| default_stroke),
                ),
                LineOrientation::Vertical => PxSize::new(
                    self.stroke_thickness.get(ctx).layout(ctx.for_x(), |_| default_stroke),
                    self.length.get(ctx).layout(ctx.for_y(), |_| Px(0)),
                ),
            };

            if bounds != self.bounds {
                self.bounds = bounds;
                ctx.updates.render();
            }

            bounds
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

/// Draws an horizontal [`rule_line!`](mod@rule_line).
#[widget($crate::widgets::hr)]
pub mod hr {
    use super::*;

    inherit!(rule_line);

    properties! {
        #[doc(hidden)]
        orientation = LineOrientation::Horizontal;

        /// Line color.
        color = theme::ColorVar;

        /// Line stroke thickness.
        stroke_thickness  = theme::StrokeThicknessVar;

        /// Line style.
        style = theme::StyleVar;
    }

    /// Context variables and properties that affect the horizontal rule line appearance from parent widgets.
    pub mod theme {
        use super::*;
        use crate::widgets::text::properties::TextColorVar;

        context_var! {
            /// Line color, default is the [`TextColorVar`] default.
            pub struct ColorVar: Rgba = TextColorVar::default_value();

            /// Line stroke thickness, default is `1.dip()`
            pub struct StrokeThicknessVar: Length = 1.dip();

            /// Line style, default is `Solid`.
            pub struct StyleVar: LineStyle = LineStyle::Solid;
        }

        /// Sets the [`ColorVar`] that affects all horizontal rules inside the widget.
        #[property(context, default(ColorVar))]
        pub fn color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, ColorVar, color)
        }

        /// Sets the [`StrokeThicknessVar`] that affects all horizontal rules inside the widget.
        #[property(context, default(StrokeThicknessVar))]
        pub fn stroke_thickness(child: impl UiNode, thickness: impl IntoVar<Length>) -> impl UiNode {
            with_context_var(child, StrokeThicknessVar, thickness)
        }

        /// Sets the [`StyleVar`] that affects all horizontal rules inside the widget.
        #[property(context, default(StyleVar))]
        pub fn style(child: impl UiNode, style: impl IntoVar<LineStyle>) -> impl UiNode {
            with_context_var(child, StyleVar, style)
        }
    }
}
