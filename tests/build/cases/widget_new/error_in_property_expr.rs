use zero_ui::properties::margin;
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        margin = {
            let _ = unknown::path();
            0
        }
    };
}
