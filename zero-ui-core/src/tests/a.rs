#[crate::widget($crate::tests::foa)]
pub mod foo {
    use crate::widget_builder::WidgetBuilder;

    fn build(builder: WidgetBuilder) -> &'static str {
        let _ = builder;
        "a"
    }
}
