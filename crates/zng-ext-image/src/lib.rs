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
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewImageHandle,
        raw_events::{LOW_MEMORY_EVENT, RAW_IMAGE_DECODE_ERROR_EVENT, RAW_IMAGE_DECODED_EVENT, RAW_IMAGE_METADATA_DECODED_EVENT},
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
use zng_layout::unit::{ByteLength, ByteUnits, Px, PxSize};
use zng_task::{UiTask, channel::IpcBytes};
use zng_txt::{ToTxt, Txt, formatx};
use zng_unique_id::{IdEntry, IdMap};
use zng_var::{Var, WeakVar, const_var, var};
use zng_view_api::image::{ImageDecoded, ImageEntryMetadata, ImageId, ImageMetadata, ImageRequest};

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
        let mut img = None;
        // handle any ok decode
        if let Some(args) = RAW_IMAGE_METADATA_DECODED_EVENT.on(update) {
            img = Some((args.handle.clone(), ImageDecoded::new(args.meta.clone(), IpcBytes::default(), true)));
        } else if let Some(args) = RAW_IMAGE_DECODED_EVENT.on(update) {
            img = Some((args.handle.clone(), args.image.clone()));
        }

        if let Some((handle, data)) = img {
            let images = IMAGES_SV.read();

            if let Some(var) = images.find_decoding(handle.image_id()) {
                // image is registered already, or is entry of registered
                var.modify(move |i| i.set_data(data));
            } else if let Some(p) = &data.meta.parent
                && let Some(var) = images.find_decoding(p.parent)
            {
                // image is not registered, but is entry of image that is, insert it
                let entry = Img::new(handle, data);
                var.modify(move |i| {
                    i.insert_entry(entry);
                });
            }
        } else if let Some(args) = RAW_IMAGE_DECODE_ERROR_EVENT.on(update) {
            let images = IMAGES_SV.read();

            if let Some(var) = images.find_decoding(args.handle.image_id()) {
                // image registered, update to error.

                let error = args.error.clone();
                var.modify(move |i| i.set_error(error));

                if let Some(key) = var.with(|i| i.cache_key)
                    && let Some(entry) = images.cache.get(&key)
                {
                    entry.error.store(true, Ordering::Relaxed);
                }
                // else is loading will flag on the pos update pass
            }
        }

        if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
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
                let old_handle = img.view_handle();

                if !old_handle.is_dummy() {
                    if old_handle.view_process_gen() == args.generation {
                        continue; // already recovered, can this happen?
                    }
                    if let Some(e) = img.error() {
                        // respawned, but image was an error.
                        img_var.set(Img::new_empty(e));
                    } else if let Some(task_i) = decoding_interrupted
                        .iter()
                        .position(|e| e.image.with(|img| img.view_handle() == old_handle))
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
                                img_var.set(Img::new_loading(img));
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

                        let img_format = if img.is_mask() {
                            ImageDataFormat::A8 { size: img.size() }
                        } else {
                            ImageDataFormat::Bgra8 {
                                size: img.size(),
                                density: img.density(),
                                original_color_type: img.original_color_type(),
                            }
                        };

                        let entries = img.entries();

                        let data = img.pixels().unwrap();
                        let request = ImageRequest::new(img_format.clone(), data.clone(), max_decoded_len.0 as u64, None, mask);
                        let img = match VIEW_PROCESS.add_image(request) {
                            Ok(img) => img,
                            Err(_) => return, // we will receive another event.
                        };
                        let mut img = Img::new_loading(img);

                        fn add_entries(max_decoded_len: ByteLength, mask: Option<ImageMaskMode>, entries: Vec<ImageVar>, img: &mut Img) {
                            for (i, entry) in entries.into_iter().enumerate() {
                                let entry = entry.get();
                                let entry_handle = entry.view_handle();
                                if !entry_handle.is_dummy() {
                                    if entry.is_loaded() {
                                        let img_format = if img.is_mask() {
                                            ImageDataFormat::A8 { size: entry.size() }
                                        } else {
                                            ImageDataFormat::Bgra8 {
                                                size: entry.size(),
                                                density: entry.density(),
                                                original_color_type: entry.original_color_type(),
                                            }
                                        };
                                        let data = entry.pixels().unwrap();
                                        let mut request =
                                            ImageRequest::new(img_format.clone(), data.clone(), max_decoded_len.0 as u64, None, mask);
                                        request.parent = Some(ImageEntryMetadata::new(img.view_handle().image_id(), i, entry.entry_kind()));
                                        let entry_img = match VIEW_PROCESS.add_image(request) {
                                            Ok(img) => img,
                                            Err(_) => return, // we will receive another event.
                                        };
                                        let entry_img = img.insert_entry(Img::new_loading(entry_img));

                                        add_entries(max_decoded_len, mask, entry.entries(), &mut entry_img.get());
                                        continue;
                                    } else if entry.is_error() {
                                        img.insert_entry(entry);
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
                            img_var.set(Img::new_loading(img));
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
            IMAGES_SV.write().event_preview_render(update);
        }
    }

    fn update_preview(&mut self) {
        // update loading tasks:

        let mut images = IMAGES_SV.write();
        let mut loading = Vec::with_capacity(images.loading.len());
        let loading_tasks = mem::take(&mut images.loading);
        let mut extensions = mem::take(&mut images.extensions);
        drop(images); // extensions can use IMAGES

        'loading_tasks: for mut t in loading_tasks {
            t.task.get_mut().update();
            match t.task.into_inner().into_result() {
                Ok(d) => {
                    match d.r {
                        Ok(data) => {
                            for ext in &mut extensions {
                                if let Some(img) = ext.image_data(t.max_decoded_len, &t.key, &data, &d.format, &t.options) {
                                    img.set_bind(&t.image).perm();
                                    t.image.hold(img).perm();
                                    continue 'loading_tasks;
                                }
                            }

                            if VIEW_PROCESS.is_available() {
                                // success and we have a view-process.
                                let mut request = ImageRequest::new(
                                    d.format.clone(),
                                    data.clone(),
                                    t.max_decoded_len.0 as u64,
                                    t.options.downscale.clone(),
                                    t.options.mask,
                                );
                                request.entries = t.options.entries;
                                match VIEW_PROCESS.add_image(request) {
                                    Ok(img) => {
                                        // request sent, add to `decoding` will receive
                                        // image decoded events
                                        t.image.modify(move |v| {
                                            v.set_handle(img);
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
                                t.image.modify(move |v| {
                                    let data = ImageDecoded::new(
                                        ImageMetadata::new(ImageId::INVALID, PxSize::splat(Px(1)), false, ColorType::BGRA8),
                                        IpcBytes::from_vec_blocking(vec![0, 0, 0, 0]).unwrap(),
                                        false,
                                    );
                                    v.set_data(data);
                                });
                            }
                        }
                        Err(e) => {
                            tracing::error!("load error: {e:?}");
                            // load error.
                            t.image.modify(move |v| {
                                v.set_error(e);
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
                        key: t.key,
                        options: t.options,
                    });
                }
            }
        }
        let mut images = IMAGES_SV.write();
        images.loading = loading;
        images.extensions = extensions;
    }

    fn update(&mut self) {
        let mut images = IMAGES_SV.write();
        let images = &mut *images;

        images.decoding.retain(|t| {
            t.image.with(|i| {
                let retain = i.is_loading() || i.has_loading_entries();

                if !retain
                    && i.is_error()
                    && let Some(key) = i.cache_key
                    && let Some(entry) = images.cache.get_mut(&key)
                {
                    *entry.error.get_mut() = true;
                }

                retain
            })
        });

        images.update_render();
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
    key: ImageHash,
    options: ImageOptions,
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
    extensions: Vec<Box<dyn ImagesExtension>>,

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
            extensions: vec![],
            loading: vec![],
            decoding: vec![],
            download_accept: Txt::from_static(""),
            cache: IdMap::new(),
            not_cached: vec![],
            render: render::ImagesRender::default(),
        }
    }

    /// Associate the `handle` with the `key` or return it with the already existing image if the `key` already has an entry.
    #[allow(clippy::result_large_err)]
    fn register(
        &mut self,
        key: Option<ImageHash>,
        image: (ViewImageHandle, ImageDecoded),
        error: Txt,
    ) -> std::result::Result<ImageVar, ((ViewImageHandle, ImageDecoded), ImageVar)> {
        let limits = self.limits.get();
        let limits = ImageLimits {
            max_encoded_len: limits.max_encoded_len,
            max_decoded_len: limits.max_decoded_len.max(image.1.pixels.len().bytes()),
            allow_path: PathFilter::BlockAll,
            #[cfg(feature = "http")]
            allow_uri: UriFilter::BlockAll,
        };

        let (handle, image) = image;
        let is_error = !error.is_empty();
        let is_loading = !is_error && (image.partial.is_some() || image.pixels.is_empty());

        if let Some(key) = key {
            match self.cache.entry(key) {
                IdEntry::Occupied(e) => Err(((handle, image), e.get().image.read_only())),
                IdEntry::Vacant(e) => {
                    let is_mask = image.meta.is_mask;
                    let format = if is_mask {
                        ImageDataFormat::A8 { size: image.meta.size }
                    } else {
                        ImageDataFormat::Bgra8 {
                            size: image.meta.size,
                            density: image.meta.density,
                            original_color_type: image.meta.original_color_type.clone(),
                        }
                    };
                    let img_var = var(Img::new(handle, image));
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
        } else if is_loading {
            let is_mask = image.meta.is_mask;
            let image = var(Img::new(handle, image));
            self.not_cached.push(NotCachedEntry {
                image: image.downgrade(),
                max_decoded_len: limits.max_decoded_len,
                downscale: None,
                mask: if is_mask { Some(ImageMaskMode::A) } else { None },
                entries: ImageEntriesMode::PRIMARY,
            });
            todo!()
        } else {
            // not cached and already loaded
            Ok(const_var(Img::new(handle, image)))
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

    fn restore_extensions(&mut self, mut extensions: Vec<Box<dyn ImagesExtension>>) {
        extensions.append(&mut self.extensions);
        self.extensions = extensions;
    }

    fn remove(mut extensions: Vec<Box<dyn ImagesExtension>>, mut key: ImageHash, mut purge: bool) -> bool {
        for ext in &mut extensions {
            if !ext.remove(&mut key, &mut purge) {
                IMAGES_SV.write().restore_extensions(extensions);
                return false;
            }
        }
        let mut sv = IMAGES_SV.write();
        sv.restore_extensions(extensions);
        sv.remove_impl(key, purge)
    }
    fn remove_impl(&mut self, key: ImageHash, purge: bool) -> bool {
        if purge || self.cache.get(&key).map(|v| v.image.strong_count() > 1).unwrap_or(false) {
            self.cache.remove(&key).is_some()
        } else {
            false
        }
    }

    fn image(
        mut extensions: Vec<Box<dyn ImagesExtension>>,
        mut source: ImageSource,
        mut options: ImageOptions,
        limits: ImageLimits,
    ) -> ImageVar {
        for ext in &mut extensions {
            ext.image(&limits, &mut source, &mut options);
        }

        let source = match source {
            ImageSource::Read(path) => {
                // check limits
                let path = crate::absolute_path(&path, || env::current_dir().expect("could not access current dir"), true);
                if !limits.allow_path.allows(&path) {
                    let error = formatx!("limits filter blocked `{}`", path.display());
                    tracing::error!("{error}");
                    IMAGES_SV.write().restore_extensions(extensions);
                    return var(Img::new_empty(error)).read_only();
                }
                ImageSource::Read(path)
            }
            #[cfg(feature = "http")]
            // check limits
            ImageSource::Download(uri, accepts) => {
                if !limits.allow_uri.allows(&uri) {
                    let error = formatx!("limits filter blocked `{uri}`");
                    tracing::error!("{error}");
                    IMAGES_SV.write().restore_extensions(extensions);
                    return var(Img::new_empty(error)).read_only();
                }
                ImageSource::Download(uri, accepts)
            }
            // Image is supposed to return directly
            ImageSource::Image(r) => {
                IMAGES_SV.write().restore_extensions(extensions);
                return r;
            }
            source => source,
        };

        // continue to loading, ext.image_data gain, decoding
        let mut sv = IMAGES_SV.write();
        sv.restore_extensions(extensions);

        sv.image_impl(source, limits, options)
    }
    #[allow(clippy::too_many_arguments)]
    fn image_impl(&mut self, source: ImageSource, limits: ImageLimits, opt: ImageOptions) -> ImageVar {
        let key = source.hash128(&opt).unwrap();

        match opt.cache_mode {
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

            let dummy = var(Img::new_empty(Txt::from_static("")));
            self.cache.insert(
                key,
                CacheEntry {
                    image: dummy.clone(),
                    error: AtomicBool::new(false),
                    max_decoded_len: limits.max_decoded_len,
                    downscale: opt.downscale,
                    mask: opt.mask,
                    entries: opt.entries,
                },
            );
            return dummy.read_only();
        }

        let max_encoded_size = limits.max_encoded_len;

        match source {
            ImageSource::Read(path) => self.load_task(
                key,
                limits.max_decoded_len,
                opt,
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
                    limits.max_decoded_len,
                    opt,
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
                self.load_task(key, limits.max_decoded_len, opt, async { r })
            }
            ImageSource::Render(rfn, args) => {
                let mask = opt.mask;
                let img = self.new_cache_image(key, limits.max_decoded_len, opt);
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
                for fmt in self.available_formats() {
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

    fn available_formats(&self) -> Vec<ImageFormat> {
        let mut formats = VIEW_PROCESS.info().image.clone();
        for ext in &self.extensions {
            ext.available_formats(&mut formats);
        }
        formats
    }

    fn cleanup_not_cached(&mut self, force: bool) {
        if force || self.not_cached.len() > 1000 {
            self.not_cached.retain(|c| c.image.strong_count() > 0);
        }
    }

    fn new_cache_image(&mut self, key: ImageHash, max_decoded_len: ByteLength, options: ImageOptions) -> Var<Img> {
        self.cleanup_not_cached(false);

        if let ImageCacheMode::Reload = options.cache_mode {
            self.cache
                .entry(key)
                .or_insert_with(|| CacheEntry {
                    image: var(Img::new_cached(key)),
                    error: AtomicBool::new(false),
                    max_decoded_len,
                    downscale: options.downscale,
                    mask: options.mask,
                    entries: options.entries,
                })
                .image
                .clone()
        } else if let ImageCacheMode::Ignore = options.cache_mode {
            let img = var(Img::new_loading(ViewImageHandle::dummy()));
            self.not_cached.push(NotCachedEntry {
                image: img.downgrade(),
                max_decoded_len,
                downscale: options.downscale,
                mask: options.mask,
                entries: options.entries,
            });
            img
        } else {
            let img = var(Img::new_cached(key));
            self.cache.insert(
                key,
                CacheEntry {
                    image: img.clone(),
                    error: AtomicBool::new(false),
                    max_decoded_len,
                    downscale: options.downscale,
                    mask: options.mask,
                    entries: options.entries,
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
        max_decoded_len: ByteLength,
        options: ImageOptions,
        fetch_bytes: impl Future<Output = ImageData> + Send + 'static,
    ) -> ImageVar {
        let img = self.new_cache_image(key, max_decoded_len, options.clone());
        let r = img.read_only();

        self.loading.push(ImageLoadingTask {
            task: Mutex::new(UiTask::new(None, fetch_bytes)),
            image: img,
            max_decoded_len,
            key,
            options,
        });
        zng_app::update::UPDATES.update(None);

        r
    }

    fn find_decoding(&self, id: zng_view_api::image::ImageId) -> Option<ImageVar> {
        self.decoding.iter().find_map(|i| {
            if i.image.with(|i| i.handle.image_id() == id) {
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
        var(Img::new_empty(error.unwrap_or_default())).read_only()
    }

    /// Cache or load an image file from a file system `path`.
    pub fn read(&self, path: impl Into<PathBuf>) -> ImageVar {
        self.image_impl(path.into().into(), ImageOptions::cache(), None)
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
            Ok(uri) => self.image(ImageSource::Download(uri, accept), ImageOptions::cache(), None),
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
        self.image_impl((data, format.into()).into(), ImageOptions::cache(), None)
    }

    /// Get a cached image from shared data.
    ///
    /// The image key is a [`ImageHash`] of the image data. The data reference is held only until the image is decoded.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    pub fn from_data(&self, data: IpcBytes, format: impl Into<ImageDataFormat>) -> ImageVar {
        self.image_impl((data, format.into()).into(), ImageOptions::cache(), None)
    }

    /// Get or load an image with full configuration.
    ///
    /// If `limits` is `None` the [`IMAGES.limits`] is used.
    ///
    /// [`IMAGES.limits`]: IMAGES::limits
    pub fn image(&self, source: impl Into<ImageSource>, options: ImageOptions, limits: Option<ImageLimits>) -> ImageVar {
        self.image_impl(source.into(), options, limits)
    }
    fn image_impl(&self, source: ImageSource, options: ImageOptions, limits: Option<ImageLimits>) -> ImageVar {
        let limits = limits.unwrap_or_else(|| IMAGES_SV.read().limits.get());
        let extensions = mem::take(&mut IMAGES_SV.write().extensions);
        ImagesService::image(extensions, source, options, limits)
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
        let img = var(Img::new_empty(Txt::from_static("")));
        task::spawn(async_clmv!(img, {
            let source = source.await;
            let actual_img = IMAGES.image_impl(source, options, limits);
            actual_img.set_bind(&img).perm();
            img.hold(actual_img).perm();
        }));
        img.read_only()
    }

    /// Associate the `image` produced by direct interaction with the view-process with the `key` in the cache.
    ///
    /// If the `key` is not set the image is not cached, the service only manages it until it is loaded.
    ///
    /// Returns `Ok(ImageVar)` with the new image var that tracks `image`, or `Err(image, ImageVar)`
    /// that returns the `image` and a clone of the var already associated with the `key`.
    ///
    /// Note that you can register entries on the returned [`Img::insert_entry`].
    #[allow(clippy::result_large_err)] // boxing here does not really help performance
    pub fn register(
        &self,
        key: Option<ImageHash>,
        image: (ViewImageHandle, ImageDecoded),
    ) -> std::result::Result<ImageVar, ((ViewImageHandle, ImageDecoded), ImageVar)> {
        IMAGES_SV.write().register(key, image, Txt::from_static(""))
    }

    /// Remove the image from the cache, if it is only held by the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the image was removed.
    pub fn clean(&self, key: ImageHash) -> bool {
        ImagesService::remove(mem::take(&mut IMAGES_SV.write().extensions), key, false)
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the image was removed, that is, if it was cached.
    pub fn purge(&self, key: ImageHash) -> bool {
        ImagesService::remove(mem::take(&mut IMAGES_SV.write().extensions), key, true)
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
        img.extensions.iter_mut().for_each(|p| p.clear(false));
        img.cache.retain(|_, v| v.image.strong_count() > 1);
    }

    /// Clear all cached images, including images that are still referenced outside of the cache.
    ///
    /// Image memory only drops when all strong references are removed, so if an image is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_all(&self) {
        let mut img = IMAGES_SV.write();
        img.cache.clear();
        img.extensions.iter_mut().for_each(|p| p.clear(true));
    }

    /// Add an images service extension.
    ///
    /// See [`ImagesExtension`] for extension capabilities.
    pub fn extend(&self, extension: Box<dyn ImagesExtension>) {
        IMAGES_SV.write().extensions.push(extension);
    }

    /// Image formats implemented by the current view-process and extensions.
    pub fn available_formats(&self) -> Vec<ImageFormat> {
        IMAGES_SV.read().available_formats()
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

/// Options for [`IMAGES.image`].
///
/// [`IMAGES.image`]: IMAGES::image
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ImageOptions {
    /// If and how the image is cached.
    pub cache_mode: ImageCacheMode,
    /// How the image is downscaled after decoding.
    pub downscale: Option<ImageDownscaleMode>,
    /// How to convert the decoded image to an alpha mask.
    pub mask: Option<ImageMaskMode>,
    /// How to decode containers with multiple images.
    pub entries: ImageEntriesMode,
}

impl ImageOptions {
    /// New.
    pub fn new(
        cache_mode: ImageCacheMode,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> Self {
        Self {
            cache_mode,
            downscale,
            mask,
            entries,
        }
    }

    /// New with only cache enabled.
    pub fn cache() -> Self {
        Self::new(ImageCacheMode::Cache, None, None, ImageEntriesMode::empty())
    }
}
