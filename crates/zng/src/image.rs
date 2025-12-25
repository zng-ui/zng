#![cfg(feature = "image")]

//! Images service, widget and other types.
//!
//! # Image
//!
//! The [`Image!`](struct@Image) widget is the primary way of presenting images, the example below defines
//! a repeating pattern image as the window background, the image source is embedded in this case, see [`ImageSource`]
//! for other supported sources.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//! # macro_rules! include_bytes { ($tt:tt) => { &[0u8] } }
//!
//! # let _ =
//! Window! {
//!     widget::background = Image! {
//!         source = include_bytes!("../res/image/pattern.png");
//!         img_fit = zng::image::ImageFit::None;
//!         img_repeat = true;
//!     };
//! }
//! # ; }
//! ```
//!
//! # Mask
//!
//! Mask images are loaded just like normal images, the [`mask::mask_image`](fn@mask::mask_image) property
//! can be set on any widget to apply a mask to it. The example below applies a mask to a button, by
//! default the mask uses the alpha channel, see [`mask`] for more details.
//!
//! ```
//! use zng::{image::mask, prelude::*};
//! # fn example() {
//! # macro_rules! include_bytes { ($tt:tt) => { &[0u8] } }
//!
//! # let _ =
//! Button! {
//!     mask::mask_image = include_bytes!("../res/image/star.png");
//! }
//! # ; }
//! ```
//!
//! # Service
//!
//! The [`IMAGES`] service manages image loading, the image cache and image rendering. Image decoding is
//! implemented by the view-process, for this reason to get image with actual pixels the service must be
//! used in a headed app or headless app with renderer, in a headless app without renderer all images are
//! a placeholder dummy.
//!
//! The images service also define security limits, the [`IMAGES.limits`](fn@IMAGES::limits)
//! variable to configure these limits. See [`ImageLimits::default`] for the defaults.
//!
//! ```
//! use zng::{image, prelude::*};
//! # fn example() {
//!
//! image::IMAGES.limits().modify(|l| {
//!     l.allow_uri = image::UriFilter::allow_host("httpbin.org");
//!     l.max_encoded_len = 1.megabytes();
//!     l.max_decoded_len = 10.megabytes();
//! }); }
//! ```
//!
//! The example above changes the global limits to allow image downloads only from an specific host and
//! only allow images with sizes less or equal to 1 megabyte and that only expands to up to 10 megabytes
//! after decoding.
//!  
//! # Full API
//!
//! See [`zng_ext_image`] for the full image API and [`zng_wgt_image`] for the full widget API.

pub use zng_ext_image::{
    IMAGE_RENDER, IMAGES, ImageCacheMode, ImageDataFormat, ImageDownscaleMode, ImageEntriesMode, ImageEntryKind, ImageHash, ImageHasher,
    ImageLimits, ImageRenderArgs, ImageSource, ImageSourceFilter, ImageVar, Img, PathFilter, render_retain, ColorType,
};

#[cfg(feature = "http")]
pub use zng_ext_image::UriFilter;

pub use zng_wgt_image::{
    Image, ImageAutoScale, ImageFit, ImageRepeat, ImgErrorArgs, ImgLoadArgs, ImgLoadingArgs, img_align, img_auto_scale, img_cache,
    img_crop, img_downscale, img_entries_mode, img_error_fn, img_fit, img_limits, img_loading_fn, img_offset, img_rendering, img_repeat,
    img_repeat_spacing, img_scale, is_error, is_loaded, on_error, on_load, on_load_layout,
};

/// Mask image properties.
///
/// See [`zng_wgt_image::mask`] for the full API.
pub mod mask {
    pub use zng_ext_image::ImageMaskMode;
    pub use zng_wgt_image::mask::{
        mask_align, mask_fit, mask_image, mask_image_cache, mask_image_downscale, mask_image_entries_mode, mask_image_limits, mask_mode,
        mask_offset,
    };
}
