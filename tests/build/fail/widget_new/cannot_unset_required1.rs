use zero_ui::core::{property, widget, UiNode};

#[property(context, allowed_in_when = false)]
pub fn foo(child: impl UiNode, value: bool) -> impl UiNode {
    println!("{}", value);
    child
}

#[widget($crate::test_widget)]
pub mod test_widget {
    properties! {
        super::foo = required!;
    }
}

fn main() {
    let _ = test_widget! {
        foo = unset!;
    };
}
