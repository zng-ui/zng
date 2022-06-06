use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        cursor =
        // we expect these properties to be used.
        margin = 0;
        enabled = true;
    };
}
