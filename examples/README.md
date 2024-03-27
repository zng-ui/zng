# Examples

This directory contains small example apps.

# Running

To run an example use `cargo do run $name`.

# Adding an Example

To add an example, create a file then add it in `./Cargo.toml`.

## Template

This is a good example template:

In `./foo.rs`: 
```rust
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

Then add in `./Cargo.toml`:

```toml
[[example]]
name = "foo"
path = "foo.rs"
```

Then run from the project root using `cargo do run foo`.