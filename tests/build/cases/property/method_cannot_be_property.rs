use zero_ui::core::{
    property,
    var::IntoVar,
    widget_instance::{NilUiNode, UiNode},
};

struct Foo;
impl Foo {
    #[property(context)]
    pub fn self_method1(self, input: impl IntoVar<bool>) -> impl UiNode {
        NilUiNode
    }

    #[property(context)]
    pub fn self_method2(self: Box<Self>, input: impl IntoVar<bool>) -> impl UiNode {
        NilUiNode
    }
}

fn main() {
    let _mtd_was_not_removed = Foo.self_method1(true);
    let _mtd_was_not_removed = Box::new(Foo).self_method2(true);
}
