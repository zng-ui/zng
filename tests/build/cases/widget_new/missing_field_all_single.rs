use zero_ui::properties::margin;
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        // margin has one field
        // this is interpreted as an unnamed assign `{ }` is the value
        margin = { }
    };
}
