<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.

<!--do doc --readme features-->
## Cargo Features

This crate provides 13 feature flags, 1 enabled by default.

#### `"debug_default"`
**deprecated** use features directly

*Enabled by default.*

#### `"dyn_node"`
**deprecated** no longer needed

#### `"inspector"`
Instrument each widget instance to retain build information.

#### `"dyn_app_extension"`
Use dynamic dispatch at the app-extension level.

This speeds-up compilation time at the cost of runtime.

#### `"dyn_closure"`
**deprecated** no longer needed

#### `"test_util"`
Like `cfg(test)` but also visible in docs and integration tests.

#### `"multi_app"`
Allows multiple app instances per-process.

This feature allows multiple apps, one app per thread at a time. The `LocalContext` tracks
what app is currently running in each thread and `app_local!` statics switch to the value of each app
depending on the current thread.

Not enabled by default, but enabled by `feature="test_util"`.

#### `"trace_widget"`
Instrument every widget outer-most node to trace UI methods.

#### `"trace_wgt_item"`
Instrument every property and intrinsic node to trace UI methods.

Note that this can cause very large trace files and bad performance.

#### `"crash_handler"`
Allow app-process crash handler.

Only enables in `not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))` builds.

#### `"trace_recorder"`
Enable trace recording.

Note that this does not auto start recording, to do that run with the `ZNG_RECORD_TRACE` env var set.

#### `"ipc"`
Enables IPC tasks and pre-build views and connecting to views running in another process.

#### `"deadlock_detection"`
Spawns a thread on app creation that checks and prints `parking_lot` deadlocks.

Not enabled by default, but enabled by `feature="test_util"`.

<!--do doc --readme #SECTION-END-->

