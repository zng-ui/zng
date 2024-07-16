use zng::{layout::margin, widget::Wgt, APP};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        // margin has one field
        // this is interpreted as an unnamed assign `{ }` is the value
        margin = {}
    };
}
