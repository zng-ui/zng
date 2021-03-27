use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::NilUiNode;
    properties! {
        foo: bool,
        zero_ui::properties::margin = 10;
    }

    fn new_child(foo: (bool, bool)) -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
