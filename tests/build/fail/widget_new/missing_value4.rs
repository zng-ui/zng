use zero_ui::properties::{cursor, drag_move, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        cursor =
        margin = 0;// < the error spill-over should end here.
        // we expect drag_move to be used
        drag_move = true;
    };
}
