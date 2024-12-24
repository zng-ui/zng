Zng is a cross-platform GUI framework, it provides ready made highly customizable widgets, responsive layout, 
live data binding, easy localization, automatic focus navigation and accessibility, async and multi-threaded tasks, robust
multi-process architecture and more.

Zng is pronounced "zing", or as an initialism: ZNG (Z Nesting Graphics).

## Usage

First add `zng` to your `Cargo.toml`, or call `cargo add zng -F view_prebuilt`: 

```toml
[dependencies]
zng = { version = "0.13.2", features = ["view_prebuilt"] }
```

Then create your first window:

```rust ,no_run
use zng::prelude::*;

fn main() {
    zng::env::init!();
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

See the [`documentation`] for more details.

[`documentation`]: https://zng-ui.github.io/doc/zng/

### Project Template

You can also use [`cargo zng new`] to generate a new project with useful boilerplate and distribution
packaging already setup.

```console
cargo install cargo-zng
cargo zng new "My App!"
```

The example above installs `cargo-zng` and uses it to generate a new './my-app' crate from the [default template].

[`cargo zng new`]: https://github.com/zng-ui/zng/tree/main/crates/cargo-zng#new
[default template]: https://github.com/zng-ui/zng-template

<!--do doc --readme features-->
## Cargo Features

This crate provides 32 feature flags, 4 enabled by default.

#### `"view"`
Include the default view-process implementation.

Only enables in `not(target_arch = "wasm32")` builds.

#### `"view_prebuilt"`
Include the default view-process implementation as an embedded precompiled binary.

Only enables in `not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))` builds.

#### `"http"`
Enables HTTP tasks and web features of widgets and services.

#### `"debug_default"`
Enable the `"dyn_*"`, `"inspector"` features in debug builds.

*Enabled by default.*

#### `"svg"`
Enable SVG images, emoji support.

#### `"dyn_node"`
Use more dynamic dispatch at the node level by enabling `UiNode::cfg_boxed` to box.

This speeds-up compilation time at the cost of runtime.

#### `"inspector"`
Instrument each property and widget instance with "Inspector" nodes and
extend windows to be inspected on Ctrl+Shift+I.

#### `"hot_reload"`
Enable hot reload builds.

Note that you must configure the target library to hot reload, see `zng::hot_reload` for details.

Only enables in `not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))` builds.

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

#### `"single_instance"`
Enables single app-process instance mode.

Builds with this feature only allow one app-process, subsequent attempts to spawn the app redirect to
the running app-process.

Only enables in `not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))` builds.

#### `"crash_handler"`
Allow app-process crash handler.

Builds with this feature spawn a crash monitor-process for each app-process.

Only enables in `not(any(target_arch = "wasm32", target_os = "android"))` builds.

*Enabled by default.*

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
`HYPHENATION::init_data_source` method.

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
Enables IPC tasks, pre-build views and connecting to views running in another process.

Only enables in `not(any(target_os = "android", target_arch = "wasm32", target_os = "ios"))` builds.

*Enabled by default.*

#### `"built_res"`
Check if `zng::env::res` path is available in `init_built_res` first.

Enabled by default in debug builds, ignored in Android and Wasm.

#### `"android_game_activity"`
Standard Android backend that requires a build system that can compile Java or Kotlin and fetch Android dependencies.

See `https://docs.rs/winit/latest/winit/platform/android/` for more details.

#### `"android_native_activity"`
Basic Android backend that does not require Java.

See `https://docs.rs/winit/latest/winit/platform/android/` for more details.

<!--do doc --readme #SECTION-END-->

## Repository

See the [`zng-ui/zng`] repository README for more information about build requirements, examples and license information.

[`zng-ui/zng`]: https://github.com/zng-ui/zng
