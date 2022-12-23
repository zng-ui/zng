use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        cursor =
        // we expect these properties to be used.
        margin = 0;
        enabled = true;
    };
}
