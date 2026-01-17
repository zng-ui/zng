use std::{any::Any, mem, path::PathBuf, pin::Pin};

use parking_lot::Mutex;
use zng_app::{
    static_id,
    update::UPDATES,
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewImageHandle,
        raw_events::{
            RAW_FRAME_RENDERED_EVENT, RAW_HEADLESS_OPEN_EVENT, RAW_IMAGE_DECODE_ERROR_EVENT, RAW_IMAGE_DECODED_EVENT,
            RAW_IMAGE_METADATA_DECODED_EVENT, RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT,
        },
    },
    widget::{
        WIDGET,
        node::{IntoUiNode, UiNode, UiNodeOp, match_node},
    },
    window::{WINDOW, WindowId},
};
use zng_app_context::app_local;
use zng_clone_move::clmv;
use zng_layout::unit::ByteLength;
use zng_state_map::StateId;
use zng_task::channel::IpcBytes;
use zng_txt::ToTxt;
use zng_txt::Txt;
use zng_unique_id::{IdEntry, IdMap};
use zng_var::{IntoVar, Var, VarHandle, const_var, var};
use zng_view_api::{
    image::{ImageDecoded, ImageRequest},
    window::RenderMode,
};

mod types;
pub use types::*;

app_local! {
    static IMAGES_SV: ImagesService = ImagesService::new();
}

struct ImagesService {
    load_in_headless: Var<bool>,
    limits: Var<ImageLimits>,

    extensions: Vec<Box<dyn ImagesExtension>>,
    render_windows: Option<Box<dyn ImageRenderWindowsService>>,

    cache: IdMap<ImageHash, ImageVar>,
}
impl ImagesService {
    pub fn new() -> Self {
        Self {
            load_in_headless: var(false),
            limits: var(ImageLimits::default()),

            extensions: vec![],
            render_windows: None,

            cache: IdMap::new(),
        }
    }

    pub fn render_windows(&self) -> Box<dyn ImageRenderWindowsService> {
        self.render_windows
            .as_ref()
            .expect("WINDOWS service not integrated with IMAGES service")
            .clone_boxed()
    }
}

/// Image loading, cache and render service.
///
/// If the app is running without a [`VIEW_PROCESS`] all images are dummy, see [`load_in_headless`] for
/// details.
///
/// [`load_in_headless`]: IMAGES::load_in_headless
/// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
pub struct IMAGES;
impl IMAGES {
    /// If should still download/read image bytes in headless/renderless mode.
    ///
    /// When an app is in headless mode without renderer no [`VIEW_PROCESS`] is available, so
    /// images cannot be decoded, in this case all images are dummy loading and no attempt
    /// to download/read the image files is made. You can enable loading in headless tests to detect
    /// IO errors, in this case if there is an error acquiring the image file the image will be a
    /// [`dummy`] with error.
    ///
    /// [`dummy`]: IMAGES::dummy
    /// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
    pub fn load_in_headless(&self) -> Var<bool> {
        IMAGES_SV.read().load_in_headless.clone()
    }

    /// Default loading and decoding limits for each image.
    pub fn limits(&self) -> Var<ImageLimits> {
        IMAGES_SV.read().limits.clone()
    }

    /// Request an image, reads from a `path` and caches it.
    ///
    /// This is shorthand for calling [`IMAGES.image`] with [`ImageSource::Read`] and [`ImageOptions::cache`].
    ///
    /// [`IMAGES.image`]: IMAGES::image
    pub fn read(&self, path: impl Into<PathBuf>) -> ImageVar {
        self.image_impl(path.into().into(), ImageOptions::cache(), None)
    }

    /// Request an image, downloads from an `uri` and caches it.
    ///
    /// Optionally define the HTTP ACCEPT header, if not set all image formats supported by the view-process
    /// backend are accepted.
    ///
    /// This is shorthand for calling [`IMAGES.image`] with [`ImageSource::Download`] and [`ImageOptions::cache`].
    ///
    /// [`IMAGES.image`]: IMAGES::image
    #[cfg(feature = "http")]
    pub fn download<U>(&self, uri: U, accept: Option<Txt>) -> ImageVar
    where
        U: TryInto<zng_task::http::Uri>,
        <U as TryInto<zng_task::http::Uri>>::Error: ToTxt,
    {
        match uri.try_into() {
            Ok(uri) => self.image_impl(ImageSource::Download(uri, accept), ImageOptions::cache(), None),
            Err(e) => const_var(ImageEntry::new_error(e.to_txt())),
        }
    }

    /// Request an image from `&'static [u8]` data.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    ///
    /// This is shorthand for calling [`IMAGES.image`] with [`ImageSource::Data`] and [`ImageOptions::cache`].
    ///
    /// # Examples
    ///
    /// Get an image from a PNG file embedded in the app executable using [`include_bytes!`].
    ///
    /// ```
    /// # use zng_ext_image::*;
    /// # macro_rules! include_bytes { ($tt:tt) => { &[] } }
    /// # fn demo() {
    /// let image_var = IMAGES.from_static(include_bytes!("ico.png"), "png");
    /// # }
    /// ```
    ///
    /// [`IMAGES.image`]: IMAGES::image
    pub fn from_static(&self, data: &'static [u8], format: impl Into<ImageDataFormat>) -> ImageVar {
        self.image_impl((data, format.into()).into(), ImageOptions::cache(), None)
    }

    /// Get a cached image from shared data.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    ///
    /// This is shorthand for calling [`IMAGES.image`] with [`ImageSource::Data`] and [`ImageOptions::cache`].
    ///
    /// [`IMAGES.image`]: IMAGES::image
    pub fn from_data(&self, data: IpcBytes, format: impl Into<ImageDataFormat>) -> ImageVar {
        self.image_impl((data, format.into()).into(), ImageOptions::cache(), None)
    }

    /// Request an image, with full load and cache configuration.
    ///
    /// If `limits` is `None` the [`IMAGES.limits`] is used.
    ///
    /// Always returns a *loading* image due to the deferred nature of services. If the image is already in cache
    /// it will be set and bound to it once the current update finishes.
    ///
    /// [`IMAGES.limits`]: IMAGES::limits
    pub fn image(&self, source: impl Into<ImageSource>, options: ImageOptions, limits: Option<ImageLimits>) -> ImageVar {
        self.image_impl(source.into(), options, limits)
    }
    fn image_impl(&self, source: ImageSource, options: ImageOptions, limits: Option<ImageLimits>) -> ImageVar {
        let r = var(ImageEntry::new_loading());
        let ri = r.read_only();
        UPDATES.once_update("IMAGES.image", move || {
            image(source, options, limits, r);
        });
        ri
    }

    /// Await for an image source, then get or load the image.
    ///
    /// If `limits` is `None` the [`IMAGES.limits`] is used.
    ///
    /// This method returns immediately with a loading [`ImageVar`], when `source` is ready it
    /// is used to get the actual [`ImageVar`] and binds it to the returned image.
    ///
    /// Note that the [`cache_mode`] always applies to the inner image, and only to the return image if `cache_key` is set.
    ///
    /// [`IMAGES.limits`]: IMAGES::limits
    /// [`cache_mode`]: ImageOptions::cache_mode
    pub fn image_task<F>(&self, source: impl IntoFuture<IntoFuture = F>, options: ImageOptions, limits: Option<ImageLimits>) -> ImageVar
    where
        F: Future<Output = ImageSource> + Send + 'static,
    {
        self.image_task_impl(Box::pin(source.into_future()), options, limits)
    }
    fn image_task_impl(
        &self,
        source: Pin<Box<dyn Future<Output = ImageSource> + Send + 'static>>,
        options: ImageOptions,
        limits: Option<ImageLimits>,
    ) -> ImageVar {
        let r = var(ImageEntry::new_loading());
        let ri = r.read_only();
        zng_task::spawn(async move {
            let source = source.await;
            image(source, options, limits, r);
        });
        ri
    }

    /// Associate the `image` produced by direct interaction with the view-process with the `key` in the cache.
    ///
    /// Returns an image var that tracks the image, note that if the `key` is already known does not use the `image` data.
    ///
    /// Note that you can register entries in [`ImageEntry::insert_entry`], this method is only for tracking a new entry.
    ///
    /// Note that the image will not automatically restore on respawn if the view-process fails while decoding.
    pub fn register(&self, key: Option<ImageHash>, image: (ViewImageHandle, ImageDecoded)) -> ImageVar {
        let r = var(ImageEntry::new_loading());
        let rr = r.read_only();
        UPDATES.once_update("IMAGES.register", move || {
            image_view(key, image.0, image.1, None, r);
        });
        rr
    }

    /// Remove the image from the cache, if it is only held by the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    pub fn clean(&self, key: ImageHash) {
        UPDATES.once_update("IMAGES.clean", move || {
            if let IdEntry::Occupied(e) = IMAGES_SV.write().cache.entry(key)
                && e.get().strong_count() == 1
            {
                e.remove();
            }
        });
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    pub fn purge(&self, key: ImageHash) {
        UPDATES.once_update("IMAGES.purge", move || {
            IMAGES_SV.write().cache.remove(&key);
        });
    }

    /// Gets the cache key of an image.
    pub fn cache_key(&self, image: &ImageEntry) -> Option<ImageHash> {
        let key = image.cache_key?;
        if IMAGES_SV.read().cache.contains_key(&key) {
            Some(key)
        } else {
            None
        }
    }

    /// If the image is cached.
    pub fn is_cached(&self, image: &ImageEntry) -> bool {
        match &image.cache_key {
            Some(k) => IMAGES_SV.read().cache.contains_key(k),
            None => false,
        }
    }

    /// Clear cached images that are not referenced outside of the cache.
    pub fn clean_all(&self) {
        UPDATES.once_update("IMAGES.clean_all", || {
            IMAGES_SV.write().cache.retain(|_, v| v.strong_count() > 1);
        });
    }

    /// Clear all cached images, including images that are still referenced outside of the cache.
    ///
    /// Image memory only drops when all strong references are removed, so if an image is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_all(&self) {
        UPDATES.once_update("IMAGES.purge_all", || {
            IMAGES_SV.write().cache.clear();
        });
    }

    /// Add an images service extension.
    ///
    /// See [`ImagesExtension`] for extension capabilities.
    pub fn extend(&self, extension: Box<dyn ImagesExtension>) {
        UPDATES.once_update("IMAGES.extend", move || {
            IMAGES_SV.write().extensions.push(extension);
        });
    }

    /// Image formats implemented by the current view-process and extensions.
    pub fn available_formats(&self) -> Vec<ImageFormat> {
        let mut formats = VIEW_PROCESS.info().image.clone();

        let mut exts = mem::take(&mut IMAGES_SV.write().extensions);
        for ext in exts.iter_mut() {
            ext.available_formats(&mut formats);
        }
        let mut s = IMAGES_SV.write();
        exts.append(&mut s.extensions);
        s.extensions = exts;

        formats
    }

    fn http_accept(&self) -> Txt {
        let mut s = String::new();
        let mut sep = "";
        for f in self.available_formats() {
            for f in f.media_type_suffixes_iter() {
                s.push_str(sep);
                s.push_str("image/");
                s.push_str(f);
                sep = ",";
            }
        }
        s.into()
    }
}

fn image(mut source: ImageSource, mut options: ImageOptions, limits: Option<ImageLimits>, r: Var<ImageEntry>) {
    let mut s = IMAGES_SV.write();

    let limits = limits.unwrap_or_else(|| s.limits.get());

    // apply extensions
    let mut exts = mem::take(&mut s.extensions);
    drop(s); // drop because extensions may use the service
    for ext in &mut exts {
        ext.image(&limits, &mut source, &mut options);
    }
    let mut s = IMAGES_SV.write();
    exts.append(&mut s.extensions);

    if let ImageSource::Image(var) = source {
        // Image is passthrough, cache config is ignored
        var.set_bind(&r).perm();
        r.hold(var).perm();
        return;
    }

    if !VIEW_PROCESS.is_available() && !s.load_in_headless.get() {
        tracing::debug!("ignoring image request due headless mode");
        return;
    }

    let key = source.hash128(&options).unwrap();

    // setup cache and drop service lock
    match options.cache_mode {
        ImageCacheMode::Ignore => (),
        ImageCacheMode::Cache => {
            match s.cache.entry(key) {
                IdEntry::Occupied(e) => {
                    // already cached
                    let var = e.get();
                    var.set_bind(&r).perm();
                    r.hold(var.clone()).perm();
                    return;
                }
                IdEntry::Vacant(e) => {
                    // cache
                    e.insert(r.clone());
                }
            }
        }
        ImageCacheMode::Retry => {
            match s.cache.entry(key) {
                IdEntry::Occupied(mut e) => {
                    let var = e.get();
                    if var.with(ImageEntry::is_error) {
                        // already cached with error

                        // bind old entry to new, in case there are listeners to it,
                        // can't use `strong_count` to optimize here because it might have weak refs out there
                        r.set_bind(var).perm();
                        var.hold(r.clone()).perm();

                        // new var `r` becomes the entry
                        e.insert(r.clone());
                    } else {
                        // already cached ok
                        var.set_bind(&r).perm();
                        r.hold(var.clone()).perm();
                        return;
                    }
                }
                IdEntry::Vacant(e) => {
                    // cache
                    e.insert(r.clone());
                }
            }
        }
        ImageCacheMode::Reload => {
            match s.cache.entry(key) {
                IdEntry::Occupied(mut e) => {
                    let var = e.get();
                    r.set_bind(var).perm();
                    var.hold(r.clone()).perm();

                    e.insert(r.clone());
                }
                IdEntry::Vacant(e) => {
                    // cache
                    e.insert(r.clone());
                }
            }
        }
    }
    drop(s);

    match source {
        ImageSource::Read(path) => {
            fn read(path: PathBuf, limit: ByteLength) -> std::io::Result<IpcBytes> {
                let file = std::fs::File::open(path)?;
                if file.metadata()?.len() > limit.bytes() as u64 {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "file length exceeds limit"));
                }
                IpcBytes::from_file_blocking(file)
            }
            let limit = limits.max_encoded_len;
            let data_format = match path.extension() {
                Some(ext) => ImageDataFormat::FileExtension(ext.to_string_lossy().to_txt()),
                None => ImageDataFormat::Unknown,
            };
            zng_task::spawn_wait(move || match read(path, limit) {
                Ok(data) => image_data(false, Some(key), data_format, data, options, limits, r),
                Err(e) => {
                    r.set(ImageEntry::new_error(e.to_txt()));
                }
            });
        }
        #[cfg(feature = "http")]
        ImageSource::Download(uri, accept) => {
            let accept = accept.unwrap_or_else(|| IMAGES.http_accept());

            use zng_task::http::*;
            async fn download(uri: Uri, accept: Txt, limit: ByteLength) -> Result<(ImageDataFormat, IpcBytes), Error> {
                let request = Request::get(uri)?.max_length(limit).header(header::ACCEPT, accept.as_str())?;
                let mut response = send(request).await?;
                let data_format = match response.header().get(&header::CONTENT_TYPE).and_then(|m| m.to_str().ok()) {
                    Some(m) => ImageDataFormat::MimeType(m.to_txt()),
                    None => ImageDataFormat::Unknown,
                };
                let data = response.body().await?;

                Ok((data_format, data))
            }

            let limit = limits.max_encoded_len;
            zng_task::spawn(async move {
                match download(uri, accept, limit).await {
                    Ok((fmt, data)) => {
                        image_data(false, Some(key), fmt, data, options, limits, r);
                    }
                    Err(e) => r.set(ImageEntry::new_error(e.to_txt())),
                }
            });
        }
        ImageSource::Data(_, data, format) => image_data(false, Some(key), format, data, options, limits, r),
        ImageSource::Render(render_fn, args) => image_render(Some(key), render_fn, args, options, r),
        _ => unreachable!(),
    }
}

// source data acquired, setup view-process handle
fn image_data(
    is_respawn: bool,
    cache_key: Option<ImageHash>,
    format: ImageDataFormat,
    data: IpcBytes,
    options: ImageOptions,
    limits: ImageLimits,
    r: Var<ImageEntry>,
) {
    if !is_respawn && let Some(key) = cache_key {
        let mut replaced = false;
        let mut exts = mem::take(&mut IMAGES_SV.write().extensions);
        for ext in &mut exts {
            if let Some(replacement) = ext.image_data(limits.max_decoded_len, &key, &data, &format, &options) {
                replacement.set_bind(&r).perm();
                r.hold(replacement).perm();

                replaced = true;
                break;
            }
        }

        {
            let mut s = IMAGES_SV.write();
            exts.append(&mut s.extensions);
            s.extensions = exts;

            if replaced {
                return;
            }
        }
    }

    if !VIEW_PROCESS.is_available() {
        tracing::debug!("ignoring image view request after test load due to headless mode");
        return;
    }

    let mut request = ImageRequest::new(
        format.clone(),
        data.clone(),
        limits.max_decoded_len.bytes() as u64,
        options.downscale.clone(),
        options.mask,
    );
    request.entries = options.entries;

    let try_gen = VIEW_PROCESS.generation();

    match VIEW_PROCESS.add_image(request) {
        Ok(view_img) => image_view(
            cache_key,
            view_img,
            ImageDecoded::default(),
            Some((format, data, options, limits)),
            r,
        ),
        Err(_) => {
            tracing::debug!("image view request failed, will retry on respawn");

            zng_task::spawn(async move {
                VIEW_PROCESS_INITED_EVENT.wait_match(move |a| a.generation != try_gen).await;
                image_data(true, cache_key, format, data, options, limits, r);
            });
        }
    }
}
// monitor view-process handle until it is loaded
fn image_view(
    cache_key: Option<ImageHash>,
    handle: ViewImageHandle,
    decoded: ImageDecoded,
    respawn_data: Option<(ImageDataFormat, IpcBytes, ImageOptions, ImageLimits)>,
    r: Var<ImageEntry>,
) {
    let img = ImageEntry::new(cache_key, handle, decoded);
    let is_loaded = img.is_loaded();
    let is_dummy = img.view_handle().is_dummy();
    r.set(img);

    if is_loaded {
        image_decoded(r);
        return;
    }

    if is_dummy {
        tracing::error!("tried to register dummy handle");
        return;
    }

    // handle respawn during image decode
    let decoding_respawn_handle = if respawn_data.is_some() {
        let r_weak = r.downgrade();
        let mut respawn_data = respawn_data;
        VIEW_PROCESS_INITED_EVENT.hook(move |_| {
            if let Some(r) = r_weak.upgrade() {
                let (format, data, options, limits) = respawn_data.take().unwrap();
                image_data(true, cache_key, format, data, options, limits, r);
            }
            false
        })
    } else {
        // image registered (without source info), respawn is the responsibility of the caller
        VarHandle::dummy()
    };

    // handle decode error
    let r_weak = r.downgrade();
    let decode_error_handle = RAW_IMAGE_DECODE_ERROR_EVENT.hook(move |args| match r_weak.upgrade() {
        Some(r) => {
            if r.with(|img| img.view_handle() == args.handle) {
                r.set(ImageEntry::new_error(args.error.clone()));
                false
            } else {
                r.with(ImageEntry::is_loading)
            }
        }
        None => false,
    });

    // handle metadata decoded
    let r_weak = r.downgrade();
    let decode_meta_handle = RAW_IMAGE_METADATA_DECODED_EVENT.hook(move |args| match r_weak.upgrade() {
        Some(r) => {
            if r.with(|img| img.view_handle() == args.handle) {
                let meta = args.meta.clone();
                r.modify(move |i| i.data.meta = meta);
            } else if let Some(p) = &args.meta.parent
                && p.parent == r.with(|img| img.view_handle().image_id())
            {
                // discovered an entry for this image, start tracking it
                let mut entry_d = ImageDecoded::default();
                entry_d.meta = args.meta.clone();
                let entry = var(ImageEntry::new(None, args.handle.clone(), entry_d.clone()));
                r.modify(clmv!(entry, |i| i.insert_entry(entry)));
                image_view(None, args.handle.clone(), entry_d, None, entry);
            }
            r.with(ImageEntry::is_loading)
        }
        None => false,
    });

    // handle pixels decoded
    let r_weak = r.downgrade();
    RAW_IMAGE_DECODED_EVENT
        .hook(move |args| {
            let _hold = [&decoding_respawn_handle, &decode_error_handle, &decode_meta_handle];
            match r_weak.upgrade() {
                Some(r) => {
                    if r.with(|img| img.view_handle() == args.handle) {
                        let data = args.image.clone();
                        let is_loading = data.partial.is_some();
                        r.modify(move |i| i.data = data);
                        if !is_loading {
                            image_decoded(r);
                        }
                        is_loading
                    } else {
                        r.with(ImageEntry::is_loading)
                    }
                }
                None => false,
            }
        })
        .perm();
}
// image decoded ok, setup respawn handle
fn image_decoded(r: Var<ImageEntry>) {
    let r_weak = r.downgrade();
    VIEW_PROCESS_INITED_EVENT
        .hook(move |_| {
            if let Some(r) = r_weak.upgrade() {
                let img = r.get();
                if !img.is_loaded() {
                    // image rebound, maybe due to cache refresh
                    return false;
                }

                // respawn the image as decoded data
                let size = img.size();
                let mut options = ImageOptions::cache();
                let format = match img.is_mask() {
                    true => {
                        options.mask = Some(ImageMaskMode::A);
                        ImageDataFormat::A8 { size }
                    }
                    false => ImageDataFormat::Bgra8 {
                        size,
                        density: img.density(),
                        original_color_type: img.original_color_type(),
                    },
                };
                image_data(
                    true,
                    img.cache_key,
                    format,
                    img.data.pixels.clone(),
                    options,
                    ImageLimits::none(),
                    r,
                );
            }
            false
        })
        .perm();
}

// image render request, respawn errors during rendering are handled by the WINDOWS service
fn image_render(
    cache_key: Option<ImageHash>,
    render_fn: crate::RenderFn,
    args: Option<ImageRenderArgs>,
    options: ImageOptions,
    r: Var<ImageEntry>,
) {
    let s = IMAGES_SV.read();
    let windows = s.render_windows();
    let windows_ctx = windows.clone_boxed();
    let mask = options.mask;
    windows.open_headless_window(Box::new(move || {
        let ctx = ImageRenderCtx::new();
        let retain = ctx.retain.clone();
        WINDOW.set_state(*IMAGE_RENDER_ID, ctx);
        let w = render_fn(&args.unwrap_or_default());
        windows_ctx.enable_frame_capture_in_window_context(mask);
        image_render_open(cache_key, WINDOW.id(), retain, r);
        w
    }));
}

fn image_render_open(cache_key: Option<ImageHash>, win_id: WindowId, retain: Var<bool>, r: Var<ImageEntry>) {
    // handle window open error
    let r_weak = r.downgrade();
    let error_handle = RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.hook(move |args| {
        if args.window_id == win_id {
            if let Some(r) = r_weak.upgrade() {
                r.set(ImageEntry::new_error(args.error.clone()));
            }
            false
        } else {
            true
        }
    });
    // hold error handle until open ok
    RAW_HEADLESS_OPEN_EVENT
        .hook(move |args| {
            let _hold = &error_handle;
            args.window_id != win_id
        })
        .perm();

    // handle frame(s)
    let r_weak = r.downgrade();
    RAW_FRAME_RENDERED_EVENT
        .hook(move |args| {
            if args.window_id == win_id {
                if let Some(r) = r_weak.upgrade() {
                    match args.frame_image.clone() {
                        Some((handle, data)) => {
                            let retain = retain.get();
                            r.set(ImageEntry::new(cache_key, handle, data));
                            if !retain {
                                IMAGES_SV.read().render_windows().close_window(win_id);
                                // image_decoded setup a normal respawn recovery for the image
                                image_decoded(r);
                            }
                            // else if it is retained on respawn the window will render again
                            retain
                        }
                        None => {
                            r.set(ImageEntry::new_error("image render window did not capture a frame".to_txt()));
                            false
                        }
                    }
                } else {
                    false
                }
            } else {
                true
            }
        })
        .perm();
}

impl IMAGES {
    /// Render the *window* generated by `render` to an image.
    ///
    /// The *window* is created as a headless surface and rendered to the returned image. You can set the
    /// [`IMAGE_RENDER.retain`] var inside `render` to create an image that updates with new frames. By default it will only render once.
    ///
    /// The closure runs in the [`WINDOW`] context of the headless window.
    ///
    /// This is shorthand for calling [`IMAGES.image`] with [`ImageSource::render`] and [`ImageOptions::none`].
    ///
    /// [`IMAGE_RENDER.retain`]: IMAGE_RENDER::retain
    /// [`WINDOW`]: zng_app::window::WINDOW
    /// [`IMAGES.image`]: IMAGES::image
    pub fn render<N, R>(&self, mask: Option<ImageMaskMode>, render: N) -> ImageVar
    where
        N: FnOnce() -> R + Send + Sync + 'static,
        R: ImageRenderWindowRoot,
    {
        let render = Mutex::new(Some(render));
        let source = ImageSource::render(move |_| render.lock().take().expect("IMAGES.render closure called more than once")());
        let options = ImageOptions::new(ImageCacheMode::Ignore, None, mask, ImageEntriesMode::empty());
        self.image_impl(source, options, None)
    }

    /// Render an [`UiNode`] to an image.
    ///
    /// This method is a shortcut to [`render`] a node without needing to declare the headless window, note that
    /// a headless window is still used, the node does not have the same context as the calling widget.
    ///
    /// This is shorthand for calling [`IMAGES.image`] with [`ImageSource::render_node`] and [`ImageOptions::none`].
    ///
    /// [`render`]: Self::render
    /// [`UiNode`]: zng_app::widget::node::UiNode
    /// [`IMAGES.image`]: IMAGES::image
    pub fn render_node(
        &self,
        render_mode: RenderMode,
        mask: Option<ImageMaskMode>,
        render: impl FnOnce() -> UiNode + Send + Sync + 'static,
    ) -> ImageVar {
        let render = Mutex::new(Some(render));
        let source = ImageSource::render_node(render_mode, move |_| {
            render.lock().take().expect("IMAGES.render closure called more than once")()
        });
        let options = ImageOptions::new(ImageCacheMode::Ignore, None, mask, ImageEntriesMode::empty());
        self.image_impl(source, options, None)
    }
}

/// Images render window hook.
#[expect(non_camel_case_types)]
pub struct IMAGES_WINDOW;
impl IMAGES_WINDOW {
    /// Sets the windows service used to manage the headless windows used to render images.
    ///
    /// This must be called by the windows implementation only.
    pub fn hook_render_windows_service(&self, service: Box<dyn ImageRenderWindowsService>) {
        let mut img = IMAGES_SV.write();
        img.render_windows = Some(service);
    }
}

/// Reference to a windows manager service that [`IMAGES`] can use to render images.
///
/// This service must be implemented by the window implementer, the `WINDOWS` service implements it.
pub trait ImageRenderWindowsService: Send + Sync + 'static {
    /// Clone the service reference.
    fn clone_boxed(&self) -> Box<dyn ImageRenderWindowsService>;

    /// Create a window root that presents the node.
    ///
    /// This is to produce a window wrapper for [`ImageSource::render_node`].
    fn new_window_root(&self, node: UiNode, render_mode: RenderMode) -> Box<dyn ImageRenderWindowRoot>;

    /// Set parent window for the headless render window.
    ///
    /// Called inside the [`WINDOW`] context for the new window.
    fn set_parent_in_window_context(&self, parent_id: WindowId);

    /// Enable frame capture for the window.
    ///
    /// If `mask` is set captures only the given channel, if not set will capture the full BGRA image.
    ///
    /// Called inside the [`WINDOW`] context for the new window.
    fn enable_frame_capture_in_window_context(&self, mask: Option<ImageMaskMode>);

    /// Open the window.
    ///
    /// The `new_window_root` must be called inside the [`WINDOW`] context for the new window.
    fn open_headless_window(&self, new_window_root: Box<dyn FnOnce() -> Box<dyn ImageRenderWindowRoot> + Send>);

    /// Close the window, does nothing if the window is not found.
    fn close_window(&self, window_id: WindowId);
}

/// Implemented for the root window type.
///
/// This is implemented for the `WindowRoot` type.
pub trait ImageRenderWindowRoot: Send + Any + 'static {}

/// Controls properties of the render window used by [`IMAGES.render`].
///
/// [`IMAGES.render`]: IMAGES::render
#[expect(non_camel_case_types)]
pub struct IMAGE_RENDER;
impl IMAGE_RENDER {
    /// If the current context is an [`IMAGES.render`] closure, window or widget.
    ///
    /// [`IMAGES.render`]: IMAGES::render
    pub fn is_in_render(&self) -> bool {
        WINDOW.contains_state(*IMAGE_RENDER_ID)
    }

    /// If the render task is kept alive after a frame is produced, this is `false` by default
    /// meaning the image only renders once, if set to `true` the image will automatically update
    /// when the render widget requests a new frame.
    pub fn retain(&self) -> Var<bool> {
        WINDOW.req_state(*IMAGE_RENDER_ID).retain
    }
}

/// If the render task is kept alive after a frame is produced, this is `false` by default
/// meaning the image only renders once, if set to `true` the image will automatically update
/// when the render widget requests a new frame.
///
/// This property sets and binds `retain` to [`IMAGE_RENDER.retain`].
///
/// [`IMAGE_RENDER.retain`]: IMAGE_RENDER::retain
#[zng_app::widget::property(CONTEXT, default(false))]
pub fn render_retain(child: impl IntoUiNode, retain: impl IntoVar<bool>) -> UiNode {
    let retain = retain.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            if IMAGE_RENDER.is_in_render() {
                let actual_retain = IMAGE_RENDER.retain();
                actual_retain.set_from(&retain);
                let handle = actual_retain.bind(&retain);
                WIDGET.push_var_handle(handle);
            } else {
                tracing::error!("can only set `render_retain` in render widgets")
            }
        }
    })
}

#[derive(Clone)]
struct ImageRenderCtx {
    retain: Var<bool>,
}
impl ImageRenderCtx {
    fn new() -> Self {
        Self { retain: var(false) }
    }
}

static_id! {
    static ref IMAGE_RENDER_ID: StateId<ImageRenderCtx>;
}
