<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 3 feature flags, 1 enabled by default.

#### `"debug_default"`
**deprecated** enable needed features directly

*Enabled by default.*

#### `"live"`
Compiles the interactive inspector.

#### `"crash_handler"`
Compiles the debug crash handler.

Only enables in `not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))` builds.

<!--do doc --readme #SECTION-END-->


