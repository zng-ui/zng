//! Image cache API.

use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    fmt,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
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
use webrender::api::*;

/// Key for a cached image in [`Images`].
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum ImageCacheKey {
    /// A path to an image file.
    Path(PathBuf),
    /// A uri to an image resource.
    Uri(http::Uri),
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
    cache: HashMap<ImageCacheKey, Image>,
    config: ImageCacheConfig,
}
impl Images {
    fn new() -> Self {
        Self {
            proxies: vec![],
            cache: HashMap::new(),
            config: ImageCacheConfig::default(),
        }
    }

    /// Gets a cached image loaded from a path or uri.
    pub fn get(&mut self, key: ImageCacheKey) -> Image {
        match self.proxy_get(&key) {
            ProxyGetResult::None => self.cache(key),
            ProxyGetResult::Cache(k) => self.cache(k),
            ProxyGetResult::Image(r) => r,
        }
    }
    fn cache(&mut self, key: ImageCacheKey) -> Image {
        self.cache_collect();
        self.cache
            .entry(key)
            .or_insert_with_key(|k| {                
                match k {
                    ImageCacheKey::Path(p) => Image::from_file(p.clone()),
                    ImageCacheKey::Uri(u) => Image::from_uri(u.clone()),
                }
            })
            .clone()
    }
    fn cache_collect(&mut self) {
        // TODO free cache if needed.
        self.clean_cache();
    }

    /// Gets a cached image loaded from a `file`.
    pub fn get_file(&mut self, file: impl Into<PathBuf>) -> Image {
        let key = ImageCacheKey::Path(file.into());
        self.get(key)
    }

    /// Gets a cached image downloaded from a `uri`.
    pub fn get_uri(&mut self, uri: impl TryUri) -> Image {
        match uri.try_into() {
            Ok(uri) => self.get(ImageCacheKey::Uri(uri)),
            Err(e) => Image::from_error(Arc::new(e)),
        }
    }

    /// Remove entry from the cache, if it is only held by the cache.
    pub fn clean_entry(&mut self, key: ImageCacheKey) -> Option<Image> {
        self.remove(key, false)
    }

    /// Remove key from the cache, even if it is still referenced outside of the cache.
    ///
    /// Returns `Some(img)` if the image was cached, and you can use [`Image::strong_count`] to determinate
    /// if dropping the image will remove it from memory.
    pub fn purge_entry(&mut self, key: ImageCacheKey) -> Option<Image> {
        self.remove(key, true)
    }

    fn remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<Image> {
        match self.proxy_remove(&key, purge) {
            ProxyRemoveResult::None => self.cache_remove(key, purge),
            ProxyRemoveResult::Remove(k, purge) => self.cache_remove(k, purge),
            ProxyRemoveResult::Removed(r) => r,
        }
    }

    fn cache_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<Image> {
        if purge {
            self.cache.remove(&key)
        } else if let std::collections::hash_map::Entry::Occupied(e) = self.cache.entry(key) {
            if e.get().strong_count() == 1 {
                Some(e.remove_entry().1)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Associate the `image` with the `key` in the cache.
    ///
    /// Returns `Some(previous)` if the `key` was already associated with an image.
    #[inline]
    pub fn register_entry(&mut self, key: ImageCacheKey, image: Image) -> Option<Image> {
        self.cache.insert(key, image)
    }

    /// Clear cached images that are not referenced outside of the cache.
    pub fn clean_cache(&mut self) {
        self.cache.retain(|_, img| img.strong_count() > 1);
        self.proxies.iter_mut().for_each(|p| p.clear(false));
    }

    /// Clear all cached images, including images that are still referenced outside of the cache.
    ///
    /// Image memory only drops when all strong references are removed, so if an image is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_cache(&mut self) {
        self.cache.clear();
        self.proxies.iter_mut().for_each(|p| p.clear(true));
    }

    /// Cache configuration.
    ///
    /// The cache automatically cleans-up old entries when a memory threshold is reached, this
    /// cleanup is configured by the values in [`ImageCacheConfig`].
    #[inline]
    pub fn config(&mut self) -> &mut ImageCacheConfig {
        &mut self.config
    }

    /// Compute total number of loaded image bytes that are referenced by the cache.
    pub fn total_bytes(&mut self) -> u64 {
        self.cache.values().map(|i| i.bytes_len() as u64).sum()
    }

    /// Compute the number of loaded image bytes that are held only by the cache.
    ///
    /// This is roughly the memory that is freed if [`clean_cache`] is called.
    ///
    /// [`clean_cache`]: Self: clean_cache.
    pub fn cache_only_bytes(&mut self) -> u64 {
        self.cache
            .values()
            .filter_map(|i| if i.strong_count() > 1 { Some(i.bytes_len() as u64) } else { None })
            .sum()
    }

    /// Add a cache proxy.
    ///
    /// Proxies can intercept cache requests and map to a different request or return an image directly.
    pub fn install_proxy(&mut self, proxy: Box<dyn ImageCacheProxy>) {
        self.proxies.push(proxy);
    }
    fn proxy_get(&mut self, key: &ImageCacheKey) -> ProxyGetResult {
        for proxy in &mut self.proxies {
            let r = proxy.get(key);
            if !matches!(r, ProxyGetResult::None) {
                return r;
            }
        }
        ProxyGetResult::None
    }
    fn proxy_remove(&mut self, key: &ImageCacheKey, purge: bool) -> ProxyRemoveResult {
        for proxy in &mut self.proxies {
            let r = proxy.remove(key, purge);
            if !matches!(r, ProxyRemoveResult::None) {
                return r;
            }
        }
        ProxyRemoveResult::None
    }
}

/// Configuration of the [`Images`] cache.
///
/// Cache cleanup removes images that are only alive because the cache is holding then.
///
/// The candidate entry attributes are normalized in between all candidates, weighted by this config
/// and sum is the priority of the image. Higher priority is more likely to stay in the cache.
#[derive(Debug, Clone)]
pub struct ImageCacheConfig {
    /// Maximum memory in bytes that can be held exclusively by
    /// the cache before it starts dropping images.
    ///
    /// By default is 100 mebibytes.
    pub memory_threshold: usize,

    /// Priority weight given to the number of requests for the image.
    pub popularity_weight: f32,
    /// Priority weight given to the byte size of the image.
    pub size_weight: f32,
    /// Priority weight given to the time it took to load the image.
    pub time_weight: f32,
}
impl Default for ImageCacheConfig {
    fn default() -> Self {
        ImageCacheConfig {
            memory_threshold: 1024 * 1024 * 100,
            popularity_weight: 1.0,
            size_weight: 0.1,
            time_weight: 2.0,
        }
    }
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
#[derive(Debug)]
pub enum ProxyGetResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Load and cache using the replacement key.
    Cache(ImageCacheKey),
    /// Return the image instead of hitting the cache.
    Image(Image),
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
    state: Rc<RefCell<ImageState>>,
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
        let (sender, state) = ImageState::new_loading();
        task::spawn_wait(move || {
            let start_time = Instant::now();
            let r: Result<_, LoadingError> = match image::open(file) {
                Ok(img) => Ok(Self::decoded_to_state(img, start_time)),
                Err(e) => Err(Arc::new(e)),
            };
            let _ = sender.send(r);
        });
        Image {
            state,
            render_keys: Rc::default(),
        }
    }

    /// Start loading an image from the web using a GET request.
    pub fn from_uri(uri: impl TryUri) -> Self {
        let state = match uri.try_into() {
            Ok(uri) => {
                let (sender, state) = ImageState::new_loading();
                task::spawn(async move {
                    let start_time = Instant::now();
                    let r: Result<_, LoadingError> = match Self::download_image(uri).await {
                        Ok(img) => Ok(Self::decoded_to_state(img, start_time)),
                        Err(e) => Err(Arc::new(e)),
                    };
                    let _ = sender.send(r);
                });
                state
            }
            Err(e) => Rc::new(RefCell::new(ImageState::Error(Arc::new(e)))),
        };
        Image {
            state,
            render_keys: Rc::default(),
        }
    }

    /// Create an image from an image buffer.
    ///
    /// The internal format is BGRA8 with pre-multiplied alpha, all other formats will be converted and if the
    /// format has an alpha component it will be pre-multiplied.
    pub fn from_decoded(image: DynamicImage) -> Self {
        let (sender, state) = ImageState::new_loading();
        task::spawn(async move {
            let loaded = Self::decoded_to_state(image, Instant::now());
            let _ = sender.send(Ok(loaded));
        });
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
        let state = Rc::new(RefCell::new(ImageState::Loaded(LoadedImage {
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
            time: Duration::ZERO,
        })));

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

        let state = Rc::new(RefCell::new(ImageState::Loaded(LoadedImage {
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
            time: Duration::ZERO,
        })));
        Image {
            state,
            render_keys: Rc::default(),
        }
    }

    /// Create an *image* in the the error state with `load_error` representing the error.
    pub fn from_error(load_error: LoadingError) -> Self {
        Image {
            state: Rc::new(RefCell::new(ImageState::Error(load_error))),
            render_keys: Rc::default(),
        }
    }

    /// Returns the loaded image data as a tuple of `(bgra, width, height, is_opaque)`.
    pub fn as_raw(&self) -> Option<(Arc<Vec<u8>>, u32, u32, bool)> {
        if self.state.borrow_mut().load() {
            if let ImageState::Loaded(s) = &*self.state.borrow() {
                let bgra = match &s.data {
                    ImageData::Raw(b) => Arc::clone(b),
                    ImageData::External(_) => unreachable!(),
                };
                Some((
                    bgra,
                    s.descriptor.size.width as u32,
                    s.descriptor.size.height as u32,
                    s.descriptor.is_opaque(),
                ))
            } else {
                None
            }
        } else {
            None
        }
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
            state: Rc::downgrade(&self.state),
            render_keys: Rc::downgrade(&self.render_keys),
        }
    }

    /// If `self` and `other` are both pointers to the same image data.
    #[inline]
    pub fn ptr_eq(&self, other: &Image) -> bool {
        Rc::ptr_eq(&self.render_keys, &other.render_keys)
    }

    /// Number of BGRA bytes held in memory by this image.
    ///
    /// Returns `0` if the image is not loaded or did not load due to an error.
    #[inline]
    pub fn bytes_len(&self) -> usize {
        if !self.is_loaded() {
            return 0;
        }

        if let ImageState::Loaded(s) = &*self.state.borrow() {
            match &s.data {
                ImageData::Raw(b) => b.len(),
                ImageData::External(_) => unreachable!(),
            }
        } else {
            0
        }
    }

    /// Returns `true` is the image has finished loading or encountered an error.
    pub fn is_loaded(&self) -> bool {
        self.state.borrow_mut().load()
    }

    /// Returns `true` if the image has finished loading with an error.
    pub fn is_error(&self) -> bool {
        self.is_loaded() && matches!(&*self.state.borrow(), ImageState::Error(_))
    }

    /// Returns the loading error if any happened.
    pub fn error(&self) -> Option<LoadingError> {
        if self.is_loaded() {
            if let ImageState::Error(e) = &*self.state.borrow() {
                Some(Arc::clone(e))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Gets the `(width, height)` in pixels of the image.
    ///
    /// Returns `(0, 0)` if the image is not loaded.
    pub fn pixel_size(&self) -> (u32, u32) {
        if !self.is_loaded() {
            return (0, 0);
        }
        if let ImageState::Loaded(s) = &*self.state.borrow() {
            (s.descriptor.size.width as u32, s.descriptor.size.height as u32)
        } else {
            (0, 0)
        }
    }

    /// Gets the `(dpiX, dpiY)` pixel scaling metadata of the image.
    pub fn dpi(&self) -> (f32, f32) {
        // TODO
        (96.0, 96.0)
    }

    /// Time from image request to loaded.
    #[inline]
    pub fn load_time(&self) -> Option<Duration> {
        if !self.is_loaded() {
            return None;
        }

        if let ImageState::Loaded(s) = &*self.state.borrow() {
            Some(s.time)
        } else {
            None
        }
    }

    fn decoded_to_state(image: DynamicImage, start_time: Instant) -> LoadedImage {
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

        LoadedImage {
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
            time: Instant::now().duration_since(start_time),
        }
    }

    async fn download_image(uri: task::http::Uri) -> Result<DynamicImage, LoadingError> {
        // map_err did not work here
        let r = match task::http::get_bytes_cached(uri).await {
            Ok(r) => r,
            Err(e) => return Err(Arc::new(e)),
        };
        let img = match image::load_from_memory(&r) {
            Ok(r) => r,
            Err(e) => return Err(Arc::new(e)),
        };
        Ok(img)
    }

    fn render_image(&self, api: &Arc<RenderApi>) -> ImageKey {
        let namespace = api.get_namespace_id();
        let mut keys = self.render_keys.borrow_mut();
        for r in keys.iter_mut() {
            if r.key.0 == namespace {
                if !r.loaded && self.state.borrow_mut().load() {
                    r.loaded = true;
                    if let ImageState::Loaded(s) = &*self.state.borrow() {
                        Self::load_image(api, r.key, s.descriptor, s.data.clone())
                    }
                }
                return r.key;
            }
        }

        let key = api.generate_image_key();

        let loaded = self.state.borrow_mut().load();
        if loaded {
            if let ImageState::Loaded(s) = &*self.state.borrow() {
                Self::load_image(api, key, s.descriptor, s.data.clone())
            }
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
    Loading(flume::Receiver<Result<LoadedImage, LoadingError>>),
    Loaded(LoadedImage),
    Error(LoadingError),
}
impl ImageState {
    fn new_loading() -> (flume::Sender<Result<LoadedImage, LoadingError>>, Rc<RefCell<ImageState>>) {
        let (sender, recv) = flume::bounded(1);
        (sender, Rc::new(RefCell::new(ImageState::Loading(recv))))
    }

    /// Returns `true` if the state is loaded or error.
    fn load(&mut self) -> bool {
        if let ImageState::Loading(recv) = self {
            match recv.try_recv() {
                Ok(r) => {
                    match r {
                        Ok(img) => *self = ImageState::Loaded(img),
                        Err(e) => *self = ImageState::Error(e),
                    }
                    true
                }
                Err(e) => match e {
                    flume::TryRecvError::Empty => false,
                    flume::TryRecvError::Disconnected => {
                        *self = ImageState::Error(Arc::new(flume::TryRecvError::Disconnected));
                        true
                    }
                },
            }
        } else {
            true
        }
    }
}
struct LoadedImage {
    descriptor: ImageDescriptor,
    data: ImageData,
    time: Duration,
}
type LoadingError = Arc<dyn Error + Send + Sync>;

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
/// Use [`Image::downgrade`] to create an weak image reference.
///
/// [`upgrade`]: WeakImage::upgrade
pub struct WeakImage {
    state: std::rc::Weak<RefCell<ImageState>>,
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
