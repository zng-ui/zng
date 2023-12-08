use zero_ui_wgt::{is_disabled, prelude::*};
use zero_ui_wgt_access::*;
use zero_ui_wgt_filters::*;
use zero_ui_wgt_input::*;
use zero_ui_wgt_style::*;
use zero_ui_wgt_text::*;

/// Button link style.
///
/// Looks like a web hyperlink.
#[widget($crate::LinkStyle)]
pub struct LinkStyle(Style);
impl LinkStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            font_color = color_scheme_map(web_colors::LIGHT_BLUE, colors::BLUE);
            cursor = CursorIcon::Pointer;
            access_role = AccessRole::Link;

            when *#is_cap_hovered {
                underline = 1, LineStyle::Solid;
            }

            when *#is_pressed {
                font_color = color_scheme_map(colors::YELLOW, web_colors::BROWN);
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}
