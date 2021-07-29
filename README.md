![License](https://img.shields.io/crates/l/zero-ui)
[![Crates.io](https://img.shields.io/crates/v/zero-ui)](https://crates.io/crates/zero-ui)
[![docs.rs](https://img.shields.io/docsrs/zero-ui)](https://docs.rs/zero-ui)

# zero-ui

Zero-Ui is the pure Rust GUI framework with batteries included.

It provides all that you need to create a beautiful, fast and responsive multi-platform GUI apps, it includes many features
that allow you to get started quickly, without sacrificing customization or performance. With features like gesture events,
common widgets, layouts, data binding, async tasks, accessibility and localization
you can focus on what makes your app unique, not the boilerplate required to get modern apps up to standard.

When you do need to customize, Zero-Ui is rightly flexible, you can create new widgets or customize existing ones, not just
new looks but new behavior, at a lower level you can introduce new event types or new event sources, making custom hardware seamless
integrate into the framework.

# Usage

First add this to your `Cargo.toml`:

```toml
[dependencies]
zero-ui = "0.2"
```

Then create your first window:

```rust
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        let size = var_from((800, 600));
        window! {
            title = size.map(|s: &Size| formatx!("Button Example - {}", s));
            size;
            content = button! {
                on_click = hn!(|_,_| {
                    println!("Button clicked!");
                });
                margin = 10;
                size = (300, 200);
                align = Alignment::CENTER;
                font_size = 28;
                content = text("Click Me!");
            }
        }
    })
}
```

See the [`API docs`] front page for more details.

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
* `libfontconfig1-dev`

Linux support is tested using the Windows Subsystem for Linux (Ubuntu image).

## Other Dependencies

For debugging this project you may also need [`cargo-expand`]
and the nightly toolchain for debugging macros (`do expand`), [`cargo-asm`] for checking
optimization (`do asm`).

You also need the nightly toolchain for building the documentation (`do doc`), although you can
build the documentation in stable using `cargo doc`, but custom pages like widget items may not
render properly because of changes in the `cargo-doc` HTML templates.

## `do`

There is a built-in task runner for managing this project, run `do help` or `./do help` for details.

The task runner is implemented as a Rust crate in `tools/do-tasks`, the shell script builds it in the first run.

[`API docs`]: https://docs.rs/zero-ui
[`cargo-expand`]: https://github.com/dtolnay/cargo-expand
[`cargo-asm`]: https://github.com/gnzlbg/cargo-asm