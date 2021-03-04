use zero_ui::properties::margin;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        // margin also gets highlighted here because generics? Good enough for now.
        margin = true
    };
}
