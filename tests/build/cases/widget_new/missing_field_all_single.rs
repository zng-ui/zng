use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        // margin has one field
        // this is interpreted as an unnamed assign `{ }` is the value
        margin = { }
    };
}
