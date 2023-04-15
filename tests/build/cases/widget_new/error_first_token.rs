use zero_ui::prelude::*;

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
    let _scope = App::minimal();
    test_1();
    test_2();
}
