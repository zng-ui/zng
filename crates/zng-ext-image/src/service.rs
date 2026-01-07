use std::{
    env, mem,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::types::*;
use parking_lot::Mutex;
use task::io::AsyncReadExt;
use zng_app::{
    APP, update::EventUpdate,
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewImageHandle,
        raw_events::{LOW_MEMORY_EVENT, RAW_IMAGE_DECODE_ERROR_EVENT, RAW_IMAGE_DECODED_EVENT, RAW_IMAGE_METADATA_DECODED_EVENT},
    },
    widget::UiTaskWidget,
};
use zng_app_context::app_local;
use zng_clone_move::clmv;
use zng_layout::unit::{ByteLength, ByteUnits, Px, PxSize};
use zng_task as task;
use zng_task::{UiTask, channel::IpcBytes};
use zng_txt::{ToTxt, Txt, formatx};
use zng_unique_id::{IdEntry, IdMap};
use zng_var::{Var, WeakVar, const_var, var};
use zng_view_api::image::{ImageDecoded, ImageEntryMetadata, ImageId, ImageMetadata, ImageRequest};

pub(crate) mod render;

app_local! {
    static IMAGES_SV: ImagesService = {
        APP.extensions().require::<crate::ImageManager>();
        ImagesService::new()
    };
}

struct ImageData {
    format: ImageDataFormat,
    r: std::result::Result<IpcBytes, Txt>,
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

pub(crate) fn load_in_headless() -> Var<bool> {
    IMAGES_SV.read().load_in_headless.clone()
}

pub(crate) fn limits() -> Var<ImageLimits> {
    IMAGES_SV.read().limits.clone()
}

pub(crate) fn on_app_event_preview(update: &mut EventUpdate) {
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
        clean_all();
    } else {
        IMAGES_SV.write().event_preview_render(update);
    }
}

pub(crate) fn on_app_update_preview() {
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

pub(crate) fn on_app_update() {
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

/// Associate the `handle` with the `key` or return it with the already existing image if the `key` already has an entry.
#[allow(clippy::result_large_err)]
pub(crate) fn register(
    key: Option<ImageHash>,
    image: (ViewImageHandle, ImageDecoded),
    error: Txt,
) -> std::result::Result<ImageVar, ((ViewImageHandle, ImageDecoded), ImageVar)> {
    let mut s = IMAGES_SV.write();
    let s = &mut *s;

    let limits = s.limits.get();
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
        match s.cache.entry(key) {
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
                    s.decoding.push(ImageDecodingTask {
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
        s.not_cached.push(NotCachedEntry {
            image: image.downgrade(),
            max_decoded_len: limits.max_decoded_len,
            downscale: None,
            mask: if is_mask { Some(ImageMaskMode::A) } else { None },
            entries: ImageEntriesMode::PRIMARY,
        });
        Ok(image.read_only())
    } else {
        // not cached and already loaded
        Ok(const_var(Img::new(handle, image)))
    }
}

pub(crate) fn detach(image: ImageVar) -> ImageVar {
    if let Some(key) = &image.with(|i| i.cache_key) {
        let mut s = IMAGES_SV.write();

        let decoded_size = image.with(|img| img.pixels().map(|b| b.len()).unwrap_or(0).bytes());
        let mut max_decoded_len = s.limits.with(|l| l.max_decoded_len.max(decoded_size));
        let mut downscale = None;
        let mut mask = None;
        let mut entries = ImageEntriesMode::PRIMARY;

        if let Some(e) = s.cache.get(key) {
            max_decoded_len = e.max_decoded_len;
            downscale = e.downscale.clone();
            mask = e.mask;
            entries = e.entries;

            // is cached, `clean` if is only external reference.
            if image.strong_count() == 2 {
                s.cache.remove(key);
            }
        }

        // remove `cache_key` from image, this clones the `Img` only-if is still in cache.
        let mut img = image.get();
        img.cache_key = None;
        let img = var(img);
        s.not_cached.push(NotCachedEntry {
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

pub(crate) fn remove(mut key: ImageHash, mut purge: bool) -> bool {
    let mut sv = IMAGES_SV.write();
    let mut extensions = mem::take(&mut sv.extensions);
    if !extensions.is_empty() {
        drop(sv);
        for ext in &mut extensions {
            if !ext.remove(&mut key, &mut purge) {
                IMAGES_SV.write().restore_extensions(extensions);
                return false;
            }
        }
        sv = IMAGES_SV.write();
        sv.restore_extensions(extensions);
    }

    if purge || sv.cache.get(&key).map(|v| v.image.strong_count() > 1).unwrap_or(false) {
        sv.cache.remove(&key).is_some()
    } else {
        false
    }
}

pub(crate) fn image(mut source: ImageSource, mut options: ImageOptions, limits: Option<ImageLimits>) -> ImageVar {
    let mut sv = IMAGES_SV.write();
    let limits = limits.unwrap_or_else(|| sv.limits.get());
    let mut extensions = mem::take(&mut sv.extensions);
    if !extensions.is_empty() {
        drop(sv);
        for ext in &mut extensions {
            ext.image(&limits, &mut source, &mut options);
        }
        sv = IMAGES_SV.write();
        sv.restore_extensions(extensions);
    }

    let source = match source {
        ImageSource::Read(path) => {
            // check limits
            let path = crate::absolute_path(&path, || env::current_dir().expect("could not access current dir"), true);
            if !limits.allow_path.allows(&path) {
                let error = formatx!("limits filter blocked `{}`", path.display());
                tracing::error!("{error}");
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
                return var(Img::new_empty(error)).read_only();
            }
            ImageSource::Download(uri, accepts)
        }
        // Image is supposed to return directly
        ImageSource::Image(r) => {
            return r;
        }
        source => source,
    };

    // continue to loading, ext.image_data gain, decoding

    let key = source.hash128(&options).unwrap();

    match options.cache_mode {
        ImageCacheMode::Cache => {
            if let Some(v) = sv.cache.get(&key) {
                return v.image.read_only();
            }
        }
        ImageCacheMode::Retry => {
            if let Some(e) = sv.cache.get(&key)
                && !e.error.load(Ordering::Relaxed)
            {
                return e.image.read_only();
            }
        }
        ImageCacheMode::Ignore | ImageCacheMode::Reload => {}
    }

    if !VIEW_PROCESS.is_available() && !sv.load_in_headless.get() {
        tracing::warn!("loading dummy image, set `load_in_headless=true` to actually load without renderer");

        let dummy = var(Img::new_empty(Txt::from_static("")));
        sv.cache.insert(
            key,
            CacheEntry {
                image: dummy.clone(),
                error: AtomicBool::new(false),
                max_decoded_len: limits.max_decoded_len,
                downscale: options.downscale,
                mask: options.mask,
                entries: options.entries,
            },
        );
        return dummy.read_only();
    }

    let max_encoded_size = limits.max_encoded_len;

    match source {
        ImageSource::Read(path) => sv.load_task(
            key,
            limits.max_decoded_len,
            options,
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
            let accept = accept.unwrap_or_else(|| sv.download_accept());

            sv.load_task(
                key,
                limits.max_decoded_len,
                options,
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
            sv.load_task(key, limits.max_decoded_len, options, async { r })
        }
        ImageSource::Render(rfn, args) => {
            let mask = options.mask;
            let img = sv.new_cache_image(key, limits.max_decoded_len, options);
            sv.render_img(mask, clmv!(rfn, || rfn(&args.unwrap_or_default())), &img);
            img.read_only()
        }
        ImageSource::Image(_) => unreachable!(),
    }
}

pub(crate) fn available_formats() -> Vec<ImageFormat> {
    IMAGES_SV.read().available_formats()
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

    fn restore_extensions(&mut self, mut extensions: Vec<Box<dyn ImagesExtension>>) {
        extensions.append(&mut self.extensions);
        self.extensions = extensions;
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

pub(crate) fn clean_all() {
    let mut s = IMAGES_SV.write();
    s.extensions.iter_mut().for_each(|p| p.clear(false));
    s.cache.retain(|_, v| v.image.strong_count() > 1);
}

pub(crate) fn contains_key(key: &ImageHash) -> bool {
    IMAGES_SV.read().cache.contains_key(key)
}

pub(crate) fn purge_all() {
    let mut s = IMAGES_SV.write();
    s.cache.clear();
    s.extensions.iter_mut().for_each(|p| p.clear(true));
}

pub(crate) fn extend(extension: Box<dyn ImagesExtension + 'static>) {
    IMAGES_SV.write().extensions.push(extension);
}
