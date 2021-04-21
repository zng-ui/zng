use zero_ui::core::widget;

#[widget($crate::base_widget)]
pub mod base_widget {
    use zero_ui::core::var::IntoVar;
    use zero_ui::core::NilUiNode;

    properties! {
        foo: impl IntoVar<bool>;
    }

    fn new_child(foo: impl IntoVar<bool>) -> NilUiNode {
        let _ = foo;
        NilUiNode
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(crate::base_widget);

    use zero_ui::core::NilUiNode;

    fn new_child() -> NilUiNode {
        NilUiNode
    }
}

fn main() {
    let _ok = test_widget!();
    let _er = test_widget! {
        // expect property not found
        foo = true;
    };
}
