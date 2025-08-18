use zng::{APP, gesture::is_pressed, widget::Wgt};

fn test_1() {
    #[rustfmt::skip]
    let _ = Wgt! {
        =
    };
}

fn test_2() {
    #[rustfmt::skip]
    let _ = Wgt! {
        when *#is_pressed {
            =
        }
    };
}

fn main() {
    let _scope = APP.minimal();
    test_1();
    test_2();
}
