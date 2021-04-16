use zero_ui::core::widget;

#[widget($crate::foo_widget)]
pub mod foo_widget {
    use zero_ui::core::{var::IntoVar, NilUiNode};

    properties! {
        #[required]
        foo: impl IntoVar<u32>;
    }

    fn new_child(foo: impl IntoVar<u32>) -> NilUiNode {
        let _ = foo;
        NilUiNode
    }
}

#[widget($crate::margin_widget)]
pub mod margin_widget {
    use zero_ui::core::{units::SideOffsets, var::IntoVar, NilUiNode};
    use zero_ui::properties::margin;

    properties! {
        #[required]
        margin;
    }

    fn new_child(margin: impl IntoVar<SideOffsets>) -> NilUiNode {
        let _ = margin;
        NilUiNode
    }
}

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::NilUiNode;

    inherit!(super::foo_widget);
    inherit!(super::margin_widget);

    fn new_child() -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
