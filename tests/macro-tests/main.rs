//! Use `cargo do test -m --all` to run all macro tests.
//!
//! Use `cargo do test -m property/*` to run all paths that match in the `./cases` folder.

mod run;

fn main() {
    run::do_request();
}
