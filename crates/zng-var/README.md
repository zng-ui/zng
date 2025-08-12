<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 3 feature flags, 1 enabled by default.

#### `"debug_default"`
Signal the build script to enable the more features in debug builds.

*Enabled by default.*

#### `"type_names"`
Add `value_type_name` method to get the diagnostics type name from variable values.

#### `"dyn_closure"`
Box closures at opportune places, such as `Var::map`, reducing the number of monomorphised types.

This speeds-up compilation time at the cost of runtime.

<!--do doc --readme #SECTION-END-->


