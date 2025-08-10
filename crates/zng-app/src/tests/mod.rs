#![cfg(test)]

mod widget;

mod a;
mod b;
mod ui_node_list;

pub use a::Foo as FooA;
pub use b::Foo as FooB;

#[test]
fn widget_macro_idents_are_unique() {
    // macro_rules! macros are declared in the crate root namespace, so if
    // we declare two widgets with the same name in the same crate there is
    // a conflict. This is resolved by generating an unique-id from the span.

    // This test asserts that even if two widgets with the same name and file span
    // are declared, there are still different because the file is different.

    let a = FooA!();
    let b = FooB!();

    assert_eq!("a", a);
    assert_eq!("b", b);
}
