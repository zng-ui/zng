use zero_ui::core::{property, UiNode, NilUiNode};

struct Foo;
impl Foo {
    #[property(capture_only)]
    pub fn self_method1(self) -> ! { }

    #[property(context)]
    pub fn self_method2(self, input: bool) -> impl UiNode {
        NilUiNode
    }

    #[property(context)]
    pub fn self_method3(self: Box<Self>, input: bool) -> impl UiNode {
        NilUiNode
    }
}

fn main(){
    let _mtd_was_not_removed = Foo.self_method2(true);
    let _mtd_was_not_removed = Box::new(Foo).self_method3(true);
    let _mtd_was_not_removed = Foo.self_method1();
}