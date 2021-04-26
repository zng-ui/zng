use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{units::SideOffsets, var::IntoVar, NilUiNode};

    properties! {
        margin = 0;
    }

    fn new_child(margin: impl IntoVar<SideOffsets>) -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
