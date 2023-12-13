use zero_ui::{layout::margin, mouse::cursor, widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        // we expected an error here.
        cursor = ;
        // we expect margin to be used here.
        margin = 0;
    };
}
