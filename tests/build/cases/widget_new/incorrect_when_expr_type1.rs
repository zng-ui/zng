use zero_ui::core::color::colors;
use zero_ui::properties::{background_color, margin};
use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        margin = 0;
        background_color = colors::BLACK;

        when *#margin {
            background_color = colors::WHITE;
        }
    };
}
