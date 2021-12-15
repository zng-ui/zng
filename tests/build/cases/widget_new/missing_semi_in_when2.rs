use zero_ui::core::window::CursorIcon;
use zero_ui::properties::{cursor, margin, states::is_pressed};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        margin = 0;
        cursor = CursorIcon::Default;
        when self.is_pressed {
            margin =
            cursor = CursorIcon::Hand;
        }
    };
}
