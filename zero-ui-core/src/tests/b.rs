#[crate::widget($crate::tests::FooB)]
pub struct Foo(crate::widget_base::WidgetBase);
impl Foo {
    pub fn build(&mut self) -> &'static str {
        "b"
    }
}
