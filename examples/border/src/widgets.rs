use zng::{
    container::Container,
    prelude_wgt::*,
    widget::{self, border},
};

#[widget($crate::widgets::MrBorders)]
pub struct MrBorders(Container);
impl MrBorders {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            padding = 20;

            child_align = Align::CENTER;

            widget::background_color = web_colors::GREEN.darken(40.pct());

            border0 = 4, colors::WHITE.with_alpha(20.pct());
            border1 = 4, colors::BLACK.with_alpha(20.pct());
            border2 = 4, colors::WHITE.with_alpha(20.pct());

            widget::foreground_highlight = 3, 1, web_colors::ORANGE;

            widget::corner_radius = 20;
        }
    }
}

#[property(BORDER, default(0, BorderStyle::Hidden), widget_impl(MrBorders))]
pub fn border0(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> UiNode {
    border(child, widths, sides)
}
#[property(BORDER, default(0, BorderStyle::Hidden), widget_impl(MrBorders))]
pub fn border1(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> UiNode {
    border(child, widths, sides)
}
#[property(BORDER, default(0, BorderStyle::Hidden), widget_impl(MrBorders))]
pub fn border2(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> UiNode {
    border(child, widths, sides)
}
