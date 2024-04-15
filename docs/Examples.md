# Examples

See the [examples README](../examples/README.md) for information about the current examples. This document is for example contributors.

## Adding an Example

Add the new example in `./examples/<example-name>.rs`:

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

Register it in `./examples/Cargo.toml`:

```toml
[[example]]
name = "<example-name>"
path = "<example-name>.rs"
```

Update the auto generated README:

```console
cargo do doc --readme-examples <example-name>
```

Done. You can run the new example using:

```console
cargo do run <example-name>
```

## Important

The README auto generator will collect a screenshot of the example, to do this it replaces the `fn main()` with a custom runner
that starts the `app_main` function. Because of this the `app_main` function is required and no example specific code should be 
added directly on the main function.