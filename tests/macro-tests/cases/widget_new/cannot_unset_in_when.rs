use zng::{APP, gesture::is_pressed, layout::margin, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0;
        when *#is_pressed {
            margin = unset!;
        }
    };
}
