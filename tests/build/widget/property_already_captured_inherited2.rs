use zero_ui::core::widget;

#[wdiget($crate::base_widget)]
pub mod base_widget {}

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{NilUiNode, WidgetId};

    inherit!(super::base_widget);

    fn new_child(id: WidgetId) -> NilUiNode {
        let _ = id;
        NilUiNode
    }
}

fn main() {}
