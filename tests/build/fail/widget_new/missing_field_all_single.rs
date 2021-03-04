use zero_ui::properties::margin;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        // margin has one field
        // this is interpreted as an unnamed assign `{ }` is the value
        margin = { }
    };
}
