[![License](https://img.shields.io/badge/License-Apache--2.0-informational)](https://choosealicense.com/licenses/apache-2.0/)
[![Crates.io](https://img.shields.io/crates/v/zero-ui)](https://crates.io/crates/zero-ui)
[![docs.rs](https://img.shields.io/docsrs/zero-ui)](https://docs.rs/zero-ui)

# zero-ui

Zero-Ui is the pure Rust UI framework with batteries included it provides all that you need to create beautiful,
fast and responsive multi-platform apps. Ready made highly customizable widgets, automatic focus and accessibility
management, responsive layout, data binding, easy localization and async tasks.

## Usage

First add this to your `Cargo.toml`:

```toml
[dependencies]
zero-ui = "0.1"
zero-ui-view = "0.1"
```

Then create your first window:

```rust
use zero_ui::prelude::*;

fn main() {
    zero_ui_view::init();
    APP.defaults().run_window(async {
        let size = var(layout::Size::new(800, 600));
        Window! {
            title = size.map(|s| formatx!("Button Example - {s}"));
            size;
            child_align = Align::CENTER;
            child = Button! {
                on_click = hn!(|_| {
                    println!("Button clicked!");
                });
                text::font_size = 28;
                child = Text!("Click Me!");
            }
        }
    })
}
```

See the [`API docs`] front page for more details.

## `cargo do`

There is a built-in task runner for managing this project, run `cargo do help` or `./do help` for details.

The task runner is implemented as a Rust crate in `tools/do-tasks` and an alias in `.cargo/config.toml`,
it builds the tool silently in the first run, after it should run without noticeable delay.

Shell script to run `do` are also provided:
 
 * cmd.exe: `do help`.
 * PowerShell: `./do.ps1 help`.
 * Bash: `/.do help`.

### `cargo do install`

The task runner depends on multiple cargo commands, you can run `cargo do install` to see a list of all required commands and run `cargo do install --accept` to run the installation commands.


## VSCode & Rust Analyzer

Some workspace settings are included in the repository, in particular, `rust-analyzer` "checkOnSave" 
and runnables are redirected to the `do` tool.

[`API docs`]: https://docs.rs/zero-ui
[`cargo-expand`]: https://github.com/dtolnay/cargo-expand
[`cargo-asm`]: https://github.com/gnzlbg/cargo-asm