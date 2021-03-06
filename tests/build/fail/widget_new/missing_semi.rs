use zero_ui::core::window::CursorIcon;
use zero_ui::properties::{cursor, drag_move::draggable, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        margin = 0
        // we expected this properties to be used.
        draggable = true;
        cursor = CursorIcon::Hand;
    };
}
