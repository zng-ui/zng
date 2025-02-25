## Contribute an Example

To contribute an example, add the new example crate in `examples/<example-name>`:

`examples/<example-name>/Cargo.toml`

```
[package]
name = "zng-example-<example-name>"
version = "0.0.0"
publish = false
edition = "2024"

[dependencies]
zng = { path = "../../crates/zng", features = ["view_prebuilt"] }
```

`examples/<example-name>/src/main.rs`: 

```rust
//! Demonstrates foo, bar.
use zng::prelude::*;

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        Window! {
            title = "Foo Example";
            child = Text!("Bar");
        }
    })
}
```

Run the example and test it.

```console
cargo do run <example-name>
```

Optionally, take a screenshot and save it to `examples/<example-name>/res/screenshot.png`. You can take a screenshot using
the inspector window, press `Ctrl+Shift+I` then click the "Save Screenshot" menu.

Run [`oxipng`](https://github.com/shssoichiro/oxipng) or another minifier on the screenshot before committing.

```console
oxipng -o max --strip safe --alpha "examples/<example-name>/res/screenshot.png"
```

Update the auto generated README:

```console
cargo do doc --readme-examples
```

Done.