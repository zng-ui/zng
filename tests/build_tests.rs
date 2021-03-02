mod build_tests {
    use serial_test::serial;

    /*
     * #[impl_ui_node(..)] asserts
     */

    #[serial]
    #[test]
    fn impl_ui_node_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/impl_ui_node_macro/fail/*.rs");
    }
    #[serial]
    #[test]
    fn impl_ui_node_pass() {
        let t = trybuild::TestCases::new();
        t.pass("tests/build/impl_ui_node_macro/pass/*.rs");
    }

    /*
     * #[property(..)] asserts
     */

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

    /*
     * #[widget!(..)] asserts
     */

    #[serial]
    #[test]
    fn widget_macro_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/widget_macro/fail/*.rs");
    }
    #[serial]
    #[test]
    fn widget_macro_pass() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/build/widget_macro/pass/*.rs");
    }
}

/*
 * do-tasks uses these to run a specific test
 */
mod do_tasks_util {
    use std::env;

    #[test]
    #[ignore]
    fn do_test_fail() {
        if let Some(test) = env::var_os("DO_TASKS_BUILD_TEST") {
            let t = trybuild::TestCases::new();
            let path = format! {"tests/build/*/fail/*{}*.rs", test.to_string_lossy()};
            t.compile_fail(path);
        }
    }

    #[test]
    #[ignore]
    fn do_test_pass() {
        if let Some(test) = env::var_os("DO_TASKS_BUILD_TEST") {
            let t = trybuild::TestCases::new();
            let path = format! {"tests/build/*/pass/*{}*.rs", test.to_string_lossy()};
            t.pass(path);
        }
    }
}
