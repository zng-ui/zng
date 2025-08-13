use zng::{APP, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        unknown = { value: 0 };
    };
}
