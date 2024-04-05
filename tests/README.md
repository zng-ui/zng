# Tests

Use this directory for integration, macro tests or any test that is not a unit test.

# Running

Use `cargo do test -t command` to run tests in the `./command.rs` file.

Use `cargo do test -m *` to run all macro tests.

Use `cargo do test -m property/*` to run build test cases that match the path relative to `./macro-tests/cases`.

# Adding an Integration Test

To add an integration test, create a file then add it in `./Cargo.toml` as a `[[bin]]`.

In `./foo.rs`:
```rust
use zng::prelude::*;

#[test]
fn foo() {
    assert!(true);
}
```

Then add in `./Cargo.toml`:

```toml
[[test]]
name = "foo"
path = "foo.rs"
```

Then run from the project root using `cargo do test -t foo`.