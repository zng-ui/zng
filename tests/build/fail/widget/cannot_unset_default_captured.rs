use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {

    properties! {
        id = unset!;
    }
}

fn main() {}
