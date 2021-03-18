use zero_ui::core::{property, UiNode};

struct PrivateFoo;

#[property(context, allowed_in_when = false)]
pub fn bar(child: impl UiNode, value: PrivateFoo) -> impl UiNode {
    let _ = value;
    child
}

fn main() {}
