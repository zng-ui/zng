<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 7 feature flags, 0 enabled by default.

#### `"embedded"`
Embedded font files.

#### `"outlined"`
Outlined icon set.

#### `"filled"`
Filled icon set.

#### `"rounded"`
Rounded icon set.

#### `"sharp"`
Sharp icon set.

#### `"usage_recorder"`
Compile with usage profile recorder.

After recording build with `"embedded_subset"` to embed only the icons used.

Saves to `ZNG_MATERIAL_ICONS_PROFILE_FILE` if set to a .rec.subset path
Or `"res/optimization-profiles/zng-wgt-material-icons.rec.subset"` by default

#### `"embedded_subset"`
Use recorded usage profile to subset icon fonts on build for embedding.

See `"usage_recorder"` for how to set the file.

<!--do doc --readme #SECTION-END-->


