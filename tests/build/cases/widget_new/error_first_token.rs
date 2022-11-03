use zero_ui::properties::states::is_pressed;
use zero_ui::widgets::blank;

fn test_1() {
    let _ = blank! {
        =
    };
}

fn test_2() {
    let _ = blank! {
        when *#is_pressed {
            =
        }
    };
}

fn main() {
    test_1();
    test_2();
}
