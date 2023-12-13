use zero_ui::{layout::margin, widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        margin = {
            margin: 0;
        };
    };
}
