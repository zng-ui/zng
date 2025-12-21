#![cfg_attr(not(feature = "image_any"), allow(unused))]

use std::{fmt, sync::Arc};
use zng_task::parking_lot::Mutex;
use zng_view_api::image::ImageEntryKind;
use zng_view_api::image::{ColorType, ImageDownscaleMode, ImageEntriesMode, ImageEntryMetadata, ImageFormat, ImageMaskMode};

use webrender::api::ImageDescriptor;
use zng_txt::formatx;

use zng_task::channel::{IpcBytes, IpcReceiver};
use zng_unit::{Px, PxDensity2d, PxSize};
use zng_view_api::{
    Event,
    image::{ImageDataFormat, ImageDecoded, ImageId, ImageMetadata, ImageRequest},
};

use crate::{AppEvent, AppEventSender};
use rustc_hash::FxHashMap;

// Image data is provided to webrender directly from the BGRA8 shared memory.
// The `ExternalImageId` is the Arc pointer to ImageData.
mod capture;
mod decode;
mod dyn_image;
mod encode;
mod external;
pub(crate) use external::{ImageUseMap, WrImageCache};

#[cfg(not(feature = "image_any"))]
pub(crate) mod lcms2 {
    pub struct Profile {}
}

pub(crate) const FORMATS: &[ImageFormat] = &[
    #[cfg(any(feature = "image_avif", zng_view_image_has_avif))]
    ImageFormat::from_static("AVIF", "avif", "avif", false),
    #[cfg(feature = "image_bmp")]
    ImageFormat::from_static("BMP", "bmp", "bmp,dib", true),
    #[cfg(feature = "image_dds")]
    ImageFormat::from_static("DirectDraw Surface", "vnd-ms.dds,x-direct-draw-surface", "dds", false),
    #[cfg(feature = "image_exr")]
    ImageFormat::from_static("OpenEXR", "x-exr", "exr", false),
    // https://www.wikidata.org/wiki/Q28206109
    #[cfg(feature = "image_ff")]
    ImageFormat::from_static("Farbfeld", "x-farbfeld", "ff,ff.bz2", false),
    #[cfg(feature = "image_gif")]
    ImageFormat::from_static("GIF", "gif", "gif", false),
    #[cfg(feature = "image_hdr")]
    ImageFormat::from_static("Radiance HDR", "vnd.radiance", "hdr", false),
    #[cfg(feature = "image_ico")]
    ImageFormat::from_static("ICO", "x-icon,vnd.microsoft.icon", "ico", true),
    #[cfg(feature = "image_jpeg")]
    ImageFormat::from_static("JPEG", "jpeg", "jpg,jpeg", true),
    #[cfg(feature = "image_png")]
    ImageFormat::from_static("PNG", "png", "png", true),
    #[cfg(feature = "image_pnm")]
    ImageFormat::from_static(
        "PNM",
        "x-portable-bitmap,x-portable-graymap,x-portable-pixmap,x-portable-anymap",
        "pbm,pgm,ppm,pam",
        false,
    ),
    // https://github.com/phoboslab/qoi/issues/167
    #[cfg(feature = "image_qoi")]
    ImageFormat::from_static("QOI", "x-qoi", "qoi", true),
    #[cfg(feature = "image_tga")]
    ImageFormat::from_static("TGA", "x-tga,x-targa", "tga,icb,vda,vst", false),
    #[cfg(feature = "image_tiff")]
    ImageFormat::from_static("TIFF", "tiff", "tif,tiff", true),
    #[cfg(feature = "image_tiff")]
    ImageFormat::from_static("WebP", "webp", "webp", true),
];

pub(crate) type ResizerCache = Mutex<fast_image_resize::Resizer>;

/// Decode and cache image resources.
pub(crate) struct ImageCache {
    app_sender: AppEventSender,
    images: FxHashMap<ImageId, Image>,
    image_id_gen: Arc<Mutex<ImageId>>,
    resizer: Arc<ResizerCache>,
}
impl ImageCache {
    pub fn new(app_sender: AppEventSender) -> Self {
        Self {
            app_sender,
            images: FxHashMap::default(),
            image_id_gen: Arc::new(Mutex::new(ImageId::first())),
            resizer: Arc::new(Mutex::new(fast_image_resize::Resizer::new())),
        }
    }

    pub fn add(
        &mut self,
        ImageRequest {
            format,
            data,
            max_decoded_len,
            downscale,
            mask,
            entries,
            parent,
            ..
        }: ImageRequest<IpcBytes>,
    ) -> ImageId {
        let id = self.image_id_gen.lock().incr();
        let id_gen = self.image_id_gen.clone();
        let app_sender = self.app_sender.clone();
        let resizer = self.resizer.clone();
        rayon::spawn(move || {
            Self::add_impl(
                id_gen,
                app_sender,
                resizer,
                false,
                id,
                format,
                data,
                max_decoded_len,
                downscale,
                mask,
                entries,
                parent,
            );
        });

        id
    }

    pub fn add_pro(
        &mut self,
        ImageRequest {
            format,
            mut data,
            max_decoded_len,
            downscale,
            mask,
            entries,
            parent,
            ..
        }: ImageRequest<IpcReceiver<IpcBytes>>,
    ) -> ImageId {
        let id = self.image_id_gen.lock().incr();
        let id_gen = self.image_id_gen.clone();
        let app_sender = self.app_sender.clone();
        let resizer = self.resizer.clone();
        rayon::spawn(move || {
            // image crate does not implement progressive decoding, just receive all payloads and continue as `add` for now

            let mut notified_header = false;

            let mut all_data = match data.recv_blocking() {
                Ok(f) => f,
                Err(e) => {
                    let _ = app_sender.send(AppEvent::Notify(Event::ImageDecodeError {
                        image: id,
                        error: formatx!("no image data, {e}"),
                    }));
                    return;
                }
            };
            if let Ok(n) = data.recv_blocking() {
                // try parse header early at least
                #[cfg(feature = "image_any")]
                if let ImageDataFormat::FileExtension(_) | ImageDataFormat::MimeType(_) | ImageDataFormat::Unknown = &format
                    && let Ok((fmt, entries)) = Self::decode_container(&format, &all_data[..])
                    && entries.first() == Some(&(0, ImageEntryKind::Page))
                    && let Ok(h) = Self::decode_metadata(&all_data[..], fmt, 0)
                {
                    let mut size = h.size;
                    let decoded_len = size.width.0 as u64 * size.height.0 as u64 * 4;
                    if decoded_len > max_decoded_len {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageDecodeError {
                            image: id,
                            error: formatx!(
                                "image {size:?} needs to allocate {decoded_len} bytes, but max allowed size is {max_decoded_len} bytes",
                            ),
                        }));
                        return;
                    } else {
                        // notify metadata already

                        let (d_size, _) = downscale_sizes(downscale.as_ref(), h.size, &[]);
                        size = d_size.unwrap_or(h.size);
                        let og_color_size = image_color_type_to_vp(h.og_color_type);
                        let mut meta = ImageMetadata::new(id, size, mask.is_some(), og_color_size.clone());
                        meta.density = h.density;

                        let _ = app_sender.send(AppEvent::Notify(Event::ImageMetadataDecoded(meta)));
                        notified_header = true;
                    }
                }

                let mut w = IpcBytes::new_writer_blocking();
                let try_result = (|| -> std::io::Result<IpcBytes> {
                    use std::io::Write as _;
                    w.write_all(&all_data[..])?;
                    w.write_all(&n[..])?;
                    while let Ok(n) = data.recv_blocking() {
                        w.write_all(&n[..])?;
                    }
                    w.finish()
                })();
                match try_result {
                    Ok(d) => all_data = d,
                    Err(e) => {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageDecodeError {
                            image: id,
                            error: formatx!("cannot receive image data, {e}"),
                        }));
                        return;
                    }
                }
            }

            Self::add_impl(
                id_gen,
                app_sender,
                resizer,
                notified_header,
                id,
                format,
                all_data,
                max_decoded_len,
                downscale,
                mask,
                entries,
                parent,
            );
        });
        id
    }

    #[allow(clippy::too_many_arguments)]
    fn add_impl(
        id_gen: Arc<Mutex<ImageId>>,
        app_sender: AppEventSender,
        resizer: Arc<ResizerCache>,
        notified_meta: bool,

        id: ImageId,
        format: ImageDataFormat,
        data: IpcBytes,
        max_decoded_len: u64,
        downscale: Option<ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
        parent: Option<ImageEntryMetadata>,
    ) {
        let data_ref = &data[..];
        macro_rules! error {
            ($($tt:tt)*) => {{
                let _ = app_sender.send(AppEvent::Notify(Event::ImageDecodeError { image: id, error: formatx!($($tt)*) }));
            }};
        }
        macro_rules! decoded {
            ($r:tt, $og_color_type:expr, $return_data:expr) => {{
                let (pixels, size, density, is_opaque, is_mask) = $r;
                let mut meta = ImageMetadata::new(id, size, is_mask, $og_color_type);
                meta.density = density;
                meta.parent = parent;
                let decoded = ImageDecoded::new(meta, pixels, is_opaque);
                let mut out = if $return_data { Some(decoded.clone()) } else { None };
                if app_sender.send(AppEvent::ImageDecoded(decoded)).is_err() {
                    out = None;
                }
                out
            }};
        }

        match format {
            ImageDataFormat::Bgra8 {
                size,
                density,
                original_color_type,
            } => {
                let downscale_sizes = self::downscale_sizes(downscale.as_ref(), size, &[]);

                let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
                if data_ref.len() != expected_len {
                    return error!(
                        "pixels.len() is not width * height * 4, expected {expected_len}, found {}",
                        data_ref.len()
                    );
                }

                if let Some(mask) = mask {
                    match Self::convert_bgra8_to_mask(size, data_ref, mask, density, downscale_sizes.0, &resizer) {
                        Ok(r) => {
                            if let Some(d) = decoded!(r, original_color_type, !downscale_sizes.1.is_empty()) {
                                Self::add_downscale_entries(id_gen, app_sender, resizer, d, downscale_sizes.1);
                            }
                        }
                        Err(e) => error!("{e}"),
                    }
                } else {
                    match Self::downscale_decoded(mask, downscale_sizes.0, &resizer, size, data_ref) {
                        Ok(Some((size, data_mut))) => match data_mut.finish_blocking() {
                            Ok(data) => {
                                let is_opaque = data_ref.chunks_exact(4).all(|c| c[3] == 255);
                                if let Some(d) = decoded!(
                                    (data, size, None, is_opaque, false),
                                    original_color_type,
                                    !downscale_sizes.1.is_empty()
                                ) {
                                    Self::add_downscale_entries(id_gen, app_sender, resizer, d, downscale_sizes.1);
                                }
                            }
                            Err(e) => error!("{e}"),
                        },
                        Ok(None) => {
                            let is_opaque = data_ref.chunks_exact(4).all(|c| c[3] == 255);
                            if let Some(d) = decoded!(
                                (data, size, None, is_opaque, false),
                                original_color_type,
                                !downscale_sizes.1.is_empty()
                            ) {
                                Self::add_downscale_entries(id_gen, app_sender, resizer, d, downscale_sizes.1);
                            }
                        }
                        Err(e) => error!("{e}"),
                    }
                }
            }
            ImageDataFormat::A8 { size } => {
                let downscale_sizes = self::downscale_sizes(downscale.as_ref(), size, &[]);

                let expected_len = size.width.0 as usize * size.height.0 as usize;
                if data.len() != expected_len {
                    return error!("pixels.len() is not width * height, expected {expected_len}, found {}", data.len());
                }

                if mask.is_none() {
                    match Self::convert_a8_to_bgra8(size, &data, None, downscale_sizes.0, &resizer) {
                        Ok(r) => {
                            if let Some(d) = decoded!(r, ColorType::A8, downscale_sizes.1.is_empty()) {
                                Self::add_downscale_entries(id_gen, app_sender, resizer, d, downscale_sizes.1);
                            }
                        }
                        Err(e) => error!("{e}"),
                    }
                } else {
                    match Self::downscale_decoded(mask, downscale_sizes.0, &resizer, size, &data) {
                        Ok(Some((size, data_mut))) => match data_mut.finish_blocking() {
                            Ok(data) => {
                                let is_opaque = data.iter().all(|&c| c == 255);
                                if let Some(d) = decoded!((data, size, None, is_opaque, true), ColorType::A8, !downscale_sizes.1.is_empty())
                                {
                                    Self::add_downscale_entries(id_gen, app_sender, resizer, d, downscale_sizes.1);
                                }
                            }
                            Err(e) => error!("{e}"),
                        },
                        Ok(None) => {
                            let is_opaque = data.iter().all(|&c| c == 255);
                            if let Some(d) = decoded!((data, size, None, is_opaque, true), ColorType::A8, !downscale_sizes.1.is_empty()) {
                                Self::add_downscale_entries(id_gen, app_sender, resizer, d, downscale_sizes.1);
                            }
                        }
                        Err(e) => error!("{e}"),
                    }
                }
            }
            // needs decoding
            #[cfg(not(feature = "image_any"))]
            fmt => {
                let _ = (max_decoded_len, downscale);
                return error!("no decoder for {fmt:?}");
            }
            #[cfg(feature = "image_any")]
            fmt => {
                let (fmt, entries_kind) = match Self::decode_container(&fmt, data_ref) {
                    Ok(r) => r,
                    Err(e) => return error!("{e}"),
                };
                if entries_kind.is_empty() {
                    return error!("empty container");
                }

                let mut headers = Vec::with_capacity(entries_kind.len());
                for (i, kind) in entries_kind {
                    let h = match Self::decode_metadata(data_ref, fmt, i) {
                        Ok(h) => h,
                        Err(e) => return error!("{e}"),
                    };
                    headers.push((i, h, kind));
                }
                headers.retain(|h| {
                    let decoded_len = h.1.size.width.0 as u64 * h.1.size.height.0 as u64 * 4;
                    decoded_len <= max_decoded_len
                });
                if headers.is_empty() {
                    return error!("cannot decode image within the allowed {max_decoded_len} bytes");
                }
                let mut page_count = 0;
                headers.retain(|h| match &h.2 {
                    ImageEntryKind::Page => {
                        // always includes the first page
                        let retain = page_count == 0 || entries.contains(ImageEntriesMode::PAGES);
                        page_count += 1;
                        retain
                    }
                    ImageEntryKind::Reduced { .. } => {
                        // reduced requested AND including all pages or is reduced for first page
                        entries.contains(ImageEntriesMode::REDUCED) && (entries.contains(ImageEntriesMode::PAGES) || page_count == 1)
                    }
                    ImageEntryKind::Other { .. } => entries.contains(ImageEntriesMode::OTHER),
                    _ => unreachable!(),
                });
                if headers.is_empty() {
                    return error!("image container has no page entries");
                }

                // group entries by page
                let mut pages: Vec<(_, Vec<_>)> = vec![];
                let mut others = vec![];
                for entry in headers {
                    match &entry.2 {
                        ImageEntryKind::Page => {
                            pages.push((entry, vec![]));
                        }
                        ImageEntryKind::Reduced { .. } => {
                            if let Some(p) = pages.last_mut() {
                                p.1.push(entry);
                            } else if entries.contains(ImageEntriesMode::OTHER) {
                                others.push(entry);
                            }
                        }
                        ImageEntryKind::Other { .. } => {
                            others.push(entry);
                        }
                        _ => unreachable!(),
                    }
                }

                // collect work, so that metadata events can be send first, before decode/downscale starts
                enum Task {
                    Decode {
                        entry_index: usize,
                        entry_header: ImageHeader,
                        id: ImageId,
                        parent: Option<ImageEntryMetadata>,
                        notify_meta: bool,
                        downscale: Option<PxSize>,
                    },
                    SynthReduced {
                        // source: previous task pixels
                        size: PxSize,
                        id: ImageId,
                        parent: ImageEntryMetadata,
                    },
                }
                let mut tasks = vec![];
                let mut root_entries = 0;
                for (page, entries) in pages.into_iter() {
                    let page_size = page.1.size;
                    let entry_sizes: Vec<_> = entries.iter().map(|e| e.1.size).collect();
                    let downscale_sizes = self::downscale_sizes(downscale.as_ref(), page_size, &entry_sizes);

                    let notify_meta = !tasks.is_empty() || !notified_meta;
                    let parent = if tasks.is_empty() {
                        parent.clone()
                    } else {
                        root_entries += 1;
                        Some(ImageEntryMetadata::new(id, root_entries, page.2))
                    };
                    let id = if tasks.is_empty() { id } else { id_gen.lock().incr() };
                    let mut page_entries = 0;
                    let page_entries = if tasks.is_empty() { &mut root_entries } else { &mut page_entries };

                    tasks.push(Task::Decode {
                        entry_index: page.0,
                        entry_header: page.1,
                        id,
                        parent,
                        notify_meta,
                        downscale: downscale_sizes.0,
                    });

                    // downscale are generated from the nearest larger entry, or previous downscale
                    let mut entries_downscale = vec![vec![]; entries.len()];
                    for synth in downscale_sizes.1 {
                        let mut best_i = usize::MAX;
                        let mut best_dist = Px::MAX;
                        let page_dist = page_size.width - synth.width;
                        if page_dist > Px(0) {
                            best_i = entries.len();
                            best_dist = page_dist;
                        }
                        for (i, entry) in entry_sizes.iter().enumerate() {
                            let entry_dist = entry.width - synth.width;
                            if entry_dist > Px(0) && entry_dist < best_dist {
                                best_i = i;
                                best_dist = entry_dist;
                            }
                        }

                        if let Some(entry) = entries_downscale.get_mut(best_i) {
                            entry.push(synth);
                        } else {
                            debug_assert_ne!(best_i, usize::MAX, "downscale_sizes did not filter correctly");
                            *page_entries += 1;
                            tasks.push(Task::SynthReduced {
                                size: synth,
                                id: id_gen.lock().incr(),
                                parent: ImageEntryMetadata::new(id, *page_entries, ImageEntryKind::Reduced { synthetic: true }),
                            });
                        }
                    }

                    for (entry, downscale) in entries.into_iter().zip(entries_downscale) {
                        *page_entries += 1;
                        tasks.push(Task::Decode {
                            entry_index: entry.0,
                            entry_header: entry.1,
                            id: id_gen.lock().incr(),
                            parent: Some(ImageEntryMetadata::new(id, *page_entries, entry.2)),
                            notify_meta,
                            downscale: None,
                        });
                        for synth in downscale {
                            *page_entries += 1;
                            tasks.push(Task::SynthReduced {
                                size: synth,
                                id: id_gen.lock().incr(),
                                parent: ImageEntryMetadata::new(id, *page_entries, ImageEntryKind::Reduced { synthetic: true }),
                            });
                        }
                    }
                }

                // notify all metadata ahead of decode/downscale
                let mut tasks_meta = vec![];
                for task in &tasks {
                    match task {
                        Task::Decode {
                            entry_header,
                            id,
                            parent,
                            notify_meta,
                            downscale,
                            ..
                        } => {
                            let size = downscale.unwrap_or(entry_header.size);
                            let og_color_type = image_color_type_to_vp(entry_header.og_color_type);
                            let mut meta = ImageMetadata::new(*id, size, mask.is_some(), og_color_type);
                            meta.density = entry_header.density;
                            meta.parent = parent.clone();

                            if *notify_meta
                                && app_sender
                                    .send(AppEvent::Notify(Event::ImageMetadataDecoded(meta.clone())))
                                    .is_err()
                            {
                                return;
                            }

                            tasks_meta.push(meta);
                        }
                        Task::SynthReduced { size, id, parent } => {
                            let mut meta = tasks_meta.last().cloned().unwrap();
                            meta.id = *id;
                            meta.size = *size;
                            meta.parent = Some(parent.clone());

                            if app_sender
                                .send(AppEvent::Notify(Event::ImageMetadataDecoded(meta.clone())))
                                .is_err()
                            {
                                return;
                            }

                            tasks_meta.push(meta);
                        }
                    }
                }
                let mut others_meta = vec![];
                for entry in others {
                    let og_color_type = image_color_type_to_vp(entry.1.og_color_type);
                    let mut meta = ImageMetadata::new(id_gen.lock().incr(), entry.1.size, mask.is_some(), og_color_type);
                    meta.density = entry.1.density;
                    root_entries += 1;
                    let kind = match entry.2 {
                        ImageEntryKind::Reduced { .. } => ImageEntryKind::Other {
                            kind: formatx!("unlinked reduced"),
                        },
                        ImageEntryKind::Page => unreachable!(),
                        other => other,
                    };
                    meta.parent = Some(ImageEntryMetadata::new(id, root_entries, kind));

                    if app_sender
                        .send(AppEvent::Notify(Event::ImageMetadataDecoded(meta.clone())))
                        .is_err()
                    {
                        return;
                    }
                    others_meta.push(meta);
                }

                let data = &data[..];
                let mut prev_size = PxSize::zero();
                let mut prev_pixels = IpcBytes::default();
                let mut prev_is_opaque = false;
                for (task, mut meta) in tasks.into_iter().zip(tasks_meta) {
                    match task {
                        Task::Decode {
                            entry_index,
                            entry_header,
                            downscale,
                            ..
                        } => match Self::decode_image(data, fmt, entry_index) {
                            Ok(img) => match Self::convert_decoded(
                                img,
                                mask,
                                entry_header.density,
                                entry_header.icc_profile,
                                downscale,
                                entry_header.orientation,
                                &resizer,
                            ) {
                                Ok((pixels, size, density, is_opaque, is_mask)) => {
                                    meta.size = size;
                                    meta.density = density;
                                    meta.is_mask = is_mask;
                                    let decoded = ImageDecoded::new(meta, pixels.clone(), is_opaque);
                                    if app_sender.send(AppEvent::ImageDecoded(decoded)).is_err() {
                                        return;
                                    }
                                    prev_pixels = pixels;
                                    prev_size = size;
                                    prev_is_opaque = is_opaque;
                                }
                                Err(e) => {
                                    return error!("{e}");
                                }
                            },
                            Err(e) => return error!("{e}"),
                        },
                        Task::SynthReduced { size, .. } => {
                            match Self::downscale_decoded(mask, Some(size), &resizer, prev_size, &prev_pixels) {
                                Ok(r) => {
                                    let (size, pixels_mut) = r.unwrap();
                                    match pixels_mut.finish_blocking() {
                                        Ok(pixels) => {
                                            meta.size = size;
                                            let decoded = ImageDecoded::new(meta, pixels.clone(), prev_is_opaque);
                                            if app_sender.send(AppEvent::ImageDecoded(decoded)).is_err() {
                                                return;
                                            }
                                            prev_pixels = pixels;
                                            prev_size = size;
                                        }
                                        Err(e) => return error!("{e}"),
                                    }
                                }
                                Err(e) => return error!("{e}"),
                            }
                        }
                    }
                }
            }
        }
    }
    fn add_downscale_entries(
        id_gen: Arc<Mutex<ImageId>>,
        app_sender: AppEventSender,
        resizer: Arc<ResizerCache>,
        source: ImageDecoded,
        entries: Vec<PxSize>,
    ) {
        if entries.is_empty() {
            return;
        }
        let mut metas = Vec::with_capacity(entries.len());
        for (i, entry) in entries.iter().enumerate() {
            let id = id_gen.lock().incr();
            let mut meta = source.meta.clone();
            meta.id = id;
            meta.size = *entry;
            meta.parent = Some(ImageEntryMetadata::new(
                source.meta.id,
                i,
                ImageEntryKind::Reduced { synthetic: true },
            ));

            metas.push(meta.clone());
            if app_sender.send(AppEvent::Notify(Event::ImageMetadataDecoded(meta))).is_err() {
                return;
            }
        }

        let mut source = source;
        for meta in metas {
            let size = meta.size;
            match Self::add_downscale_entry(&app_sender, &resizer, &source, meta) {
                Some(downscaled) => {
                    source.meta.size = size;
                    source.pixels = downscaled;
                }
                None => return,
            }
        }
    }
    fn add_downscale_entry(
        app_sender: &AppEventSender,
        resizer: &ResizerCache,
        source: &ImageDecoded,
        entry: ImageMetadata,
    ) -> Option<IpcBytes> {
        use fast_image_resize as fr;

        let px_type = if entry.is_mask { fr::PixelType::U8x4 } else { fr::PixelType::U8 };
        let source_img = fr::images::ImageRef::new(
            source.meta.size.width.0 as _,
            source.meta.size.height.0 as _,
            &source.pixels,
            px_type,
        )
        .unwrap();
        let mut dest_buf = IpcBytes::new_mut_blocking(entry.size.width.0 as usize * entry.size.height.0 as usize * px_type.size()).ok()?;
        let mut dest_img =
            fr::images::Image::from_slice_u8(entry.size.width.0 as _, entry.size.height.0 as _, &mut dest_buf[..], px_type).unwrap();

        let mut resize_opt = fr::ResizeOptions::new();
        // is already pre multiplied
        resize_opt.mul_div_alpha = false;
        // default, best quality
        resize_opt.algorithm = fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3);
        // try to reuse cache
        match resizer.try_lock() {
            Some(mut r) => r.resize(&source_img, &mut dest_img, Some(&resize_opt)),
            None => fr::Resizer::new().resize(&source_img, &mut dest_img, Some(&resize_opt)),
        }
        .unwrap();

        let pixels = dest_buf.finish_blocking().ok()?;

        app_sender
            .send(AppEvent::Notify(Event::ImageDecoded(ImageDecoded::new(
                entry,
                pixels.clone(),
                source.is_opaque,
            ))))
            .ok()?;

        Some(pixels)
    }

    pub fn forget(&mut self, id: ImageId) {
        self.images.remove(&id);
    }

    pub fn get(&self, id: ImageId) -> Option<&Image> {
        self.images.get(&id)
    }

    /// Called after receive and decode completes correctly.
    pub(crate) fn loaded(&mut self, data: ImageDecoded) {
        self.images.insert(
            data.meta.id,
            Image(Arc::new(ImageData::RawData {
                size: data.meta.size,
                range: 0..data.pixels.len(),
                pixels: data.pixels.clone(),
                is_opaque: data.is_opaque,
                density: data.meta.density,
                stripes: Mutex::new(Box::new([])),
            })),
        );

        let _ = self.app_sender.send(AppEvent::Notify(Event::ImageDecoded(data)));
    }

    pub(crate) fn on_low_memory(&mut self) {
        // app-process controls what images are dropped so hopefully it will respond the
        // memory pressure event
        self.resizer.lock().reset_internal_buffers();
    }

    pub(crate) fn clear(&mut self) {
        self.images.clear();
    }
}
#[cfg(feature = "image_any")]
struct ImageHeader {
    size: PxSize,
    orientation: image::metadata::Orientation,
    density: Option<PxDensity2d>,
    icc_profile: Option<lcms2::Profile>,
    og_color_type: image::ExtendedColorType,
}

fn image_color_type_to_vp(color_type: image::ExtendedColorType) -> ColorType {
    let channels = color_type.channel_count();
    ColorType::new(
        format!("{color_type:?}").to_uppercase().into(),
        (color_type.bits_per_pixel() / channels as u16) as u8,
        channels,
    )
}

/// (pixels, size, density, is_opaque, is_mask)
type RawLoadedImg = (IpcBytes, PxSize, Option<PxDensity2d>, bool, bool);

pub(crate) enum ImageData {
    RawData {
        size: PxSize,
        pixels: IpcBytes,
        is_opaque: bool,
        density: Option<PxDensity2d>,
        range: std::ops::Range<usize>,
        stripes: Mutex<Box<[Image]>>,
    },
    NativeTexture {
        uv: webrender::api::units::TexelRect,
        texture: gleam::gl::GLuint,
    },
}
impl ImageData {
    pub fn is_opaque(&self) -> bool {
        match self {
            ImageData::RawData { is_opaque, .. } => *is_opaque,
            ImageData::NativeTexture { .. } => false,
        }
    }

    pub fn is_mask(&self) -> bool {
        match self {
            ImageData::RawData { size, range, .. } => size.width.0 as usize * size.height.0 as usize == range.len(),
            ImageData::NativeTexture { .. } => false,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Image(Arc<ImageData>);
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.0 {
            ImageData::RawData {
                size,
                pixels,
                is_opaque,
                density,
                range,
                ..
            } => f
                .debug_struct("Image")
                .field("size", size)
                .field("is_opaque", is_opaque)
                .field("density", density)
                .field("pixels", &format_args!("<{} of {} shared bytes>", range.len(), pixels.len()))
                .finish(),
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }
}
impl Image {
    pub fn descriptor(&self) -> ImageDescriptor {
        match &*self.0 {
            ImageData::RawData {
                size, is_opaque, range, ..
            } => {
                // no Webrender mipmaps here, thats only for the GPU,
                // it does not help with performance rendering gigapixel images scaled to fit
                let mut flags = webrender::api::ImageDescriptorFlags::empty();
                if *is_opaque {
                    flags |= webrender::api::ImageDescriptorFlags::IS_OPAQUE;
                }
                let is_mask = size.width.0 as usize * size.height.0 as usize == range.len();
                ImageDescriptor {
                    format: if is_mask {
                        webrender::api::ImageFormat::R8
                    } else {
                        webrender::api::ImageFormat::BGRA8
                    },
                    size: size.cast().cast_unit(),
                    stride: None,
                    offset: 0,
                    flags,
                }
            }
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn is_opaque(&self) -> bool {
        match &*self.0 {
            ImageData::RawData { is_opaque, .. } => *is_opaque,
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn size(&self) -> PxSize {
        match &*self.0 {
            ImageData::RawData { size, .. } => *size,
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn pixels(&self) -> &IpcBytes {
        match &*self.0 {
            ImageData::RawData { pixels, .. } => pixels,
            ImageData::NativeTexture { .. } => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn density(&self) -> Option<PxDensity2d> {
        match &*self.0 {
            ImageData::RawData { density, .. } => *density,
            _ => unreachable!(),
        }
    }

    #[allow(unused)]
    pub fn range(&self) -> std::ops::Range<usize> {
        match &*self.0 {
            ImageData::RawData { range, .. } => range.clone(),
            _ => unreachable!(),
        }
    }

    /// If this is `true` needs to replace with `wr_stripes`
    pub fn overflows_wr(&self) -> bool {
        self.pixels().len() > Self::MAX_LEN
    }

    /// Returns the image split in "stripes" that fit the Webrender buffer length constraints.
    ///
    /// If the image cannot be split into stripes returns an empty list. This only happens if the image width is absurdly wide.
    pub fn wr_stripes(&self) -> Box<[Image]> {
        if !self.overflows_wr() {
            return Box::new([self.clone()]);
        }

        match &*self.0 {
            ImageData::RawData {
                size,
                pixels,
                is_opaque,
                density,
                range,
                stripes,
                ..
            } => {
                debug_assert_eq!(range.len(), pixels.len());

                let mut stripes = stripes.lock();
                if stripes.is_empty() {
                    *stripes = self.generate_stripes(*size, pixels, *is_opaque, *density);
                }
                (*stripes).clone()
            }
            _ => unreachable!(),
        }
    }
    const MAX_LEN: usize = i32::MAX as usize;
    fn generate_stripes(&self, full_size: PxSize, pixels: &IpcBytes, is_opaque: bool, density: Option<PxDensity2d>) -> Box<[Image]> {
        let w = full_size.width.0 as usize * 4;
        if w > Self::MAX_LEN {
            tracing::error!("renderer does not support images with width * 4 > {}", Self::MAX_LEN);
            return Box::new([]);
        }

        // find proportional split that fits, to avoid having the last stripe be to thin
        let full_height = full_size.height.0 as usize;
        let mut stripe_height = full_height / 2;
        while w * stripe_height > Self::MAX_LEN {
            stripe_height /= 2;
        }
        let stripe_len = w * stripe_height;
        let stripes_len = full_height.div_ceil(stripe_height);
        let stripe_height = Px(stripe_height as _);

        let mut stripes = Vec::with_capacity(stripes_len);

        for i in 0..stripes_len {
            let y = stripe_height * Px(i as _);
            let mut size = full_size;
            size.height = stripe_height.min(full_size.height - y);

            let offset = stripe_len * i;
            let range = offset..((offset + stripe_len).min(pixels.len()));

            let stripe = Image(Arc::new(ImageData::RawData {
                size,
                pixels: pixels.clone(),
                is_opaque,
                density,
                range,
                // always empty
                stripes: Mutex::new(Box::new([])),
            }));

            stripes.push(stripe);
        }

        stripes.into_boxed_slice()
    }
}

fn downscale_sizes(downscale: Option<&ImageDownscaleMode>, page_size: PxSize, reduced_sizes: &[PxSize]) -> (Option<PxSize>, Vec<PxSize>) {
    match downscale {
        Some(d) => d.sizes(page_size, reduced_sizes),
        None => (None, vec![]),
    }
}
