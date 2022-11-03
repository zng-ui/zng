use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        margin = // missing 0; here
        when *#is_pressed {
            margin = 20;
        }
    };
}
