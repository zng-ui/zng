use zero_ui::core::window::CursorIcon;
use zero_ui::properties::{cursor, margin, states::is_pressed};
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        margin = 0;
        cursor = CursorIcon::Default;
        when *#is_pressed {
            margin =
            cursor = CursorIcon::Hand;
        }
    };
}
