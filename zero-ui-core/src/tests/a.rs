#[crate::widget($crate::tests::foa)]
pub mod foo {
    use crate::{UiNode, WidgetId};

    fn new(child: impl UiNode, id: impl Into<WidgetId>) -> &'static str {
        let _ = child;
        let _ = id;
        "a"
    }
}
