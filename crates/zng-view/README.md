<!--do doc --readme header-->
This crate is part of the [`zng`](https://github.com/zng-ui/zng?tab=readme-ov-file#crates) project.


<!--do doc --readme features-->
## Cargo Features

This crate provides 20 feature flags, 3 enabled by default.

#### `"ipc"`
Enables pre-build and init as view-process.

If this is enabled all communication with the view is serialized/deserialized,
even in same-process mode.

Only enables in `cfg(not(target_os = "android"))` builds.

*Enabled by default.*

#### `"software"`
Enables software renderer fallback.

If enabled and a native OpenGL 3.2 driver is not available the `swgl` software renderer is used.

*Enabled by default.*

#### `"bundle_licenses"`
Bundle third party licenses.

Needs `cargo-about` and Internet connection during build.

Not enabled by default. Note that `"view_prebuilt"` always bundles licenses.

#### `"android_game_activity"`
Standard Android backend that requires a build system that can compile Java or Kotlin and fetch Android dependencies.

See `https://docs.rs/winit/latest/winit/platform/android/` for more details.

#### `"android_native_activity"`
Basic Android backend that does not require Java.

See `https://docs.rs/winit/latest/winit/platform/android/` for more details.

#### `"image_bmp"`
Enable BMP image decoder and encoder.

#### `"image_dds"`
Enable DDS image decoder.

#### `"image_exr"`
Enable EXR image decoder and encoder.

#### `"image_ff"`
Enable Farbfeld image decoder and encoder.

#### `"image_gif"`
Enable GIF image decoder and encoder.

#### `"image_hdr"`
Enable Radiance HDR image decoder and encoder.

#### `"image_ico"`
Enable ICO image decoder and encoder.

#### `"image_jpeg"`
Enable JPEG image decoder and encoder.

#### `"image_png"`
Enable PNG image decoder and encoder.

#### `"image_pnm"`
Enable PNM image decoder and encoder.

#### `"image_qoi"`
Enable QOI image decoder and encoder.

#### `"image_tga"`
Enable TGA image decoder and encoder.

#### `"image_tiff"`
Enable TIFF image decoder and encoder.

#### `"image_webp"`
Enable WEBP image decoder.

#### `"image_all"`
Enable all image decoders and encoders.

*Enabled by default.*

<!--do doc --readme #SECTION-END-->


