use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        margin = 0;
        when {
            let a: u32 = true;
            *#is_pressed
        } {
            margin = 10;
        }
    };
}
