use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        cursor =
        // we expect these properties to be used.
        margin = 0;
        enabled = true;
    };
}
