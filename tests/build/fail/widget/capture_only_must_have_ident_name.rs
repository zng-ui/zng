use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::{var::IntoVar, NilUiNode};

    properties! {
        foo as bar(impl IntoVar<bool>);
        zero_ui::properties::margin(impl IntoVar<bool>);
    }

    fn new_child(bar: impl IntoVar<bool>, margin: impl IntoVar<bool>) -> NilUiNode {
        let _ = (bar, margin);
        NilUiNode
    }
}

fn main() {}
