[![License](https://img.shields.io/badge/License-Apache--2.0-informational)](https://choosealicense.com/licenses/apache-2.0/)
[![Crates.io](https://img.shields.io/crates/v/zng)](https://crates.io/crates/zng)
[![docs.rs](https://img.shields.io/docsrs/zng)](https://docs.rs/zng)

# zng

Zng is a cross-platform GUI framework, it provides ready made highly customizable widgets, responsive layout, 
live data binding, easy localization, automatic focus navigation and accessibility, async and multi-threaded tasks, robust
multi-process architecture and more.

Zng is pronounced "zing", or as an initialism: ZNG (z+nesting+graphics).

## Usage

First add this to your `Cargo.toml`:

```toml
[dependencies]
zng = { version = "0.2.0", features = ["view_prebuilt"] }
```

Then create your first window:

```rust ,no_run
use zng::prelude::*;

fn main() {
    zng::view_process::prebuilt::init();
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

See the [`API docs`] for more details.

## Cargo Features

Zng provides the following features which can be enabled in your `Cargo.toml` file:

- **`view`** — Include the default view-process implementation.
- **`view_prebuilt`** — Include the default view-process implementation as an embedded precompiled binary.
- **`inspector`** — Instrument each property and widget instance with inspector nodes and extend windows to be inspected on Ctrl+Shift+I.
- **`trace_widget`** — Instrument every widget outer-most node to trace UI methods.
- **`trace_wgt_item`** — Instrument every property and intrinsic node to trace UI methods.
- **`deadlock_detection`** — Spawns a thread on app creation that checks and prints `parking_lot` deadlocks.
- **`http`** — Enables HTTP tasks, images download.
- **`test_util`** — Test utilities.
- **`multi_app`** — Allows multiple app instances per-process, one app per thread at a time. The `LocalContext` tracks
what app is currently running in each thread and `app_local!` statics switch to the value of each app
depending on the current thread.
- **`hyphenation_embed_all`** — Embed hyphenation dictionaries for all supported languages. If enabled some 2.8MB of data is embedded, you can provide an alternative dictionary source using the `Hyphenation::dictionary_source` method.
- **`dyn_node`** — Use more dynamic dispatch at the node level by enabling `UiNode::cfg_boxed` to box.
- **`dyn_app_extension`** — Use dynamic dispatch at the app-extension level.
- **`dyn_closure`** — Box closures at opportune places, such as `Var::map`, reducing the number of monomorphised types.
- **`toml`** — Enable TOML configs.
- **`ron`** — Enable RON configs.
- **`yaml`** — Enable YAML configs.
- **`material_icons`** — Include all *Material Icons* icon sets, each icon set embeds some 300KB of data.
- **`material_icons_outlined`** Include *Material Icons Outlined* icon set. If enabled some icons of this set are used for some of the commands.
- **`material_icons_filled`** Include *Material Icons Filled* icon set.
- **`material_icons_rounded`** Include *Material Icons Rounded* icon set.
- **`material_icons_sharp`** Include *Material Icons Sharp* icon set.

These features are enabled by default:

- **`debug_default`** — Enable the `dyn_*` and `inspector` features for debug builds only.
- **`ipc`** — Enables pre-build views and connecting to views running in another process.
- **`view_software`** — Enables software renderer fallback in the default view-process (`"view"`).

## Requirements

On Windows:

* To build with `"view"` and `"view_software"` feature:
    - Env vars `CC` and `CXX` must be set to "clang-cl".
    - You can install clang using the [Visual Studio installer] or by installing LLVM directly.

[Visual Studio installer]: https://learn.microsoft.com/en-us/cpp/build/clang-support-msbuild?view=msvc-170

on Linux:

* Packages need to build:
    - `pkg-config`
    - `libfontconfig1-dev`

* Packages need to build with `"http"` feature:
    - `libssl-dev`

## `cargo do`

**Do** is a built-in task runner for managing this project, run `cargo do help` or `./do help` for details.

The task runner is implemented as a Rust crate in `tools/do-tasks` and an alias in `.cargo/config.toml`.
The builds the tool silently in the first run, after, it runs without noticeable delay.

Shell script to run `do` are also provided:
 
 * cmd.exe: `do help`.
 * PowerShell: `./do.ps1 help`.
 * Bash: `/.do help`.

### `cargo do run <example>`

The task runner can be used to run the examples in `./examples` folder, for example: `cargo do run calculator` runs the
`./examples/calculator.rs` example.

### `cargo do install`

The task runner depends on multiple cargo commands, you can run `cargo do install` to see a list of all required commands and run `cargo do install --accept` to run the installation commands.

## VSCode & Rust Analyzer

Some workspace settings are included in the repository, in particular, `rust-analyzer` "checkOnSave" 
and runnables are redirected to the `do` tool.

[`API docs`]: https://docs.rs/zng
[`cargo-expand`]: https://github.com/dtolnay/cargo-expand
[`cargo-asm`]: https://github.com/gnzlbg/cargo-asm