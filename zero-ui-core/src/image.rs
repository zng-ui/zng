//! Image cache API.

use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    fmt,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use crate::{
    app::AppExtension,
    service::Service,
    task::{
        self,
        http::{self, TryUri},
    },
};
use image::DynamicImage;
use parking_lot::Mutex;
use webrender::api::*;

/// The [`Image`] cache service.
#[derive(Service)]
pub struct Images {
    proxies: Vec<Box<dyn ImageCacheProxy>>,
    file_cache: HashMap<PathBuf, Image>,
    uri_cache: HashMap<http::Uri, Image>,
}
impl Images {
    fn new() -> Self {
        Self {
            proxies: vec![],
            file_cache: HashMap::new(),
            uri_cache: HashMap::new(),
        }
    }

    /// Gets a cached image loaded from a `file`.
    pub fn get_file(&mut self, file: impl Into<PathBuf>) -> Image {
        let file = file.into();
        match self.proxy_get(|p| p.get_file(&file)) {
            ProxyGetResult::None => self.cache_file(file),
            ProxyGetResult::CacheFile(f) => self.cache_file(f),
            ProxyGetResult::CacheUri(u) => self.cache_uri(u),
            ProxyGetResult::Image(r) => r,
        }
    }
    fn cache_file(&mut self, file: PathBuf) -> Image {
        if let Some(img) = self.file_cache.get(&file) {
            img.clone()
        } else {
            let img = Image::from_file(file.clone());
            self.file_cache.insert(file, img.clone());
            img
        }
    }

    /// Gets a cached image downloaded from a `uri`.
    pub fn get_uri(&mut self, uri: impl TryUri) -> Image {
        match uri.try_into() {
            Ok(uri) => match self.proxy_get(|p| p.get_uri(&uri)) {
                ProxyGetResult::None => self.cache_uri(uri),
                ProxyGetResult::CacheFile(f) => self.cache_file(f),
                ProxyGetResult::CacheUri(u) => self.cache_uri(u),
                ProxyGetResult::Image(r) => r,
            },
            Err(e) => Image::from_error(Box::new(e)),
        }
    }
    fn cache_uri(&mut self, uri: http::Uri) -> Image {
        if let Some(img) = self.uri_cache.get(&uri) {
            img.clone()
        } else {
            let img = Image::from_uri(uri.clone());
            self.uri_cache.insert(uri, img.clone());
            img
        }
    }

    /// Remove the file from the cache, if it is only held by the cache.
    pub fn clean_file(&mut self, file: &Path) -> Option<Image> {
        self.remove_file(file, false)
    }

    /// Remove the file from the cache, even if it is still referenced outside of the cache.
    ///
    /// Returns `Some(img)` if the image was cached, and you can use [`Image::strong_count`] to determinate
    /// if dropping the image will remove if from memory.
    pub fn purge_file(&mut self, file: &Path) -> Option<Image> {
        self.remove_file(file, true)
    }

    fn remove_file(&mut self, file: &Path, purge: bool) -> Option<Image> {
        match self.proxy_remove(|p| p.remove_file(file, purge)) {
            ProxyRemoveResult::None => self.cache_remove_file(file, purge),
            ProxyRemoveResult::RemoveFile(f, purge) => self.cache_remove_file(&f, purge),
            ProxyRemoveResult::RemoveUri(u, purge) => self.cache_remove_uri(&u, purge),
            ProxyRemoveResult::Removed(r) => r,
        }
    }

    fn cache_remove_file(&mut self, file: &Path, purge: bool) -> Option<Image> {
        if purge {
            self.file_cache.remove(file)
        } else if let Some(img) = self.file_cache.get(file) {
            if img.strong_count() == 1 {
                self.file_cache.remove(file)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Remove the download file from the cache, if it is only held by the cache.
    pub fn clean_uri(&mut self, uri: &http::Uri) -> Option<Image> {
        self.remove_uri(uri, false)
    }

    /// Remove the download file from the cache, even if it is still referenced outside of the cache.
    ///
    /// Returns `Some(img)` if the image was cached, and you can use [`Image::strong_count`] to determinate
    /// if dropping the image will remove if from memory.
    pub fn purge_uri(&mut self, uri: &http::Uri) -> Option<Image> {
        self.remove_uri(uri, true)
    }

    fn remove_uri(&mut self, uri: &http::Uri, purge: bool) -> Option<Image> {
        match self.proxy_remove(|p| p.remove_uri(uri, purge)) {
            ProxyRemoveResult::None => self.cache_remove_uri(uri, purge),
            ProxyRemoveResult::RemoveFile(f, purge) => self.cache_remove_file(&f, purge),
            ProxyRemoveResult::RemoveUri(u, purge) => self.cache_remove_uri(&u, purge),
            ProxyRemoveResult::Removed(r) => r,
        }
    }
    fn cache_remove_uri(&mut self, uri: &http::Uri, purge: bool) -> Option<Image> {
        if purge {
            self.uri_cache.remove(uri)
        } else if let Some(img) = self.uri_cache.get(uri) {
            if img.strong_count() == 1 {
                self.uri_cache.remove(uri)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Associate the `image` with the `file` in the cache.
    ///
    /// Returns `Some(previous)` if the `file` was already associated with an image.
    #[inline]
    pub fn register_file(&mut self, file: impl Into<PathBuf>, image: Image) -> Option<Image> {
        self.file_cache.insert(file.into(), image)
    }

    /// Associate the `image` with the `uri` in the cache.
    ///
    /// Returns `Some(previous)` if the `uri` was already associated with an image.
    #[inline]
    pub fn register_uri(&mut self, uri: http::Uri, image: Image) -> Option<Image> {
        self.uri_cache.insert(uri, image)
    }

    /// Clear cached images that are not referenced outside of the cache.
    pub fn clean_cache(&mut self) {
        self.file_cache.retain(|_, img| img.strong_count() > 1);
        self.uri_cache.retain(|_, img| img.strong_count() > 1);
        self.proxies.iter_mut().for_each(|p| p.clear(false));
    }

    /// Clear all cached images, including images that are still referenced outside of the cache.
    ///
    /// Image memory only drops when all strong references are removed, so if an image is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_cache(&mut self) {
        self.file_cache.clear();
        self.uri_cache.clear();
        self.proxies.iter_mut().for_each(|p| p.clear(true));
    }

    /// Add a cache proxy.
    ///
    /// Proxies can intercept cache requests and map to a different request or return an image directly.
    pub fn install_proxy(&mut self, proxy: Box<dyn ImageCacheProxy>) {
        self.proxies.push(proxy);
    }
    fn proxy_get(&mut self, request: impl Fn(&mut Box<dyn ImageCacheProxy>) -> ProxyGetResult) -> ProxyGetResult {
        for proxy in &mut self.proxies {
            let r = request(proxy);
            if !matches!(r, ProxyGetResult::None) {
                return r;
            }
        }
        ProxyGetResult::None
    }
    fn proxy_remove(&mut self, request: impl Fn(&mut Box<dyn ImageCacheProxy>) -> ProxyRemoveResult) -> ProxyRemoveResult {
        for proxy in &mut self.proxies {
            let r = request(proxy);
            if !matches!(r, ProxyRemoveResult::None) {
                return r;
            }
        }
        ProxyRemoveResult::None
    }
}

/// A custom proxy in [`Images`].
///
/// Implementers can intercept cache requests and redirect to another cache request or returns an image directly.
pub trait ImageCacheProxy {
    /// Intercept a file request.
    fn get_file(&mut self, file: &Path) -> ProxyGetResult {
        let _ = file;
        ProxyGetResult::None
    }

    /// Intercept a download request.
    fn get_uri(&mut self, uri: &http::Uri) -> ProxyGetResult {
        let _ = uri;
        ProxyGetResult::None
    }

    /// Intercept a remove request.
    fn remove_file(&mut self, file: &Path, purge: bool) -> ProxyRemoveResult {
        let _ = (file, purge);
        ProxyRemoveResult::None
    }

    /// Intercept a remove request.
    fn remove_uri(&mut self, uri: &http::Uri, purge: bool) -> ProxyRemoveResult {
        let _ = (uri, purge);
        ProxyRemoveResult::None
    }

    /// Called when the cache is cleaned or purged.
    fn clear(&mut self, purge: bool);
}

/// Result of an [`ImageCacheProxy`] *get* redirect.
#[derive(Debug)]
pub enum ProxyGetResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Load and cache the file.
    CacheFile(PathBuf),
    /// Load and cache the Uri.
    CacheUri(http::Uri),
    /// Return the image instead of hitting the cache.
    Image(Image),
}

/// Result of an [`ImageCacheProxy`] *remove* redirect.
pub enum ProxyRemoveResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Removes another cached file.
    ///
    /// The `bool` indicates if the file should be purged.
    RemoveFile(PathBuf, bool),
    /// Removes another cached uri.
    ///
    /// The `bool` indicates if the file should be purged.
    RemoveUri(http::Uri, bool),
    /// Consider the request fulfilled.
    Removed(Option<Image>),
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

/// A reference to a loaded or loading image.
///
/// This struct is just pointers, cloning it is cheap. If all clones are dropped
/// the image data is dropped, including image data in renderers.
#[derive(Clone)]
pub struct Image {
    state: Arc<Mutex<ImageState>>,
    render_keys: Rc<RefCell<Vec<RenderImage>>>,
}
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image").finish_non_exhaustive()
    }
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
    ///
    /// The internal format is BGRA8 with pre-multiplied alpha, all other formats will be converted and if the
    /// format has an alpha component it will be pre-multiplied.
    pub async fn from_decoded(image: DynamicImage) -> Self {
        let state = task::run(async move { Arc::new(Mutex::new(Self::decoded_to_state(image))) }).await;

        Image {
            state,
            render_keys: Rc::default(),
        }
    }

    /// Create a loaded image from an image buffer that is already BGRA8 and with pre-multiplied alpha.
    ///
    /// The `is_opaque` argument indicates if the `image` is fully opaque, with all alpha values equal to `255`.
    pub fn from_premultiplied(image: image::ImageBuffer<image::Bgra<u8>, Vec<u8>>, is_opaque: bool) -> Self {
        let (width, height) = image.dimensions();
        let state = Arc::new(Mutex::new(ImageState::Loaded {
            descriptor: ImageDescriptor::new(
                width as i32,
                height as i32,
                ImageFormat::BGRA8,
                if is_opaque {
                    ImageDescriptorFlags::IS_OPAQUE
                } else {
                    ImageDescriptorFlags::empty()
                },
            ),
            data: ImageData::Raw(Arc::new(image.into_raw())),
        }));

        Image {
            state,
            render_keys: Rc::default(),
        }
    }

    /// Create a loaded image from a shared image buffer  that is already BGRA8 and with pre-multiplied alpha.
    ///
    /// The `is_opaque` argument indicates if the `bgra` is fully opaque, with all alpha values equal to `255`.
    ///
    /// # Panics
    ///
    /// Panics if the `bgra` length is not `width * height * 4`.
    pub fn from_raw(bgra: Arc<Vec<u8>>, width: u32, height: u32, is_opaque: bool) -> Self {
        assert_eq!(bgra.len(), width as usize * height as usize * 4);

        let state = Arc::new(Mutex::new(ImageState::Loaded {
            descriptor: ImageDescriptor::new(
                width as i32,
                height as i32,
                ImageFormat::BGRA8,
                if is_opaque {
                    ImageDescriptorFlags::IS_OPAQUE
                } else {
                    ImageDescriptorFlags::empty()
                },
            ),
            data: ImageData::Raw(bgra),
        }));
        Image {
            state,
            render_keys: Rc::default(),
        }
    }

    /// Create an *image* in the the error state with `load_error` representing the error.
    pub fn from_error(load_error: Box<dyn Error + Send + Sync>) -> Self {
        Image {
            state: Arc::new(Mutex::new(ImageState::Error(load_error))),
            render_keys: Rc::default(),
        }
    }

    /// Returns the loaded image data as a tuple of `(bgra, width, height, is_opaque)`.
    pub fn as_raw(&self) -> Option<(Arc<Vec<u8>>, u32, u32, bool)> {
        todo!()
    }

    /// Returns the number of strong references to this image.
    ///
    /// Note that the [`Images`] cache holds a strong reference to the image.
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.render_keys)
    }

    /// Returns a weak reference to this image.
    ///
    /// Weak references do not hold the image in memory but can be used to reacquire the [`Image`].
    pub fn downgrade(&self) -> WeakImage {
        WeakImage {
            state: Arc::downgrade(&self.state),
            render_keys: Rc::downgrade(&self.render_keys),
        }
    }

    /// If `self` and `other` are both pointers to the same image data.
    #[inline]
    pub fn ptr_eq(&self, other: &Image) -> bool {
        Rc::ptr_eq(&self.render_keys, &other.render_keys)
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

/// A weak reference to an [`Image`].
///
/// This weak reference does not hold the image in memory, but you can call [`upgrade`]
/// to attempt to reacquire the image, if it is still in memory.
///
/// [`upgrade`]: WeakImage::upgrade
pub struct WeakImage {
    state: std::sync::Weak<Mutex<ImageState>>,
    render_keys: std::rc::Weak<RefCell<Vec<RenderImage>>>,
}
impl WeakImage {
    /// Attempts to upgrade to a strong reference.
    ///
    /// Returns `None` if the image no longer exists.
    pub fn upgrade(&self) -> Option<Image> {
        Some(Image {
            state: self.state.upgrade()?,
            render_keys: self.render_keys.upgrade()?,
        })
    }

    /// Gets the number of [`Image`] references that are holding this image in memory.
    pub fn strong_count(&self) -> usize {
        self.render_keys.strong_count()
    }
}
