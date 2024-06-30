use zng::{prelude::*, prelude_wgt::*};

#[widget($crate::widgets::DemoEntry)]
pub struct DemoEntry(Container);

impl DemoEntry {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            grid::cell::at = grid::cell::AT_AUTO;

            layout::size = (150, 80);
            layout::align = Align::CENTER;

            tooltip = Tip!(Text!("tooltip position"));
            tip::tooltip_anchor = {
                let mut mode = AnchorMode::tooltip();
                mode.transform = layer::AnchorTransform::Cursor {
                    offset: layer::AnchorOffset::out_bottom_in_left(),
                    include_touch: true,
                    bounds: None,
                };
                mode
            };
            tip::tooltip_delay = 0.ms();

            layout::margin = 1;
            widget::background_color = light_dark(colors::WHITE, colors::BLACK);

            #[easing(150.ms())]
            text::font_color = light_dark(rgb(115, 115, 115), rgb(140, 140, 140));

            when *#gesture::is_hovered {
                #[easing(0.ms())]
                text::font_color = light_dark(colors::BLACK, colors::WHITE);
            }

            text::font_family = "monospace";
            text::font_size = 16;
            text::font_weight = FontWeight::BOLD;

            child_align = Align::TOP_LEFT;
            padding = (2, 5);
        }
    }
}
