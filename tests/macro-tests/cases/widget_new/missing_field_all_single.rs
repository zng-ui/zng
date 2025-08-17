use zng::{APP, layout::margin, widget::Wgt};

fn main() {
    let _scope = APP.minimal();
    let _ = Wgt! {
        // margin has one field
        // this is interpreted as an unnamed assign `{ }` is the value
        margin = {};
    };
}
