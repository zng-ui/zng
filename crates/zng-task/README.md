<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 4 feature flags, 0 enabled by default.

#### `"deadlock_detection"`
Enables parking_lot deadlock detection.

#### `"ipc"`
Enables ipc tasks.

Only enables in `cfg(not(any(target_os = "android", target_arch = "wasm32")))` builds.

#### `"http"`
Enables http tasks.

#### `"test_util"`
Enabled by doc tests.

<!--do doc --readme #SECTION-END-->


