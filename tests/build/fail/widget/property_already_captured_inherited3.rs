use zero_ui::core::widget;

#[widget($crate::base_widget)]
pub mod base_widget {
    use zero_ui::core::{units::SideOffsets, var::IntoVar, NilUiNode};
    use zero_ui::properties::margin;

    properties! {
        margin
    }

    fn new_child(margin: impl IntoVar<SideOffsets>) -> NilUiNode {
        let _ = margin;
        NilUiNode
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{units::SideOffsets, var::IntoVar, UiNode};

    inherit!(super::base_widget);

    fn new(child: impl UiNode, margin: impl IntoVar<SideOffsets>) {
        let _ = (child, margin);
    }
}

fn main() {}
