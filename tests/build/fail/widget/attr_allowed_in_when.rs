use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::NilUiNode;
    use zero_ui::properties::margin;

    properties! {
        #[allowed_in_when]
        foo: bool;

        #[allowed_in_when(false)]
        bar: bool;

        #[allowed_in_when: false]
        baz: bool;

        #[allowed_in_when =]
        qux: bool;

        #[allowed_in_when = 0]
        quux: bool;

        margin;
    }

    fn new_child(foo: bool, bar: bool, baz: bool, qux: bool, quux: bool) -> NilUiNode {
        let _ = (foo, bar, baz, qux, quux);
        NilUiNode
    }
}

fn main() {
    // properties still declared.
    let _ = test_widget! {
        foo = true;
        bar = true;
        baz = true;
        qux = true;
        quux = true;
        margin = 10;
    };
}
