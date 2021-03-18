use zero_ui::core::{property, UiNode};
use zero_ui::widgets::blank;

pub struct NotVarValue;
impl NotVarValue {
    fn is(&self) -> bool {
        true
    }
}

#[property(context, allowed_in_when = false)]
pub fn foo(child: impl UiNode, value: NotVarValue) -> impl UiNode {
    let _ = value;
    child
}

fn main() {
    let _ = blank! {
        foo = NotVarValue;
        // empty when should validate.
        when self.foo.is() { }
    };
}
