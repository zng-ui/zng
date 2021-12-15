//! Use `cargo do test -b *` to run all build tests.
//!
//! Use `cargo do test -b property/*` to run all paths that match in the `./cases` folder.

fn main() {
    if let Some(test) = std::env::var_os("DO_TASKS_TEST_BUILD") {
        let mut test = test.to_string_lossy();

        if ["*", "**"].contains(&test.as_ref()) {
            test = "*/*".into();
        }

        std::env::set_current_dir(format!("{}/cases", env!("CARGO_MANIFEST_DIR"))).unwrap();

        trybuild::TestCases::new().compile_fail(format!("{}.rs", test));
    } else {
        eprintln!("run with `cargo do test --build *`");
    }
}
