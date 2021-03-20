# zero-ui

A Rust UI Framework.

# Dependencies

Extra system dependencies needed for building a crate that uses the `zero-ui` crate.

## Windows

You just need the latest stable Rust toolchain installed.

## Linux

* Latest stable Rust.
* `build-essential` or equivalent C/C++ compiler package.
* `cmake`
* `pkg-config`
* `libfreetype6-dev`
* `libexpat1-dev`

Linux support is tested using the Windows Subsystem for Linux (Ubuntu image)

## Other Dependencies

For debugging this project you may also need [`cargo-expand`](https://github.com/dtolnay/cargo-expand)
and the nightly toolchain for debugging macros (`do expand`), [`cargo-asm`](https://github.com/gnzlbg/cargo-asm) for checking
optimization (`do asm`).

## `do`

There is a built-in task runner for managing this project, run `do help` or `./do help` for details.

The task runner is implemented as a Rust crate in `tools/do-tasks`, the shell script builds it in the first run.