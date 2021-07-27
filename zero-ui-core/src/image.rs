//! Image cache API.

use std::{cell::RefCell, error::Error, path::PathBuf, rc::Rc, sync::Arc};

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
/// This struct is just pointers, cloning it is cheap.
#[derive(Clone)]
pub struct Image {
    state: Arc<Mutex<ImageState>>,
    render_keys: Rc<RefCell<Vec<RenderImage>>>,
}
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
        Image {
            state,
            render_keys: Rc::default(),
        }
    }

    /// Start loading an image from the web using a GET request.
    pub fn from_uri(uri: impl TryUri) -> Self {
        let state = match uri.try_into() {
            Ok(uri) => {
                let state = Arc::new(Mutex::new(ImageState::Loading));
                task::spawn(async_clone_move!(state, {
                    *state.lock() = match Self::download_image(uri).await {
                        Ok(img) => Self::decoded_to_state(img),
                        Err(e) => ImageState::Error(e),
                    }
                }));
                state
            }
            Err(e) => Arc::new(Mutex::new(ImageState::Error(Box::new(e)))),
        };
        Image {
            state,
            render_keys: Rc::default(),
        }
    }

    /// Create a loaded image.
    pub fn from_decoded(image: DynamicImage) -> Self {
        let state = Arc::new(Mutex::new(Self::decoded_to_state(image)));
        Image {
            state,
            render_keys: Rc::default(),
        }
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
        let namespace = api.get_namespace_id();
        let mut keys = self.render_keys.borrow_mut();
        for r in keys.iter_mut() {
            if r.key.0 == namespace {
                if !r.loaded {                           
                    if let ImageState::Loaded { descriptor, data } = &*self.state.lock() {
                        r.loaded = true;
                        Self::load_image(api, r.key, *descriptor, data.clone())
                    }
                }
                return r.key;
            }
        }

        let key = api.generate_image_key();

        let mut loaded = false;        
        if let ImageState::Loaded { descriptor, data } = &*self.state.lock() {
            loaded = true;
            Self::load_image(api, key, *descriptor, data.clone())
        }

        keys.push(RenderImage {
            api: Arc::downgrade(api),
            key,
            loaded,
        });        

        key
    }

    fn load_image(api: &Arc<RenderApi>, key: ImageKey, descriptor: ImageDescriptor, data: ImageData) {
        let mut txn = webrender::api::Transaction::new();
        txn.add_image(key, descriptor, data, None);
        api.update_resources(txn.resource_updates);
    }
}
impl crate::render::Image for Image {
    fn image_key(&self, api: &Arc<RenderApi>) -> ImageKey {
        self.render_image(api)
    }
}
enum ImageState {
    Loading,
    Loaded { descriptor: ImageDescriptor, data: ImageData },
    Error(Box<dyn Error + Send + Sync>),
}
struct RenderImage {
    api: std::sync::Weak<RenderApi>,
    key: ImageKey,
    loaded: bool,
}
impl Drop for RenderImage {
    fn drop(&mut self) {
        if let Some(api) = self.api.upgrade() {
            let mut txn = webrender::api::Transaction::new();
            txn.delete_image(self.key);
            api.update_resources(txn.resource_updates);
        }
    }
}
