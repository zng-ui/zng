#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Image loading and cache.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    env, mem,
    path::{Path, PathBuf},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
};

use parking_lot::Mutex;
use task::io::AsyncReadExt;
use zng_app::{
    APP, AppExtension,
    update::EventUpdate,
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewImage,
        raw_events::{
            LOW_MEMORY_EVENT, RAW_IMAGE_LOAD_ERROR_EVENT, RAW_IMAGE_LOADED_EVENT, RAW_IMAGE_METADATA_LOADED_EVENT,
            RAW_IMAGE_PARTIALLY_LOADED_EVENT,
        },
    },
    widget::UiTaskWidget,
};
use zng_app_context::app_local;
use zng_clone_move::{async_clmv, clmv};
use zng_task as task;

mod types;
pub use types::*;

mod render;
#[doc(inline)]
pub use render::{IMAGE_RENDER, IMAGES_WINDOW, ImageRenderWindowRoot, ImageRenderWindowsService, render_retain};
use zng_layout::unit::{ByteLength, ByteUnits};
use zng_task::{UiTask, channel::IpcBytes};
use zng_txt::{ToTxt, Txt, formatx};
use zng_unique_id::{IdEntry, IdMap};
use zng_var::{Var, WeakVar, var};
use zng_view_api::image::{ImageEntryMetadata, ImageRequest};

/// Application extension that provides an image cache.
///
/// # Services
///
/// Services this extension provides.
///
/// * [`IMAGES`]
#[derive(Default)]
#[non_exhaustive]
pub struct ImageManager {}
impl AppExtension for ImageManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_IMAGE_METADATA_LOADED_EVENT
            .on(update)
            .or_else(|| RAW_IMAGE_PARTIALLY_LOADED_EVENT.on(update))
            .or_else(|| RAW_IMAGE_LOADED_EVENT.on(update))
            .or_else(|| RAW_IMAGE_LOAD_ERROR_EVENT.on(update))
        {
            let images = IMAGES_SV.read();

            let mut found_key = None;

            if let Some(id) = args.image.id()
                && let Some(var) = images.find_decoding(id)
            {
                // ViewImage updates with internal mutation, notify this
                var.modify(|i| i.update());
                found_key = var.with(|i| i.cache_key);
            } else if let Some(parent) = args.image.entry_parent()
                && let Some(parent) = images.find_decoding(parent)
            {
                // entry image first update, generate var for it and siblings in parent image
                let image = args.image.clone();
                parent.modify(|p| {
                    p.register_entry(image);
                });
            }

            if args.image.is_loaded() || args.image.is_error() {
                // finished loading, cleanup 'decoding' strong references
                drop(images);
                let mut images = IMAGES_SV.write();
                images
                    .decoding
                    .retain(|i| i.image.with(|i| i.is_loading() || i.has_loading_entries()));

                if let Some(key) = found_key
                    && args.image.is_error()
                {
                    if let Some(e) = images.cache.get(&key) {
                        e.error.store(true, Ordering::Relaxed);
                    }
                    tracing::error!("decode error: {:?}", args.image.error().unwrap());
                }
            }
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            let mut images = IMAGES_SV.write();
            let images = &mut *images;
            images.cleanup_not_cached(true);
            images.download_accept.clear();

            let mut decoding_interrupted = mem::take(&mut images.decoding);
            for (img_var, max_decoded_len, downscale, mask, entries) in images
                .cache
                .values()
                .map(|e| (e.image.clone(), e.max_decoded_len, &e.downscale, e.mask, e.entries))
                .chain(
                    images
                        .not_cached
                        .iter()
                        .filter_map(|e| e.image.upgrade().map(|v| (v, e.max_decoded_len, &e.downscale, e.mask, e.entries))),
                )
            {
                let img = img_var.get();

                if let Some(view) = img.view.get() {
                    if view.generation() == args.generation {
                        continue; // already recovered, can this happen?
                    }
                    if let Some(e) = view.error() {
                        // respawned, but image was an error.
                        img_var.set(Img::dummy(Some(e.to_owned())));
                    } else if let Some(task_i) = decoding_interrupted
                        .iter()
                        .position(|e| e.image.with(|img| img.view() == Some(view)))
                    {
                        let task = decoding_interrupted.swap_remove(task_i);
                        // respawned, but image was decoding, need to restart decode.
                        let mut request = ImageRequest::new(
                            task.format.clone(),
                            task.data.clone(),
                            max_decoded_len.0 as u64,
                            downscale.clone(),
                            mask,
                        );
                        request.entries = entries;
                        match VIEW_PROCESS.add_image(request) {
                            Ok(img) => {
                                img_var.set(Img::new(img));
                            }
                            Err(_) => { /*will receive another event.*/ }
                        }
                        images.decoding.push(ImageDecodingTask {
                            format: task.format.clone(),
                            data: task.data.clone(),
                            image: img_var,
                        });
                    } else {
                        // respawned and image was loaded.

                        let img_format = if view.is_mask() {
                            ImageDataFormat::A8 { size: view.size() }
                        } else {
                            ImageDataFormat::Bgra8 {
                                size: view.size(),
                                density: view.density(),
                                original_color_type: view.original_color_type(),
                            }
                        };

                        let entries = img.entries();

                        let data = view.pixels().unwrap();
                        let request = ImageRequest::new(img_format.clone(), data.clone(), max_decoded_len.0 as u64, None, mask);
                        let img = match VIEW_PROCESS.add_image(request) {
                            Ok(img) => img,
                            Err(_) => return, // we will receive another event.
                        };
                        let mut img = Img::new(img);

                        fn add_entries(max_decoded_len: ByteLength, mask: Option<ImageMaskMode>, entries: Vec<ImageVar>, img: &mut Img) {
                            for (i, entry) in entries.into_iter().enumerate() {
                                let entry = entry.get();
                                if let Some(view) = entry.view() {
                                    if entry.is_loaded() {
                                        let img_format = if view.is_mask() {
                                            ImageDataFormat::A8 { size: view.size() }
                                        } else {
                                            ImageDataFormat::Bgra8 {
                                                size: view.size(),
                                                density: view.density(),
                                                original_color_type: view.original_color_type(),
                                            }
                                        };
                                        let data = view.pixels().unwrap();
                                        let mut request =
                                            ImageRequest::new(img_format.clone(), data.clone(), max_decoded_len.0 as u64, None, mask);
                                        request.parent =
                                            Some(ImageEntryMetadata::new(img.view().unwrap().id().unwrap(), i, entry.entry_kind()));
                                        let entry_img = match VIEW_PROCESS.add_image(request) {
                                            Ok(img) => img,
                                            Err(_) => return, // we will receive another event.
                                        };
                                        let entry_img = img.register_entry(entry_img);

                                        add_entries(max_decoded_len, mask, entry.entries(), &mut entry_img.get());
                                        continue;
                                    } else if entry.is_error() {
                                        img.register_entry(view.clone());
                                        continue;
                                    }
                                }
                                tracing::warn!("respawn not implemented for multi entry image partially decoded on crash");
                            }
                        }
                        add_entries(max_decoded_len, mask, entries, &mut img);

                        img_var.set(img);

                        images.decoding.push(ImageDecodingTask {
                            format: img_format,
                            data,
                            image: img_var,
                        });
                    }
                } else if let Some(task_i) = decoding_interrupted.iter().position(|e| e.image.var_eq(&img_var)) {
                    // respawned, but image had not started decoding, start it now.
                    let task = decoding_interrupted.swap_remove(task_i);
                    let mut request = ImageRequest::new(
                        task.format.clone(),
                        task.data.clone(),
                        max_decoded_len.0 as u64,
                        downscale.clone(),
                        mask,
                    );
                    request.entries = entries;
                    match VIEW_PROCESS.add_image(request) {
                        Ok(img) => {
                            img_var.set(Img::new(img));
                        }
                        Err(_) => { /*will receive another event.*/ }
                    }
                    images.decoding.push(ImageDecodingTask {
                        format: task.format.clone(),
                        data: task.data.clone(),
                        image: img_var,
                    });
                }
                // else { *is loading, will continue normally in self.update_preview()* }
            }
        } else if LOW_MEMORY_EVENT.on(update).is_some() {
            IMAGES.clean_all();
        } else {
            self.event_preview_render(update);
        }
    }

    fn update_preview(&mut self) {
        // update loading tasks:

        let mut images = IMAGES_SV.write();
        let mut loading = Vec::with_capacity(images.loading.len());
        let loading_tasks = mem::take(&mut images.loading);
        let mut proxies = mem::take(&mut images.proxies);
        drop(images); // proxies can use IMAGES

        'loading_tasks: for t in loading_tasks {
            t.task.lock().update();
            match t.task.into_inner().into_result() {
                Ok(d) => {
                    match d.r {
                        Ok(data) => {
                            if let Some((key, mode)) = &t.is_data_proxy_source {
                                for proxy in &mut proxies {
                                    if proxy.is_data_proxy()
                                        && let Some(replaced) =
                                            proxy.data(key, &data, &d.format, *mode, t.downscale.as_ref(), t.mask, t.entries, true)
                                    {
                                        replaced.set_bind(&t.image).perm();
                                        t.image.hold(replaced).perm();
                                        continue 'loading_tasks;
                                    }
                                }
                            }

                            if VIEW_PROCESS.is_available() {
                                // success and we have a view-process.
                                let mut request = ImageRequest::new(
                                    d.format.clone(),
                                    data.clone(),
                                    t.max_decoded_len.0 as u64,
                                    t.downscale.clone(),
                                    t.mask,
                                );
                                request.entries = t.entries;
                                match VIEW_PROCESS.add_image(request) {
                                    Ok(img) => {
                                        // request sent, add to `decoding` will receive
                                        // image decoded events
                                        t.image.modify(move |v| {
                                            v.inner_set_or_replace(img, false);
                                        });
                                    }
                                    Err(_) => {
                                        // will recover in VIEW_PROCESS_INITED_EVENT
                                    }
                                }
                                IMAGES_SV.write().decoding.push(ImageDecodingTask {
                                    format: d.format,
                                    data,
                                    image: t.image,
                                });
                            } else {
                                // success, but we are only doing `load_in_headless` validation.
                                let img = ViewImage::dummy(None);
                                t.image.modify(move |v| {
                                    v.inner_set_or_replace(img, true);
                                });
                            }
                        }
                        Err(e) => {
                            tracing::error!("load error: {e:?}");
                            // load error.
                            let img = ViewImage::dummy(Some(e));
                            t.image.modify(move |v| {
                                v.inner_set_or_replace(img, true);
                            });

                            // flag error for user retry
                            if let Some(k) = &t.image.with(|img| img.cache_key)
                                && let Some(e) = IMAGES_SV.read().cache.get(k)
                            {
                                e.error.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }
                Err(task) => {
                    loading.push(ImageLoadingTask {
                        task: Mutex::new(task),
                        image: t.image,
                        max_decoded_len: t.max_decoded_len,
                        downscale: t.downscale,
                        mask: t.mask,
                        entries: t.entries,
                        is_data_proxy_source: t.is_data_proxy_source,
                    });
                }
            }
        }
        let mut images = IMAGES_SV.write();
        images.loading = loading;
        images.proxies = proxies;
    }

    fn update(&mut self) {
        self.update_render();
    }
}

app_local! {
    static IMAGES_SV: ImagesService = {
        APP.extensions().require::<ImageManager>();
        ImagesService::new()
    };
}

struct ImageLoadingTask {
    task: Mutex<UiTask<ImageData>>,
    image: Var<Img>,
    max_decoded_len: ByteLength,
    downscale: Option<ImageDownscaleMode>,
    mask: Option<ImageMaskMode>,
    entries: ImageEntriesMode,
    is_data_proxy_source: Option<(ImageHash, ImageCacheMode)>,
}

struct ImageDecodingTask {
    format: ImageDataFormat,
    data: IpcBytes,
    image: Var<Img>,
}

struct CacheEntry {
    image: Var<Img>,
    error: AtomicBool,
    max_decoded_len: ByteLength,
    downscale: Option<ImageDownscaleMode>,
    mask: Option<ImageMaskMode>,
    entries: ImageEntriesMode,
}

struct NotCachedEntry {
    image: WeakVar<Img>,
    max_decoded_len: ByteLength,
    downscale: Option<ImageDownscaleMode>,
    mask: Option<ImageMaskMode>,
    entries: ImageEntriesMode,
}

struct ImagesService {
    load_in_headless: Var<bool>,
    limits: Var<ImageLimits>,

    download_accept: Txt,
    proxies: Vec<Box<dyn ImageCacheProxy>>,

    loading: Vec<ImageLoadingTask>,
    decoding: Vec<ImageDecodingTask>,
    cache: IdMap<ImageHash, CacheEntry>,
    not_cached: Vec<NotCachedEntry>,

    render: render::ImagesRender,
}
impl ImagesService {
    fn new() -> Self {
        Self {
            load_in_headless: var(false),
            limits: var(ImageLimits::default()),
            proxies: vec![],
            loading: vec![],
            decoding: vec![],
            download_accept: Txt::from_static(""),
            cache: IdMap::new(),
            not_cached: vec![],
            render: render::ImagesRender::default(),
        }
    }

    fn register(&mut self, key: ImageHash, image: ViewImage) -> std::result::Result<ImageVar, (ViewImage, ImageVar)> {
        let limits = self.limits.get();
        let limits = ImageLimits {
            max_encoded_len: limits.max_encoded_len,
            max_decoded_len: limits.max_decoded_len.max(image.pixels().map(|b| b.len()).unwrap_or(0).bytes()),
            allow_path: PathFilter::BlockAll,
            #[cfg(feature = "http")]
            allow_uri: UriFilter::BlockAll,
        };

        match self.cache.entry(key) {
            IdEntry::Occupied(e) => Err((image, e.get().image.read_only())),
            IdEntry::Vacant(e) => {
                let is_error = image.is_error();
                let is_loading = !is_error && !image.is_loaded();
                let is_mask = image.is_mask();
                let format = if is_mask {
                    ImageDataFormat::A8 { size: image.size() }
                } else {
                    ImageDataFormat::Bgra8 {
                        size: image.size(),
                        density: image.density(),
                        original_color_type: image.original_color_type(),
                    }
                };
                let img_var = var(Img::new(image));
                if is_loading {
                    self.decoding.push(ImageDecodingTask {
                        format,
                        data: IpcBytes::default(),
                        image: img_var.clone(),
                    });
                }

                Ok(e.insert(CacheEntry {
                    error: AtomicBool::new(is_error),
                    image: img_var,
                    max_decoded_len: limits.max_decoded_len,
                    downscale: None,
                    mask: if is_mask { Some(ImageMaskMode::A) } else { None },
                    entries: ImageEntriesMode::PRIMARY,
                })
                .image
                .read_only())
            }
        }
    }

    fn detach(&mut self, image: ImageVar) -> ImageVar {
        if let Some(key) = &image.with(|i| i.cache_key) {
            let decoded_size = image.with(|img| img.pixels().map(|b| b.len()).unwrap_or(0).bytes());
            let mut max_decoded_len = self.limits.with(|l| l.max_decoded_len.max(decoded_size));
            let mut downscale = None;
            let mut mask = None;
            let mut entries = ImageEntriesMode::PRIMARY;

            if let Some(e) = self.cache.get(key) {
                max_decoded_len = e.max_decoded_len;
                downscale = e.downscale.clone();
                mask = e.mask;
                entries = e.entries;

                // is cached, `clean` if is only external reference.
                if image.strong_count() == 2 {
                    self.cache.remove(key);
                }
            }

            // remove `cache_key` from image, this clones the `Img` only-if is still in cache.
            let mut img = image.get();
            img.cache_key = None;
            let img = var(img);
            self.not_cached.push(NotCachedEntry {
                image: img.downgrade(),
                max_decoded_len,
                downscale,
                mask,
                entries,
            });
            img.read_only()
        } else {
            // already not cached
            image
        }
    }

    fn proxy_then_remove(mut proxies: Vec<Box<dyn ImageCacheProxy>>, key: &ImageHash, purge: bool) -> bool {
        for proxy in &mut proxies {
            let r = proxy.remove(key, purge);
            match r {
                ProxyRemoveResult::None => continue,
                ProxyRemoveResult::Remove(r, p) => return IMAGES_SV.write().proxied_remove(proxies, &r, p),
                ProxyRemoveResult::Removed => {
                    IMAGES_SV.write().proxies.append(&mut proxies);
                    return true;
                }
            }
        }
        IMAGES_SV.write().proxied_remove(proxies, key, purge)
    }
    fn proxied_remove(&mut self, mut proxies: Vec<Box<dyn ImageCacheProxy>>, key: &ImageHash, purge: bool) -> bool {
        self.proxies.append(&mut proxies);
        if purge || self.cache.get(key).map(|v| v.image.strong_count() > 1).unwrap_or(false) {
            self.cache.remove(key).is_some()
        } else {
            false
        }
    }

    fn proxy_then_get(
        mut proxies: Vec<Box<dyn ImageCacheProxy>>,
        source: ImageSource,
        mode: ImageCacheMode,
        limits: ImageLimits,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageVar {
        let source = match source {
            ImageSource::Read(path) => {
                let path = crate::absolute_path(&path, || env::current_dir().expect("could not access current dir"), true);
                if !limits.allow_path.allows(&path) {
                    let error = formatx!("limits filter blocked `{}`", path.display());
                    tracing::error!("{error}");
                    IMAGES_SV.write().proxies.append(&mut proxies);
                    return var(Img::dummy(Some(error))).read_only();
                }
                ImageSource::Read(path)
            }
            #[cfg(feature = "http")]
            ImageSource::Download(uri, accepts) => {
                if !limits.allow_uri.allows(&uri) {
                    let error = formatx!("limits filter blocked `{uri}`");
                    tracing::error!("{error}");
                    IMAGES_SV.write().proxies.append(&mut proxies);
                    return var(Img::dummy(Some(error))).read_only();
                }
                ImageSource::Download(uri, accepts)
            }
            ImageSource::Image(r) => {
                IMAGES_SV.write().proxies.append(&mut proxies);
                return r;
            }
            source => source,
        };

        let key = source.hash128(downscale.as_ref(), mask, entries).unwrap();
        for proxy in &mut proxies {
            if proxy.is_data_proxy() && !matches!(source, ImageSource::Data(_, _, _)) {
                continue;
            }

            let r = proxy.get(&key, &source, mode, downscale.as_ref(), mask, entries);
            match r {
                ProxyGetResult::None => continue,
                ProxyGetResult::Cache(source, mode, downscale, mask, entries) => {
                    return IMAGES_SV.write().proxied_get(
                        proxies,
                        source.hash128(downscale.as_ref(), mask, entries).unwrap(),
                        source,
                        mode,
                        limits,
                        downscale,
                        mask,
                        entries,
                    );
                }
                ProxyGetResult::Image(img) => {
                    IMAGES_SV.write().proxies.append(&mut proxies);
                    return img;
                }
            }
        }
        IMAGES_SV
            .write()
            .proxied_get(proxies, key, source, mode, limits, downscale, mask, entries)
    }
    #[allow(clippy::too_many_arguments)]
    fn proxied_get(
        &mut self,
        mut proxies: Vec<Box<dyn ImageCacheProxy>>,
        key: ImageHash,
        source: ImageSource,
        mode: ImageCacheMode,
        limits: ImageLimits,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageVar {
        self.proxies.append(&mut proxies);
        match mode {
            ImageCacheMode::Cache => {
                if let Some(v) = self.cache.get(&key) {
                    return v.image.read_only();
                }
            }
            ImageCacheMode::Retry => {
                if let Some(e) = self.cache.get(&key)
                    && !e.error.load(Ordering::Relaxed)
                {
                    return e.image.read_only();
                }
            }
            ImageCacheMode::Ignore | ImageCacheMode::Reload => {}
        }

        if !VIEW_PROCESS.is_available() && !self.load_in_headless.get() {
            tracing::warn!("loading dummy image, set `load_in_headless=true` to actually load without renderer");

            let dummy = var(Img::new(ViewImage::dummy(None)));
            self.cache.insert(
                key,
                CacheEntry {
                    image: dummy.clone(),
                    error: AtomicBool::new(false),
                    max_decoded_len: limits.max_decoded_len,
                    downscale,
                    mask,
                    entries,
                },
            );
            return dummy.read_only();
        }

        let max_encoded_size = limits.max_encoded_len;

        match source {
            ImageSource::Read(path) => self.load_task(
                key,
                mode,
                limits.max_decoded_len,
                downscale,
                mask,
                entries,
                true,
                task::run(async move {
                    let mut r = ImageData {
                        format: path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|s| ImageDataFormat::FileExtension(Txt::from_str(s)))
                            .unwrap_or(ImageDataFormat::Unknown),
                        r: Err(Txt::from_static("")),
                    };

                    let mut file = match task::fs::File::open(path).await {
                        Ok(f) => f,
                        Err(e) => {
                            r.r = Err(e.to_txt());
                            return r;
                        }
                    };

                    let len = match file.metadata().await {
                        Ok(m) => m.len() as usize,
                        Err(e) => {
                            r.r = Err(e.to_txt());
                            return r;
                        }
                    };

                    if len > max_encoded_size.0 {
                        r.r = Err(formatx!("file size `{}` exceeds the limit of `{max_encoded_size}`", len.bytes()));
                        return r;
                    }

                    let mut data = Vec::with_capacity(len);
                    r.r = match file.read_to_end(&mut data).await {
                        Ok(_) => match IpcBytes::from_vec_blocking(data) {
                            Ok(r) => Ok(r),
                            Err(e) => Err(e.to_txt()),
                        },
                        Err(e) => Err(e.to_txt()),
                    };

                    r
                }),
            ),
            #[cfg(feature = "http")]
            ImageSource::Download(uri, accept) => {
                let accept = accept.unwrap_or_else(|| self.download_accept());

                self.load_task(
                    key,
                    mode,
                    limits.max_decoded_len,
                    downscale,
                    mask,
                    entries,
                    true,
                    task::run(async move {
                        let mut r = ImageData {
                            format: ImageDataFormat::Unknown,
                            r: Err(Txt::from_static("")),
                        };

                        let request = task::http::Request::get(uri)
                            .unwrap()
                            .header(task::http::header::ACCEPT, accept.as_str())
                            .unwrap()
                            .max_length(max_encoded_size);

                        match task::http::send(request).await {
                            Ok(mut rsp) => {
                                if let Some(m) = rsp.header().get(&task::http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
                                    let m = m.to_lowercase();
                                    if m.starts_with("image/") {
                                        r.format = ImageDataFormat::MimeType(Txt::from_str(&m));
                                    }
                                }

                                r.r = rsp.body().await.map_err(|e| formatx!("download error: {e}"));
                            }
                            Err(e) => {
                                r.r = Err(formatx!("request error: {e}"));
                            }
                        }

                        r
                    }),
                )
            }
            ImageSource::Data(_, bytes, fmt) => {
                let r = ImageData { format: fmt, r: Ok(bytes) };
                self.load_task(key, mode, limits.max_decoded_len, downscale, mask, entries, false, async { r })
            }
            ImageSource::Render(rfn, args) => {
                let img = self.new_cache_image(key, mode, limits.max_decoded_len, downscale, mask, entries);
                self.render_img(mask, clmv!(rfn, || rfn(&args.unwrap_or_default())), &img);
                img.read_only()
            }
            ImageSource::Image(_) => unreachable!(),
        }
    }

    #[cfg(feature = "http")]
    fn download_accept(&mut self) -> Txt {
        if self.download_accept.is_empty() {
            if VIEW_PROCESS.is_available() {
                let mut r = String::new();
                let mut sep = "";
                for fmt in VIEW_PROCESS.info().image.iter() {
                    for t in fmt.media_type_suffixes_iter() {
                        r.push_str(sep);
                        r.push_str("image/");
                        r.push_str(t);
                        sep = ",";
                    }
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
            self.not_cached.retain(|c| c.image.strong_count() > 0);
        }
    }

    fn new_cache_image(
        &mut self,
        key: ImageHash,
        mode: ImageCacheMode,
        max_decoded_len: ByteLength,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> Var<Img> {
        self.cleanup_not_cached(false);

        if let ImageCacheMode::Reload = mode {
            self.cache
                .entry(key)
                .or_insert_with(|| CacheEntry {
                    image: var(Img::new_none(Some(key))),
                    error: AtomicBool::new(false),
                    max_decoded_len,
                    downscale,
                    mask,
                    entries,
                })
                .image
                .clone()
        } else if let ImageCacheMode::Ignore = mode {
            let img = var(Img::new_none(None));
            self.not_cached.push(NotCachedEntry {
                image: img.downgrade(),
                max_decoded_len,
                downscale,
                mask,
                entries,
            });
            img
        } else {
            let img = var(Img::new_none(Some(key)));
            self.cache.insert(
                key,
                CacheEntry {
                    image: img.clone(),
                    error: AtomicBool::new(false),
                    max_decoded_len,
                    downscale,
                    mask,
                    entries,
                },
            );
            img
        }
    }

    /// The `fetch_bytes` future is polled in the UI thread, use `task::run` for futures that poll a lot.
    #[allow(clippy::too_many_arguments)]
    fn load_task(
        &mut self,
        key: ImageHash,
        mode: ImageCacheMode,
        max_decoded_len: ByteLength,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
        is_data_proxy_source: bool,
        fetch_bytes: impl Future<Output = ImageData> + Send + 'static,
    ) -> ImageVar {
        let img = self.new_cache_image(key, mode, max_decoded_len, downscale.clone(), mask, entries);
        let r = img.read_only();

        self.loading.push(ImageLoadingTask {
            task: Mutex::new(UiTask::new(None, fetch_bytes)),
            image: img,
            max_decoded_len,
            downscale,
            mask,
            entries,
            is_data_proxy_source: if is_data_proxy_source { Some((key, mode)) } else { None },
        });
        zng_app::update::UPDATES.update(None);

        r
    }

    fn find_decoding(&self, id: zng_view_api::image::ImageId) -> Option<ImageVar> {
        self.decoding.iter().find_map(|i| {
            if i.image.with(|i| i.view().map(|v| v.id() == Some(id))).unwrap_or(false) {
                Some(i.image.clone())
            } else {
                i.image.with(|i| i.find_entry(id))
            }
        })
    }
}

/// Image loading, cache and render service.
///
/// If the app is running without a [`VIEW_PROCESS`] all images are dummy, see [`load_in_headless`] for
/// details.
///
/// # Provider
///
/// This service is provided by the [`ImageManager`] extension, it will panic if used in an app not extended.
///
/// [`load_in_headless`]: IMAGES::load_in_headless
/// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
pub struct IMAGES;
impl IMAGES {
    /// If should still download/read image bytes in headless/renderless mode.
    ///
    /// When an app is in headless mode without renderer no [`VIEW_PROCESS`] is available, so
    /// images cannot be decoded, in this case all images are the [`dummy`] image and no attempt
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

    /// Returns a dummy image that reports it is loaded or an error.
    pub fn dummy(&self, error: Option<Txt>) -> ImageVar {
        var(Img::dummy(error)).read_only()
    }

    /// Cache or load an image file from a file system `path`.
    pub fn read(&self, path: impl Into<PathBuf>) -> ImageVar {
        self.cache(path.into())
    }

    /// Get a cached `uri` or download it.
    ///
    /// Optionally define the HTTP ACCEPT header, if not set all image formats supported by the view-process
    /// backend are accepted.
    #[cfg(feature = "http")]
    pub fn download<U>(&self, uri: U, accept: Option<Txt>) -> ImageVar
    where
        U: TryInto<task::http::Uri>,
        <U as TryInto<task::http::Uri>>::Error: ToTxt,
    {
        match uri.try_into() {
            Ok(uri) => self.cache(ImageSource::Download(uri, accept)),
            Err(e) => self.dummy(Some(e.to_txt())),
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
    /// # use zng_ext_image::*;
    /// # macro_rules! include_bytes { ($tt:tt) => { &[] } }
    /// # fn demo() {
    /// let image_var = IMAGES.from_static(include_bytes!("ico.png"), "png");
    /// # }
    pub fn from_static(&self, data: &'static [u8], format: impl Into<ImageDataFormat>) -> ImageVar {
        self.cache((data, format.into()))
    }

    /// Get a cached image from shared data.
    ///
    /// The image key is a [`ImageHash`] of the image data. The data reference is held only until the image is decoded.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    pub fn from_data(&self, data: IpcBytes, format: impl Into<ImageDataFormat>) -> ImageVar {
        self.cache((data, format.into()))
    }

    /// Get a cached image or add it to the cache.
    pub fn cache(&self, source: impl Into<ImageSource>) -> ImageVar {
        self.image_impl(source.into(), ImageCacheMode::Cache, None, None, None, ImageEntriesMode::PRIMARY)
    }

    /// Get a cached image or add it to the cache or retry if the cached image is an error.
    pub fn retry(&self, source: impl Into<ImageSource>) -> ImageVar {
        self.image_impl(source.into(), ImageCacheMode::Retry, None, None, None, ImageEntriesMode::PRIMARY)
    }

    /// Load an image, if it was already cached update the cached image with the reloaded data.
    pub fn reload(&self, source: impl Into<ImageSource>) -> ImageVar {
        self.image_impl(source.into(), ImageCacheMode::Reload, None, None, None, ImageEntriesMode::PRIMARY)
    }

    /// Get or load an image.
    ///
    /// If `limits` is `None` the [`IMAGES.limits`] is used.
    ///
    /// [`IMAGES.limits`]: IMAGES::limits
    pub fn image(
        &self,
        source: impl Into<ImageSource>,
        cache_mode: impl Into<ImageCacheMode>,
        limits: Option<ImageLimits>,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageVar {
        self.image_impl(source.into(), cache_mode.into(), limits, downscale, mask, entries)
    }
    fn image_impl(
        &self,
        source: ImageSource,
        cache_mode: ImageCacheMode,
        limits: Option<ImageLimits>,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageVar {
        let limits = limits.unwrap_or_else(|| IMAGES_SV.read().limits.get());
        let proxies = mem::take(&mut IMAGES_SV.write().proxies);
        ImagesService::proxy_then_get(proxies, source, cache_mode, limits, downscale, mask, entries)
    }

    /// Await for an image source, then get or load the image.
    ///
    /// If `limits` is `None` the [`IMAGES.limits`] is used.
    ///
    /// This method returns immediately with a loading [`ImageVar`], when `source` is ready it
    /// is used to get the actual [`ImageVar`] and binds it to the returned image.
    ///
    /// Note that the `cache_mode` always applies to the inner image, and only to the return image if `cache_key` is set.
    ///
    /// [`IMAGES.limits`]: IMAGES::limits
    #[allow(clippy::too_many_arguments)]
    pub fn image_task<F>(
        &self,
        source: impl IntoFuture<IntoFuture = F>,
        cache_mode: impl Into<ImageCacheMode>,
        cache_key: Option<ImageHash>,
        limits: Option<ImageLimits>,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageVar
    where
        F: Future<Output = ImageSource> + Send + 'static,
    {
        self.image_task_impl(
            Box::pin(source.into_future()),
            cache_mode.into(),
            cache_key,
            limits,
            downscale,
            mask,
            entries,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn image_task_impl(
        &self,
        source: Pin<Box<dyn Future<Output = ImageSource> + Send + 'static>>,
        cache_mode: ImageCacheMode,
        cache_key: Option<ImageHash>,
        limits: Option<ImageLimits>,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageVar {
        if let Some(key) = cache_key {
            match cache_mode {
                ImageCacheMode::Cache => {
                    if let Some(v) = IMAGES_SV.read().cache.get(&key) {
                        return v.image.read_only();
                    }
                }
                ImageCacheMode::Retry => {
                    if let Some(e) = IMAGES_SV.read().cache.get(&key)
                        && !e.error.load(Ordering::Relaxed)
                    {
                        return e.image.read_only();
                    }
                }
                ImageCacheMode::Ignore | ImageCacheMode::Reload => {}
            }
        }

        let img = var(Img::new_none(cache_key));

        task::spawn(async_clmv!(img, {
            let source = source.await;
            let actual_img = IMAGES.image_impl(source, cache_mode, limits, downscale, mask, entries);
            actual_img.set_bind(&img).perm();
            img.hold(actual_img).perm();
        }));
        img.read_only()
    }

    /// Associate the `image` with the `key` in the cache.
    ///
    /// Returns `Ok(ImageVar)` with the new image var that tracks `image`, or `Err(ViewImage, ImageVar)`
    /// that returns the `image` and a clone of the var already associated with the `key`.
    ///
    /// Note that you can register entries on the returned [`Img::register_entry`].
    pub fn register(&self, key: ImageHash, image: ViewImage) -> std::result::Result<ImageVar, (ViewImage, ImageVar)> {
        IMAGES_SV.write().register(key, image)
    }

    /// Remove the image from the cache, if it is only held by the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the image was removed.
    pub fn clean(&self, key: ImageHash) -> bool {
        ImagesService::proxy_then_remove(mem::take(&mut IMAGES_SV.write().proxies), &key, false)
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the image was cached.
    pub fn purge(&self, key: &ImageHash) -> bool {
        ImagesService::proxy_then_remove(mem::take(&mut IMAGES_SV.write().proxies), key, true)
    }

    /// Gets the cache key of an image.
    pub fn cache_key(&self, image: &Img) -> Option<ImageHash> {
        if let Some(key) = &image.cache_key
            && IMAGES_SV.read().cache.contains_key(key)
        {
            return Some(*key);
        }
        None
    }

    /// If the image is cached.
    pub fn is_cached(&self, image: &Img) -> bool {
        image
            .cache_key
            .as_ref()
            .map(|k| IMAGES_SV.read().cache.contains_key(k))
            .unwrap_or(false)
    }

    /// Returns an image that is not cached.
    ///
    /// If the `image` is the only reference returns it and removes it from the cache. If there are other
    /// references a new [`ImageVar`] is generated from a clone of the image.
    pub fn detach(&self, image: ImageVar) -> ImageVar {
        IMAGES_SV.write().detach(image)
    }

    /// Clear cached images that are not referenced outside of the cache.
    pub fn clean_all(&self) {
        let mut img = IMAGES_SV.write();
        img.proxies.iter_mut().for_each(|p| p.clear(false));
        img.cache.retain(|_, v| v.image.strong_count() > 1);
    }

    /// Clear all cached images, including images that are still referenced outside of the cache.
    ///
    /// Image memory only drops when all strong references are removed, so if an image is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_all(&self) {
        let mut img = IMAGES_SV.write();
        img.cache.clear();
        img.proxies.iter_mut().for_each(|p| p.clear(true));
    }

    /// Add a cache proxy.
    ///
    /// Proxies can intercept cache requests and map to a different request or return an image directly.
    pub fn install_proxy(&self, proxy: Box<dyn ImageCacheProxy>) {
        IMAGES_SV.write().proxies.push(proxy);
    }

    /// Image formats implemented by the current view-process.
    pub fn available_formats(&self) -> Vec<ImageFormat> {
        VIEW_PROCESS.info().image.clone()
    }
}
struct ImageData {
    format: ImageDataFormat,
    r: std::result::Result<IpcBytes, Txt>,
}

fn absolute_path(path: &Path, base: impl FnOnce() -> PathBuf, allow_escape: bool) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        let mut dir = base();
        if allow_escape {
            dir.push(path);
            normalize_path(&dir)
        } else {
            dir.push(normalize_path(path));
            dir
        }
    }
}
/// Resolves `..` components, without any system request.
///
/// Source: https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}
