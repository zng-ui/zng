# zero-ui

A Rust UI Framework.

# Dependencies

Extra dependencies needed for building a crate that uses `zero-ui`.

## Windows

You just need `cargo` with an up-to-date stable toolchain installed.

## Linux

Apart from `cargo` with an up-to-date stable toolchain you may need to install:

* `build-essential`
* `cmake`
* `pkg-config`
* `libfreetype6-dev`
* `libexpat1-dev`

Linux support is tested in the Ubuntu sub-system for Windows.

## Other Dependencies

For debugging this project you may also need [`cargo-expand`](https://github.com/dtolnay/cargo-expand)
and the nightly toolchain for debugging macros.

## `do`

There is a built-in task runner for managing this project, run `do help` or `./do help` for details.

The task runner is implemented as a Rust crate in `tools/do-tasks`, the shell script builds it in the first run.