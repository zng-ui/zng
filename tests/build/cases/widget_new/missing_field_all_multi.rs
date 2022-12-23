use zero_ui::properties::background_gradient;
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        // background_gradient has two fields
        // this is interpreted as an unnamed assign `{ }` is the value
        // and the second value is missing
        background_gradient = { }
    };
}
