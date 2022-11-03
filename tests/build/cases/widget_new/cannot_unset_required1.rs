use zero_ui::core::{property, widget, UiNode};

#[property(context)]
pub fn foo(child: impl UiNode, value: bool) -> impl UiNode {
    println!("{value}");
    child
}

#[widget($crate::test_widget)]
pub mod test_widget {
    properties! {
        #[required]
        super::foo;
    }
}

fn main() {
    let _ = test_widget! {
        foo = unset!;
    };
}
