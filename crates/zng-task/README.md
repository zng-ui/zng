<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 5 feature flags, 0 enabled by default.

#### `"ipc"`
Enables real worker processes and IPC channels.

Only enables in `cfg(not(any(target_os = "android", target_arch = "wasm32", target_os = "ios")))` builds.

#### `"http"`
Enables HTTP client tasks.

#### `"http_cookie"`
Enables HTTP cookie storage option.

#### `"http_compression"`
Enables HTTP compression option.

#### `"test_util"`
Enabled by doc tests.

<!--do doc --readme #SECTION-END-->


