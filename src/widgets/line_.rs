use crate::prelude::new_widget::*;
use webrender_api as w_api;

pub use w_api::LineOrientation;

/// Draws a horizontal or vertical line.
#[widget($crate::widgets::line_w)]
pub mod line_w {
    use super::*;

    properties! {
        /// Line orientation.
        orientation { impl IntoVar<LineOrientation> } = LineOrientation::Horizontal;

        /// Line color.
        color { impl IntoVar<Rgba> } = rgb(0, 0, 0);

        /// Line stroke thickness.
        width { impl IntoVar<Length> } = 1;

        /// Line length.
        length { impl IntoVar<Length> } = 100.pct();

        /// Line style.
        style { impl IntoVar<LineStyle> } = LineStyle::Solid;
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
            render_command: RenderLineCommand::Line(w_api::LineStyle::Solid, 0.0),
            orientation: orientation.into_local(),
            length: length.into_local(),
            width: width.into_local(),
            color: color.into_local(),
            style: style.into_var(),
        }
    }

    struct LineNode<W, L, O, C, S> {
        width: W,
        length: L,
        orientation: O,
        color: C,
        style: S,

        render_command: RenderLineCommand,
        bounds: LayoutSize,
    }
    #[impl_ui_node(none)]
    impl<W, L, O, C, S> UiNode for LineNode<W, L, O, C, S>
    where
        W: VarLocal<Length>,
        L: VarLocal<Length>,
        O: VarLocal<LineOrientation>,
        C: VarLocal<Rgba>,
        S: Var<LineStyle>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.width.init_local(ctx.vars);
            self.length.init_local(ctx.vars);
            self.color.init_local(ctx.vars);
            self.orientation.init_local(ctx.vars);
            self.render_command = self.style.get(ctx.vars).render_command();
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
            if let Some(style) = self.style.get_new(ctx.vars) {
                self.render_command = style.render_command();
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
            match self.render_command {
                RenderLineCommand::Line(style, thickness) => frame.push_line(bounds, orientation, &color.into(), style, thickness),
                RenderLineCommand::Border(style) => {
                    let widths = match orientation {
                        LineOrientation::Vertical => LayoutSideOffsets::new(0.0, 0.0, 0.0, self.bounds.width),
                        LineOrientation::Horizontal => LayoutSideOffsets::new(self.bounds.height, 0.0, 0.0, 0.0),
                    };
                    let details = BorderDetails::new_all(BorderSide { color, style });

                    frame.push_border(bounds, widths, details.into());
                }
            }
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

enum RenderLineCommand {
    Line(w_api::LineStyle, f32),
    Border(BorderStyle),
}

/// Represents a line style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineStyle {
    /// A solid line.
    Solid,
    /// Two solid lines in parallel.
    Double,

    /// Dotted line.
    Dotted,
    /// Dashed line.
    Dashed,

    /// Faux shadow with carved appearance.
    Groove,
    /// Faux shadow with extruded appearance.
    Ridge,

    /// A wavy line, like an error underline.
    ///
    /// The wave magnitude is defined by the overall line thickness, the associated value
    /// here defines the thickness of the wavy line.
    Wavy(f32),

    /// Fully transparent line.
    Hidden,
}

impl LineStyle {
    fn render_command(self) -> RenderLineCommand {
        use RenderLineCommand::*;
        match self {
            LineStyle::Solid => Line(w_api::LineStyle::Solid, 0.0),
            LineStyle::Double => Border(BorderStyle::Double),
            LineStyle::Dotted => Line(w_api::LineStyle::Dotted, 0.0),
            LineStyle::Dashed => Line(w_api::LineStyle::Dashed, 0.0),
            LineStyle::Groove => Border(BorderStyle::Groove),
            LineStyle::Ridge => Border(BorderStyle::Ridge),
            LineStyle::Wavy(thickness) => Line(w_api::LineStyle::Wavy, thickness),
            LineStyle::Hidden => Border(BorderStyle::Hidden),
        }
    }
}
