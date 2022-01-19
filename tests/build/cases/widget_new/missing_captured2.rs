use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::UiNode;

    properties! {
        #[allowed_in_when = false]
        foo(bool);
    }

    fn new_outer(child: impl UiNode, foo: bool) -> impl UiNode {
        println!("{foo}");
        child
    }
}

fn main() {
    let _ = test_widget! {
        // foo = true;
    };
}
