use zero_ui::core::window::CursorIcon;
use zero_ui::properties::{cursor, margin, states::is_pressed};
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        margin = 0;
        cursor = CursorIcon::Default;
        when *#is_pressed {
            margin =
            cursor = CursorIcon::Hand;
        }
    };
}
