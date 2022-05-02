# Profile Build Time

This crate is the target for `rustc` profiling done by `do profile`. The crate is build using nightly and `-Z self-profile`, then
the `summarize` cargo command is used to print the result.

See https://github.com/rust-lang/measureme/blob/master/summarize/README.md for details.

Note that this is not "cargo-timings", that can be done using the `do build --timings` command.

## Usage

Replace the `main.rs` content with a snippet or example that is building slow, then run `do profile --build`.

This crate is not part of the main workspace, you can clean its build artefacts using `do clean --profile-build`.
