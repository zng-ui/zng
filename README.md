[![License-APACHE](https://img.shields.io/badge/License-Apache--2.0-informational)](https://github.com/zng-ui/zng/blob/master/LICENSE-APACHE)
[![License-MIT](https://img.shields.io/badge/license-MIT-informational)](https://github.com/zng-ui/zng/blob/master/LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/zng)](https://crates.io/crates/zng)
[![Documentation](https://img.shields.io/badge/github.io-docs-success)](https://zng-ui.github.io/doc/zng)

# zng

Zng is a cross-platform GUI framework, it provides ready made highly customizable widgets, responsive layout, 
live data binding, easy localization, automatic focus navigation and accessibility, async and multi-threaded tasks, robust
multi-process architecture and more.

Zng is pronounced "zing", or as an initialism: ZNG (Z Nesting Graphics).

## Usage

First add `zng` to your `Cargo.toml`, or call `cargo add zng -F view_prebuilt`: 

```toml
[dependencies]
zng = { version = "0.10.1", features = ["view_prebuilt"] }
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

[`cargo zng new`]: crates/cargo-zng#new
[default template]: https://github.com/zng-ui/zng-template

## Crates

The `zng` crate is the only dependency you need to create apps, it re-exports the primary API of the other 
crates in well organized and documented modules.

The other crates provide the full API that you might need to implement more advanced features, for example, a 
custom property that modifies the behavior of a widget might need to reference the widget's internal state,
this *internal* API will only be available in the widget's crate.

## Cargo Features

The Cargo features of each crate are documented in the README file for that crate. See [`./crates/zng`] for the Cargo features of the main crate.

[`./crates/zng`]:https://github.com/zng-ui/zng/tree/master/crates/zng#cargo-features

## Requirements

On Windows:

* To build with `"view"` and `"view_software"` feature:
    - Env vars `CC` and `CXX` must be set to "clang-cl".
    - You can install clang using the [Visual Studio installer] or by installing LLVM directly.

[Visual Studio installer]: https://learn.microsoft.com/en-us/cpp/build/clang-support-msbuild?view=msvc-170

On Linux:

* Packages needed to build:
    - `pkg-config`
    - `libfontconfig1-dev`

* Packages needed to build with `"http"` feature:
    - `libssl-dev`

* Packages needed to build with `"view_prebuilt"` feature:
    - `curl`

On macOS:

* To build with `"crash_handler"` feature, enabled by default:
    - XCode 14 or newer.

## Examples

Clone this repository and call `cargo do run <example>` to run an example.

See the [`./examples`] README file for a list of examples with description and screenshots.

[`./examples`]: https://github.com/zng-ui/zng/tree/master/examples#readme

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
