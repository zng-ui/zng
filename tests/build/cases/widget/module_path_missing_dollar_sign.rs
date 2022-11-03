use zero_ui::core::widget;

// doesn't start with $
#[widget(crate::widget)]
pub mod test_widget {
    inherit!(zero_ui::core::widget_base::base);
}

fn main() {}
