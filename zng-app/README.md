<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.

<!--do doc --readme features-->
## Cargo Features

This crate provides 11 feature flags, 2 enabled by default.

#### ipc
Enables pre-build views and connecting to views running in another process.

*Enabled by default.*

#### debug_default
Enable the `"dyn_*"` and `"inspector"` features in debug builds.

*Enabled by default.*

#### dyn_node
Use dynamic dispatch at the node level by placing each property node in a `BoxedUiNode` and enabling `UiNode::cfg_boxed`.

This speeds-up compilation time at the cost of runtime.

#### inspector
Instrument each widget instance to retain build information.

#### dyn_app_extension
Use dynamic dispatch at the app-extension level.

This speeds-up compilation time at the cost of runtime.

#### dyn_closure
Box closures at opportune places, such as `Var::map`, reducing the number of monomorphised types.

This speeds-up compilation time at the cost of runtime.

#### test_util
Like `cfg(test)` but also visible in docs and integration tests.

#### multi_app
Allows multiple app instances per-process.

This feature allows multiple apps, one app per thread at a time. The `LocalContext` tracks
what app is currently running in each thread and `app_local!` statics switch to the value of each app
depending on the current thread.

Not enabled by default, but enabled by `feature="test_util"`.

#### trace_widget
Instrument every widget outer-most node to trace UI methods.

#### trace_wgt_item
Instrument every property and intrinsic node to trace UI methods.

Note that this can cause very large trace files and bad performance.

#### deadlock_detection
Spawns a thread on app creation that checks and prints `parking_lot` deadlocks.

Not enabled by default, but enabled by `feature="test_util"`.

<!--do doc --readme #SECTION-END-->

