use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::var::IntoVar;
    use zero_ui::core::NilUiNode;

    #[derive(Debug, Clone, Copy)]
    pub struct Foo;
    #[derive(Debug, Clone, Copy)]
    pub struct NotFoo;

    properties! {
        #[allowed_in_when = false]
        foo: Foo = Foo;
    }

    fn new_child(foo: NotFoo) -> NilUiNode {
        NilUiNode
    }
}

fn main() {}
