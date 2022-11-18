use zero_ui::properties::states::is_pressed;
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        when *#is_pressed
    };
}
