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
        // convert to pre-multiplied BGRA, WebRender converts to this format internally if we don't

        let mut opaque = true;
        let ((width, height), bgra) = match image {
            DynamicImage::ImageLuma8(img) => (img.dimensions(), img.into_raw().into_iter().flat_map(|l| [l, l, l, 255]).collect()),
            DynamicImage::ImageLumaA8(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(2)
                    .flat_map(|la| {
                        if la[1] < 255 {
                            opaque = false;
                            let l = la[0] as f32 * la[1] as f32 / 255.0;
                            let l = l as u8;
                            [l, l, l, la[1]]
                        } else {
                            let l = la[0];
                            [l, l, l, la[1]]
                        }
                    })
                    .collect(),
            ),
            DynamicImage::ImageRgb8(img) => (
                img.dimensions(),
                img.into_raw().chunks(3).flat_map(|c| [c[2], c[1], c[0], 255]).collect(),
            ),
            DynamicImage::ImageRgba8(img) => (img.dimensions(), {
                let mut buf = img.into_raw();
                buf.chunks_mut(4).for_each(|c| {
                    if c[3] < 255 {
                        opaque = false;
                        let a = c[3] as f32 / 255.0;
                        c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                    }
                    c.swap(0, 2);
                });
                buf
            }),
            DynamicImage::ImageBgr8(img) => (
                img.dimensions(),
                img.into_raw().chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            ),
            DynamicImage::ImageBgra8(img) => (img.dimensions(), {
                let mut buf = img.into_raw();
                buf.chunks_mut(4).for_each(|c| {
                    if c[3] < 255 {
                        opaque = false;
                        let a = c[3] as f32 / 255.0;
                        c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                    }
                });
                buf
            }),
            DynamicImage::ImageLuma16(img) => (
                img.dimensions(),
                img.into_raw()
                    .into_iter()
                    .flat_map(|l| {
                        let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                        [l, l, l, 255]
                    })
                    .collect(),
            ),
            DynamicImage::ImageLumaA16(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(2)
                    .flat_map(|la| {
                        let max = u16::MAX as f32;
                        let l = la[0] as f32 / max;
                        let a = la[1] as f32 / max * 255.0;

                        if la[1] < u16::MAX {
                            opaque = false;
                            let l = (l * a) as u8;
                            [l, l, l, a as u8]
                        } else {
                            let l = (l * 255.0) as u8;
                            [l, l, l, a as u8]
                        }
                    })
                    .collect(),
            ),
            DynamicImage::ImageRgb16(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(3)
                    .flat_map(|c| {
                        let to_u8 = 255.0 / u16::MAX as f32;
                        [
                            (c[2] as f32 * to_u8) as u8,
                            (c[1] as f32 * to_u8) as u8,
                            (c[0] as f32 * to_u8) as u8,
                            255,
                        ]
                    })
                    .collect(),
            ),
            DynamicImage::ImageRgba16(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(4)
                    .flat_map(|c| {
                        if c[3] < u16::MAX {
                            opaque = false;
                            let max = u16::MAX as f32;
                            let a = c[3] as f32 / max * 255.0;
                            [
                                (c[2] as f32 / max * a) as u8,
                                (c[1] as f32 / max * a) as u8,
                                (c[0] as f32 / max * a) as u8,
                                a as u8,
                            ]
                        } else {
                            let to_u8 = 255.0 / u16::MAX as f32;
                            [
                                (c[2] as f32 * to_u8) as u8,
                                (c[1] as f32 * to_u8) as u8,
                                (c[0] as f32 * to_u8) as u8,
                                255,
                            ]
                        }
                    })
                    .collect(),
            ),
        };

        ImageState::Loaded {
            descriptor: ImageDescriptor::new(
                width as i32,
                height as i32,
                ImageFormat::BGRA8,
                if opaque {
                    ImageDescriptorFlags::IS_OPAQUE
                } else {
                    ImageDescriptorFlags::empty()
                },
            ),
            data: ImageData::Raw(Arc::new(bgra)),
        }
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
