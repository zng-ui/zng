use zero_ui::core::widget_mixin;

#[widget_mixin($crate::test_mixin)]
pub mod test_mixin {}

fn main() {
    let _ = test_mixin!();
}
