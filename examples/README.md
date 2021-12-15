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

use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Foo Example";
            content = text("Bar");
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