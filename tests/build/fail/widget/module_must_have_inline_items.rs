use zero_ui::core::widget;

#[widget($crate::widget)]
#[path = "util/a_mod.rs"]
pub mod widget;

fn main() {
    widget::mod_exists();
}
