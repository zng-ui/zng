use zero_ui::properties::background::background_gradient;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        // background_gradient has two fields
        // this is interpreted as an unnamed assign `{ }` is the value
        // and the second value is missing
        background_gradient = { }
    };
}
