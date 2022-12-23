use zero_ui::core::window::CursorIcon;
use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        margin = 0
        // we expect this properties to be used.
        enabled = true;
        cursor = CursorIcon::Hand;
    };
}
