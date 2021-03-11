use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{NilUiNode, UiNode, WidgetId};

    properties! {
        foo: bool;
    }

    fn new(child: impl UiNode, id: WidgetId, foo: bool) -> NilUiNode {
        println!("{}", foo);
        NilUiNode
    }
}

fn main() {
    let _ = test_widget! {
        // foo = true;
    };
}
