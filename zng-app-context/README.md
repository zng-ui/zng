<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng) project.


<!--do doc --readme features-->
## Cargo Features

The `zng-app-context` crate provides 1 feature flags, 1 enabled by default.

#### multi_app
Allows multiple app instances per-process.

This feature allows multiple apps, one app per thread at a time. The `LocalContext` tracks
what app is currently running in each thread and `app_local!` statics switch to the value of each app
depending on the current thread.


<!--do doc --readme #SECTION-END-->


