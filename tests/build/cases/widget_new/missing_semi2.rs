use zero_ui::core::window::CursorIcon;
use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::blank;

fn main() {
    let margin = 0;
    let _ = blank! {
        margin
        // we expect this properties to be used.
        enabled = true;
        cursor = CursorIcon::Hand;
    };
}
