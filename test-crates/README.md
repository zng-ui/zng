# Integration Test Crates

Integration tests that need to manually define dependencies.

The main test cases are the proc-macros that need to emulate the macro_rules `$crate`.
For example, if a crate uses a widget defined in another crate and does not directly depend `zero-ui` nor `zero-ui-core` will
`widget_new!` still be called when the widget macro is used?

# Running Tests

A call must be manually added to the task runner:

```shell
cargo test --workspace --no-fail-fast --manifest-path "test-crates/no-direct-dep/Cargo.toml"
```