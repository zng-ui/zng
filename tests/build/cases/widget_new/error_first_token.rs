use zero_ui::properties::states::is_pressed;
use zero_ui::widgets::wgt;

fn test_1() {
    let _ = Wgt! {
        =
    };
}

fn test_2() {
    let _ = Wgt! {
        when *#is_pressed {
            =
        }
    };
}

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    test_1();
    test_2();
}
