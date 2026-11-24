<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 3 feature flags, 0 enabled by default.

#### `"svg"`
Enable support for SVG image glyphs.

This feature signals that the `IMAGES` service supports SVG. The app must ensure that this is so by
using `zng-ext-svg` or another SVG extension.

#### `"hypher_full"`
Embed hyphenation data for all supported languages.

If enabled some 1.1 MiB of data is embedded. Instead of enabling this feature you can select
specific languages to support by adding a `hypher` dependency:

```toml
[dependencies]
# only embed English hyphenation data.
hypher = { version = "0.1", default-features = false, features = ["english"] }
```

#### `"ipc"`
Enables memory mapped font files.

<!--do doc --readme #SECTION-END-->


