//! Image cache API.

use parking_lot::Mutex;
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    path::PathBuf,
    sync::{Arc, Weak},
    time::Duration,
};
use webrender::api::ImageKey;

use crate::{
    app::AppExtension,
    service::Service,
    task::{
        self,
        http::{TryUri, Uri},
    },
    var::{response_done_var, ResponseVar, Vars, WithVars},
};

/// Represents a loaded image.
#[derive(Debug, Clone)]
pub struct Image {
    bgra: Arc<Vec<u8>>,
    size: (u32, u32),
    opaque: bool,
    render_keys: Arc<Mutex<Vec<ImageKey>>>,
}
impl Image {
    /// Open and decode `path` from the file system.
    ///
    /// This is not cached, use [`Images::load`] to cache the request.
    pub async fn read(path: impl Into<PathBuf>) -> Result<Image, ImageError> {
        let path = path.into();
        task::run(async move {
            let img = task::wait(move || image::open(&path)).await?;
            Ok(Self::convert_decoded(img))
        })
        .await
    }

    /// Download and decode `uri` using an HTTP GET request.
    ///
    /// This is not cached, use [`Images::download`] to cache the request.
    pub async fn download(uri: impl TryUri) -> Result<Image, ImageError> {
        let uri = uri.try_into()?;
        task::run(async move {
            let img = task::http::get_bytes(uri).await?;
            let img = image::load_from_memory(&img)?;
            Ok(Self::convert_decoded(img))
        })
        .await
    }

    /// Convert a decoded image from the [`image`] crate to the internal format.
    ///
    /// The internal format if BGRA with pre-multiplied alpha.
    ///
    /// [`image`]: https://docs.rs/image/
    pub async fn from_decoded(image: image::DynamicImage) -> Image {
        task::run(async move { Self::convert_decoded(image) }).await
    }

    /// Create a loaded image from an image buffer that is already BGRA8 with pre-multiplied alpha.
    ///
    /// The `opaque` argument indicates if the `image` is fully opaque, with all alpha values equal to `255`.
    pub fn from_premultiplied(image: image::ImageBuffer<image::Bgra<u8>, Vec<u8>>, opaque: bool) -> Self {
        Image {
            size: image.dimensions(),
            opaque,
            bgra: Arc::new(image.into_raw()),
            render_keys: Arc::default(),
        }
    }

    /// Create an image from raw parts.
    pub fn from_raw(bgra: Arc<Vec<u8>>, size: (u32, u32), opaque: bool) -> Self {
        assert_eq!(bgra.len(), size.0 as usize * size.1 as usize * 4);

        Image {
            bgra,
            size,
            opaque,
            render_keys: Arc::default(),
        }
    }

    /// Reference the pre-multiplied BGRA buffer.
    #[inline]
    pub fn bgra(&self) -> &Arc<Vec<u8>> {
        &self.bgra
    }

    /// Reference the pixel size.
    #[inline]
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Gets the `(dpiX, dpiY)` pixel scaling metadata of the image.
    pub fn dpi(&self) -> (f32, f32) {
        // TODO
        (96.0, 96.0)
    }

    /// Time from image request to loaded.
    #[inline]
    pub fn load_time(&self) -> Duration {
        // TODO
        Duration::ZERO
    }

    /// Returns a flag that indicates if the image is fully opaque.
    #[inline]
    pub fn opaque(&self) -> bool {
        self.opaque
    }

    /// Returns the number of strong references to this image.
    ///
    /// Note that the [`Images`] cache holds a strong reference to the image.
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.render_keys)
    }

    /// Returns a weak reference to this image.
    ///
    /// Weak references do not hold the image in memory but can be used to reacquire the [`Image`].
    pub fn downgrade(&self) -> WeakImage {
        WeakImage {
            bgra: Arc::downgrade(&self.bgra),
            size: self.size,
            opaque: self.opaque,
            render_keys: Arc::downgrade(&self.render_keys),
        }
    }

    /// If `self` and `other` are both pointers to the same image data.
    #[inline]
    pub fn ptr_eq(&self, other: &Image) -> bool {
        Arc::ptr_eq(&self.render_keys, &other.render_keys)
    }

    fn convert_decoded(image: image::DynamicImage) -> Image {
        use image::DynamicImage::*;

        let mut opaque = true;
        let (size, bgra) = match image {
            ImageLuma8(img) => (img.dimensions(), img.into_raw().into_iter().flat_map(|l| [l, l, l, 255]).collect()),
            ImageLumaA8(img) => (
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
            ImageRgb8(img) => (
                img.dimensions(),
                img.into_raw().chunks(3).flat_map(|c| [c[2], c[1], c[0], 255]).collect(),
            ),
            ImageRgba8(img) => (img.dimensions(), {
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
            ImageBgr8(img) => (
                img.dimensions(),
                img.into_raw().chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            ),
            ImageBgra8(img) => (img.dimensions(), {
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
            ImageLuma16(img) => (
                img.dimensions(),
                img.into_raw()
                    .into_iter()
                    .flat_map(|l| {
                        let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                        [l, l, l, 255]
                    })
                    .collect(),
            ),
            ImageLumaA16(img) => (
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
            ImageRgb16(img) => (
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
            ImageRgba16(img) => (
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

        Image {
            bgra: Arc::new(bgra),
            size,
            opaque,
            render_keys: Arc::default(),
        }
    }
}
impl crate::render::Image for Image {
    fn image_key(&self, api: &Arc<webrender::api::RenderApi>) -> ImageKey {
        use webrender::api::*;

        let namespace = api.get_namespace_id();
        let mut keys = self.render_keys.lock();
        if let Some(key) = keys.iter().find(|k| k.0 == namespace) {
            return *key;
        }
        let key = api.generate_image_key();

        let mut txn = Transaction::new();
        txn.add_image(
            key,
            ImageDescriptor::new(
                self.size.0 as i32,
                self.size.1 as i32,
                ImageFormat::BGRA8,
                if self.opaque {
                    ImageDescriptorFlags::IS_OPAQUE
                } else {
                    ImageDescriptorFlags::empty()
                },
            ),
            ImageData::Raw(self.bgra.clone()),
            None,
        );
        api.update_resources(txn.resource_updates);

        keys.push(key);
        key
    }
}

/// A weak reference to an [`Image`].
///
/// This weak reference does not hold the image in memory, but you can call [`upgrade`]
/// to attempt to reacquire the image, if it is still in memory.
///
/// Use [`Image::downgrade`] to create an weak image reference.
///
/// [`upgrade`]: WeakImage::upgrade
pub struct WeakImage {
    bgra: Weak<Vec<u8>>,
    size: (u32, u32),
    opaque: bool,
    render_keys: Weak<Mutex<Vec<ImageKey>>>,
}
impl WeakImage {
    /// Attempts to upgrade to a strong reference.
    ///
    /// Returns `None` if the image no longer exists.
    pub fn upgrade(&self) -> Option<Image> {
        Some(Image {
            bgra: self.bgra.upgrade()?,
            render_keys: self.render_keys.upgrade()?,
            size: self.size,
            opaque: self.opaque,
        })
    }

    /// Gets the size of the image that may no longer exist.
    ///
    /// This value is local.
    #[inline]
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Gets the number of [`Image`] references that are holding this image in memory.
    pub fn strong_count(&self) -> usize {
        self.render_keys.strong_count()
    }
}

/// An error loading an [`Image`].
#[derive(Debug, Clone)]
pub enum ImageError {
    /// Error from the [`image`] crate.
    ///
    /// [`image`]: https://docs.rs/image/
    Image(Arc<image::error::ImageError>),

    /// Error from the [`task::http`] tasks.
    Http(task::http::Error),
}
impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::Image(e) => fmt::Display::fmt(e, f),
            ImageError::Http(e) => fmt::Display::fmt(e, f),
        }
    }
}
impl std::error::Error for ImageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ImageError::Image(e) => e.source(),
            ImageError::Http(e) => e.source(),
        }
    }
}
impl From<image::error::ImageError> for ImageError {
    fn from(e: image::error::ImageError) -> Self {
        ImageError::Image(Arc::new(e))
    }
}
impl From<task::http::Error> for ImageError {
    fn from(e: task::http::Error) -> Self {
        ImageError::Http(e)
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

/// The [`Image`] cache service.
///
/// # Cache
///
/// The cache holds images in memory, configured TODO.
///
/// Image downloads are also cached in disk because the requests are
/// made using [`http::get_bytes_cached`].
#[derive(Service)]
pub struct Images {
    proxies: Vec<Box<dyn ImageCacheProxy>>,
    cache: HashMap<ImageCacheKey, CachedImageVar>,
}
impl Images {
    fn new() -> Self {
        Self {
            proxies: vec![],
            cache: HashMap::default(),
        }
    }

    /// Get or load an image file from a file system `path`.
    pub fn read<Vw: WithVars>(&mut self, path: impl Into<PathBuf>, vars: &Vw) -> CachedImageVar {
        self.get(ImageCacheKey::Read(path.into()), vars)
    }

    /// Get a cached `uri` or download it.
    pub fn download<Vw: WithVars>(&mut self, uri: impl TryUri, vars: &Vw) -> CachedImageVar {
        match uri.try_into() {
            Ok(uri) => self.get(ImageCacheKey::Download(uri), vars),
            Err(e) => response_done_var(Err(e.into())),
        }
    }

    /// Get a cached image or add it to the cache.
    pub fn get<Vw: WithVars>(&mut self, key: ImageCacheKey, vars: &Vw) -> CachedImageVar {
        vars.with_vars(move |vars| self.proxy_then_get(key, vars))
    }

    /// Associate the `image` with the `key` in the cache.
    ///
    /// Returns `Some(previous)` if the `key` was already associated with an image.
    #[inline]
    pub fn register(&mut self, key: ImageCacheKey, image: CachedImageVar) -> Option<CachedImageVar> {
        self.cache.insert(key, image)
    }

    /// Remove the image from the cache, if it is only held by the cache.
    pub fn clean(&mut self, key: ImageCacheKey) -> Option<CachedImageVar> {
        self.proxy_then_remove(key, false)
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// Returns `Some(img)` if the image was cached.
    pub fn purge(&mut self, key: ImageCacheKey) -> Option<CachedImageVar> {
        self.proxy_then_remove(key, true)
    }

    fn proxy_then_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<CachedImageVar> {
        for proxy in &mut self.proxies {
            let r = proxy.remove(&key, purge);
            match r {
                ProxyRemoveResult::None => continue,
                ProxyRemoveResult::Remove(r, p) => return self.proxied_remove(r, p),
                ProxyRemoveResult::Removed(img) => return img,
            }
        }
        self.proxied_remove(key, purge)
    }
    fn proxied_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<CachedImageVar> {
        if purge {
            self.cache.remove(&key)
        } else {
            todo!()
        }
    }

    /// Clear cached images that are not referenced outside of the cache.
    pub fn clean_all(&mut self) {
        self.proxies.iter_mut().for_each(|p| p.clear(false));
        todo!()
    }

    /// Clear all cached images, including images that are still referenced outside of the cache.
    ///
    /// Image memory only drops when all strong references are removed, so if an image is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_all(&mut self) {
        self.cache.clear();
        self.proxies.iter_mut().for_each(|p| p.clear(true));
    }

    /// Add a cache proxy.
    ///
    /// Proxies can intercept cache requests and map to a different request or return an image directly.
    pub fn install_proxy(&mut self, proxy: Box<dyn ImageCacheProxy>) {
        self.proxies.push(proxy);
    }

    fn proxy_then_get(&mut self, key: ImageCacheKey, vars: &Vars) -> CachedImageVar {
        for proxy in &mut self.proxies {
            let r = proxy.get(&key);
            match r {
                ProxyGetResult::None => continue,
                ProxyGetResult::Cache(r) => return self.proxied_get(r, vars),
                ProxyGetResult::Image(img) => return img,
            }
        }
        self.proxied_get(key, vars)
    }
    fn proxied_get(&mut self, key: ImageCacheKey, vars: &Vars) -> CachedImageVar {
        self.cache
            .entry(key)
            .or_insert_with_key(|key| match key {
                ImageCacheKey::Read(path) => task::respond(vars, Image::read(path.clone())),
                ImageCacheKey::Download(uri) => task::respond(vars, Image::download(uri.clone())),
            })
            .clone()
    }
}

/// A variable that represents a loading or loaded cached image.
pub type CachedImageVar = ResponseVar<Result<Image, ImageError>>;

/// Key for a cached image in [`Images`].
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum ImageCacheKey {
    /// A path to an image file in the file system.
    Read(PathBuf),
    /// A uri to an image resource downloaded using HTTP GET.
    Download(Uri),
}

/// A custom proxy in [`Images`].
///
/// Implementers can intercept cache requests and redirect to another cache request or returns an image directly.
pub trait ImageCacheProxy {
    /// Intercept a get request.
    fn get(&mut self, key: &ImageCacheKey) -> ProxyGetResult {
        let _ = key;
        ProxyGetResult::None
    }

    /// Intercept a remove request.
    fn remove(&mut self, key: &ImageCacheKey, purge: bool) -> ProxyRemoveResult {
        let _ = (key, purge);
        ProxyRemoveResult::None
    }

    /// Called when the cache is cleaned or purged.
    fn clear(&mut self, purge: bool);
}

/// Result of an [`ImageCacheProxy`] *get* redirect.
pub enum ProxyGetResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Load and cache using the replacement key.
    Cache(ImageCacheKey),
    /// Return the image instead of hitting the cache.
    Image(CachedImageVar),
}

/// Result of an [`ImageCacheProxy`] *remove* redirect.
pub enum ProxyRemoveResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Removes another cached entry.
    ///
    /// The `bool` indicates if the entry should be purged.
    Remove(ImageCacheKey, bool),
    /// Consider the request fulfilled.
    Removed(Option<CachedImageVar>),
}
