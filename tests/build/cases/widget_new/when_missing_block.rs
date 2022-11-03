use zero_ui::properties::states::is_pressed;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        when *#is_pressed
    };
}
