mod tests {
    use serial_test::serial;

    #[serial]
    #[test]
    fn property_macro_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/property_macro/fail/*.rs");
    }

    #[serial]
    #[test]
    fn property_macro_pass() {
        let t = trybuild::TestCases::new();
        t.pass("tests/build/property_macro/pass/*.rs");
    }

    #[serial]
    #[test]
    fn widget_macro() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/widget_macro/*.rs");
    }
}
