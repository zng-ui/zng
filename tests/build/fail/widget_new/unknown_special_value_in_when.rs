use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        margin = 0;
        when self.is_pressed {
            margin = required!;
        }
    };

    let _ = blank! {
        margin = 0;
        when self.is_pressed {
            margin = foo!;
        }
    };
}
