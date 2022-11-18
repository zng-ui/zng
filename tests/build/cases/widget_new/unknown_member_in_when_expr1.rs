use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        margin = 0;
        when *#is_pressed.1 { // only .0 or .state allowed.
            margin = 1;
        }
    };
}
