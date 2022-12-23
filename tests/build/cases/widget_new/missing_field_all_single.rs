use zero_ui::properties::margin;
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        // margin has one field
        // this is interpreted as an unnamed assign `{ }` is the value
        margin = { }
    };
}
