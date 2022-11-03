use zero_ui::core::property;
use zero_ui::widgets::blank;

#[property(capture_only)]
pub fn foo(value: bool) -> ! {}

fn main() {
    let _ = blank! {
        foo = true;
    };
}
