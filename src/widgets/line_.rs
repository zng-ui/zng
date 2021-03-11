use crate::prelude::new_widget::*;
use crate::properties::capture_only::color;
use webrender_api as w_api;

pub use w_api::LineOrientation;

struct LineNode<W: Var<f32>, L: VarLocal<f32>, O: VarLocal<LineOrientation>, C: VarLocal<Rgba>, S: Var<LineStyle>> {
    width: W,
    length: L,
    orientation: O,
    color: C,
    style: S,
    render_command: RenderLineCommand,
    bounds: LayoutSize,
}
#[impl_ui_node(none)]
impl<W: Var<f32>, L: VarLocal<f32>, O: VarLocal<LineOrientation>, C: VarLocal<Rgba>, S: Var<LineStyle>> LineNode<W, L, O, C, S> {
    fn refresh(&mut self, ctx: &mut WidgetContext) {
        let length = *self.length.get(ctx.vars);
        let width = *self.width.get(ctx.vars);
        self.bounds = match *self.orientation.get(ctx.vars) {
            LineOrientation::Horizontal => LayoutSize::new(length, width),
            LineOrientation::Vertical => LayoutSize::new(width, length),
        };
        self.render_command = self.style.get(ctx.vars).render_command();
    }

    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.length.init_local(ctx.vars);
        self.color.init_local(ctx.vars);
        self.orientation.init_local(ctx.vars);
        self.refresh(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.width.is_new(ctx.vars) || self.length.update_local(ctx.vars).is_some() || self.orientation.update_local(ctx.vars).is_some()
        {
            self.refresh(ctx);
            ctx.updates.layout();
        } else if self.color.update_local(ctx.vars).is_some() || self.style.is_new(ctx.vars) {
            self.refresh(ctx);
            ctx.updates.render();
        }
    }

    #[UiNode]
    fn measure(&mut self, _: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        if is_layout_any_size(*self.length.get_local()) {
            // if length is infinity we use the available size.
            match *self.orientation.get_local() {
                LineOrientation::Vertical => {
                    self.bounds.height = 0.0;
                }
                LineOrientation::Horizontal => {
                    self.bounds.width = 0.0;
                }
            }
        }
        self.bounds = self.bounds.snap_to(ctx.pixel_grid());
        self.bounds
    }

    #[UiNode]
    fn arrange(&mut self, final_size: LayoutSize, _: &mut LayoutContext) {
        if is_layout_any_size(*self.length.get_local()) {
            match *self.orientation.get_local() {
                LineOrientation::Vertical => {
                    self.bounds.height = final_size.height;
                }
                LineOrientation::Horizontal => {
                    self.bounds.width = final_size.width;
                }
            }
        }
    }

    #[UiNode]
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
                let details = BorderDetails::new_all_same(BorderSide { color, style });

                frame.push_border(bounds, widths, details.into());
            }
        }
    }
}

/// Draws a horizontal or vertical line.
#[widget($crate::widgets::line_w)]
pub mod line_w {
    use super::*;

    properties! {
        /// Line orientation.
        line_orientation as orientation = LineOrientation::Horizontal;

        /// Line color.
        color = rgb(0, 0, 0);

        /// Line stroke thickness.
        width = 1.0;

        /// Line length.
        ///
        /// Set to `f32::INFINITY` to fill the available space.
        length = f32::INFINITY;

        /// Line style.
        line_style as style = LineStyle::Solid;
    }

    fn new_child(
        orientation: impl IntoVar<LineOrientation>,
        length: impl IntoVar<f32>,
        width: impl IntoVar<f32>,
        color: impl IntoVar<Rgba>,
        style: impl IntoVar<LineStyle>,
    ) -> impl UiNode {
        LineNode {
            bounds: LayoutSize::zero(),
            render_command: RenderLineCommand::Line(w_api::LineStyle::Solid, 0.0),
            orientation: orientation.into_local(),
            length: length.into_local(),
            width: width.into_var(),
            color: color.into_local(),
            style: style.into_var(),
        }
    }
}

/// Draws a horizontal or vertical line.
pub fn line_w(
    orientation: impl IntoVar<LineOrientation> + 'static,
    length: impl IntoVar<f32> + 'static,
    width: impl IntoVar<f32> + 'static,
    color: impl IntoVar<Rgba> + 'static,
    style: impl IntoVar<LineStyle> + 'static,
) -> impl Widget {
    line_w! { orientation; length; width; color; style; }
}

enum RenderLineCommand {
    Line(w_api::LineStyle, f32),
    Border(BorderStyle),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Groove,
    Ridge,
    Wavy(f32),
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
        }
    }
}
