#[test]
fn property_macro_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/build/property_macro/fail/*.rs");
}

#[test]
fn property_macro_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/build/property_macro/pass/*.rs");
}

#[test]
fn widget_macro() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/build/widget_macro/*.rs");
}
