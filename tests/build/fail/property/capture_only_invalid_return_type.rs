use zero_ui::core::{property, UiNode};

#[property(capture_only, allowed_in_when = false)]
pub fn invalid_return1(input: bool) -> bool {
    input
}

#[property(capture_only, allowed_in_when = false)]
pub fn invalid_return2(input: impl UiNode) -> impl UiNode {
    input
}

#[property(capture_only, allowed_in_when = false)]
pub fn missing_return(input: bool) {
    let _ = input;
}

fn main() {}
