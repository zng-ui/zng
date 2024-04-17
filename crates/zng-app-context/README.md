<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 3 feature flags, 1 enabled by default.

#### `"multi_app"`
Allows multiple app instances per-process.

This feature allows multiple apps, one app per thread at a time. The `LocalContext` tracks
what app is currently running in each thread and `app_local!` statics switch to the value of each app
depending on the current thread.

#### `"debug_default"`
Enable `"multi_app"` in debug builds.

*Enabled by default.*

#### `"dyn_closure"`
Box closures at opportune places, such as `Var::map`, reducing the number of monomorphised types.

This speeds-up compilation time at the cost of runtime.

<!--do doc --readme #SECTION-END-->


