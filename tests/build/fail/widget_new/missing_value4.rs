use zero_ui::properties::{cursor, drag_move::draggable, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        cursor =
        // we expect these properties to be used.
        margin = 0;
        draggable = true;
    };
}
