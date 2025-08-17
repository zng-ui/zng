use zng::{APP, gesture::is_pressed, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    #[rustfmt::skip]
    let _ = Wgt! {
        when *#is_pressed
    };
}
