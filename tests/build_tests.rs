mod build_tests {
    use serial_test::serial;

    #[serial]
    #[test]
    fn impl_ui_node() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/impl_ui_node/*.rs");
    }

    #[serial]
    #[test]
    fn property() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/property/*.rs");
    }

    #[serial]
    #[test]
    fn widget_and_widget_mixin() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/widget/*.rs");
    }
    #[serial]
    #[test]
    fn widget_new() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/widget_new/*.rs");
    }
}

// `do test --build <test-pattern>` uses these to run specific tests.
#[test]
#[ignore]
fn do_tasks_test_runner() {
    use std::env;

    if let Some(test) = env::var_os("DO_TASKS_TEST_BUILD") {
        let test = test.to_string_lossy();
        let t = trybuild::TestCases::new();
        let path = format!("tests/build/{}.rs", test);
        t.compile_fail(path);
    }
}
