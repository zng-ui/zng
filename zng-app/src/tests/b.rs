use zero_ui_app_proc_macros::widget;

#[widget($crate::tests::FooB)]
pub struct Foo(crate::widget::base::WidgetBase);
impl Foo {
    pub fn widget_build(&mut self) -> &'static str {
        "b"
    }
}
