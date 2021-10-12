//! Image loading and cache.

use std::{
    cell::RefCell,
    collections::HashMap,
    convert::TryFrom,
    fmt,
    future::Future,
    mem,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use once_cell::unsync::OnceCell;
use zero_ui_view_api::{webrender_api::ImageKey, IpcSharedMemory};

use crate::{
    app::{
        raw_events::{RawImageLoadErrorEvent, RawImageLoadedEvent, RawImageMetadataLoadedEvent},
        view_process::{EncodeError, Respawned, ViewImage, ViewProcess, ViewProcessRespawnedEvent, ViewRenderer},
        AppEventSender, AppExtension,
    },
    context::{AppContext, LayoutMetrics},
    event::EventUpdateArgs,
    impl_from_and_into_var,
    service::Service,
    task::{
        self, fs,
        http::{self, header, Request, TryUri, Uri},
        io::*,
        ui::UiTask,
        SignalOnce,
    },
    text::Text,
    units::*,
    var::{var, RcVar, ReadOnlyRcVar, Var},
};

pub use crate::app::view_process::{ImageDataFormat, ImagePpi};

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
        let images = Images::new(ctx.services.get::<ViewProcess>().cloned(), ctx.updates.sender());
        ctx.services.register(images);
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = RawImageMetadataLoadedEvent.update(args) {
            let images = ctx.services.images();
            let vars = ctx.vars;
            if let Some(var) = images.decoding.iter().find(|v| v.get(vars).view.get().unwrap() == &args.image) {
                var.touch(ctx.vars);
            }
        } else if let Some(image) = RawImageLoadedEvent
            .update(args)
            .map(|a| &a.image)
            .or_else(|| RawImageLoadErrorEvent.update(args).map(|a| &a.image))
        {
            // image is ready for use or failed to decode, remove from `decoding`
            // and notify the ViewImage inner state update.
            let images = ctx.services.images();
            let vars = ctx.vars;

            if let Some(i) = images.decoding.iter().position(|v| v.get(vars).view.get().unwrap() == image) {
                let var = images.decoding.swap_remove(i);
                var.touch(ctx.vars);
                var.get(ctx.vars).done_signal.set();
            }
        } else if ViewProcessRespawnedEvent.update(args).is_some() {
            let (vp, images) = ctx.services.req_multi::<(ViewProcess, Images)>();
            images.view = Some(vp.clone());
            images.download_accept.clear();
            images.decoding.clear();
            for v in images.cache.values() {
                let img = v.get(ctx.vars);

                if let Some(view) = img.view.get() {
                    if let Some(e) = view.error() {
                        // respawned, but image was an error.
                        v.set(ctx.vars, Image::dummy(Some(e.to_owned())));
                    } else {
                        // respawned and image was loaded.
                        let img = vp
                            .add_image(
                                ImageDataFormat::Bgra8 {
                                    size: view.size(),
                                    ppi: view.ppi(),
                                },
                                view.shared_bgra8().unwrap(),
                                images.max_decoded_size.0 as u64,
                            )
                            .expect("TODO");

                        v.set(ctx.vars, Image::new(img));

                        images.decoding.push(v.clone());
                    }
                } else {
                    // respawned while loading image.
                    let _ = img.view.set(ViewImage::dummy(Some("reloading".to_owned())));
                    img.done_signal.set();

                    todo!("")
                }
            }
        }
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        // update loading tasks:

        let images = ctx.services.images();
        let view = &images.view;
        let vars = ctx.vars;
        let decoding = &mut images.decoding;
        let mut loading = Vec::with_capacity(images.loading.len());

        for (mut task, var) in mem::take(&mut images.loading) {
            task.update();
            match task.into_result() {
                Ok(d) => {
                    match d.r {
                        Ok(data) => {
                            if let Some(vp) = view {
                                // success and we have a view-process.
                                match vp.add_image(d.format, data, images.max_decoded_size.0 as u64) {
                                    Ok(img) => {
                                        // request send, add to `decoding` will receive
                                        // `RawImageLoadedEvent` or `RawImageLoadErrorEvent` event
                                        // when done.
                                        var.modify(vars, move |v| {
                                            v.view.set(img).unwrap();
                                            v.touch();
                                        });
                                        decoding.push(var);
                                        break;
                                    }
                                    Err(Respawned) => {
                                        let img = ViewImage::dummy(Some("view-process respawned during image load".to_owned()));
                                        var.modify(vars, move |v| {
                                            v.view.set(img).unwrap();
                                            v.touch();
                                            v.done_signal.set();
                                        });
                                    }
                                }
                            } else {
                                // success, but we are only doing `load_in_headless` validation.
                                let img = ViewImage::dummy(None);
                                var.modify(vars, move |v| {
                                    v.view.set(img).unwrap();
                                    v.touch();
                                    v.done_signal.set();
                                });
                            }
                        }
                        Err(e) => {
                            // load error.
                            let img = ViewImage::dummy(Some(e));
                            var.modify(vars, move |v| {
                                v.view.set(img).unwrap();
                                v.touch();
                                v.done_signal.set();
                            });
                        }
                    }
                }
                Err(task) => {
                    loading.push((task, var));
                }
            }
        }
        images.loading = loading;
    }
}

/// The [`Image`] loading cache service.
///
/// If the app is running without a [`ViewProcess`] all images are dummy, see [`load_in_headless`] for
/// details.
///
/// [`load_in_headless`]: Images::load_in_headless
#[derive(Service)]
pub struct Images {
    /// If should still download/read image bytes in headless/renderless mode.
    ///
    /// When an app is in headless mode without renderer no [`ViewProcess`] is available, so
    /// images cannot be decoded, in this case all images are the [`dummy`] image and no attempt
    /// to download/read the image files is made. You can enable loading in headless tests to detect
    /// IO errors, in this case if there is an error acquiring the image file the image will be a
    /// [`dummy`] with error.
    ///
    /// [`dummy`]: Images::dummy
    pub load_in_headless: bool,

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

    view: Option<ViewProcess>,
    download_accept: String,
    updates: AppEventSender,
    proxies: Vec<Box<dyn ImageCacheProxy>>,

    loading: Vec<(UiTask<ImageData>, RcVar<Image>)>,
    decoding: Vec<RcVar<Image>>,
    cache: HashMap<ImageCacheKey, RcVar<Image>>,
}
impl Images {
    fn new(view: Option<ViewProcess>, updates: AppEventSender) -> Self {
        Images {
            load_in_headless: false,
            max_encoded_size: 100.mebi_bytes(), // TODO
            max_decoded_size: 100.mebi_bytes(),
            view,
            updates,
            proxies: vec![],
            loading: vec![],
            decoding: vec![],
            download_accept: String::new(),
            cache: HashMap::default(),
        }
    }

    /// Returns a dummy image that reports it is loaded with optional error.
    pub fn dummy(&self, error: Option<String>) -> ImageVar {
        var(Image::dummy(error)).into_read_only()
    }

    /// Get or load an image file from a file system `path`.
    pub fn read(&mut self, path: impl Into<PathBuf>) -> ImageVar {
        self.get(ImageCacheKey::Read(path.into()))
    }

    /// Get a cached `uri` or download it.
    pub fn download(&mut self, uri: impl TryUri) -> ImageVar {
        match uri.try_into() {
            Ok(uri) => self.get(ImageCacheKey::Download(uri)),
            Err(e) => self.dummy(Some(e.to_string())),
        }
    }

    /// Get a cached image from `&'static [u8]` data.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    ///
    /// # Examples
    ///
    /// Get an image from a PNG file embedded in the app executable using [`include_bytes!`].
    ///
    /// ```
    /// # use zero_ui_core::{image::*, context::AppContext};
    /// # macro_rules! include_bytes { ($tt:tt) => { &[] } }
    /// # fn demo(ctx: &mut AppContext) {
    /// let image_var = ctx.services.images().from_static(include_bytes!("ico.png"), "png");
    /// # }
    pub fn from_static(&mut self, data: &'static [u8], format: impl Into<ImageDataFormat>) -> ImageVar {
        self.get(ImageCacheKey::Static(data, format.into()))
    }

    /// Get a cached image from shared data.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    pub fn from_data(&mut self, data: Arc<Vec<u8>>, format: impl Into<ImageDataFormat>) -> ImageVar {
        self.get(ImageCacheKey::Data(data, format.into()))
    }

    /// Get a cached image or add it to the cache.
    pub fn get(&mut self, key: ImageCacheKey) -> ImageVar {
        self.proxy_then_get(key)
    }

    /// Associate the `image` with the `key` in the cache.
    ///
    /// Returns `Some(previous)` if the `key` was already associated with an image.
    #[inline]
    pub fn register(&mut self, key: ImageCacheKey, image: ViewImage) -> Option<ImageVar> {
        self.cache.insert(key, var(Image::new(image))).map(|v| v.into_read_only())
    }

    /// Remove the image from the cache, if it is only held by the cache.
    pub fn clean(&mut self, key: ImageCacheKey) -> Option<ImageVar> {
        self.proxy_then_remove(key, false)
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// Returns `Some(img)` if the image was cached.
    pub fn purge(&mut self, key: ImageCacheKey) -> Option<ImageVar> {
        self.proxy_then_remove(key, true)
    }

    fn proxy_then_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<ImageVar> {
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
    fn proxied_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<ImageVar> {
        if purge {
            self.cache.remove(&key).map(|v| v.into_read_only())
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

    fn proxy_then_get(&mut self, key: ImageCacheKey) -> ImageVar {
        for proxy in &mut self.proxies {
            let r = proxy.get(&key);
            match r {
                ProxyGetResult::None => continue,
                ProxyGetResult::Cache(r) => return self.proxied_get(r),
                ProxyGetResult::Image(img) => return img,
            }
        }
        self.proxied_get(key)
    }
    fn proxied_get(&mut self, key: ImageCacheKey) -> ImageVar {
        if let Some(img) = self.cache.get(&key) {
            return img.clone().into_read_only();
        }

        if self.view.is_none() && !self.load_in_headless {
            let dummy = var(Image::new(ViewImage::dummy(None)));
            self.cache.insert(key, dummy.clone());
            return dummy.into_read_only();
        }

        let max_encoded_size = self.max_encoded_size;

        match key.clone() {
            ImageCacheKey::Read(path) => self.load_task(key, async move {
                let mut r = ImageData {
                    format: path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| ImageDataFormat::FileExtension(s.to_owned()))
                        .unwrap_or(ImageDataFormat::Unknown),
                    r: Err(String::new()),
                };

                let mut file = match fs::File::open(path).await {
                    Ok(f) => f,
                    Err(e) => {
                        r.r = Err(e.to_string());
                        return r;
                    }
                };

                let len = match file.metadata().await {
                    Ok(m) => m.len() as usize,
                    Err(e) => {
                        r.r = Err(e.to_string());
                        return r;
                    }
                };

                if len > max_encoded_size.0 {
                    r.r = Err(format!("file size `{}` exceeds the limit of `{}`", len.bytes(), max_encoded_size));
                    return r;
                }

                let mut data = vec![0; len]; // TODO can we trust the metadata length?
                r.r = match file.read_exact(&mut data).await {
                    Ok(_) => Ok(IpcSharedMemory::from_bytes(&data)),
                    Err(e) => Err(e.to_string()),
                };

                r
            }),
            ImageCacheKey::Download(uri) => {
                let accept = self.download_accept();
                self.load_task(key, async move {
                    let mut r = ImageData {
                        format: ImageDataFormat::Unknown,
                        r: Err(String::new()),
                    };

                    // for image crate:
                    // image/webp decoder is only grayscale: https://docs.rs/image/0.23.14/image/codecs/webp/struct.WebPDecoder.html
                    // image/avif decoder does not build in Windows
                    let request = Request::get(uri).unwrap().header(header::ACCEPT, accept).unwrap().build();

                    match http::send(request).await {
                        Ok(mut rsp) => {
                            if let Some(m) = rsp.headers().get(&header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
                                let m = m.to_lowercase();
                                if m.starts_with("image/") {
                                    r.format = ImageDataFormat::MimeType(m);
                                }
                            }

                            if let Some(len) = rsp.content_len() {
                                if len > max_encoded_size {
                                    r.r = Err(format!("file size `{}` exceeds the limit of `{}`", len.bytes(), max_encoded_size));
                                    return r;
                                }
                            }

                            match rsp.bytes_limited(max_encoded_size + 1.bytes()).await {
                                Ok(d) => {
                                    if d.len().bytes() > max_encoded_size {
                                        r.r = Err(format!("download exceeded the limit of `{}`", max_encoded_size));
                                    } else {
                                        r.r = Ok(IpcSharedMemory::from_bytes(&d))
                                    }
                                }
                                Err(e) => {
                                    r.r = Err(format!("download error: {}", e));
                                }
                            }

                            let _ = rsp.consume();
                        }
                        Err(e) => {
                            r.r = Err(format!("request error: {}", e));
                        }
                    }

                    r
                })
            }
            ImageCacheKey::Static(bytes, fmt) => {
                let r = ImageData {
                    format: fmt,
                    r: Ok(IpcSharedMemory::from_bytes(bytes)),
                };
                self.load_task(key, async { r })
            }
            ImageCacheKey::Data(bytes, fmt) => {
                let r = ImageData {
                    format: fmt,
                    r: Ok(IpcSharedMemory::from_bytes(&bytes)),
                };
                self.load_task(key, async { r })
            }
        }
    }

    fn download_accept(&mut self) -> String {
        if self.download_accept.is_empty() {
            if let Some(view) = &self.view {
                let mut r = String::new();
                let mut fmts = view.image_decoders().unwrap_or_default().into_iter();
                if let Some(fmt) = fmts.next() {
                    r.push_str("image/");
                    r.push_str(&fmt);
                    for fmt in fmts {
                        r.push_str(",image/");
                        r.push_str(&fmt);
                    }
                    self.download_accept = r;
                }
            }
            if self.download_accept.is_empty() {
                self.download_accept = "image/*".to_owned();
            }
        }
        self.download_accept.clone()
    }

    fn load_task(&mut self, key: ImageCacheKey, fetch_bytes: impl Future<Output = ImageData> + 'static) -> ImageVar {
        let img = var(Image::new_none());
        let task = UiTask::new(&self.updates, fetch_bytes);

        self.cache.insert(key, img.clone());
        self.loading.push((task, img.clone()));

        img.into_read_only()
    }
}

struct ImageData {
    format: ImageDataFormat,
    r: std::result::Result<IpcSharedMemory, String>,
}

/// Key for a cached image in [`Images`].
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum ImageCacheKey {
    /// A path to an image file in the file system.
    Read(PathBuf),
    /// A uri to an image resource downloaded using HTTP GET.
    Download(Uri),
    /// Static bytes for an encoded or decoded image.
    Static(&'static [u8], ImageDataFormat),
    /// Shared reference to bytes.
    Data(Arc<Vec<u8>>, ImageDataFormat),
}
impl_from_and_into_var! {
    fn from(path: PathBuf) -> ImageCacheKey {
        ImageCacheKey::Read(path)
    }
    fn from(path: &Path) -> ImageCacheKey {
        ImageCacheKey::Read(path.to_owned())
    }
    fn from(uri: Uri) -> ImageCacheKey {
        ImageCacheKey::Download(uri)
    }
    /// Converts `http://` and `https://` to [`Download`], `file://` to
    /// [`Read`] the path component, and the rest to [`Read`] the string as a path.
    ///
    /// [`Download`]: ImageCacheKey::Download
    /// [`Read`]: ImageCacheKey::Read
    fn from(s: &str) -> ImageCacheKey {
        use crate::task::http::uri::*;
        if let Ok(uri) = Uri::try_from(s) {
            if let Some(scheme) = uri.scheme() {
                if scheme == &Scheme::HTTPS || scheme == &Scheme::HTTP {
                    return ImageCacheKey::Download(uri);
                } else if scheme.as_str() == "file" {
                    return PathBuf::from(uri.path()).into();
                }
            }
        }
        PathBuf::from(s).into()
    }
    /// Same as conversion from `&str`.
    fn from(s: String) -> ImageCacheKey {
        s.as_str().into()
    }
    /// Same as conversion from `&str`.
    fn from(s: Text) -> ImageCacheKey {
        s.as_str().into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: &'static [u8]) -> ImageCacheKey {
        ImageCacheKey::Static(data, ImageDataFormat::Unknown)
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from<const N: usize>(data: &'static [u8; N]) -> ImageCacheKey {
        ImageCacheKey::Static(&data[..], ImageDataFormat::Unknown)
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: Arc<Vec<u8>>) -> ImageCacheKey {
        ImageCacheKey::Data(data, ImageDataFormat::Unknown)
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: Vec<u8>) -> ImageCacheKey {
        ImageCacheKey::Data(Arc::new(data), ImageDataFormat::Unknown)
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (&'static [u8], F)) -> ImageCacheKey {
        ImageCacheKey::Static(data, format.into())
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone, const N: usize>((data, format): (&'static [u8; N], F)) -> ImageCacheKey {
        ImageCacheKey::Static(data, format.into())
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (Arc<Vec<u8>>, F)) -> ImageCacheKey {
        ImageCacheKey::Data(data, format.into())
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (Vec<u8>, F)) -> ImageCacheKey {
        ImageCacheKey::Data(Arc::new(data), format.into())
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
pub enum ProxyGetResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Load and cache using the replacement key.
    Cache(ImageCacheKey),
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
    Remove(ImageCacheKey, bool),
    /// Consider the request fulfilled.
    Removed(Option<ImageVar>),
}

/// Represents an [`Image`] tracked by the [`Images`] cache.
///
/// The variable updates when the image updates.
pub type ImageVar = ReadOnlyRcVar<Image>;

/// State of an [`ImageVar`].
///
/// Each instance of this struct represent a single state,
#[derive(Debug, Clone)]
pub struct Image {
    view: OnceCell<ViewImage>,
    render_keys: Rc<RefCell<Vec<RenderImage>>>,
    done_signal: SignalOnce,
}
impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.view == other.view
    }
}
impl Image {
    fn new_none() -> Self {
        Image {
            view: OnceCell::new(),
            render_keys: Rc::default(),
            done_signal: SignalOnce::new(),
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
    pub fn awaiter(&self) -> impl std::future::Future<Output = ()> + Send + Sync + 'static {
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

        let screen_res = ctx.screen_ppi;
        let mut s = self.size();

        s.width *= (dpi_x / screen_res) * ctx.scale_factor;
        s.height *= (dpi_y / screen_res) * ctx.scale_factor;

        s
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
    pub async fn save(&self, path: impl Into<PathBuf>) -> std::io::Result<()> {
        let path = path.into();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            self.save_impl(ext.to_owned(), path).await
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "could not determinate image format from path extension",
            ))
        }
    }

    /// Encode and write the image to `path`.
    ///
    /// The image is encoded to the `format`, the file extension can be anything.
    pub async fn save_with_format(&self, format: String, path: impl Into<PathBuf>) -> std::io::Result<()> {
        self.save_impl(format, path.into()).await
    }

    async fn save_impl(&self, format: String, path: PathBuf) -> std::io::Result<()> {
        let view = self.view.get().unwrap();
        let data = view.encode(format).await.map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        task::wait(move || std::fs::write(path, &data[..])).await
    }
}
impl crate::render::Image for Image {
    fn image_key(&self, renderer: &ViewRenderer) -> ImageKey {
        if self.is_loaded() {
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

            let key = match renderer.use_image(self.view.get().unwrap()) {
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

/// Spooky Hash V2.
///
/// This hash is used to identify image files in the [`Images`] cache.
#[derive(Clone, Copy)]
pub struct Hash128([u8; 16]);
impl Hash128 {
    /// Compute the hash for `data`.
    pub fn compute(data: &[u8]) -> Self {
        use std::hash::Hasher;
        let mut hasher =
            hashers::jenkins::spooky_hash::SpookyHasher::new(u64::from_le_bytes(*b"-Images-"), u64::from_le_bytes(*b"-Hash---"));
        hasher.write(data);
        let (s0, s1) = hasher.finish128();
        let mut hash = [0; 16];
        hash[..8].copy_from_slice(&s0.to_le_bytes());
        hash[8..].copy_from_slice(&s1.to_le_bytes());
        Hash128(hash)
    }
}
impl fmt::Debug for Hash128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("Hash128").field(&self.0).finish()
        } else {
            write!(f, "{}", base64::encode(&self.0))
        }
    }
}
impl fmt::Display for Hash128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl std::hash::Hash for Hash128 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.0)
    }
}
impl PartialEq for Hash128 {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for Hash128 {}
