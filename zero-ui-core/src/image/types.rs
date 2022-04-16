use std::{
    cell::RefCell,
    env, fmt, fs, io, mem, ops,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use once_cell::unsync::OnceCell;
use zero_ui_view_api::{webrender_api::ImageKey, ViewProcessOffline};

use crate::{
    app::view_process::{EncodeError, ViewImage, ViewRenderer},
    context::{LayoutMetrics, WindowContext},
    impl_from_and_into_var,
    task::{self, SignalOnce},
    text::Text,
    units::*,
    var::ReadOnlyRcVar,
    BoxedUiNode, UiNode,
};

pub use crate::app::view_process::{ImageDataFormat, ImagePpi};

use super::RenderConfig;

/// A custom proxy in [`Images`].
///
/// Implementers can intercept cache requests and redirect to another cache request or returns an image directly.
///
/// [`Images`]: super::Images
pub trait ImageCacheProxy {
    /// Intercept a get request.
    fn get(&mut self, key: &ImageHash, source: &ImageSource, mode: ImageCacheMode) -> ProxyGetResult {
        let _ = (key, source, mode);
        ProxyGetResult::None
    }

    /// Intercept a remove request.
    fn remove(&mut self, key: &ImageHash, purge: bool) -> ProxyRemoveResult {
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
    /// Load and cache using the replacement source.
    Cache(ImageSource, ImageCacheMode),
    /// Return the image instead of hitting the cache.
    Image(ImageVar),
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
    Remove(ImageHash, bool),
    /// Consider the request fulfilled.
    Removed,
}

/// Represents an [`Image`] tracked by the [`Images`] cache.
///
/// The variable updates when the image updates.
///
/// [`Images`]: super::Images
pub type ImageVar = ReadOnlyRcVar<Image>;

/// State of an [`ImageVar`].
///
/// Each instance of this struct represent a single state,
#[derive(Debug, Clone)]
pub struct Image {
    pub(super) view: OnceCell<ViewImage>,
    render_keys: Rc<RefCell<Vec<RenderImage>>>,
    pub(super) done_signal: SignalOnce,
    pub(super) cache_key: Option<ImageHash>,
}
impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.view == other.view
    }
}
impl Image {
    pub(super) fn new_none(cache_key: Option<ImageHash>) -> Self {
        Image {
            view: OnceCell::new(),
            render_keys: Rc::default(),
            done_signal: SignalOnce::new(),
            cache_key,
        }
    }

    /// New from existing `ViewImage`.
    pub fn new(view: ViewImage) -> Self {
        let sig = view.done_signal();
        let v = OnceCell::new();
        let _ = v.set(view);
        Image {
            view: v,
            render_keys: Rc::default(),
            done_signal: sig,
            cache_key: None,
        }
    }

    /// Create a dummy image in the loaded or error state.
    pub fn dummy(error: Option<String>) -> Self {
        Self::new(ViewImage::dummy(error))
    }

    /// Returns `true` if the is still acquiring or decoding the image bytes.
    pub fn is_loading(&self) -> bool {
        match self.view.get() {
            Some(v) => !v.is_loaded() && !v.is_error(),
            None => true,
        }
    }

    /// If the image is successfully loaded in the view-process.
    pub fn is_loaded(&self) -> bool {
        match self.view.get() {
            Some(v) => v.is_loaded(),
            None => false,
        }
    }

    /// If the image failed to load.
    pub fn is_error(&self) -> bool {
        match self.view.get() {
            Some(v) => v.is_error(),
            None => false,
        }
    }

    /// Returns an error message if the image failed to load.
    pub fn error(&self) -> Option<&str> {
        match self.view.get() {
            Some(v) => v.error(),
            None => None,
        }
    }

    /// Returns a future that awaits until this image is loaded or encountered an error.
    pub fn wait_done(&self) -> impl std::future::Future<Output = ()> + Send + Sync + 'static {
        self.done_signal.clone()
    }

    /// Returns the image size in pixels, or zero if it is not loaded.
    pub fn size(&self) -> PxSize {
        self.view.get().map(|v| v.size()).unwrap_or_else(PxSize::zero)
    }

    /// Returns the image pixel-per-inch metadata if the image is loaded and the
    /// metadata was retrieved.
    pub fn ppi(&self) -> ImagePpi {
        self.view.get().and_then(|v| v.ppi())
    }

    /// Returns `true` if the image is fully opaque or it is not loaded.
    pub fn is_opaque(&self) -> bool {
        self.view.get().map(|v| v.is_opaque()).unwrap_or(true)
    }

    /// Connection to the image resource, if it is loaded.
    pub fn view(&self) -> Option<&ViewImage> {
        match self.view.get() {
            Some(v) => {
                if v.is_loaded() {
                    Some(v)
                } else {
                    None
                }
            }
            None => None,
        }
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

        let s_ppi = ctx.screen_ppi;
        let mut size = self.size();

        size.width *= (s_ppi / dpi_x) * ctx.scale_factor.0;
        size.height *= (s_ppi / dpi_y) * ctx.scale_factor.0;

        size
    }

    /// Reference the decoded pre-multiplied BGRA8 pixel buffer.
    #[inline]
    pub fn bgra8(&self) -> Option<&[u8]> {
        self.view.get().and_then(|v| v.bgra8())
    }

    /// Copy the `rect` selection from `bgra8`.
    ///
    /// The `rect` is in pixels, with the origin (0, 0) at the top-left of the image.
    ///
    /// Returns the copied selection and the BGRA8 pre-multiplied pixel buffer.
    ///
    /// Note that the selection can change if `rect` is not fully contained by the image area.
    pub fn copy_pixels(&self, rect: PxRect) -> Option<(PxRect, Vec<u8>)> {
        self.bgra8().map(|bgra8| {
            let area = PxRect::from_size(self.size()).intersection(&rect).unwrap_or_default();
            if area.size.width.0 == 0 || area.size.height.0 == 0 {
                (area, vec![])
            } else {
                let x = area.origin.x.0 as usize;
                let y = area.origin.y.0 as usize;
                let width = area.size.width.0 as usize;
                let height = area.size.height.0 as usize;
                let mut bytes = Vec::with_capacity(width * height * 4);
                for l in y..y + height {
                    let line_start = (l + x) * 4;
                    let line_end = (l + x + width) * 4;
                    let line = &bgra8[line_start..line_end];
                    bytes.extend(line);
                }
                (area, bytes)
            }
        })
    }

    /// Encode the image to the format.
    pub async fn encode(&self, format: String) -> std::result::Result<Arc<Vec<u8>>, EncodeError> {
        self.done_signal.clone().await;
        if let Some(e) = self.error() {
            Err(EncodeError::Encode(e.to_owned()))
        } else {
            self.view.get().unwrap().encode(format).await
        }
    }

    /// Encode and write the image to `path`.
    ///
    /// The image format is guessed from the file extension.
    pub async fn save(&self, path: impl Into<PathBuf>) -> io::Result<()> {
        let path = path.into();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            self.save_impl(ext.to_owned(), path).await
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "could not determinate image format from path extension",
            ))
        }
    }

    /// Encode and write the image to `path`.
    ///
    /// The image is encoded to the `format`, the file extension can be anything.
    pub async fn save_with_format(&self, format: String, path: impl Into<PathBuf>) -> io::Result<()> {
        self.save_impl(format, path.into()).await
    }

    async fn save_impl(&self, format: String, path: PathBuf) -> io::Result<()> {
        let view = self.view.get().unwrap();
        let data = view
            .encode(format)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        task::wait(move || fs::write(path, &data[..])).await
    }
}
impl crate::render::Image for Image {
    fn image_key(&self, renderer: &ViewRenderer) -> ImageKey {
        if self.is_loaded() {
            use crate::render::webrender_api::*;

            let namespace = match renderer.namespace_id() {
                Ok(n) => n,
                Err(ViewProcessOffline) => {
                    tracing::debug!("respawned calling `namespace_id`, will return DUMMY");
                    return ImageKey::DUMMY;
                }
            };
            let mut rms = self.render_keys.borrow_mut();
            if let Some(rm) = rms.iter().find(|k| k.key.0 == namespace) {
                return rm.key;
            }

            let key = match renderer.use_image(self.view.get().unwrap()) {
                Ok(k) => {
                    if k == ImageKey::DUMMY {
                        tracing::error!("received DUMMY from `use_image`");
                        return k;
                    }
                    k
                }
                Err(ViewProcessOffline) => {
                    tracing::debug!("respawned `add_image`, will return DUMMY");
                    return ImageKey::DUMMY;
                }
            };

            rms.push(RenderImage {
                key,
                renderer: renderer.clone(),
            });
            key
        } else {
            ImageKey::DUMMY
        }
    }
}

struct RenderImage {
    key: ImageKey,
    renderer: ViewRenderer,
}
impl Drop for RenderImage {
    fn drop(&mut self) {
        // error here means the entire renderer was dropped.
        let _ = self.renderer.delete_image_use(self.key);
    }
}
impl fmt::Debug for RenderImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.key, f)
    }
}

/// A 256-bit hash for image entries.
///
/// This hash is used to identify image files in the [`Images`] cache.
///
/// Use [`ImageHasher`] to compute.
///
/// [`Images`]: super::Images
#[derive(Clone, Copy)]
pub struct ImageHash([u8; 32]);
impl ImageHash {
    /// Compute the hash for `data`.
    pub fn compute(data: &[u8]) -> Self {
        let mut h = Self::hasher();
        h.update(data);
        h.finish()
    }

    /// Start a new [`ImageHasher`].
    pub fn hasher() -> ImageHasher {
        ImageHasher::default()
    }
}
impl fmt::Debug for ImageHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("ImageHash").field(&self.0).finish()
        } else {
            write!(f, "{}", base64::encode(&self.0))
        }
    }
}
impl fmt::Display for ImageHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::hash::Hash for ImageHash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let h64 = [
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7],
        ];
        state.write_u64(u64::from_ne_bytes(h64))
    }
}
impl PartialEq for ImageHash {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for ImageHash {}

/// Hasher that computes a [`ImageHash`].
pub struct ImageHasher(sha2::Sha512_256);
impl Default for ImageHasher {
    fn default() -> Self {
        use sha2::Digest;
        Self(sha2::Sha512_256::new())
    }
}
impl ImageHasher {
    /// New default hasher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process data, updating the internal state.
    pub fn update(&mut self, data: impl AsRef<[u8]>) {
        use sha2::Digest;
        self.0.update(data);
    }

    /// Finish computing the hash.
    pub fn finish(self) -> ImageHash {
        use sha2::Digest;
        ImageHash(self.0.finalize().as_slice().try_into().unwrap())
    }
}
impl std::hash::Hasher for ImageHasher {
    fn finish(&self) -> u64 {
        tracing::warn!("Hasher::finish called for ImageHasher");

        use sha2::Digest;
        let hash = self.0.clone().finalize();
        u64::from_le_bytes(hash[..8].try_into().unwrap())
    }

    fn write(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }
}

// We don't use Rc<dyn ..> because of this issue: https://github.com/rust-lang/rust/issues/69757
type RenderFn = Rc<Box<dyn Fn(&mut WindowContext) -> BoxedUiNode>>;

/// The different sources of an image resource.
#[derive(Clone)]
pub enum ImageSource {
    /// A path to an image file in the file system.
    ///
    /// Image equality is defined by the path, a copy of the image in another path is a different image.
    Read(PathBuf),
    /// A uri to an image resource downloaded using HTTP GET with an optional HTTP ACCEPT string.
    ///
    /// If the ACCEPT line is not given, all image formats supported by the view-process backend are accepted.
    ///
    /// Image equality is defined by the URI and ACCEPT string.
    #[cfg(http)]
    #[cfg_attr(doc_nightly, doc(cfg(http)))]
    Download(crate::task::http::Uri, Option<Text>),
    /// Static bytes for an encoded or decoded image.
    ///
    /// Image equality is defined by the hash, it is usually the hash of the bytes but it does not need to be.
    Static(ImageHash, &'static [u8], ImageDataFormat),
    /// Shared reference to bytes for an encoded or decoded image.
    ///
    /// Image equality is defined by the hash, it is usually the hash of the bytes but it does not need to be.
    ///
    /// Inside [`Images`] the reference to the bytes is held only until the image finishes decoding.
    ///
    /// [`Images`]: super::Images
    Data(ImageHash, Arc<Vec<u8>>, ImageDataFormat),

    /// A boxed closure that instantiates a boxed [`UiNode`] that draws the image.
    ///
    /// Use the [`render`](Self::render) function to construct this variant.
    Render(RenderFn, RenderConfig),

    /// Already loaded image.
    ///
    /// The image is passed-through, not cached.
    Image(ImageVar),
}
impl ImageSource {
    /// New image from a function that generates a new [`UiNode`].
    ///
    /// The function is called every time the image source is resolved and it is not found in the cache.
    ///
    /// See [`Images::render`] for more information.
    ///
    /// [`Images::render`]: crate::image::Images::render
    pub fn render<I: UiNode, F: Fn(&mut WindowContext) -> I + 'static>(new_img: F) -> Self {
        Self::Render(Rc::new(Box::new(move |ctx| new_img(ctx).boxed())), RenderConfig::default())
    }

    /// Render with custom [`RenderConfig`].
    pub fn render_cfg<I: UiNode, F: Fn(&mut WindowContext) -> I + 'static>(new_img: F, config: impl Into<RenderConfig>) -> Self {
        Self::Render(Rc::new(Box::new(move |ctx| new_img(ctx).boxed())), config.into())
    }

    /// Returns the image hash, unless the source is [`Image`].
    ///
    /// [`Image`]: Self::Image
    pub fn hash128(&self) -> Option<ImageHash> {
        match self {
            ImageSource::Read(p) => Some(Self::hash128_read(p)),
            #[cfg(http)]
            ImageSource::Download(u, a) => Some(Self::hash128_download(u, a)),
            ImageSource::Static(h, _, _) => Some(*h),
            ImageSource::Data(h, _, _) => Some(*h),
            ImageSource::Render(rfn, cfg) => Some(Self::hash128_render(rfn, cfg)),
            ImageSource::Image(_) => None,
        }
    }

    /// Compute hash for a borrowed [`Read`] path.
    ///
    /// [`Read`]: Self::Read
    pub fn hash128_read(path: &Path) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        0u8.hash(&mut h);
        path.hash(&mut h);
        h.finish()
    }

    /// Compute hash for a borrowed [`Download`] URI and HTTP-ACCEPT.
    ///
    /// [`Download`]: Self::Download
    #[cfg(http)]
    #[cfg_attr(doc_nightly, doc(cfg(http)))]
    pub fn hash128_download(uri: &crate::task::http::Uri, accept: &Option<Text>) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        1u8.hash(&mut h);
        uri.hash(&mut h);
        accept.hash(&mut h);
        h.finish()
    }

    /// Compute hash for a borrowed [`Render`] source.
    ///
    /// Pointer equality is used to identify the node closure.
    ///
    /// [`Render`]: Self::Render
    pub fn hash128_render(rfn: &RenderFn, cfg: &RenderConfig) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        2u8.hash(&mut h);
        (Rc::as_ptr(rfn) as usize).hash(&mut h);
        cfg.hash(&mut h);
        h.finish()
    }
}
impl PartialEq for ImageSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Read(l), Self::Read(r)) => l == r,
            #[cfg(http)]
            (Self::Download(lu, la), Self::Download(ru, ra)) => lu == ru && la == ra,
            (Self::Render(lf, lc), Self::Render(rf, rc)) => Rc::ptr_eq(lf, rf) && lc == rc,
            (Self::Image(l), Self::Image(r)) => l.ptr_eq(r),
            (l, r) => {
                let l_hash = match l {
                    ImageSource::Static(h, _, _) => h,
                    ImageSource::Data(h, _, _) => h,
                    _ => return false,
                };
                let r_hash = match r {
                    ImageSource::Static(h, _, _) => h,
                    ImageSource::Data(h, _, _) => h,
                    _ => return false,
                };

                l_hash == r_hash
            }
        }
    }
}
impl fmt::Debug for ImageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ImageSource::")?;
        }
        match self {
            ImageSource::Read(p) => f.debug_tuple("Read").field(p).finish(),
            #[cfg(http)]
            ImageSource::Download(u, a) => f.debug_tuple("Download").field(u).field(a).finish(),
            ImageSource::Static(key, _, fmt) => f.debug_tuple("Static").field(key).field(fmt).finish(),
            ImageSource::Data(key, _, fmt) => f.debug_tuple("Data").field(key).field(fmt).finish(),
            ImageSource::Render(_, cfg) => write!(f, "Render(_, {cfg:?})"),
            ImageSource::Image(_) => write!(f, "Image(_)"),
        }
    }
}

#[cfg(http)]
impl_from_and_into_var! {
    #[cfg_attr(doc_nightly, doc(cfg(http)))]
    fn from(uri: crate::task::http::Uri) -> ImageSource {
        ImageSource::Download(uri, None)
    }
    /// From (URI, HTTP-ACCEPT).
    #[cfg_attr(doc_nightly, doc(cfg(http)))]
    fn from((uri, accept): (crate::task::http::Uri, &'static str)) -> ImageSource {
        ImageSource::Download(uri, Some(accept.into()))
    }

    /// Converts `http://` and `https://` to [`Download`], `file://` to
    /// [`Read`] the path component, and the rest to [`Read`] the string as a path.
    ///
    /// [`Download`]: ImageSource::Download
    /// [`Read`]: ImageSource::Read
    fn from(s: &str) -> ImageSource {
        use crate::task::http::uri::*;
        if let Ok(uri) = Uri::try_from(s) {
            if let Some(scheme) = uri.scheme() {
                if scheme == &Scheme::HTTPS || scheme == &Scheme::HTTP {
                    return ImageSource::Download(uri, None);
                } else if scheme.as_str() == "file" {
                    return PathBuf::from(uri.path()).into();
                }
            }
        }
        PathBuf::from(s).into()
    }
}

#[cfg(not(http))]
impl_from_and_into_var! {
    /// Converts to [`Read`].
    ///
    /// [`Read`]: ImageSource::Read
    fn from(s: &str) -> ImageSource {
        PathBuf::from(s).into()
    }
}

impl_from_and_into_var! {
    fn from(image: ImageVar) -> ImageSource {
        ImageSource::Image(image)
    }
    fn from(path: PathBuf) -> ImageSource {
        ImageSource::Read(path)
    }
    fn from(path: &Path) -> ImageSource {
        path.to_owned().into()
    }

    /// Same as conversion from `&str`.
    fn from(s: String) -> ImageSource {
       s.as_str().into()
    }
    /// Same as conversion from `&str`.
    fn from(s: Text) -> ImageSource {
        s.as_str().into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: &'static [u8]) -> ImageSource {
        ImageSource::Static(ImageHash::compute(data), data, ImageDataFormat::Unknown)
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from<const N: usize>(data: &'static [u8; N]) -> ImageSource {
        (&data[..]).into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: Arc<Vec<u8>>) -> ImageSource {
        ImageSource::Data(ImageHash::compute(&data[..]), data, ImageDataFormat::Unknown)
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: Vec<u8>) -> ImageSource {
        Arc::new(data).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (&'static [u8], F)) -> ImageSource {
        ImageSource::Static(ImageHash::compute(data), data, format.into())
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone, const N: usize>((data, format): (&'static [u8; N], F)) -> ImageSource {
        (&data[..], format).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (Vec<u8>, F)) -> ImageSource {
        (Arc::new(data), format).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (Arc<Vec<u8>>, F)) -> ImageSource {
        ImageSource::Data(ImageHash::compute(&data[..]), data, format.into())
    }
}

/// Cache mode of [`Images`].
///
/// [`Images`]: super::Images
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageCacheMode {
    /// Don't hit the cache, just loads the image.
    Ignore,
    /// Gets a cached image or loads the image and caches it.
    Cache,
    /// Cache or reload if the cached image is an error.
    Retry,
    /// Reloads the cache image or loads the image and caches it.
    ///
    /// The [`ImageVar`] is not replaced, other references to the image also receive the update.
    Reload,
}
impl fmt::Debug for ImageCacheMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "CacheMode::")?;
        }
        match self {
            Self::Ignore => write!(f, "Ignore"),
            Self::Cache => write!(f, "Cache"),
            Self::Retry => write!(f, "Retry"),
            Self::Reload => write!(f, "Reload"),
        }
    }
}

/// Represents a [`PathFilter`] and [`UriFilter`].
#[derive(Clone)]
pub enum ImageSourceFilter<U> {
    /// Block all requests of this type.
    BlockAll,
    /// Allow all requests of this type.
    AllowAll,
    /// Custom filter, returns `true` to allow a request, `false` to block.
    Custom(Rc<dyn Fn(&U) -> bool>),
}
impl<U> ImageSourceFilter<U> {
    /// New [`Custom`] filter.
    ///
    /// [`Custom`]: Self::Custom
    pub fn custom(allow: impl Fn(&U) -> bool + 'static) -> Self {
        Self::Custom(Rc::new(allow))
    }

    /// Combine `self` with `other`, if they both are [`Custom`], otherwise is [`BlockAll`] if any is [`BlockAll`], else
    /// is [`AllowAll`] if any is [`AllowAll`].
    ///
    /// If both are [`Custom`] both filters must allow a request to pass the new filter.
    ///
    /// [`Custom`]: Self::Custom
    /// [`BlockAll`]: Self::BlockAll
    /// [`AllowAll`]: Self::AllowAll
    pub fn and(self, other: Self) -> Self
    where
        U: 'static,
    {
        use ImageSourceFilter::*;
        match (self, other) {
            (BlockAll, _) | (_, BlockAll) => BlockAll,
            (AllowAll, _) | (_, AllowAll) => AllowAll,
            (Custom(c0), Custom(c1)) => Custom(Rc::new(move |u| c0(u) && c1(u))),
        }
    }

    /// Combine `self` with `other`, if they both are [`Custom`], otherwise is [`AllowAll`] if any is [`AllowAll`], else
    /// is [`BlockAll`] if any is [`BlockAll`].
    ///
    /// If both are [`Custom`] at least one of the filters must allow a request to pass the new filter.
    ///
    /// [`Custom`]: Self::Custom
    /// [`BlockAll`]: Self::BlockAll
    /// [`AllowAll`]: Self::AllowAll
    pub fn or(self, other: Self) -> Self
    where
        U: 'static,
    {
        use ImageSourceFilter::*;
        match (self, other) {
            (AllowAll, _) | (_, AllowAll) => AllowAll,
            (BlockAll, _) | (_, BlockAll) => BlockAll,
            (Custom(c0), Custom(c1)) => Custom(Rc::new(move |u| c0(u) || c1(u))),
        }
    }

    /// Returns `true` if the filter allows the request.
    pub fn allows(&self, item: &U) -> bool {
        match self {
            ImageSourceFilter::BlockAll => false,
            ImageSourceFilter::AllowAll => true,
            ImageSourceFilter::Custom(f) => f(item),
        }
    }
}
impl<U> fmt::Debug for ImageSourceFilter<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlockAll => write!(f, "BlockAll"),
            Self::AllowAll => write!(f, "AllowAll"),
            Self::Custom(_) => write!(f, "Custom(_)"),
        }
    }
}
impl<U: 'static> ops::BitAnd for ImageSourceFilter<U> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}
impl<U: 'static> ops::BitOr for ImageSourceFilter<U> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}
impl<U: 'static> ops::BitAndAssign for ImageSourceFilter<U> {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = mem::replace(self, Self::BlockAll).and(rhs);
    }
}
impl<U: 'static> ops::BitOrAssign for ImageSourceFilter<U> {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = mem::replace(self, Self::BlockAll).or(rhs);
    }
}

/// Represents a [`ImageSource::Read`] path request filter.
///
/// Only absolute, normalized paths are shared with the [`Custom`] filter, there is no relative paths or `..` components.
///
/// The paths are **not** canonicalized or checked if exists as a file, no system requests are made with unfiltered paths.
///
/// See [`ImageLimits::allow_path`] for more information.
///
/// [`Custom`]: ImageSourceFilter::Custom
pub type PathFilter = ImageSourceFilter<PathBuf>;
impl PathFilter {
    /// Allow any file inside `dir` or sub-directories of `dir`.
    pub fn allow_dir(dir: impl AsRef<Path>) -> Self {
        let dir = crate::crate_util::absolute_path(dir.as_ref(), || env::current_dir().expect("could not access current dir"), true);
        PathFilter::custom(move |r| r.starts_with(&dir))
    }

    /// Allow any path with the `ext` extension.
    pub fn allow_ext(ext: impl Into<std::ffi::OsString>) -> Self {
        let ext = ext.into();
        PathFilter::custom(move |r| r.extension().map(|e| e == ext).unwrap_or(false))
    }

    /// Allow any file inside the [`env::current_dir`] or sub-directories.
    ///
    /// Note that the current directory can be changed and the filter always uses the
    /// *fresh* current directory, use [`allow_dir`] to create a filter the always points
    /// to the current directory at the filter creation time.
    ///
    /// [`allow_dir`]: Self::allow_dir
    pub fn allow_current_dir() -> Self {
        PathFilter::custom(|r| env::current_dir().map(|d| r.starts_with(&d)).unwrap_or(false))
    }

    /// Allow any file inside the current executable directory or sub-directories.
    pub fn allow_exe_dir() -> Self {
        if let Ok(mut p) = env::current_exe() {
            if p.pop() {
                return Self::allow_dir(p);
            }
        }

        // not `BlockAll` so this can still be composed using `or`.
        Self::custom(|_| false)
    }
}

/// Represents a [`ImageSource::Download`] path request filter.
///
/// See [`ImageLimits::allow_uri`] for more information.
#[cfg(http)]
#[cfg_attr(doc_nightly, doc(cfg(http)))]
pub type UriFilter = ImageSourceFilter<crate::task::http::Uri>;
#[cfg(http)]
impl UriFilter {
    /// Allow any file from the `host` site.
    pub fn allow_host(host: impl Into<Text>) -> Self {
        let host = host.into();
        UriFilter::custom(move |u| u.authority().map(|a| a.host() == host).unwrap_or(false))
    }
}

impl<F: Fn(&PathBuf) -> bool + 'static> From<F> for PathFilter {
    fn from(custom: F) -> Self {
        PathFilter::custom(custom)
    }
}

#[cfg(http)]
impl<F: Fn(&task::http::Uri) -> bool + 'static> From<F> for UriFilter {
    fn from(custom: F) -> Self {
        UriFilter::custom(custom)
    }
}

/// Limits for image loading and decoding.
#[derive(Clone, Debug)]
pub struct ImageLimits {
    /// Maximum encoded file size allowed.
    ///
    /// An error is returned if the file size surpasses this value. If the size can read before
    /// read/download it is, otherwise the error happens when this limit is reached and all already
    /// loaded bytes are dropped.
    ///
    /// The default is `100mb`.
    pub max_encoded_size: ByteLength,
    /// Maximum decoded file size allowed.
    ///
    /// An error is returned if the decoded image memory would surpass the `width * height * 4`
    pub max_decoded_size: ByteLength,

    /// Filter for [`ImageSource::Read`] paths.
    ///
    /// Only paths allowed by this filter are loaded
    pub allow_path: PathFilter,

    /// Filter for [`ImageSource::Download`] URIs.
    #[cfg(http)]
    #[cfg_attr(doc_nightly, doc(cfg(http)))]
    pub allow_uri: UriFilter,
}
impl ImageLimits {
    /// No size limits, allow all paths and URIs.
    pub fn none() -> Self {
        ImageLimits {
            max_encoded_size: ByteLength::MAX,
            max_decoded_size: ByteLength::MAX,
            allow_path: PathFilter::AllowAll,
            #[cfg(http)]
            allow_uri: UriFilter::AllowAll,
        }
    }

    /// Set the [`max_encoded_size`].
    ///
    /// [`max_encoded_size`]: Self::max_encoded_size
    pub fn with_max_encoded_size(mut self, max_encoded_size: impl Into<ByteLength>) -> Self {
        self.max_encoded_size = max_encoded_size.into();
        self
    }

    /// Set the [`max_decoded_size`].
    ///
    /// [`max_decoded_size`]: Self::max_encoded_size
    pub fn with_max_decoded_size(mut self, max_decoded_size: impl Into<ByteLength>) -> Self {
        self.max_decoded_size = max_decoded_size.into();
        self
    }

    /// Set the [`allow_path`].
    ///
    /// [`allow_path`]: Self::allow_path
    pub fn with_allow_path(mut self, allow_path: impl Into<PathFilter>) -> Self {
        self.allow_path = allow_path.into();
        self
    }

    /// Set the [`allow_uri`].
    ///
    /// [`allow_uri`]: Self::allow_uri
    #[cfg(http)]
    #[cfg_attr(doc_nightly, doc(cfg(http)))]
    pub fn with_allow_uri(mut self, allow_url: impl Into<UriFilter>) -> Self {
        self.allow_uri = allow_url.into();
        self
    }
}
impl Default for ImageLimits {
    /// 100 megabytes encoded and 4096 megabytes decoded (BMP max).
    ///
    /// Allows all paths, blocks all URIs.
    fn default() -> Self {
        Self {
            max_encoded_size: 100.megabytes(),
            max_decoded_size: 4096.megabytes(),
            allow_path: PathFilter::AllowAll,
            #[cfg(http)]
            allow_uri: UriFilter::BlockAll,
        }
    }
}
