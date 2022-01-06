//! Image loading and cache.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    convert::TryFrom,
    env, fmt,
    future::Future,
    mem, ops,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use once_cell::unsync::OnceCell;
use zero_ui_view_api::{webrender_api::ImageKey, IpcBytes};

use crate::{
    app::{
        raw_events::{RawImageLoadErrorEvent, RawImageLoadedEvent, RawImageMetadataLoadedEvent},
        view_process::{EncodeError, Respawned, ViewImage, ViewProcess, ViewProcessRespawnedEvent, ViewRenderer},
        AppEventSender, AppExtension,
    },
    context::{AppContext, LayoutMetrics},
    crate_util::IdMap,
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
    var::{var, RcVar, ReadOnlyRcVar, Var, Vars, WeakVar},
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
            if let Some(var) = images
                .decoding
                .iter()
                .map(|(_, _, v)| v)
                .find(|v| v.get(vars).view.get().unwrap() == &args.image)
            {
                var.touch(ctx.vars);
            }
        } else if let Some(args) = RawImageLoadedEvent.update(args) {
            let image = &args.image;

            // image finished decoding, remove from `decoding`
            // and notify image var value update.
            let images = ctx.services.images();
            let vars = ctx.vars;

            if let Some(i) = images
                .decoding
                .iter()
                .position(|(_, _, v)| v.get(vars).view.get().unwrap() == image)
            {
                let (_, _, var) = images.decoding.swap_remove(i);
                var.touch(ctx.vars);
                let img = var.get(ctx.vars);
                img.done_signal.set();
            }
        } else if let Some(args) = RawImageLoadErrorEvent.update(args) {
            let image = &args.image;

            // image failed to decode, remove from `decoding`
            // and notify image var value update.
            let images = ctx.services.images();
            let vars = ctx.vars;

            if let Some(i) = images
                .decoding
                .iter()
                .position(|(_, _, v)| v.get(vars).view.get().unwrap() == image)
            {
                let (_, _, var) = images.decoding.swap_remove(i);
                var.touch(ctx.vars);
                let img = var.get(ctx.vars);
                img.done_signal.set();
                if let Some(k) = &img.cache_key {
                    if let Some(e) = images.cache.get(k) {
                        e.error.set(true);
                    }
                }

                tracing::error!("decode error: {:?}", img.error().unwrap());
            }
        } else if let Some(args) = ViewProcessRespawnedEvent.update(args) {
            let images = ctx.services.images();
            images.cleanup_not_cached(true);
            images.download_accept.clear();
            let vp = images.view.as_mut().unwrap();
            let decoding_interrupted = mem::take(&mut images.decoding);
            for (img_var, max_decoded_size) in images
                .cache
                .values()
                .map(|e| (e.img.clone(), e.max_decoded_size))
                .chain(images.not_cached.iter().filter_map(|(v, m)| v.upgrade().map(|v| (v, *m))))
            {
                let img = img_var.get(ctx.vars);

                let vars = ctx.vars;
                if let Some(view) = img.view.get() {
                    if view.generation() == args.generation {
                        continue; // already recovered, can this happen?
                    }
                    if let Some(e) = view.error() {
                        // respawned, but image was an error.
                        img_var.set(vars, Image::dummy(Some(e.to_owned())));
                    } else if let Some((img_format, data, _)) =
                        decoding_interrupted.iter().find(|(_, _, v)| v.get(vars).view() == Some(view))
                    {
                        // respawned, but image was decoding, need to restart decode.

                        match vp.add_image(img_format.clone(), data.clone(), max_decoded_size.0 as u64) {
                            Ok(img) => {
                                img_var.set(vars, Image::new(img));
                            }
                            Err(Respawned) => { /*will receive another event.*/ }
                        }
                        images.decoding.push((img_format.clone(), data.clone(), img_var));
                    } else {
                        // respawned and image was loaded.

                        let img_format = ImageDataFormat::Bgra8 {
                            size: view.size(),
                            ppi: view.ppi(),
                        };

                        let data = view.shared_bgra8().unwrap();
                        let img = match vp.add_image(img_format.clone(), data.clone(), max_decoded_size.0 as u64) {
                            Ok(img) => img,
                            Err(Respawned) => return, // we will receive another event.
                        };

                        img_var.set(vars, Image::new(img));

                        images.decoding.push((img_format, data, img_var));
                    }
                } // else { *is loading, will continue normally in self.update_preview()* }
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

        for (mut task, var, max_decoded_size) in mem::take(&mut images.loading) {
            task.update();
            match task.into_result() {
                Ok(d) => {
                    match d.r {
                        Ok(data) => {
                            if let Some(vp) = view {
                                // success and we have a view-process.
                                match vp.add_image(d.format.clone(), data.clone(), max_decoded_size.0 as u64) {
                                    Ok(img) => {
                                        // request sent, add to `decoding` will receive
                                        // `RawImageLoadedEvent` or `RawImageLoadErrorEvent` event
                                        // when done.
                                        var.modify(vars, move |v| {
                                            v.view.set(img).unwrap();
                                            v.touch();
                                        });
                                    }
                                    Err(Respawned) => {
                                        // will recover in ViewProcessRespawnedEvent
                                    }
                                }
                                decoding.push((d.format, data, var));
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
                            tracing::error!("load error: {:?}", e);
                            // load error.
                            let img = ViewImage::dummy(Some(e));
                            var.modify(vars, move |v| {
                                v.view.set(img).unwrap();
                                v.touch();
                                v.done_signal.set();
                            });

                            // flag error for user retry
                            if let Some(k) = &var.get(ctx.vars).cache_key {
                                if let Some(e) = images.cache.get(k) {
                                    e.error.set(true)
                                }
                            }
                        }
                    }
                }
                Err(task) => {
                    loading.push((task, var, max_decoded_size));
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

    /// Default loading and decoding limits for each image.
    pub limits: ImageLimits,

    view: Option<ViewProcess>,
    download_accept: Text,
    updates: AppEventSender,
    proxies: Vec<Box<dyn ImageCacheProxy>>,

    loading: Vec<(UiTask<ImageData>, RcVar<Image>, ByteLength)>,
    decoding: Vec<(ImageDataFormat, IpcBytes, RcVar<Image>)>,
    cache: IdMap<ImageHash, CacheEntry>,
    not_cached: Vec<(WeakVar<Image>, ByteLength)>,
}
struct CacheEntry {
    img: RcVar<Image>,
    error: Cell<bool>,
    max_decoded_size: ByteLength,
}
impl Images {
    fn new(view: Option<ViewProcess>, updates: AppEventSender) -> Self {
        Images {
            load_in_headless: false,
            limits: ImageLimits::default(),
            view,
            updates,
            proxies: vec![],
            loading: vec![],
            decoding: vec![],
            download_accept: Text::empty(),
            cache: HashMap::default(),
            not_cached: vec![],
        }
    }

    /// Returns a dummy image that reports it is loaded or an error.
    pub fn dummy(&self, error: Option<String>) -> ImageVar {
        var(Image::dummy(error)).into_read_only()
    }

    /// Cache or load an image file from a file system `path`.
    pub fn read(&mut self, path: impl Into<PathBuf>) -> ImageVar {
        self.cache(path.into())
    }

    /// Get a cached `uri` or download it.
    ///
    /// Optionally define the HTTP ACCEPT header, if not set all image formats supported by the view-process
    /// backend are accepted.
    pub fn download(&mut self, uri: impl TryUri, accept: Option<Text>) -> ImageVar {
        match uri.try_uri() {
            Ok(uri) => self.cache(ImageSource::Download(uri, accept)),
            Err(e) => self.dummy(Some(e.to_string())),
        }
    }

    /// Get a cached image from `&'static [u8]` data.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    ///
    /// The image key is a [`ImageHash`] of the image data.
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
        self.cache((data, format.into()))
    }

    /// Get a cached image from shared data.
    ///
    /// The image key is a [`ImageHash`] of the image data. The data reference is held only until the image is decoded.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    pub fn from_data(&mut self, data: Arc<Vec<u8>>, format: impl Into<ImageDataFormat>) -> ImageVar {
        self.cache((data, format.into()))
    }

    /// Get a cached image or add it to the cache.
    pub fn cache(&mut self, source: impl Into<ImageSource>) -> ImageVar {
        self.get(source, ImageCacheMode::Cache, None)
    }

    /// Get a cached image or add it to the cache or retry if the cached image is an error.
    pub fn retry(&mut self, source: impl Into<ImageSource>) -> ImageVar {
        self.get(source, ImageCacheMode::Retry, None)
    }

    /// Load an image, if it was already cached update the cached image with the reloaded data.
    pub fn reload(&mut self, source: impl Into<ImageSource>) -> ImageVar {
        self.get(source, ImageCacheMode::Reload, None)
    }

    /// Get or load an image.
    ///
    /// If `limits` is `None` the [`Images::limits`] is used.
    pub fn get(&mut self, source: impl Into<ImageSource>, cache_mode: impl Into<ImageCacheMode>, limits: Option<ImageLimits>) -> ImageVar {
        self.proxy_then_get(source.into(), cache_mode.into(), limits.unwrap_or_else(|| self.limits.clone()))
    }

    /// Associate the `image` with the `key` in the cache.
    ///
    /// Returns `Some(previous)` if the `key` was already associated with an image.
    #[inline]
    pub fn register(&mut self, key: ImageHash, image: ViewImage) -> Option<ImageVar> {
        let limits = ImageLimits {
            max_encoded_size: self.limits.max_encoded_size,
            max_decoded_size: self
                .limits
                .max_decoded_size
                .max(image.bgra8().map(|b| b.len()).unwrap_or(0).bytes()),
            allow_path: PathFilter::BlockAll,
            allow_uri: UriFilter::BlockAll,
        };
        let entry = CacheEntry {
            error: Cell::new(image.is_error()),
            img: var(Image::new(image)),
            max_decoded_size: limits.max_decoded_size,
        };
        self.cache.insert(key, entry).map(|v| v.img.into_read_only())
    }

    /// Remove the image from the cache, if it is only held by the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the image was removed.
    pub fn clean(&mut self, key: ImageHash) -> bool {
        self.proxy_then_remove(&key, false)
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the image was cached.
    pub fn purge(&mut self, key: &ImageHash) -> bool {
        self.proxy_then_remove(key, true)
    }

    /// Gets the cache key of an image.
    pub fn cache_key(&self, image: &Image) -> Option<ImageHash> {
        if let Some(key) = &image.cache_key {
            if self.cache.contains_key(key) {
                return Some(*key);
            }
        }
        None
    }

    /// If the image is cached.
    pub fn is_cached(&self, image: &Image) -> bool {
        image.cache_key.as_ref().map(|k| self.cache.contains_key(k)).unwrap_or(false)
    }

    /// Returns an image that is not cached.
    ///
    /// If the `image` is the only reference returns it and removes it from the cache. If there are other
    /// references a new [`ImageVar`] is generated from a clone of the image.
    pub fn detach(&mut self, image: ImageVar, vars: &Vars) -> ImageVar {
        if let Some(key) = &image.get(vars).cache_key {
            let decoded_size = image.get(vars).bgra8().map(|b| b.len()).unwrap_or(0).bytes();
            let mut max_decoded_size = self.limits.max_decoded_size.max(decoded_size);

            if let Some(e) = self.cache.get(key) {
                max_decoded_size = e.max_decoded_size;

                // is cached, `clean` if is only external reference.
                if image.strong_count() == 2 {
                    self.cache.remove(key);
                }
            }

            // remove `cache_key` from image, this clones the `Image` only-if is still in cache.
            let mut img = image.into_value(vars);
            img.cache_key = None;
            let img = var(img);
            self.not_cached.push((img.downgrade(), max_decoded_size));
            img.into_read_only()
        } else {
            // already not cached
            image
        }
    }

    fn proxy_then_remove(&mut self, key: &ImageHash, purge: bool) -> bool {
        for proxy in &mut self.proxies {
            let r = proxy.remove(key, purge);
            match r {
                ProxyRemoveResult::None => continue,
                ProxyRemoveResult::Remove(r, p) => return self.proxied_remove(&r, p),
                ProxyRemoveResult::Removed => return true,
            }
        }
        self.proxied_remove(key, purge)
    }
    fn proxied_remove(&mut self, key: &ImageHash, purge: bool) -> bool {
        if purge || self.cache.get(key).map(|v| v.img.strong_count() > 1).unwrap_or(false) {
            self.cache.remove(key).is_some()
        } else {
            false
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

    fn proxy_then_get(&mut self, source: ImageSource, mode: ImageCacheMode, limits: ImageLimits) -> ImageVar {
        let source = match source {
            ImageSource::Read(path) => {
                let path = crate::crate_util::absolute_path(&path, || env::current_dir().expect("could not access current dir"), true);
                if !limits.allow_path.allows(&path) {
                    let error = format!("limits filter blocked `{}`", path.display());
                    tracing::error!("{}", error);
                    return var(Image::dummy(Some(error))).into_read_only();
                }
                ImageSource::Read(path)
            }
            ImageSource::Download(uri, accepts) => {
                if !limits.allow_uri.allows(&uri) {
                    let error = format!("limits filter blocked `{}`", uri);
                    tracing::error!("{}", error);
                    return var(Image::dummy(Some(error))).into_read_only();
                }
                ImageSource::Download(uri, accepts)
            }
            ImageSource::Image(r) => return r,
            source => source,
        };

        let key = source.hash128().unwrap();
        for proxy in &mut self.proxies {
            let r = proxy.get(&key, &source, mode);
            match r {
                ProxyGetResult::None => continue,
                ProxyGetResult::Cache(source, mode) => return self.proxied_get(key, source, mode, limits),
                ProxyGetResult::Image(img) => return img,
            }
        }
        self.proxied_get(key, source, mode, limits)
    }
    fn proxied_get(&mut self, key: ImageHash, source: ImageSource, mode: ImageCacheMode, limits: ImageLimits) -> ImageVar {
        match mode {
            ImageCacheMode::Cache => {
                if let Some(v) = self.cache.get(&key) {
                    return v.img.clone().into_read_only();
                }
            }
            ImageCacheMode::Retry => {
                if let Some(e) = self.cache.get(&key) {
                    if !e.error.get() {
                        return e.img.clone().into_read_only();
                    }
                }
            }
            ImageCacheMode::Ignore | ImageCacheMode::Reload => {}
        }

        if self.view.is_none() && !self.load_in_headless {
            tracing::warn!("loading dummy image, set `load_in_headless=true` to actually load without renderer");

            let dummy = var(Image::new(ViewImage::dummy(None)));
            self.cache.insert(
                key,
                CacheEntry {
                    img: dummy.clone(),
                    error: Cell::new(false),
                    max_decoded_size: limits.max_decoded_size,
                },
            );
            return dummy.into_read_only();
        }

        let max_encoded_size = limits.max_encoded_size;

        match source {
            ImageSource::Read(path) => self.load_task(
                key,
                mode,
                limits.max_decoded_size,
                task::run(async move {
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
                        Ok(_) => Ok(IpcBytes::from_vec(data)),
                        Err(e) => Err(e.to_string()),
                    };

                    r
                }),
            ),
            ImageSource::Download(uri, accept) => {
                let accept = accept.unwrap_or_else(|| self.download_accept());

                self.load_task(
                    key,
                    mode,
                    limits.max_decoded_size,
                    task::run(async move {
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
                                        r.r = Err(format!("file size `{}` exceeds the limit of `{}`", len, max_encoded_size));
                                        return r;
                                    }
                                }

                                match rsp.bytes_limited(max_encoded_size.saturating_add(1.bytes())).await {
                                    Ok(d) => {
                                        if d.len().bytes() > max_encoded_size {
                                            r.r = Err(format!("download exceeded the limit of `{}`", max_encoded_size));
                                        } else {
                                            r.r = Ok(IpcBytes::from_vec(d))
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
                    }),
                )
            }
            ImageSource::Static(_, bytes, fmt) => {
                let r = ImageData {
                    format: fmt,
                    r: Ok(IpcBytes::from_slice(bytes)),
                };
                self.load_task(key, mode, limits.max_decoded_size, async { r })
            }
            ImageSource::Data(_, bytes, fmt) => {
                let r = ImageData {
                    format: fmt,
                    r: Ok(IpcBytes::from_slice(&bytes)),
                };
                self.load_task(key, mode, limits.max_decoded_size, async { r })
            }
            ImageSource::Image(_) => unreachable!(),
        }
    }

    fn download_accept(&mut self) -> Text {
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
                    self.download_accept = r.into();
                }
            }
            if self.download_accept.is_empty() {
                self.download_accept = "image/*".into();
            }
        }
        self.download_accept.clone()
    }

    fn cleanup_not_cached(&mut self, force: bool) {
        if force || self.not_cached.len() > 1000 {
            self.not_cached.retain(|c| c.0.strong_count() > 0);
        }
    }

    /// The `fetch_bytes` future is polled in the UI thread, use `task::run` for futures that poll a lot.
    fn load_task(
        &mut self,
        key: ImageHash,
        mode: ImageCacheMode,
        max_decoded_size: ByteLength,
        fetch_bytes: impl Future<Output = ImageData> + 'static,
    ) -> ImageVar {
        self.cleanup_not_cached(false);

        let img = if let ImageCacheMode::Reload = mode {
            self.cache
                .entry(key)
                .or_insert_with(|| CacheEntry {
                    img: var(Image::new_none(Some(key))),
                    error: Cell::new(false),
                    max_decoded_size,
                })
                .img
                .clone()
        } else if let ImageCacheMode::Ignore = mode {
            let img = var(Image::new_none(None));
            self.not_cached.push((img.downgrade(), max_decoded_size));
            img
        } else {
            let img = var(Image::new_none(Some(key)));
            self.cache.insert(
                key,
                CacheEntry {
                    img: img.clone(),
                    error: Cell::new(false),
                    max_decoded_size,
                },
            );
            img
        };

        let task = UiTask::new(&self.updates, fetch_bytes);

        self.loading.push((task, img.clone(), max_decoded_size));

        img.into_read_only()
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
pub type UriFilter = ImageSourceFilter<Uri>;
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

impl<F: Fn(&Uri) -> bool + 'static> From<F> for UriFilter {
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
    pub allow_uri: UriFilter,
}
impl ImageLimits {
    /// No size limits, allow all paths and URIs.
    pub fn none() -> Self {
        ImageLimits {
            max_encoded_size: ByteLength::MAX,
            max_decoded_size: ByteLength::MAX,
            allow_path: PathFilter::AllowAll,
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
            allow_uri: UriFilter::BlockAll,
        }
    }
}

struct ImageData {
    format: ImageDataFormat,
    r: std::result::Result<IpcBytes, String>,
}

/// A custom proxy in [`Images`].
///
/// Implementers can intercept cache requests and redirect to another cache request or returns an image directly.
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
pub type ImageVar = ReadOnlyRcVar<Image>;

/// State of an [`ImageVar`].
///
/// Each instance of this struct represent a single state,
#[derive(Debug, Clone)]
pub struct Image {
    view: OnceCell<ViewImage>,
    render_keys: Rc<RefCell<Vec<RenderImage>>>,
    done_signal: SignalOnce,
    cache_key: Option<ImageHash>,
}
impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.view == other.view
    }
}
impl Image {
    fn new_none(cache_key: Option<ImageHash>) -> Self {
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

        s.width *= (dpi_x / screen_res) * ctx.scale_factor.0;
        s.height *= (dpi_y / screen_res) * ctx.scale_factor.0;

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
                Err(Respawned) => {
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
        write!(f, "{:?}", self)
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
    Download(Uri, Option<Text>),
    /// Static bytes for an encoded or decoded image.
    ///
    /// Image equality is defined by the hash, it is usually the hash of the bytes but it does not need to be.
    Static(ImageHash, &'static [u8], ImageDataFormat),
    /// Shared reference to bytes for an encoded or decoded image.
    ///
    /// Image equality is defined by the hash, it is usually the hash of the bytes but it does not need to be.
    ///
    /// Inside [`Images`] the reference to the bytes is held only until the image finishes decoding.
    Data(ImageHash, Arc<Vec<u8>>, ImageDataFormat),
    /// Already loaded image.
    ///
    /// The image is passed-through, not cached.
    Image(ImageVar),
}
impl ImageSource {
    /// Returns the image hash, unless the source is [`Image`].
    ///
    /// [`Image`]: Self::Image
    pub fn hash128(&self) -> Option<ImageHash> {
        match self {
            ImageSource::Read(p) => Some(Self::hash128_read(p)),
            ImageSource::Download(u, a) => Some(Self::hash128_download(u, a)),
            ImageSource::Static(h, _, _) => Some(*h),
            ImageSource::Data(h, _, _) => Some(*h),
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
    pub fn hash128_download(uri: &Uri, accept: &Option<Text>) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        1u8.hash(&mut h);
        uri.hash(&mut h);
        accept.hash(&mut h);
        h.finish()
    }
}
impl PartialEq for ImageSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Read(l), Self::Read(r)) => l == r,
            (Self::Download(lu, la), Self::Download(ru, ra)) => lu == ru && la == ra,
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
            ImageSource::Download(u, a) => f.debug_tuple("Download").field(u).field(a).finish(),
            ImageSource::Static(key, _, fmt) => f.debug_tuple("Static").field(key).field(fmt).finish(),
            ImageSource::Data(key, _, fmt) => f.debug_tuple("Data").field(key).field(fmt).finish(),
            ImageSource::Image(_) => write!(f, "Image(_)"),
        }
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
    fn from(uri: Uri) -> ImageSource {
        ImageSource::Download(uri, None)
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
    /// Same as conversion from `&str`.
    fn from(s: String) -> ImageSource {
       s.as_str().into()
    }
    /// Same as conversion from `&str`.
    fn from(s: Text) -> ImageSource {
        s.as_str().into()
    }
    /// From (URI, HTTP-ACCEPT).
    fn from((uri, accept): (Uri, &'static str)) -> ImageSource {
        ImageSource::Download(uri, Some(accept.into()))
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
