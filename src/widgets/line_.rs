use crate::core::context::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::var::*;
use crate::core::{impl_ui_node, profiler::profile_scope, widget, UiNode};
use webrender::api::BorderStyle as WrBorderStyle;
use webrender::api::LineStyle as WrLineStyle;

struct Line<W: Var<f32>, L: Var<f32>, O: Var<LineOrientation>, C: Var<ColorF>, S: Var<LineStyle>> {
    width: W,
    length: L,
    orientation: O,
    color: C,
    style: S,
    render_command: RenderLineCommand,
    bounds: LayoutSize,
}

#[impl_ui_node(none)]
impl<W: Var<f32>, L: Var<f32>, O: Var<LineOrientation>, C: Var<ColorF>, S: Var<LineStyle>> Line<W, L, O, C, S> {
    fn refresh(&mut self, ctx: &mut WidgetContext) {
        let length = *self.length.get(ctx.vars);
        let width = *self.width.get(ctx.vars);
        self.bounds = match *self.orientation.get(ctx.vars) {
            LineOrientation::Horizontal => LayoutSize::new(length, width),
            LineOrientation::Vertical => LayoutSize::new(width, length),
        };
    }

    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.refresh(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.width.update(ctx.vars).is_some() || self.length.update(ctx.vars).is_some() || self.orientation.update(ctx.vars).is_some() {
            self.refresh(ctx);
            ctx.updates.push_layout();
        } else if self.color.update(ctx.vars).is_some() || self.style.update(ctx.vars).is_some() {
            self.refresh(ctx);
            ctx.updates.push_render();
        }
    }

    #[UiNode]
    fn measure(&mut self, _: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        self.bounds = self.bounds.snap_to(pixels);
        self.bounds
    }

    #[UiNode]
    fn render(&self, frame: &mut FrameBuilder) {
        let bounds = LayoutRect::from_size(self.bounds);
        match self.render_command {
            RenderLineCommand::Line(style, thickness) => { /* frame.push_line(bounds, orientation, color, style, wavy_line_thickness) */ }
            RenderLineCommand::Border(_) => { /* frame.push_border(bounds, widths, details) */ }
        }
    }
}

enum RenderLineCommand {
    Line(WrLineStyle, f32),
    Border(WrBorderStyle),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum LineStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Groove,
    Ridge,
    Inset,
    Outset,
    Wavy(f32),
}

impl LineStyle {
    fn render_command(self) -> RenderLineCommand {
        use RenderLineCommand::*;
        match self {
            LineStyle::Solid => Line(WrLineStyle::Solid, 0.0),
            LineStyle::Double => Border(WrBorderStyle::Double),
            LineStyle::Dotted => Line(WrLineStyle::Dotted, 0.0),
            LineStyle::Dashed => Line(WrLineStyle::Dashed, 0.0),
            LineStyle::Groove => Border(WrBorderStyle::Groove),
            LineStyle::Ridge => Border(WrBorderStyle::Ridge),
            LineStyle::Inset => Border(WrBorderStyle::Inset),
            LineStyle::Outset => Border(WrBorderStyle::Outset),
            LineStyle::Wavy(thickness) => Line(WrLineStyle::Wavy, thickness),
        }
    }
}
