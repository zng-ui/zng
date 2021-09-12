# About

This workspace demonstrates splitting the "View-Process" from the "App-Process" so that the `app_process`
crate does not need to build the windowing and renderer crates.

# Run

To run, first build using `cargo build --workspace` then `cargo run --bin app_process`.