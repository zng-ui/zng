use zng::prelude_wgt::property;

#[property(CONTEXT)]
pub struct Foo {}

#[property(CONTEXT)]
pub mod bar {
    pub fn baz() {}
}

fn main() {
    let _ = Foo {};
    bar::baz();
}
