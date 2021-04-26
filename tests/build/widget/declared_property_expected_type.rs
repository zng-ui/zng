use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    properties! {
        foo(10);
        bar(32) = 10;
    }
}

fn main() {}
