use zero_ui::core::{property, widget, UiNode};

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

#[widget($crate::bar)]
pub mod bar {
    use super::*;

    properties! {
        foo = NotVarValue;

        // empty when should validate.
        when self.foo.is() { }
    }
}

fn main() {}
