use zero_ui::core::color::colors;
use zero_ui::properties::{background_color, margin};
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        margin = 0;
        background_color = colors::BLACK;

        when *#margin {
            background_color = colors::WHITE;
        }
    };
}
