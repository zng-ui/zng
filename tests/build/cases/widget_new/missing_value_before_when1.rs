use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        margin = // missing 0; here
        when *#is_pressed {
            margin = 20;
        }
    };
}
