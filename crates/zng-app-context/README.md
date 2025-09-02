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
**deprecated** enable needed features directly

*Enabled by default.*

#### `"dyn_closure"`
**deprecated** no longer needed

<!--do doc --readme #SECTION-END-->


