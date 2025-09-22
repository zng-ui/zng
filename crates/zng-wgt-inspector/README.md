<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 3 feature flags, 0 enabled by default.

#### `"live"`
Compiles the interactive inspector.

#### `"crash_handler"`
Compiles the debug crash handler.

Only enables in `not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))` builds.

#### `"image"`
Enable screenshot capture.

<!--do doc --readme #SECTION-END-->


