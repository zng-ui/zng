pub fn do_request() {
    if let Some(test) = std::env::var_os("DO_TASKS_TEST_BUILD") {
        let mut test = test.to_string_lossy();

        if ["*", "**"].contains(&test.as_ref()) {
            test = "*/*".into();
        }

        std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

        trybuild::TestCases::new().compile_fail(format!("cases/{test}.rs"));
    } else {
        eprintln!("run with `cargo do test --build *`");
    }
}
