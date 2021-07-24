//! Image cache API.

use std::{error::Error, path::PathBuf, sync::Arc};

use crate::{
    app::AppExtension,
    service::Service,
    task::{self, http::TryUri},
};
use image::DynamicImage;
use parking_lot::Mutex;
use webrender::api::*;

/// The [`Image`] cache service.
#[derive(Service)]
pub struct Images {}
impl Images {
    fn new() -> Self {
        Self {}
    }
}

/// Application extension that provides an image cache.
///
/// # Services
///
/// Services this extension provides.
///
/// * [Images]
///
/// # Default
///
/// This extension is included in the [default app], events provided by it
/// are required by multiple other extensions.
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct ImageManager {}
impl AppExtension for ImageManager {
    fn init(&mut self, ctx: &mut crate::context::AppContext) {
        ctx.services.register(Images::new());
    }
}

/// A loaded or loading image.
///
/// This struct is an [`Arc`] pointer, cloning it is cheap.
#[derive(Clone, Debug)]
pub struct Image(Arc<Mutex<ImageState>>);
impl Image {
    /// Start loading an image file.
    pub fn from_file(file: impl Into<PathBuf>) -> Self {
        let file = file.into();
        let state = Arc::new(Mutex::new(ImageState::Loading));
        task::spawn_wait(clone_move!(state, || {
            *state.lock() = match image::open(file) {
                Ok(img) => Self::decoded_to_state(img),
                Err(e) => ImageState::Error(Box::new(e)),
            }
        }));
        Image(state)
    }

    /// Start loading an image from the web using a GET request.
    pub fn from_uri(uri: impl TryUri) -> Self {
        match uri.try_into() {
            Ok(uri) => {
                let state = Arc::new(Mutex::new(ImageState::Loading));
                task::spawn(async_clone_move!(state, {
                    *state.lock() = match Self::download_image(uri).await {
                        Ok(img) => Self::decoded_to_state(img),
                        Err(e) => ImageState::Error(e),
                    }
                }));
                Image(state)
            }
            Err(e) => Image(Arc::new(Mutex::new(ImageState::Error(Box::new(e))))),
        }
    }

    /// Create a loaded image.
    pub fn from_decoded(image: DynamicImage) -> Self {
        let state = Arc::new(Mutex::new(Self::decoded_to_state(image)));
        Image(state)
    }

    fn decoded_to_state(image: DynamicImage) -> ImageState {
        use image::{buffer::ConvertBuffer, Bgra, ImageBuffer, Luma, LumaA, RgbaImage};

        let ((width, height), format, data, opaque) = match image {
            DynamicImage::ImageLuma8(img) => (img.dimensions(), ImageFormat::R8, img.into_raw(), true),
            DynamicImage::ImageLumaA8(img) => (img.dimensions(), ImageFormat::RG8, img.into_raw(), false),
            DynamicImage::ImageRgb8(img) => (
                img.dimensions(),
                ImageFormat::RGBA8,
                ConvertBuffer::<RgbaImage>::convert(&img).into_raw(),
                true,
            ),
            DynamicImage::ImageRgba8(img) => (img.dimensions(), ImageFormat::RGBA8, img.into_raw(), false),
            DynamicImage::ImageBgr8(img) => (
                img.dimensions(),
                ImageFormat::BGRA8,
                ConvertBuffer::<ImageBuffer<Bgra<u8>, Vec<u8>>>::convert(&img).into_raw(),
                true,
            ),
            DynamicImage::ImageBgra8(img) => (img.dimensions(), ImageFormat::BGRA8, img.into_raw(), false),
            DynamicImage::ImageLuma16(img) => (
                img.dimensions(),
                ImageFormat::R8, // TODO use R16
                ConvertBuffer::<ImageBuffer<Luma<u8>, Vec<u8>>>::convert(&img).into_raw(),
                true,
            ),
            DynamicImage::ImageLumaA16(img) => (
                img.dimensions(),
                ImageFormat::RG8, // TODO use RG16
                ConvertBuffer::<ImageBuffer<LumaA<u8>, Vec<u8>>>::convert(&img).into_raw(),
                false,
            ),
            DynamicImage::ImageRgb16(img) => (
                img.dimensions(),
                ImageFormat::RGBA8,
                ConvertBuffer::<RgbaImage>::convert(&img).into_raw(),
                true,
            ),
            DynamicImage::ImageRgba16(img) => (
                img.dimensions(),
                ImageFormat::RGBA8,
                ConvertBuffer::<RgbaImage>::convert(&img).into_raw(),
                false,
            ),
        };
        let flags = if opaque {
            ImageDescriptorFlags::IS_OPAQUE
        } else {
            ImageDescriptorFlags::empty()
        };
        let descriptor = ImageDescriptor::new(width as i32, height as i32, format, flags);
        let data = ImageData::Raw(Arc::new(data));
        ImageState::Loaded { data, descriptor }
    }

    async fn download_image(uri: task::http::Uri) -> Result<DynamicImage, Box<dyn Error + Send + Sync>> {
        let r = task::http::get_bytes(uri).await?;
        let img = image::load_from_memory(&r)?;
        Ok(img)
    }

    fn render_image(&self, api: &Arc<RenderApi>) -> ImageKey {
        api.generate_image_key()
    }
}
impl crate::render::Image for Image {
    fn image_key(&self, api: &Arc<RenderApi>) -> ImageKey {
        self.render_image(api)
    }
}
#[derive(Debug)]
enum ImageState {
    Loading,
    Loaded { data: ImageData, descriptor: ImageDescriptor },
    Error(Box<dyn Error + Send + Sync>),
}
