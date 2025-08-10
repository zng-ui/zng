use zng::prelude_wgt::{hot_node, IntoVar, UiNode};

zng::hot_reload::zng_hot_entry!();

struct Foo;
impl Foo {
    #[hot_node]
    pub fn self_method1(self, input: impl IntoVar<bool>) -> UiNode {
        let _ = input;
        UiNode::nil()
    }

    #[hot_node]
    pub fn self_method2(self: Box<Self>, input: impl IntoVar<bool>) -> UiNode {
        let _ = input;
        UiNode::nil()
    }
}

fn main() {
    let _mtd_was_not_removed = Foo.self_method1(true);
    let _mtd_was_not_removed = Box::new(Foo).self_method2(true);
}
