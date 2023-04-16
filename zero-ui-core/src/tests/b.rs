#[crate::widget($crate::tests::FooB)]
pub struct Foo(crate::widget_base::WidgetBase);
impl Foo {
    pub fn widget_build(&mut self) -> &'static str {
        "b"
    }
}
