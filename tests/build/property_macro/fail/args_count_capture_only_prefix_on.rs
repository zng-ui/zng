use zero_ui::core::property;

#[property(capture_only)]
pub fn on_event_no_inputs() -> ! {}

#[property(capture_only)]
pub fn on_event_two_inputs(input1: bool, input2: bool) -> ! {}

fn main() {}
