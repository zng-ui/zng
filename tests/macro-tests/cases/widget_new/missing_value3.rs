use zng::{APP, layout::margin, mouse::cursor, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    #[rustfmt::skip]
    let _ = Wgt! {
        // we expected an error here.
        cursor = ;
        // we expect margin to be used here.
        margin = 0;
    };
}
