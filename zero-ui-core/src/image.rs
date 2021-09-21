//! Image cache API.

use crate::render::webrender_api::ImageKey;
use std::{
    cell::RefCell,
    collections::HashMap,
    error, fmt,
    path::PathBuf,
    rc::{self, Rc},
    sync::{Arc, Weak},
    time::Duration,
};

//pub mod bmp;
//pub mod farbfeld;
//mod formats;
//pub use formats::*;

use crate::{
    app::{
        view_process::{Respawned, ViewProcessRespawnedEvent, ViewRenderer},
        AppExtension,
    },
    context::{AppContext, LayoutMetrics},
    event::EventUpdateArgs,
    service::Service,
    task::{
        self,
        http::{TryUri, Uri},
    },
    units::*,
    var::{response_done_var, ResponseVar, Vars, WithVars},
};

/// Represents a loaded image.
#[derive(Clone)]
pub struct Image {
    bgra: Arc<Vec<u8>>,
    size: PxSize,
    opaque: bool,
    render_keys: Rc<RefCell<Vec<RenderImage>>>,
}
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("size", &self.size)
            .field("opaque", &self.opaque)
            .field("bgra", &format_args!("<{} bytes>", self.bgra.len()))
            .field("render_keys", &format_args!("<{} keys>", self.render_keys.borrow().len()))
            .finish()
    }
}
impl Image {
    /// Open and decode `path` from the file system.
    ///
    /// This is not cached, use [`Images::read`] to cache the request.
    pub async fn read(path: impl Into<PathBuf>) -> Result<Image, ImageError> {
        let path = path.into();
        let (bgra, size, opaque) = task::run(async move { Self::read_raw(path).await }).await?;
        Ok(Image::from_raw(bgra, size, opaque))
    }

    /// Like [`read`] but sets a response variable.
    ///
    /// [`read`]: Self::read
    pub fn read_rsp(vars: &impl WithVars, path: impl Into<PathBuf>) -> ImageRequestVar {
        let path = path.into();
        task::respond_ctor(vars, async move {
            let r = Self::read_raw(path).await;
            move || r.map(|(b, s, o)| Image::from_raw(b, s, o))
        })
    }

    async fn read_raw(path: PathBuf) -> Result<(Arc<Vec<u8>>, (u32, u32), bool), ImageError> {
        let img = task::wait(move || image::open(&path)).await?;
        Ok(Self::convert_decoded(img))
    }

    /// Download and decode `uri` using an HTTP GET request.
    ///
    /// This is not cached, use [`Images::download`] to cache the request.
    pub async fn download(uri: impl TryUri) -> Result<Image, ImageError> {
        let uri = uri.try_into()?;
        let (bgra, size, opaque) = task::run(async move { Self::download_raw(uri).await }).await?;
        Ok(Image::from_raw(bgra, size, opaque))
    }

    /// Like [`download`] but sets a response variable.
    ///
    /// [`download`]: Self::download
    pub fn download_rsp<Vw, U>(vars: &Vw, uri: U) -> ImageRequestVar
    where
        Vw: WithVars,
        U: TryUri + Send + 'static,
    {
        task::respond_ctor(vars, async move {
            let r = Self::download_raw(uri).await;
            move || r.map(|(b, s, o)| Image::from_raw(b, s, o))
        })
    }

    async fn download_raw(uri: impl TryUri) -> Result<(Arc<Vec<u8>>, (u32, u32), bool), ImageError> {
        use task::http::*;

        let img = send(
            Request::get(uri)?
                // image/webp decoder is only grayscale: https://docs.rs/image/0.23.14/image/codecs/webp/struct.WebPDecoder.html
                // image/avif decoder does not build in Windows
                .header(header::ACCEPT, "image/apng,image/*")?
                .build(),
        )
        .await?
        .bytes()
        .await?;

        let img = image::load_from_memory(&img)?;
        Ok(Self::convert_decoded(img))
    }

    /// Convert a decoded image from the [`image`] crate to the internal format.
    ///
    /// The internal format if BGRA with pre-multiplied alpha.
    ///
    /// [`image`]: https://docs.rs/image/
    pub async fn from_decoded(image: image::DynamicImage) -> Image {
        let (bgra, size, opaque) = task::run(async move { Self::convert_decoded(image) }).await;
        Image::from_raw(bgra, size, opaque)
    }

    /// Like [`from_decoded`] by sets a response variable.
    ///
    /// [`from_decoded`]: Self::from_decoded
    pub fn from_decoded_rsp(vars: &impl WithVars, image: image::DynamicImage) -> ResponseVar<Image> {
        task::respond_ctor(vars, async move {
            let (b, s, o) = Self::convert_decoded(image);
            move || Image::from_raw(b, s, o)
        })
    }

    /// Create a loaded image from an image buffer that is already BGRA8 with pre-multiplied alpha.
    ///
    /// The `opaque` argument indicates if the `image` is fully opaque, with all alpha values equal to `255`.
    pub fn from_premultiplied(image: image::ImageBuffer<image::Bgra<u8>, Vec<u8>>, opaque: bool) -> Self {
        let (w, h) = image.dimensions();
        Image {
            size: PxSize::new(Px(w as i32), Px(h as i32)),
            opaque,
            bgra: Arc::new(image.into_raw()),
            render_keys: Rc::default(),
        }
    }

    /// Create an image from raw parts.
    pub fn from_raw(bgra: Arc<Vec<u8>>, size: (u32, u32), opaque: bool) -> Self {
        assert_eq!(bgra.len(), size.0 as usize * size.1 as usize * 4);

        Image {
            bgra,
            size: PxSize::new(Px(size.0 as i32), Px(size.1 as i32)),
            opaque,
            render_keys: Rc::default(),
        }
    }

    /// Reference the pre-multiplied BGRA buffer.
    #[inline]
    pub fn bgra(&self) -> &Arc<Vec<u8>> {
        &self.bgra
    }

    /// Reference the pixel size.
    #[inline]
    pub fn size(&self) -> PxSize {
        self.size
    }

    /// Gets the image resolution in "pixel-per-inch" or "dot-per-inch" units.
    ///
    /// If the image format uses a different unit it is converted. Returns `(x, y)` resolutions
    /// most of the time both values are the same.
    ///
    /// # Metadata
    ///
    /// Currently the metadata supported are:
    ///
    /// * EXIF resolution. TODO
    /// * Jpeg built-in. TODO
    /// * PNG built-in. TODO
    /// * TODO check other `image` formats.
    pub fn ppi(&self) -> Option<(f32, f32)> {
        // TODO
        None
    }

    /// Calculate an *ideal* layout size for the image.
    ///
    /// The image is scaled considering the [`ppi`] and screen scale factor. If the
    /// image has no [`ppi`] falls-back to the [`screen_ppi`] in both dimensions.
    ///
    /// [`ppi`]: Self::ppi
    /// [`screen_ppi`]: LayoutMetrics::screen_ppi
    #[inline]
    pub fn layout_size(&self, ctx: &LayoutMetrics) -> PxSize {
        self.calc_size(ctx, (ctx.screen_ppi, ctx.screen_ppi), false)
    }

    /// Calculate a layout size for the image.
    ///
    /// # Parameters
    ///
    /// * `ctx`: Used to get the screen resolution.
    /// * `fallback_ppi`: Resolution used if [`ppi`] is `None`.
    /// * `ignore_image_ppi`: If `true` always uses the `fallback_ppi` as the resolution.
    ///
    /// [`ppi`]: Self::ppi
    pub fn calc_size(&self, ctx: &LayoutMetrics, fallback_ppi: (f32, f32), ignore_image_ppi: bool) -> PxSize {
        let (dpi_x, dpi_y) = if ignore_image_ppi {
            fallback_ppi
        } else {
            self.ppi().unwrap_or(fallback_ppi)
        };

        let screen_res = ctx.screen_ppi;
        let mut s = self.size();

        s.width *= (dpi_x / screen_res) * ctx.scale_factor;
        s.height *= (dpi_y / screen_res) * ctx.scale_factor;

        s
    }

    /// Gets the flip-rotate transform requested by the image metadata.
    ///
    /// Returns the computed flip-rotate render transform. The transform does not include the resolution scaling.
    ///
    /// # Metadata
    ///
    /// Currently the metadata supported are:
    ///
    /// * EXIF flip and rotate. TODO
    pub fn transform(&self) -> Transform {
        Transform::identity()
    }

    /// Time from image request to loaded.
    #[inline]
    pub fn load_time(&self) -> Duration {
        // TODO
        Duration::ZERO
    }

    /// Returns `true` if all pixels in the image are fully opaque.
    #[inline]
    pub fn opaque(&self) -> bool {
        self.opaque
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
            bgra: Arc::downgrade(&self.bgra),
            size: self.size,
            opaque: self.opaque,
            render_keys: Rc::downgrade(&self.render_keys),
        }
    }

    /// If `self` and `other` are both pointers to the same image data.
    #[inline]
    pub fn ptr_eq(&self, other: &Image) -> bool {
        Rc::ptr_eq(&self.render_keys, &other.render_keys)
    }

    fn convert_decoded(image: image::DynamicImage) -> (Arc<Vec<u8>>, (u32, u32), bool) {
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

        (Arc::new(bgra), size, opaque)
    }
}
impl crate::render::Image for Image {
    fn image_key(&self, renderer: &ViewRenderer) -> ImageKey {
        use crate::render::webrender_api::*;

        let namespace = match renderer.namespace_id() {
            Ok(n) => n,
            Err(Respawned) => {
                log::debug!("respawned calling `namespace_id`, will return DUMMY");
                return ImageKey::DUMMY;
            }
        };
        let mut rms = self.render_keys.borrow_mut();
        if let Some(rm) = rms.iter().find(|k| k.key.0 == namespace) {
            return rm.key;
        }

        let descriptor = ImageDescriptor::new(
            self.size.width.0,
            self.size.height.0,
            ImageFormat::BGRA8,
            if self.opaque {
                ImageDescriptorFlags::IS_OPAQUE
            } else {
                ImageDescriptorFlags::empty()
            },
        );

        // TODO can we send the image without cloning?
        let key = match renderer.add_image(descriptor, (*self.bgra).clone()) {
            Ok(k) => k,
            Err(Respawned) => {
                log::debug!("respawned `add_image`, will return DUMMY");
                return ImageKey::DUMMY;
            }
        };

        rms.push(RenderImage {
            key,
            renderer: renderer.clone(),
        });
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
    size: PxSize,
    opaque: bool,
    render_keys: rc::Weak<RefCell<Vec<RenderImage>>>,
}
impl WeakImage {
    /// Attempts to upgrade to a strong reference.
    ///
    /// Returns `None` if the image no longer exists.
    pub fn upgrade(&self) -> Option<Image> {
        Some(Image {
            render_keys: self.render_keys.upgrade()?,
            bgra: self.bgra.upgrade()?,
            size: self.size,
            opaque: self.opaque,
        })
    }

    /// Gets the size of the image that may no longer exist.
    ///
    /// This value is local.
    #[inline]
    pub fn size(&self) -> PxSize {
        self.size
    }

    /// Gets the number of [`Image`] references that are holding this image in memory.
    pub fn strong_count(&self) -> usize {
        self.render_keys.strong_count()
    }
}

struct RenderImage {
    key: ImageKey,
    renderer: ViewRenderer,
}
impl Drop for RenderImage {
    fn drop(&mut self) {
        // error here means the entire renderer was dropped.
        let _ = self.renderer.delete_image(self.key);
    }
}
impl fmt::Debug for RenderImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.key, f)
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

    /// An IO error.
    ///
    /// Note that the other variants can also contains an IO error.
    Io(Arc<std::io::Error>),
}
impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::Image(e) => fmt::Display::fmt(e, f),
            ImageError::Http(e) => fmt::Display::fmt(e, f),
            ImageError::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}
impl error::Error for ImageError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ImageError::Image(e) => e.source(),
            ImageError::Http(e) => e.source(),
            ImageError::Io(e) => e.source(),
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
impl From<std::io::Error> for ImageError {
    fn from(e: std::io::Error) -> Self {
        ImageError::Io(Arc::new(e))
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
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Images::new());
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if ViewProcessRespawnedEvent.update(args).is_some() {
            for v in ctx.services.images().cache.values() {
                if let Some(Ok(img)) = v.rsp(ctx.vars) {
                    // TODO how to respawn disconnected (non-cached) images?.
                    img.render_keys.borrow_mut().clear();
                }
            }
        }
    }
}

/// The [`Image`] cache service.
///
/// # Cache
///
/// The cache holds images in memory, configured TODO.
#[derive(Service)]
pub struct Images {
    proxies: Vec<Box<dyn ImageCacheProxy>>,
    cache: HashMap<ImageCacheKey, ImageRequestVar>,
}
impl Images {
    fn new() -> Self {
        Self {
            proxies: vec![],
            cache: HashMap::default(),
        }
    }

    /// Get or load an image file from a file system `path`.
    pub fn read<Vw: WithVars>(&mut self, vars: &Vw, path: impl Into<PathBuf>) -> ImageRequestVar {
        self.get(vars, ImageCacheKey::Read(path.into()))
    }

    /// Get a cached `uri` or download it.
    pub fn download<Vw: WithVars>(&mut self, vars: &Vw, uri: impl TryUri) -> ImageRequestVar {
        match uri.try_into() {
            Ok(uri) => self.get(vars, ImageCacheKey::Download(uri)),
            Err(e) => response_done_var(Err(e.into())),
        }
    }

    /// Get a cached image or add it to the cache.
    pub fn get<Vw: WithVars>(&mut self, vars: &Vw, key: ImageCacheKey) -> ImageRequestVar {
        vars.with_vars(move |vars| self.proxy_then_get(key, vars))
    }

    /// Associate the `image` with the `key` in the cache.
    ///
    /// Returns `Some(previous)` if the `key` was already associated with an image.
    #[inline]
    pub fn register(&mut self, key: ImageCacheKey, image: ImageRequestVar) -> Option<ImageRequestVar> {
        self.cache.insert(key, image)
    }

    /// Remove the image from the cache, if it is only held by the cache.
    pub fn clean(&mut self, key: ImageCacheKey) -> Option<ImageRequestVar> {
        self.proxy_then_remove(key, false)
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// Returns `Some(img)` if the image was cached.
    pub fn purge(&mut self, key: ImageCacheKey) -> Option<ImageRequestVar> {
        self.proxy_then_remove(key, true)
    }

    fn proxy_then_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<ImageRequestVar> {
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
    fn proxied_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<ImageRequestVar> {
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

    fn proxy_then_get(&mut self, key: ImageCacheKey, vars: &Vars) -> ImageRequestVar {
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
    fn proxied_get(&mut self, key: ImageCacheKey, vars: &Vars) -> ImageRequestVar {
        self.cache
            .entry(key)
            .or_insert_with_key(|key| match key {
                ImageCacheKey::Read(path) => Image::read_rsp(vars, path.clone()),
                ImageCacheKey::Download(uri) => Image::download_rsp(vars, uri.clone()),
            })
            .clone()
    }
}

/// A variable that represents a loading or loaded image.
pub type ImageRequestVar = ResponseVar<Result<Image, ImageError>>;

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
    Image(ImageRequestVar),
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
    Removed(Option<ImageRequestVar>),
}
