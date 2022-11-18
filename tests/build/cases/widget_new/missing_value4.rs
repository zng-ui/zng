use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        cursor =
        // we expect these properties to be used.
        margin = 0;
        enabled = true;
    };
}
