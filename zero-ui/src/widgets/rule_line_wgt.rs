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
        LineNode {
            bounds: PxSize::zero(),
            orientation: orientation.into_var(),
            length: length.into_var(),
            stroke_thickness: stroke_thickness.into_var(),
            color: color.into_var(),
            style: style.into_var(),
        }
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.vars(ctx)
                .var(&self.stroke_thickness)
                .var(&self.length)
                .var(&self.orientation)
                .var(&self.color)
                .var(&self.style);
        }

        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if self.stroke_thickness.is_new(ctx) || self.length.is_new(ctx) || self.orientation.is_new(ctx) {
                ctx.updates.layout();
            }
            if self.color.is_new(ctx) || self.style.is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let default_stroke = Dip::new(1).to_px(ctx.scale_factor().0);

            match *self.orientation.get(ctx) {
                LineOrientation::Horizontal => PxSize::new(
                    self.length.get(ctx).layout(ctx.for_x(), |c| c.constrains().fill()),
                    self.stroke_thickness.get(ctx).layout(ctx.for_y(), |_| default_stroke),
                ),
                LineOrientation::Vertical => PxSize::new(
                    self.stroke_thickness.get(ctx).layout(ctx.for_x(), |_| default_stroke),
                    self.length.get(ctx).layout(ctx.for_y(), |c| c.constrains().fill()),
                ),
            }
        }
        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            let default_stroke = Dip::new(1).to_px(ctx.scale_factor().0);

            let bounds = match *self.orientation.get(ctx) {
                LineOrientation::Horizontal => PxSize::new(
                    self.length.get(ctx).layout(ctx.for_x(), |c| c.constrains().fill()),
                    self.stroke_thickness.get(ctx).layout(ctx.for_y(), |_| default_stroke),
                ),
                LineOrientation::Vertical => PxSize::new(
                    self.stroke_thickness.get(ctx).layout(ctx.for_x(), |_| default_stroke),
                    self.length.get(ctx).layout(ctx.for_y(), |c| c.constrains().fill()),
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
        color = vis::COLOR_VAR;

        /// Line stroke thickness.
        stroke_thickness  = vis::STROKE_THICKNESS_VAR;

        /// Line style.
        style = vis::STYLE_VAR;
    }

    /// Context variables and properties that affect the horizontal rule line appearance from parent widgets.
    pub mod vis {
        use super::*;
        use crate::widgets::text::properties::TEXT_COLOR_VAR;

        context_var! {
            /// Line color, inherits from [`TEXT_COLOR_VAR`].
            pub static COLOR_VAR: Rgba => TEXT_COLOR_VAR;

            /// Line stroke thickness, default is `1.dip()`
            pub static STROKE_THICKNESS_VAR: Length = 1.dip();

            /// Line style, default is `Solid`.
            pub static STYLE_VAR: LineStyle = LineStyle::Solid;
        }

        /// Sets the [`COLOR_VAR`] that affects all horizontal rules inside the widget.
        #[property(context, default(COLOR_VAR))]
        pub fn color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, COLOR_VAR, color)
        }

        /// Sets the [`STROKE_THICKNESS_VAR`] that affects all horizontal rules inside the widget.
        #[property(context, default(STROKE_THICKNESS_VAR))]
        pub fn stroke_thickness(child: impl UiNode, thickness: impl IntoVar<Length>) -> impl UiNode {
            with_context_var(child, STROKE_THICKNESS_VAR, thickness)
        }

        /// Sets the [`STYLE_VAR`] that affects all horizontal rules inside the widget.
        #[property(context, default(STYLE_VAR))]
        pub fn style(child: impl UiNode, style: impl IntoVar<LineStyle>) -> impl UiNode {
            with_context_var(child, STYLE_VAR, style)
        }
    }
}
