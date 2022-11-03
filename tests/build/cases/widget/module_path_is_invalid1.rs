use zero_ui::core::widget;

#[widget($crate::not::a::valid::path)]
pub mod test_widget {
    inherit!(zero_ui::core::widget_base::base);
}

fn main() {}
