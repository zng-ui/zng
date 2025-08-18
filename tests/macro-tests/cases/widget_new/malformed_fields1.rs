use zng::{APP, layout::margin, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    #[rustfmt::skip]
    let _ = Wgt! {
        margin = {
            margin: 0;
        };
    };
}
