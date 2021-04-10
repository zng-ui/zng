use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{units::SideOffsets, var::IntoVar, NilUiNode};
    use zero_ui::properties::margin;

    properties! {
        remove { margin }
    }

    fn new_child(margin: impl IntoVar<SideOffsets>) -> NilUiNode {
        let _ = margin;
        NilUiNode
    }
}

fn main() {}
