use zero_ui::core::property;

#[property(context)]
pub struct Foo {}

#[property(context)]
pub mod bar {
    pub fn baz() {}
}

fn main() {
    let _ = Foo {};
    bar::baz();
}
