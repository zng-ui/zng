use zng::{APP, gesture::is_pressed, layout::margin, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    #[rustfmt::skip]
    let _ = Wgt! {
        margin = 0 // missing ; here
        when *#is_pressed {
            margin = 20;
        }
    };
}
