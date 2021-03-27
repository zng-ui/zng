use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::NilUiNode;

    properties! {
        foo := 10;
    }

    fn new_child(foo: bool) -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
