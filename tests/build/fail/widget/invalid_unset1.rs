use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::properties::margin;

    properties! {
        zero_ui::properties::cursor = unset!;
        margin as spacing = unset!;
        margin = unset!;
    }
}

fn main() {}
