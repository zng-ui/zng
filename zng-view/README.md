<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features


The `zng-view` crate provides 3 feature flags, 2 enabled by default.

#### ipc
Enables pre-build and init as view-process.

If this is enabled all communication with the view is serialized/deserialized,
even in same-process mode.

Feature enabled by default.



 **Enabled by default.**

#### software
Enables software renderer fallback.

If enabled and a native OpenGL 3.2 driver is not available the `swgl` software renderer is used.

Feature enabled by default.



 **Enabled by default.**

#### bundle_licenses
Bundle third party licenses.

Needs `cargo-about` and Internet connection during build.

Not enabled by default. Note that `"view_prebuilt"` always bundles licenses.


<!--do doc --readme #SECTION-END-->


