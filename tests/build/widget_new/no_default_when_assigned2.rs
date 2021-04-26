use zero_ui::core::window::CursorIcon;
use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        margin = 10;
        when self.cursor == CursorIcon::Hand {
            margin = 20;
        }
    };
}
