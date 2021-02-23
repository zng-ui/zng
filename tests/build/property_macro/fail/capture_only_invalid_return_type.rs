use zero_ui::core::property;

#[property(capture_only)]
pub fn invalid_return(input: bool) -> bool {
    input
}

#[property(capture_only)]
pub fn missing_return(input: bool) {
    let _ = input;
}

fn main() {}
