//! Image loading and cache.

use std::{cell::Cell, collections::HashMap, env, future::Future, mem, path::PathBuf, sync::Arc};

use zero_ui_view_api::IpcBytes;

use crate::{
    app::{
        raw_events::{RawImageLoadErrorEvent, RawImageLoadedEvent, RawImageMetadataLoadedEvent},
        view_process::{ViewImage, ViewProcess, ViewProcessInitedEvent, ViewProcessOffline},
        AppEventSender, AppExtension,
    },
    context::AppContext,
    crate_util::IdMap,
    event::EventUpdateArgs,
    service::Service,
    task::{self, fs, io::*, ui::UiTask},
    text::Text,
    units::*,
    var::{types::WeakRcVar, var, RcVar, Var, Vars, WeakVar},
};

mod types;
pub use types::*;

mod render;
pub use render::*;

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
        } else if let Some(args) = ViewProcessInitedEvent.update(args) {
            if !args.is_respawn {
                return;
            }

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
                            Err(ViewProcessOffline) => { /*will receive another event.*/ }
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
                            Err(ViewProcessOffline) => return, // we will receive another event.
                        };

                        img_var.set(vars, Image::new(img));

                        images.decoding.push((img_format, data, img_var));
                    }
                } // else { *is loading, will continue normally in self.update_preview()* }
            }
        } else {
            self.event_preview_render(ctx, args);
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
                                        var.modify(vars, move |mut v| {
                                            v.view.set(img).unwrap();
                                            v.touch();
                                        });
                                    }
                                    Err(ViewProcessOffline) => {
                                        // will recover in ViewProcessInitedEvent
                                    }
                                }
                                decoding.push((d.format, data, var));
                            } else {
                                // success, but we are only doing `load_in_headless` validation.
                                let img = ViewImage::dummy(None);
                                var.modify(vars, move |mut v| {
                                    v.view.set(img).unwrap();
                                    v.touch();
                                    v.done_signal.set();
                                });
                            }
                        }
                        Err(e) => {
                            tracing::error!("load error: {e:?}");
                            // load error.
                            let img = ViewImage::dummy(Some(e));
                            var.modify(vars, move |mut v| {
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

    fn update(&mut self, ctx: &mut AppContext) {
        self.update_render(ctx);
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
    not_cached: Vec<(WeakRcVar<Image>, ByteLength)>,

    render: ImagesRender,
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
            render: ImagesRender::default(),
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
    #[cfg(http)]
    #[cfg_attr(doc_nightly, doc(cfg(http)))]
    pub fn download(&mut self, uri: impl task::http::TryUri, accept: Option<Text>) -> ImageVar {
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
    pub fn register(&mut self, key: ImageHash, image: ViewImage) -> Option<ImageVar> {
        let limits = ImageLimits {
            max_encoded_size: self.limits.max_encoded_size,
            max_decoded_size: self
                .limits
                .max_decoded_size
                .max(image.bgra8().map(|b| b.len()).unwrap_or(0).bytes()),
            allow_path: PathFilter::BlockAll,
            #[cfg(http)]
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
                    tracing::error!("{error}");
                    return var(Image::dummy(Some(error))).into_read_only();
                }
                ImageSource::Read(path)
            }
            #[cfg(http)]
            ImageSource::Download(uri, accepts) => {
                if !limits.allow_uri.allows(&uri) {
                    let error = format!("limits filter blocked `{uri}`");
                    tracing::error!("{error}");
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
                        r.r = Err(format!("file size `{}` exceeds the limit of `{max_encoded_size}`", len.bytes()));
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
            #[cfg(http)]
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
                        let request = task::http::Request::get(uri)
                            .unwrap()
                            .header(task::http::header::ACCEPT, accept)
                            .unwrap()
                            .max_length(max_encoded_size)
                            .build();

                        match task::http::send(request).await {
                            Ok(mut rsp) => {
                                if let Some(m) = rsp.headers().get(&task::http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
                                    let m = m.to_lowercase();
                                    if m.starts_with("image/") {
                                        r.format = ImageDataFormat::MimeType(m);
                                    }
                                }

                                match rsp.bytes().await {
                                    Ok(d) => r.r = Ok(IpcBytes::from_vec(d)),
                                    Err(e) => {
                                        r.r = Err(format!("download error: {e}"));
                                    }
                                }

                                let _ = rsp.consume();
                            }
                            Err(e) => {
                                r.r = Err(format!("request error: {e}"));
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
            ImageSource::Render(rfn, cfg) => {
                let img = self.new_cache_image(key, mode, limits.max_decoded_size);
                self.render_image(clone_move!(rfn, |ctx| rfn(ctx)), cfg, &img);
                img.into_read_only()
            }
            ImageSource::Image(_) => unreachable!(),
        }
    }

    #[cfg(http)]
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

    fn new_cache_image(&mut self, key: ImageHash, mode: ImageCacheMode, max_decoded_size: ByteLength) -> RcVar<Image> {
        self.cleanup_not_cached(false);

        if let ImageCacheMode::Reload = mode {
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
        let img = self.new_cache_image(key, mode, max_decoded_size);

        let task = UiTask::new(&self.updates, fetch_bytes);
        self.loading.push((task, img.clone(), max_decoded_size));

        img.into_read_only()
    }
}
struct ImageData {
    format: ImageDataFormat,
    r: std::result::Result<IpcBytes, String>,
}
