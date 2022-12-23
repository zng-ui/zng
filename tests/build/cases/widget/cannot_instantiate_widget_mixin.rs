use zero_ui::core::widget_mixin;

#[widget_mixin($crate::test_mixin)]
pub mod test_mixin {}

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = test_mixin!();
}
