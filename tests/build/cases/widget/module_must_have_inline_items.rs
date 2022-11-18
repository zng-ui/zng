use zero_ui::core::widget;

#[widget($crate::test_widget)]
#[path = "util/a_mod.rs"]
pub mod test_widget;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    test_widget::mod_exists();
}
