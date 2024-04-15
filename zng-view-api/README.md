<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 2 feature flags, 1 enabled by default.

#### ipc
Enables creation of separate or pre-build view.

When this is enabled communication with the view is (de)serialized which can add a
minor cost, something like a 1ms per 3MB frame request.

*Enabled by default.*

#### var
Implement `IntoVar<T>` for API types.

<!--do doc --readme #SECTION-END-->


