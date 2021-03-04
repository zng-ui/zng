mod build_tests {
    use serial_test::serial;

    /*
     * #[impl_ui_node(..)] asserts
     */

    #[serial]
    #[test]
    fn impl_ui_node_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/fail/impl_ui_node/*.rs");
    }
    #[serial]
    #[test]
    fn impl_ui_node_pass() {
        let t = trybuild::TestCases::new();
        t.pass("tests/build/pass/impl_ui_node/*.rs");
    }

    /*
     * #[property(..)] asserts
     */

    #[serial]
    #[test]
    fn property_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/fail/property/*.rs");
    }
    #[serial]
    #[test]
    fn property_pass() {
        let t = trybuild::TestCases::new();
        t.pass("tests/build/pass/property/*.rs");
    }

    /*
     * #[widget!(..)] asserts
     */

    #[serial]
    #[test]
    fn widget_macro_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/fail/widget/*.rs");
    }
    #[serial]
    #[test]
    fn widget_macro_pass() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/pass/widget/*.rs");
    }
}

// `do test --build [-f <fail-name>] [-p <pass-name>]` uses these to run specific tests.
#[test]
#[ignore]
fn do_tasks_test_runner() {
    use std::env;

    if let Some(test) = env::var_os("DO_TASKS_TEST_BUILD") {
        let test = test.to_string_lossy();
        let env_mode = env::var_os("DO_TASKS_TEST_BUILD_MODE");
        let env_mode_clean = env_mode.as_ref().map(|m| m.to_string_lossy());
        let mode = env_mode_clean.as_ref().map(|m| m.as_ref()).unwrap_or("fail");

        let t = trybuild::TestCases::new();
        let path = format!("tests/build/{}/{}.rs", mode, test);

        match mode {
            "fail" => t.compile_fail(path),
            "pass" => t.pass(path),
            unknown => panic!("unknown build test mode `{}`", unknown),
        }
    }
}
