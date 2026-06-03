<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 3 feature flags, 0 enabled by default.

#### `"tar"`
Support for loading localization resources from TAR and Tarball.

#### `"lang_autonym"`
Embed language and region names for `Lang::autonym`.

#### `"usage_recorder"`
Compile with dependency usage profile recorder.

The recorded profile can be used to by `cargo zng res` to pack only the used dependency strings.

Saves to `ZNG_L10N_PROFILE_FILE` if set to a .rec.subset path
Or `"res/optimization-profiles/zng-ext-l10n.rec.subset"` by default

<!--do doc --readme #SECTION-END-->


