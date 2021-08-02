use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{units::SideOffsets, var::{IntoVar, IntoValue}, NilUiNode, UiNode, WidgetId};
    use zero_ui::properties::margin;

    properties! {
        margin = 1;
    }

    fn new<'a>(child: impl UiNode + 'a, id: impl IntoValue<WidgetId>) -> impl UiNode {
        child
    }
    fn new_child<'a>(margin: impl IntoVar<SideOffsets> + 'a) -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
