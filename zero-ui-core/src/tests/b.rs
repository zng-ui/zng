#[crate::widget($crate::tests::fob)]
pub mod foo {
    use crate::widget_builder::WidgetBuilder;

    fn build(builder: WidgetBuilder) -> &'static str {
        let _ = builder;
        "b"
    }
}
