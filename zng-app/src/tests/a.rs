use zng_app_proc_macros::widget;

#[widget($crate::tests::FooA)]
pub struct Foo(crate::widget::base::WidgetBase);
impl Foo {
    pub fn widget_build(&mut self) -> &'static str {
        "a"
    }
}
