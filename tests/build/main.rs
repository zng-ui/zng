//! Use `cargo do test -b *` to run all build tests.
//!
//! Use `cargo do test -b property/*` to run all paths that match in the `./cases` folder.

mod run;

fn main() {
    run::do_request();
}
