use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    inherit!(super::not::a::thing);
}

fn main() {}
