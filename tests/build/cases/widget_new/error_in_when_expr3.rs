use zero_ui::{layout::margin, widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = 0;
        when *#margin.0. {
            margin = 10;
        }
    };
}
