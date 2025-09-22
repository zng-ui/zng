<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 2 feature flags, 0 enabled by default.

#### `"ipc"`
Enables creation of separate or pre-build view.

Only enables in `cfg(not(any(target_os = "android", target_arch = "wasm32", target_os = "ios")))` builds.

#### `"var"`
Implement `IntoVar<T>` for API types.

<!--do doc --readme #SECTION-END-->


