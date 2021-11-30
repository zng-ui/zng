#[crate::widget($crate::tests::fob)]
pub mod foo {
    use crate::{var::IntoValue, UiNode, WidgetId};

    fn new(child: impl UiNode, id: impl IntoValue<WidgetId>) -> &'static str {
        let _ = child;
        let _ = id;
        "b"
    }
}
