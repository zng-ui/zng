use zero_ui::core::{widget, window::CursorIcon};
use zero_ui::properties::{cursor, states::is_pressed};

#[widget($crate::foo)]
pub mod foo {
    use zero_ui::properties::margin;

    properties! {
        margin = 10;
    }
}

use CursorIcon::Hand; // < no unused warning here

fn main() {
    let _ = foo! {
        margin = unset!;
        cursor = CursorIcon::Default;
        when self.is_pressed {
            margin = 5;// < error here
            cursor = Hand;// < when still included
        }
    };
}
