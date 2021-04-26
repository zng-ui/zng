use zero_ui::core::{property, UiNode};

pub struct NoDebug;

// ok
#[property(context, allowed_in_when = false)]
pub fn foo(child: impl UiNode, foo: NoDebug) -> impl UiNode {
    let _ = foo;
    child
}

// error, but should only highlight NoDebug
#[property(context)]
pub fn bar(child: impl UiNode, bar: NoDebug) -> impl UiNode {
    let _ = bar;
    child
}

fn main() {}
