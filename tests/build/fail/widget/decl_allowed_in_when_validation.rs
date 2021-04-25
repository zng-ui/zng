use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::NilUiNode;

    properties! {
        // #[allowed_in_when = false]
        foo(&'static str) = "";

        #[allowed_in_when = false]
        bar(&'static str) = "bar";
    }

    fn new_child(foo: &'static str, bar: &'static str) -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
