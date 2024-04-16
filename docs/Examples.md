# Examples

See the [examples README](../examples/README.md) for information about the current examples. This document is for example contributors.

## Adding an Example

Add the new example in `examples/<example-name>.rs`:

```rust
//! Demonstrates foo, bar.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::prelude::*;

use zng::view_process::prebuilt as view_process;

fn main() {
    examples_util::print_info();
    // view_process::run_same_process(app_main);

    view_process::init();
    app_main();
}

fn app_main() {
    APP.defaults().run_window(async {
        Window! {
            title = "Foo Example";
            child = Text!("Bar");
        }
    })
}
```

Register it in `examples/Cargo.toml`:

```toml
[[example]]
name = "<example-name>"
path = "<example-name>.rs"
```

Run the example and test it.

```console
cargo do run <example-name>
```

Optionally, take a screenshot and save it to `examples/res/screenshots/<example-name>.png`. You can take a screenshot using
the inspector window, press `Ctrl+Shift+I` then press the screenshot button.

Run [`oxipng`](https://github.com/shssoichiro/oxipng) or another minifier on the screenshot before committing.

```console
oxipng -o max --strip safe --alpha "examples/res/screenshots/<example-name>.png"
```

Update the auto generated README:

```console
cargo do doc --readme-examples
```

Done. You can run the new example using:

```console
cargo do run <example-name>
```
