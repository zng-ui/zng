//! Images service, widget and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_image`] for the full image API and [`zero_ui_wgt_image`] for the full widget API.

pub use zero_ui_ext_image::{
    render_retain, ImageCacheMode, ImageDataFormat, ImageDownscale, ImageHash, ImageHasher, ImageLimits, ImageMaskMode, ImagePpi,
    ImageRenderArgs, ImageSource, ImageSourceFilter, ImageVar, Img, PathFilter, IMAGES, IMAGE_RENDER,
};

#[cfg(http)]
pub use zero_ui_ext_image::UriFilter;

pub use zero_ui_wgt_image::{
    img_align, img_cache, img_crop, img_downscale, img_error_fn, img_fit, img_limits, img_loading_fn, img_offset, img_rendering,
    img_repeat, img_repeat_spacing, img_scale, img_scale_factor, img_scale_ppi, is_error, is_loaded, on_error, on_load, Image, ImageFit,
    ImageRepeat, ImgErrorArgs, ImgLoadArgs, ImgLoadingArgs,
};

/// Mask image properties.
///
/// See [`zero_ui_wgt_image::mask`] for the full API.
pub mod mask {
    pub use zero_ui_wgt_image::mask::{
        mask_align, mask_fit, mask_image, mask_image_cache, mask_image_downscale, mask_image_limits, mask_mode, mask_offset,
    };
}
