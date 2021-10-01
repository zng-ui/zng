//! Image cache API.

use std::{
    cell::RefCell,
    collections::HashMap,
    error, fmt,
    path::PathBuf,
    rc::{self, Rc},
    sync::Arc,
    time::Duration,
};

//pub mod bmp;
//pub mod farbfeld;
//mod formats;
//pub use formats::*;

use zero_ui_view_api::ImageDataFormat;

use crate::render::webrender_api::ImageKey;
use crate::{
    app::{
        view_process::{Respawned, ViewImage, ViewProcess, ViewProcessRespawnedEvent, ViewRenderer, WeakViewImage},
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

pub use crate::app::view_process::ImagePpi;

/// Represents a loaded image.
#[derive(Clone)]
pub struct Image {
    view: ViewImage,
    render_keys: Rc<RefCell<Vec<RenderImage>>>,
}
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("size", &self.size())
            .field("render_keys", &format_args!("<{} keys>", self.render_keys.borrow().len()))
            .finish_non_exhaustive()
    }
}
impl Image {
    fn read(vars: &impl WithVars, view: &ViewProcess, path: impl Into<PathBuf>) -> ImageRequestVar {
        let path = path.into();
        let view = view.clone();

        let format = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| ImageDataFormat::FileExt(s.to_owned()))
            .unwrap_or(ImageDataFormat::Unknown);

        task::respond_ctor(vars, async {
            let r = task::wait(move || std::fs::read(path)).await;
            move || todo!()
        })
    }
    fn download<Vw, U>(vars: &Vw, view: &ViewProcess, uri: U) -> ImageRequestVar
    where
        Vw: WithVars,
        U: TryUri + Send + 'static,
    {
        task::respond_ctor(vars, async {
            use task::http::*;

            let request = Request::get(uri)?
                // image/webp decoder is only grayscale: https://docs.rs/image/0.23.14/image/codecs/webp/struct.WebPDecoder.html
                // image/avif decoder does not build in Windows
                .header(header::ACCEPT, "image/apng,image/*")?
                .build();

            let r = send(request).await;

            move || todo!()
        })
    }

    /// Create an [`Image`] from the [`ViewImage`].
    pub fn from_view(view_img: ViewImage) -> Self {
        Image {
            view: view_img,
            render_keys: Rc::default(),
        }
    }

    /// Reference the pixel size.
    #[inline]
    pub fn size(&self) -> PxSize {
        self.view.size()
    }

    /// Gets the image resolution in "pixel-per-inch" or "dot-per-inch" units.
    ///
    /// If the image format uses a different unit it is converted. Returns `(x, y)` resolutions
    /// most of the time both values are the same.
    pub fn ppi(&self) -> ImagePpi {
        self.view.ppi()
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

    /// Returns `true` if all pixels in the image are fully opaque.
    #[inline]
    pub fn opaque(&self) -> bool {
        self.view.opaque()
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
            view: self.view.downgrade(),
            render_keys: Rc::downgrade(&self.render_keys),
        }
    }

    /// If `self` and `other` are both pointers to the same image data.
    #[inline]
    pub fn ptr_eq(&self, other: &Image) -> bool {
        Rc::ptr_eq(&self.render_keys, &other.render_keys)
    }

    /// Reference the underlying view image.
    #[inline]
    pub fn view(&self) -> &ViewImage {
        &self.view
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

        // TODO can we send the image without cloning?
        let key = match renderer.add_image(&self.0.view) {
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
    view: WeakViewImage,
    render_keys: rc::Weak<RefCell<Vec<RenderImage>>>,
}
impl WeakImage {
    /// Attempts to upgrade to a strong reference.
    ///
    /// Returns `None` if the image no longer exists.
    pub fn upgrade(&self) -> Option<Image> {
        Some(Image {
            view: self.view.upgrade()?,
            render_keys: self.render_keys.upgrade()?,
        })
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
    /// Error from the view API implementation.
    View(String),

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
            ImageError::View(e) => fmt::Display::fmt(e, f),
            ImageError::Http(e) => fmt::Display::fmt(e, f),
            ImageError::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}
impl error::Error for ImageError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ImageError::View(e) => None,
            ImageError::Http(e) => e.source(),
            ImageError::Io(e) => e.source(),
        }
    }
}
impl From<String> for ImageError {
    fn from(e: String) -> Self {
        ImageError::View(e)
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
/// * [Images], only if the app is headed or headless with renderer.
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
        if let Some(view) = ctx.services.get::<ViewProcess>() {
            ctx.services.register(Images::new(view.clone()));
        }
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
    view: ViewProcess,
    proxies: Vec<Box<dyn ImageCacheProxy>>,
    cache: HashMap<ImageCacheKey, ImageRequestVar>,
}
impl Images {
    fn new(vp: ViewProcess) -> Self {
        Self {
            view: vp.clone(),
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
                ImageCacheKey::Read(path) => Image::read(vars, &self.view, path.clone()),
                ImageCacheKey::Download(uri) => Image::download(vars, &self.view, uri.clone()),
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
