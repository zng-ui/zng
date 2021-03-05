use zero_ui::core::window::CursorIcon;
use zero_ui::properties::{cursor, drag_move, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        margin = 0
        drag_move = true;
        // we expected this property to be used.
        cursor = CursorIcon::Hand;
    };
}
