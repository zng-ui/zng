use zng::{APP, gesture::is_pressed, layout::margin, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    #[rustfmt::skip]
    let _ = Wgt! {
        margin = // missing 0; here
        when *#is_pressed {
            margin = 20;
        }
    };
}
