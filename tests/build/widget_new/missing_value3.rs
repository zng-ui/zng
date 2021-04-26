use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        // we expected an error here.
        cursor = ;
        // we expect margin to be used here.
        margin = 0;
    };
}
