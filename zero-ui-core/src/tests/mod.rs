#![cfg(test)]

mod property;
mod widget;

mod a;
mod b;

pub use a::foo as foa;
pub use b::foo as fob;

#[test]
fn widget_macro_idents_are_unique() {
    // macro_rules! macros are declared in the crate root namespace, so if
    // we declare two widgets with the same name in the same crate there is
    // a conflict. This is resolved by generating an unique-id from the span.

    // This test asserts that even if two widgets with the same name and file span
    // are declared, there are still different because the file is different.

    let a = foa!();
    let b = fob!();

    assert_eq!("a", a);
    assert_eq!("b", b);
}
