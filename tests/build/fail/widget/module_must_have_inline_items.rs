use zero_ui::core::widget2;

#[widget2($crate::widget)]
#[path = "util/a_mod.rs"]
pub mod widget;

fn main() {
    widget::mod_exists();
}
