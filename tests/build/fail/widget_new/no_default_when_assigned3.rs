use zero_ui::core::window::CursorIcon;
use zero_ui::properties::cursor;
use zero_ui::widgets::blank;

fn main() {
    let _ = blank! {
        when self.cursor == CursorIcon::Hand { }
    };
}
