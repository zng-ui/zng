use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{units::SideOffsets, var::IntoVar, NilUiNode};
    properties! {
        foo(impl IntoVar<bool>),
        zero_ui::properties::margin = 10;
    }

    fn new_child(foo: impl IntoVar<bool>, margin: impl IntoVar<SideOffsets>) -> NilUiNode {
        let _ = (foo, margin);
        NilUiNode
    }
}

fn main() {}
