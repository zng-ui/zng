[![License-APACHE](https://img.shields.io/badge/License-Apache--2.0-informational)](https://github.com/zng-ui/zng/blob/master/LICENSE-APACHE)
[![License-MIT](https://img.shields.io/badge/license-MIT-informational)](https://github.com/zng-ui/zng/blob/master/LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/zng)](https://crates.io/crates/zng)
[![Documentation](https://img.shields.io/badge/github.io-docs-success)](https://zng-ui.github.io/doc/zng)

# zng

Zng is a cross-platform GUI framework, it provides ready made highly customizable widgets, responsive layout, 
live data binding, easy localization, automatic focus navigation and accessibility, async and multi-threaded tasks, robust
multi-process architecture and more.

Zng is pronounced "zing", or as an initialism: ZNG (z+nesting+graphics).

## Usage

First add this to your `Cargo.toml`:

```toml
[dependencies]
zng = { version = "0.3.3", features = ["view_prebuilt"] }
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

## Crates

The `zng` crate is the only dependency you need to create apps, it re-exports the primary API of the other 
crates in well organized and documented modules. 

The other crates provide the full API that you might need to implement more advanced features, for example, a 
custom property that modifies the behavior of an widget might need to reference the widget's internal state,
this *internal* API will only be available in the widget's crate.

<!--do doc --readme features-->
## Cargo Features

This crate provides 25 feature flags, 3 enabled by default.

#### `"view"`
Include the default view-process implementation.

#### `"view_prebuilt"`
Include the default view-process implementation as an embedded precompiled binary.

#### `"http"`
Enables HTTP tasks and web features of widgets and services.

#### `"debug_default"`
Enable the `"dyn_*"`, `"inspector"` features in debug builds.

*Enabled by default.*

#### `"dyn_node"`
Use more dynamic dispatch at the node level by enabling `UiNode::cfg_boxed` to box.

This speeds-up compilation time at the cost of runtime.

#### `"inspector"`
Instrument each property and widget instance with "Inspector" nodes and
extend windows to be inspected on Ctrl+Shift+I.

#### `"dyn_app_extension"`
Use dynamic dispatch at the app-extension level.

This speeds-up compilation time at the cost of runtime.

#### `"dyn_closure"`
Box closures at opportune places, such as `Var::map`, reducing the number of monomorphised types.

This speeds-up compilation time at the cost of runtime.

#### `"test_util"`
Test utilities.

#### `"multi_app"`
Allows multiple app instances per-process.

This feature allows multiple apps, one app per thread at a time. The `LocalContext` tracks
what app is currently running in each thread and `app_local!` statics switch to the value of each app
depending on the current thread.

Not enabled by default, but enabled by `feature="test_util"`.

#### `"trace_widget"`
Instrument every widget outer-most node to trace UI methods.

#### `"trace_wgt_item"`
Instrument every property and intrinsic node to trace UI methods.

Note that this can cause very large trace files and bad performance.

#### `"deadlock_detection"`
Spawns a thread on app creation that checks and prints `parking_lot` deadlocks.

#### `"hyphenation_embed_all"`
Embed hyphenation dictionaries for all supported languages.

If enabled some 2.8MB of data is embedded, you can provide an alternative dictionary source using the
`Hyphenation::dictionary_source` method.

#### `"material_icons"`
Include all Material Icons icon sets in the default app.

#### `"material_icons_outlined"`
Material Icons Outlined icon set.

If enabled some icons of this set are used for some of the commands.

#### `"material_icons_filled"`
Material Icons Filled icon set.

#### `"material_icons_rounded"`
Material Icons Rounded icon set.

#### `"material_icons_sharp"`
Material Icons Sharp icon set.

#### `"toml"`
Enable TOML configs.

#### `"ron"`
Enable RON configs.

#### `"yaml"`
Enable YAML configs.

#### `"view_software"`
Enables software renderer fallback in the default view-process.

If enabled and a native OpenGL 3.2 driver is not available the `swgl` software renderer is used.

*Enabled by default.*

#### `"view_bundle_licenses"`
Collects and bundles third-party licenses used by the `zng-view` crate.

Needs `cargo-about` and Internet connection during build.

Not enabled by default. Note that `"view_prebuilt"` always bundles licenses.

#### `"ipc"`
Enables pre-build views and connecting to views running in another process.

*Enabled by default.*

<!--do doc --readme #SECTION-END-->


## Requirements

On Windows:

* To build with `"view"` and `"view_software"` feature:
    - Env vars `CC` and `CXX` must be set to "clang-cl".
    - You can install clang using the [Visual Studio installer] or by installing LLVM directly.

[Visual Studio installer]: https://learn.microsoft.com/en-us/cpp/build/clang-support-msbuild?view=msvc-170

On Windows 8 or older:

* To build with `"view_prebuilt"`  feature:
    - The `curl` and `tar` commands are required, they are available in Windows 10+, but must be 
      installed in older Windows and must be added to the PATH env var.

on Linux:

* Packages needed to build:
    - `pkg-config`
    - `libfontconfig1-dev`

* Packages needed to build with `"http"` feature:
    - `libssl-dev`

* Packages needed to build with `"view_prebuilt"` feature:
    - `curl`

## Examples

Call `cargo do run <example>` to run an example.

See the [`./examples/README.md`] doc for a list of examples with description and screenshots.

[`./examples/README.md`]: https://github.com/zng-ui/zng/tree/master/examples#readme

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
