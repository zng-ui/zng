use std::{
    any::Any,
    env, fmt, fs, io, mem, ops,
    path::{Path, PathBuf},
    sync::Arc,
};

use parking_lot::Mutex;
use zng_app::{
    view_process::{EncodeError, VIEW_PROCESS, ViewImageHandle, ViewRenderer},
    window::WindowId,
};
use zng_color::{
    Hsla, Rgba,
    gradient::{ExtendMode, GradientStops},
};
use zng_layout::{
    context::{LAYOUT, LayoutMetrics, LayoutPassId},
    unit::{ByteLength, ByteUnits, FactorUnits as _, LayoutAxis, Px, PxDensity2d, PxLine, PxPoint, PxRect, PxSize, about_eq},
};
use zng_task::{
    self as task,
    channel::{IpcBytes, IpcBytesMut},
};
use zng_txt::Txt;
use zng_var::{Var, VarEq, animation::Transitionable, impl_from_and_into_var};
use zng_view_api::image::{ImageDecoded, ImageEncodeRequest, ImageId, ImageMetadata, ImageTextureId};

use crate::ImageRenderWindowRoot;

pub use zng_view_api::image::{
    ColorType, ImageDataFormat, ImageDownscaleMode, ImageEntriesMode, ImageEntryKind, ImageFormat, ImageFormatCapability, ImageMaskMode,
    PartialImageKind,
};

/// A custom extension for the [`IMAGES`] service.
///
/// Extensions can intercept and modify requests.
///
/// [`IMAGES`]: crate::IMAGES
pub trait ImagesExtension: Send + Sync + Any {
    /// Modify a [`IMAGES.image`] request.
    ///
    /// Note that all other request methods are shorthand helpers so this will be called for every request.
    ///
    /// Note that the [`IMAGES`] service can be used in extensions and [`ImageSource::Image`] is returned directly by the service.
    /// This can be used to fully replace a request here.
    ///
    /// [`IMAGES.image`]: crate::IMAGES::image
    /// [`IMAGES`]: crate::IMAGES
    fn image(&mut self, limits: &ImageLimits, source: &mut ImageSource, options: &mut ImageOptions) {
        let _ = (limits, source, options);
    }

    /// Image data loaded.
    ///
    /// This is called for [`ImageSource::Read`], [`ImageSource::Download`] and [`ImageSource::Data`] after the data is loaded and before
    /// decoding starts.
    ///
    /// Return a replacement variable to skip decoding or redirect to a different image. Note that by the time this is called the service
    /// has already returned a variable in loading state, that variable will be cached according to `mode`. The replacement variable
    /// is bound to the return variable and lives as long as it does.
    ///
    /// Note that the [`IMAGES`] service can be used in extensions.
    ///
    /// [`IMAGES`]: crate::IMAGES
    #[allow(clippy::too_many_arguments)]
    fn image_data(
        &mut self,
        max_decoded_len: ByteLength,
        key: &ImageHash,
        data: &IpcBytes,
        format: &ImageDataFormat,
        options: &ImageOptions,
    ) -> Option<ImageVar> {
        let _ = (max_decoded_len, key, data, format, options);
        None
    }

    /// Modify a [`IMAGES.clean`] or [`IMAGES.purge`] request.
    ///
    /// Return `false` to cancel the removal.
    ///
    /// [`IMAGES.clean`]: crate::IMAGES::clean
    /// [`IMAGES.purge`]: crate::IMAGES::purge
    fn remove(&mut self, key: &mut ImageHash, purge: &mut bool) -> bool {
        let _ = (key, purge);
        true
    }

    /// Called on [`IMAGES.clean_all`] and [`IMAGES.purge_all`].
    ///
    /// These operations cannot be intercepted, the service cache will be cleaned after this call.
    ///
    /// [`IMAGES.clean_all`]: crate::IMAGES::clean_all
    /// [`IMAGES.purge_all`]: crate::IMAGES::purge_all
    fn clear(&mut self, purge: bool) {
        let _ = purge;
    }

    /// Add or remove formats this extension affects.
    ///
    /// The `formats` value starts with all formats implemented by the current view-process and will be returned
    /// by [`IMAGES.available_formats`] after all proxies edit it.
    ///
    /// [`IMAGES.available_formats`]: crate::IMAGES::available_formats
    fn available_formats(&self, formats: &mut Vec<ImageFormat>) {
        let _ = formats;
    }
}

/// Represents an [`Img`] tracked by the [`IMAGES`] cache.
///
/// The variable updates when the image updates.
///
/// [`IMAGES`]: super::IMAGES
pub type ImageVar = Var<ImageEntry>;

#[derive(Default, Debug)]
struct ImgMut {
    render_ids: Vec<RenderImage>,
}

/// State of an [`ImageVar`].
///
/// [`IMAGES`]: crate::IMAGES
#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub(crate) cache_key: Option<ImageHash>,

    pub(crate) handle: ViewImageHandle,
    data: ImageDecoded,
    entries: Vec<VarEq<ImageEntry>>,

    error: Txt,

    img_mut: Arc<Mutex<ImgMut>>,
}
impl PartialEq for ImageEntry {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
            && self.cache_key == other.cache_key
            && self.error == other.error
            && self.data == other.data
            && self.entries == other.entries
    }
}
impl ImageEntry {
    pub(super) fn new_cached(cache_key: ImageHash) -> Self {
        let mut s = Self::new_loading(ViewImageHandle::dummy());
        s.cache_key = Some(cache_key);
        s
    }

    pub(super) fn new_loading(handle: ViewImageHandle) -> Self {
        Self {
            cache_key: None,
            handle,
            data: ImageDecoded::new(
                ImageMetadata::new(ImageId::INVALID, PxSize::zero(), false, ColorType::BGRA8),
                IpcBytes::default(),
                true,
            ),
            entries: vec![],
            error: Txt::from_static(""),
            img_mut: Arc::default(),
        }
    }

    /// New from existing view image.
    pub(super) fn new(handle: ViewImageHandle, data: ImageDecoded) -> Self {
        Self {
            cache_key: None,
            handle,
            data,
            entries: vec![],
            error: Txt::from_static(""),
            img_mut: Arc::default(),
        }
    }

    /// Create a dummy image in the loading or error state.
    ///
    /// Note that you can use the [`IMAGES.register`] method to integrate with images from other sources. The intention
    /// of this function is creating an initial loading image or an error message image.
    ///
    /// [`IMAGES.register`]: crate::IMAGES::register
    pub fn new_empty(error: Txt) -> Self {
        let mut s = Self::new_loading(ViewImageHandle::dummy());
        s.error = error;
        s
    }

    /// Returns `true` if the is still acquiring or decoding the image bytes.
    pub fn is_loading(&self) -> bool {
        self.error.is_empty() && (self.data.pixels.is_empty() || self.data.partial.is_some())
    }

    /// If the image is has finished loading ok or due to error.
    ///
    /// The image variable may still update after
    pub fn is_loaded(&self) -> bool {
        !self.is_loading()
    }

    /// If the image failed to load.
    pub fn is_error(&self) -> bool {
        !self.error.is_empty()
    }

    /// Returns an error message if the image failed to load.
    pub fn error(&self) -> Option<Txt> {
        if self.error.is_empty() { None } else { Some(self.error.clone()) }
    }

    /// Pixel size of the image after it finishes loading.
    ///
    /// Note that this value is set as soon as the header finishes decoding, but the [`pixels`] will
    /// only be set after it the entire image decodes.
    ///
    /// If the view-process implements progressive decoding you can use [`partial_size`] and [`partial_pixels`]
    /// to use the partially decoded image top rows as it decodes.
    ///
    /// [`pixels`]: Self::pixels
    /// [`partial_size`]: Self::partial_size
    /// [`partial_pixels`]: Self::partial_pixels
    pub fn size(&self) -> PxSize {
        self.data.meta.size
    }

    /// Size of [`partial_pixels`].
    ///
    /// Can be different from [`size`] if the image is progressively decoding.
    ///
    /// [`size`]: Self::size
    /// [`partial_pixels`]: Self::partial_pixels
    pub fn partial_size(&self) -> Option<PxSize> {
        match self.data.partial.as_ref()? {
            PartialImageKind::Placeholder { pixel_size } => Some(*pixel_size),
            PartialImageKind::Rows { height, .. } => Some(PxSize::new(self.data.meta.size.width, *height)),
            _ => None,
        }
    }

    /// Kind of [`partial_pixels`].
    ///
    /// [`partial_pixels`]: Self::partial_pixels
    pub fn partial_kind(&self) -> Option<PartialImageKind> {
        self.data.partial.clone()
    }

    /// Returns the image pixel density metadata if the image is loaded and the
    /// metadata was retrieved.
    pub fn density(&self) -> Option<PxDensity2d> {
        self.data.meta.density
    }

    /// Image color type before it was converted to BGRA8 or A8.
    pub fn original_color_type(&self) -> ColorType {
        self.data.meta.original_color_type.clone()
    }

    /// Gets A8 for masks and BGRA8 for others.
    pub fn color_type(&self) -> ColorType {
        if self.is_mask() { ColorType::A8 } else { ColorType::BGRA8 }
    }

    /// Returns `true` if the image is fully opaque or it is not loaded.
    pub fn is_opaque(&self) -> bool {
        self.data.is_opaque
    }

    /// Returns `true` if the image pixels are a single channel (A8).
    pub fn is_mask(&self) -> bool {
        self.data.meta.is_mask
    }

    /// If [`entries`] is not empty.
    ///
    /// [`entries`]: Self::entries
    pub fn has_entries(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Other images from the same container that are a *child* of this image.
    pub fn entries(&self) -> Vec<ImageVar> {
        self.entries.iter().map(|e| e.read_only()).collect()
    }

    /// All other images from the same container that are a *descendant* of this image.
    ///
    /// The values are a tuple of each entry and the length of descendants entries that follow it.
    ///
    /// The returned variable will update every time any entry descendant var updates.
    ///
    /// [`entries`]: Self::entries
    pub fn flat_entries(&self) -> Var<Vec<(VarEq<ImageEntry>, usize)>> {
        // idea here is to just rebuild the flat list on any update,
        // assuming the image variables don't update much and tha there are not many entries
        // this is more simple than some sort of recursive Var::flat_map_vec setup

        // each entry updates this var on update
        let update_signal = zng_var::var(());

        // init value and update bindings
        let mut out = vec![];
        let mut update_handles = vec![];
        self.flat_entries_init(&mut out, update_signal.clone(), &mut update_handles);
        let out = zng_var::var(out);

        // bind signal to rebuild list on update and rebind update signal
        let self_ = self.clone();
        let signal_weak = update_signal.downgrade();
        update_signal
            .bind_modify(&out, move |_, out| {
                out.clear();
                update_handles.clear();
                self_.flat_entries_init(&mut *out, signal_weak.upgrade().unwrap(), &mut update_handles);
            })
            .perm();
        out.hold(update_signal).perm();
        out.read_only()
    }
    fn flat_entries_init(&self, out: &mut Vec<(VarEq<ImageEntry>, usize)>, update_signal: Var<()>, handles: &mut Vec<zng_var::VarHandle>) {
        for entry in self.entries.iter() {
            Self::flat_entries_recursive_init(entry.clone(), out, update_signal.clone(), handles);
        }
    }
    fn flat_entries_recursive_init(
        img: VarEq<ImageEntry>,
        out: &mut Vec<(VarEq<ImageEntry>, usize)>,
        signal: Var<()>,
        handles: &mut Vec<zng_var::VarHandle>,
    ) {
        handles.push(img.hook(zng_clone_move::clmv!(signal, |_| {
            signal.update();
            true
        })));
        let i = out.len();
        out.push((img.clone(), 0));
        img.with(move |img| {
            for entry in img.entries.iter() {
                Self::flat_entries_recursive_init(entry.clone(), out, signal.clone(), handles);
            }
            let len = out.len() - i;
            out[i].1 = len;
        });
    }

    /// Kind of image entry this image is in the source container.
    pub fn entry_kind(&self) -> ImageEntryKind {
        match &self.data.meta.parent {
            Some(p) => p.kind.clone(),
            None => ImageEntryKind::Page,
        }
    }

    /// Sort index of the image in the list of entries of the source container.
    pub fn entry_index(&self) -> usize {
        match &self.data.meta.parent {
            Some(p) => p.index,
            None => 0,
        }
    }

    /// Calls `visit` with the image or [`ImageEntryKind::Reduced`] entry that is nearest to `size` and greater or equal to it.
    ///
    /// Does not call `visit` if none of the images are loaded, returns `None` in that case.
    pub fn with_best_reduce<R>(&self, size: PxSize, visit: impl FnOnce(&ImageEntry) -> R) -> Option<R> {
        fn cmp(target_size: PxSize, a: PxSize, b: PxSize) -> std::cmp::Ordering {
            let target_ratio = target_size.width.0 as f32 / target_size.height.0 as f32;
            let a_ratio = a.width.0 as f32 / b.height.0 as f32;
            let b_ratio = b.width.0 as f32 / b.height.0 as f32;

            let a_distortion = (target_ratio - a_ratio).abs();
            let b_distortion = (target_ratio - b_ratio).abs();

            if !about_eq(a_distortion, b_distortion, 0.01) && a_distortion < b_distortion {
                // prefer a, has less distortion
                return std::cmp::Ordering::Less;
            }

            let a_dist = a - target_size;
            let b_dist = b - target_size;

            if a_dist.width < Px(0) || a_dist.height < Px(0) {
                if b_dist.width < Px(0) || b_dist.height < Px(0) {
                    // a and b need upscaling, prefer near target_size
                    a_dist.width.abs().cmp(&b_dist.width.abs())
                } else {
                    // prefer b, a needs upscaling
                    std::cmp::Ordering::Greater
                }
            } else if b_dist.width < Px(0) || b_dist.height < Px(0) {
                // prefer a, b needs upscaling
                std::cmp::Ordering::Less
            } else {
                // a and b need downscaling, prefer near target_size
                a_dist.width.cmp(&b_dist.width)
            }
        }

        let mut best_i = usize::MAX;
        let mut best_size = PxSize::zero();

        if self.is_loaded() {
            best_i = self.entries.len();
            best_size = self.size();
        }

        for (i, entry) in self.entries.iter().enumerate() {
            entry.with(|e| {
                if e.is_loaded() {
                    let entry_size = e.size();
                    if cmp(size, entry_size, best_size).is_lt() {
                        best_i = i;
                        best_size = entry_size;
                    }
                }
            })
        }

        if best_i == usize::MAX {
            // image and all reduced are smaller than `size`, return the largest to reduce upscaling
            None
        } else if best_i == self.entries.len() {
            Some(visit(self))
        } else {
            Some(self.entries[best_i].with(visit))
        }
    }

    /// Connection to the image resource in the view-process.
    pub fn view_handle(&self) -> ViewImageHandle {
        self.handle.clone()
    }

    /// Calculate an *ideal* layout size for the image.
    ///
    /// The image is scaled considering the [`density`] and screen scale factor. If the
    /// image has no [`density`] falls back to the [`screen_density`] in both dimensions.
    ///
    /// [`density`]: Self::density
    /// [`screen_density`]: LayoutMetrics::screen_density
    pub fn layout_size(&self, ctx: &LayoutMetrics) -> PxSize {
        self.calc_size(ctx, PxDensity2d::splat(ctx.screen_density()), false)
    }

    /// Calculate a layout size for the image.
    ///
    /// # Parameters
    ///
    /// * `ctx`: Used to get the screen resolution.
    /// * `fallback_density`: Resolution used if [`density`] is `None`.
    /// * `ignore_image_density`: If `true` always uses the `fallback_density` as the resolution.
    ///
    /// [`density`]: Self::density
    pub fn calc_size(&self, ctx: &LayoutMetrics, fallback_density: PxDensity2d, ignore_image_density: bool) -> PxSize {
        let dpi = if ignore_image_density {
            fallback_density
        } else {
            self.density().unwrap_or(fallback_density)
        };

        let s_density = ctx.screen_density();
        let mut size = self.size();

        let fct = ctx.scale_factor().0;
        size.width *= (s_density.ppcm() / dpi.width.ppcm()) * fct;
        size.height *= (s_density.ppcm() / dpi.height.ppcm()) * fct;

        size
    }

    /// Reference the decoded pre-multiplied BGRA8 pixel buffer or A8 if [`is_mask`].
    ///
    /// [`is_mask`]: Self::is_mask
    pub fn pixels(&self) -> Option<IpcBytes> {
        if self.is_loaded() { Some(self.data.pixels.clone()) } else { None }
    }

    /// Reference the partially decoded pixels if the image is progressively decoding
    /// and has not finished decoding.
    ///
    /// Format is BGRA8 for normal images or A8 if [`is_mask`].
    ///
    /// [`is_mask`]: Self::is_mask
    pub fn partial_pixels(&self) -> Option<IpcBytes> {
        if self.is_loading() && self.data.partial.is_some() {
            Some(self.data.pixels.clone())
        } else {
            None
        }
    }

    fn actual_pixels_and_size(&self) -> Option<(PxSize, IpcBytes)> {
        match (self.partial_pixels(), self.partial_size()) {
            (Some(b), Some(s)) => Some((s, b)),
            _ => Some((self.size(), self.pixels()?)),
        }
    }

    /// Copy the `rect` selection from `pixels` or `partial_pixels`.
    ///
    /// The `rect` is in pixels, with the origin (0, 0) at the top-left of the image.
    ///
    /// Returns the copied selection and the pixel buffer.
    ///
    /// Note that the selection can change if `rect` is not fully contained by the image area.
    pub fn copy_pixels(&self, rect: PxRect) -> Option<(PxRect, IpcBytesMut)> {
        self.actual_pixels_and_size().and_then(|(size, pixels)| {
            let area = PxRect::from_size(size).intersection(&rect).unwrap_or_default();
            if area.size.width.0 == 0 || area.size.height.0 == 0 {
                Some((area, IpcBytes::new_mut_blocking(0).unwrap()))
            } else {
                let x = area.origin.x.0 as usize;
                let y = area.origin.y.0 as usize;
                let width = area.size.width.0 as usize;
                let height = area.size.height.0 as usize;
                let pixel = if self.is_mask() { 1 } else { 4 };
                let mut bytes = IpcBytes::new_mut_blocking(width * height * pixel).ok()?;
                let mut write = &mut bytes[..];
                let row_stride = self.size().width.0 as usize * pixel;
                for l in y..y + height {
                    let line_start = l * row_stride + x * pixel;
                    let line_end = line_start + width * pixel;
                    let line = &pixels[line_start..line_end];
                    write[..line.len()].copy_from_slice(line);
                    write = &mut write[line.len()..];
                }
                Some((area, bytes))
            }
        })
    }

    /// Encode the image to the format.
    ///
    /// Note that [`entries`] are ignored, only this image is encoded. Use [`encode_with_entries`] to encode
    /// multiple images in the same container.
    ///
    /// [`entries`]: Self::entries
    /// [`encode_with_entries`]: Self::encode_with_entries
    pub async fn encode(&self, format: Txt) -> std::result::Result<IpcBytes, EncodeError> {
        self.encode_with_entries(&[], format).await
    }

    /// Encode the images to the format.
    ///
    /// This image is the first *page* followed by the `entries` in the given order.
    pub async fn encode_with_entries(
        &self,
        entries: &[(ImageEntry, ImageEntryKind)],
        format: Txt,
    ) -> std::result::Result<IpcBytes, EncodeError> {
        if self.is_loading() {
            if self.handle.is_dummy() {
                return Err(EncodeError::Dummy);
            }
            return Err(EncodeError::Loading);
        } else if let Some(e) = self.error() {
            return Err(e.into());
        }

        let mut r = ImageEncodeRequest::new(self.handle.image_id(), format);
        r.entries = entries.iter().map(|(img, kind)| (img.handle.image_id(), kind.clone())).collect();

        match VIEW_PROCESS.encode_image(r).recv().await {
            Ok(r) => r,
            Err(_) => Err(EncodeError::Disconnected),
        }
    }

    /// Encode and write the image to `path`.
    ///
    /// The image format is guessed from the file extension. Use [`save_with_format`] to specify the format.
    ///
    /// Note that [`entries`] are ignored, only this image is encoded. Use [`save_with_entries`] to encode
    /// multiple images in the same container.
    ///
    /// [`entries`]: Self::entries
    /// [`save_with_format`]: Self::save_with_format
    /// [`save_with_entries`]: Self::save_with_entries
    pub async fn save(&self, path: impl Into<PathBuf>) -> io::Result<()> {
        let path = path.into();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            self.save_impl(&[], Txt::from_str(ext), path).await
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
    ///
    /// Note that [`entries`] are ignored, only this image is encoded. Use [`save_with_entries`] to encode
    /// multiple images in the same container.
    ///
    /// [`entries`]: Self::entries
    /// [`save_with_entries`]: Self::save_with_entries
    pub async fn save_with_format(&self, format: impl Into<Txt>, path: impl Into<PathBuf>) -> io::Result<()> {
        self.save_impl(&[], format.into(), path.into()).await
    }

    /// Encode and write the image to `path`.
    ///
    /// The image is encoded to the `format`, the file extension can be anything.
    ///
    /// This image is the first *page* followed by the `entries` in the given order.
    pub async fn save_with_entries(
        &self,
        entries: &[(ImageEntry, ImageEntryKind)],
        format: impl Into<Txt>,
        path: impl Into<PathBuf>,
    ) -> io::Result<()> {
        self.save_impl(entries, format.into(), path.into()).await
    }

    async fn save_impl(&self, entries: &[(ImageEntry, ImageEntryKind)], format: Txt, path: PathBuf) -> io::Result<()> {
        let data = self.encode_with_entries(entries, format).await.map_err(io::Error::other)?;
        task::wait(move || fs::write(path, &data[..])).await
    }

    pub(crate) fn find_entry(&self, id: zng_view_api::image::ImageId) -> Option<ImageVar> {
        self.entries.iter().find_map(|v| {
            if v.with(|i| i.handle.image_id() == id) {
                Some(v.0.clone())
            } else {
                v.with(|i| i.find_entry(id))
            }
        })
    }

    pub(crate) fn has_loading_entries(&self) -> bool {
        self.entries.iter().any(|e| e.with(|e| e.is_loading() || e.has_loading_entries()))
    }

    pub(crate) fn set_handle(&mut self, handle: ViewImageHandle) {
        self.handle = handle;
    }

    pub(crate) fn set_data(&mut self, data: ImageDecoded) {
        self.data = data;
        self.error = Txt::from_static("");
    }

    pub(crate) fn set_error(&mut self, error: Txt) {
        self.error = error;
    }

    /// Insert `entry` in [`entries`].
    ///
    /// [`entries`]: Self::entries
    pub fn insert_entry(&mut self, image: ImageEntry) -> ImageVar {
        let i = image.entry_index();
        let i = self
            .entries
            .iter()
            .position(|v| {
                let entry_i = v.with(|i| i.entry_index());
                entry_i > i
            })
            .unwrap_or(self.entries.len());
        let entry = zng_var::var(image);
        self.entries.insert(i, VarEq(entry.clone()));
        entry
    }
}
impl zng_app::render::Img for ImageEntry {
    fn renderer_id(&self, renderer: &ViewRenderer) -> ImageTextureId {
        if self.is_loaded() {
            let mut img = self.img_mut.lock();
            let rms = &mut img.render_ids;
            if let Some(rm) = rms.iter().find(|k| &k.renderer == renderer) {
                return rm.image_id;
            }

            let key = match renderer.use_image(&self.handle) {
                Ok(k) => {
                    if k == ImageTextureId::INVALID {
                        tracing::error!("received INVALID from `use_image`");
                        return k;
                    }
                    k
                }
                Err(_) => {
                    tracing::debug!("respawned `add_image`, will return INVALID");
                    return ImageTextureId::INVALID;
                }
            };

            rms.push(RenderImage {
                image_id: key,
                renderer: renderer.clone(),
            });
            key
        } else {
            ImageTextureId::INVALID
        }
    }

    fn size(&self) -> PxSize {
        self.size()
    }
}

struct RenderImage {
    image_id: ImageTextureId,
    renderer: ViewRenderer,
}
impl Drop for RenderImage {
    fn drop(&mut self) {
        // error here means the entire renderer was dropped.
        let _ = self.renderer.delete_image_use(self.image_id);
    }
}
impl fmt::Debug for RenderImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.image_id, f)
    }
}

/// A 256-bit hash for image entries.
///
/// This hash is used to identify image files in the [`IMAGES`] cache.
///
/// Use [`ImageHasher`] to compute.
///
/// [`IMAGES`]: super::IMAGES
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
            use base64::*;
            write!(f, "{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0))
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
    pub fn update(&mut self, data: &[u8]) {
        use sha2::Digest;

        // some gigantic images can take to long to hash, we just
        // need the hash for identification so we sample the data
        const NUM_SAMPLES: usize = 1000;
        const SAMPLE_CHUNK_SIZE: usize = 1024;

        let total_size = data.len();
        if total_size == 0 {
            return;
        }
        if total_size < 1000 * 1000 * 4 {
            return self.0.update(data);
        }

        let step_size = total_size.checked_div(NUM_SAMPLES).unwrap_or(total_size);
        for n in 0..NUM_SAMPLES {
            let start_index = n * step_size;
            if start_index >= total_size {
                break;
            }
            let end_index = (start_index + SAMPLE_CHUNK_SIZE).min(total_size);
            let s = &data[start_index..end_index];
            self.0.update(s);
        }
    }

    /// Finish computing the hash.
    pub fn finish(self) -> ImageHash {
        use sha2::Digest;
        // dependencies `sha2 -> digest` need to upgrade
        // https://github.com/RustCrypto/traits/issues/2036
        // https://github.com/fizyk20/generic-array/issues/158
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

// We don't use Arc<dyn ..> because of this issue: https://github.com/rust-lang/rust/issues/69757
type RenderFn = Arc<Box<dyn Fn(&ImageRenderArgs) -> Box<dyn ImageRenderWindowRoot> + Send + Sync>>;

/// Arguments for the [`ImageSource::Render`] closure.
///
/// The arguments are set by the widget that will render the image.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ImageRenderArgs {
    /// Window that will render the image.
    pub parent: Option<WindowId>,
}
impl ImageRenderArgs {
    /// New with parent window.
    pub fn new(parent: WindowId) -> Self {
        Self { parent: Some(parent) }
    }
}

/// The different sources of an image resource.
#[derive(Clone)]
#[non_exhaustive]
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
    #[cfg(feature = "http")]
    Download(crate::task::http::Uri, Option<Txt>),
    /// Shared reference to bytes for an encoded or decoded image.
    ///
    /// Image equality is defined by the hash, it is usually the hash of the bytes but it does not need to be.
    ///
    /// Inside [`IMAGES`] the reference to the bytes is held only until the image finishes decoding.
    ///
    /// [`IMAGES`]: super::IMAGES
    Data(ImageHash, IpcBytes, ImageDataFormat),

    /// A boxed closure that instantiates a `WindowRoot` that draws the image.
    ///
    /// Use the [`render`](Self::render) or [`render_node`](Self::render_node) functions to construct this variant.
    ///
    /// The closure is set by the image widget user, the args is set by the image widget.
    Render(RenderFn, Option<ImageRenderArgs>),

    /// Already resolved (loaded or loading) image.
    ///
    /// The image is passed-through, not cached.
    Image(ImageVar),
}
impl ImageSource {
    /// New source from data.
    pub fn from_data(data: IpcBytes, format: ImageDataFormat) -> Self {
        let mut hasher = ImageHasher::default();
        hasher.update(&data[..]);
        let hash = hasher.finish();
        Self::Data(hash, data, format)
    }

    /// Returns the image hash, unless the source is [`Img`].
    pub fn hash128(&self, options: &ImageOptions) -> Option<ImageHash> {
        match self {
            ImageSource::Read(p) => Some(Self::hash128_read(p, options)),
            #[cfg(feature = "http")]
            ImageSource::Download(u, a) => Some(Self::hash128_download(u, a, options)),
            ImageSource::Data(h, _, _) => Some(Self::hash128_data(*h, options)),
            ImageSource::Render(rfn, args) => Some(Self::hash128_render(rfn, args, options)),
            ImageSource::Image(_) => None,
        }
    }

    /// Compute hash for a borrowed [`Data`] image.
    ///
    /// [`Data`]: Self::Data
    pub fn hash128_data(data_hash: ImageHash, options: &ImageOptions) -> ImageHash {
        if options.downscale.is_some() || options.mask.is_some() || !options.entries.is_empty() {
            use std::hash::Hash;
            let mut h = ImageHash::hasher();
            data_hash.0.hash(&mut h);
            options.downscale.hash(&mut h);
            options.mask.hash(&mut h);
            options.entries.hash(&mut h);
            h.finish()
        } else {
            data_hash
        }
    }

    /// Compute hash for a borrowed [`Read`] path.
    ///
    /// [`Read`]: Self::Read
    pub fn hash128_read(path: &Path, options: &ImageOptions) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        0u8.hash(&mut h);
        path.hash(&mut h);
        options.downscale.hash(&mut h);
        options.mask.hash(&mut h);
        options.entries.hash(&mut h);
        h.finish()
    }

    /// Compute hash for a borrowed [`Download`] URI and HTTP-ACCEPT.
    ///
    /// [`Download`]: Self::Download
    #[cfg(feature = "http")]
    pub fn hash128_download(uri: &crate::task::http::Uri, accept: &Option<Txt>, options: &ImageOptions) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        1u8.hash(&mut h);
        uri.hash(&mut h);
        accept.hash(&mut h);
        options.downscale.hash(&mut h);
        options.mask.hash(&mut h);
        options.entries.hash(&mut h);
        h.finish()
    }

    /// Compute hash for a borrowed [`Render`] source.
    ///
    /// Pointer equality is used to identify the node closure.
    ///
    /// [`Render`]: Self::Render
    pub fn hash128_render(rfn: &RenderFn, args: &Option<ImageRenderArgs>, options: &ImageOptions) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        2u8.hash(&mut h);
        (Arc::as_ptr(rfn) as usize).hash(&mut h);
        args.hash(&mut h);
        options.downscale.hash(&mut h);
        options.mask.hash(&mut h);
        options.entries.hash(&mut h);
        h.finish()
    }
}

impl ImageSource {
    /// New image data from solid color.
    pub fn flood(size: impl Into<PxSize>, color: impl Into<Rgba>, density: Option<PxDensity2d>) -> Self {
        Self::flood_impl(size.into(), color.into(), density)
    }
    fn flood_impl(size: PxSize, color: Rgba, density: Option<PxDensity2d>) -> Self {
        let pixels = size.width.0 as usize * size.height.0 as usize;
        let bgra = color.to_bgra_bytes();
        let mut b = IpcBytes::new_mut_blocking(pixels * 4).expect("cannot allocate IpcBytes");
        for b in b.chunks_exact_mut(4) {
            b.copy_from_slice(&bgra);
        }
        Self::from_data(
            b.finish_blocking().expect("cannot allocate IpcBytes"),
            ImageDataFormat::Bgra8 {
                size,
                density,
                original_color_type: ColorType::RGBA8,
            },
        )
    }

    /// New image data from vertical linear gradient.
    pub fn linear_vertical(
        size: impl Into<PxSize>,
        stops: impl Into<GradientStops>,
        density: Option<PxDensity2d>,
        mask: Option<ImageMaskMode>,
    ) -> Self {
        Self::linear_vertical_impl(size.into(), stops.into(), density, mask)
    }
    fn linear_vertical_impl(size: PxSize, stops: GradientStops, density: Option<PxDensity2d>, mask: Option<ImageMaskMode>) -> Self {
        assert!(size.width > Px(0));
        assert!(size.height > Px(0));

        let mut line = PxLine::new(PxPoint::splat(Px(0)), PxPoint::new(Px(0), size.height));
        let mut render_stops = vec![];

        LAYOUT.with_root_context(LayoutPassId::new(), LayoutMetrics::new(1.fct(), size, Px(14)), || {
            stops.layout_linear(LayoutAxis::Y, ExtendMode::Clamp, &mut line, &mut render_stops);
        });
        let line_a = line.start.y.0 as f32;
        let line_b = line.end.y.0 as f32;

        let mut bgra = Vec::with_capacity(size.height.0 as usize);
        let mut render_stops = render_stops.into_iter();
        let mut stop_a = render_stops.next().unwrap();
        let mut stop_b = render_stops.next().unwrap();
        'outer: for y in 0..size.height.0 {
            let yf = y as f32;
            let yf = (yf - line_a) / (line_b - line_a);
            if yf < stop_a.offset {
                // clamp start
                bgra.push(stop_a.color.to_bgra_bytes());
                continue;
            }
            while yf > stop_b.offset {
                if let Some(next_b) = render_stops.next() {
                    // advance
                    stop_a = stop_b;
                    stop_b = next_b;
                } else {
                    // clamp end
                    for _ in y..size.height.0 {
                        bgra.push(stop_b.color.to_bgra_bytes());
                    }
                    break 'outer;
                }
            }

            // lerp a-b
            let yf = (yf - stop_a.offset) / (stop_b.offset - stop_a.offset);
            let sample = stop_a.color.lerp(&stop_b.color, yf.fct());
            bgra.push(sample.to_bgra_bytes());
        }

        match mask {
            Some(m) => {
                let len = size.width.0 as usize * size.height.0 as usize;
                let mut data = Vec::with_capacity(len);

                for y in 0..size.height.0 {
                    let c = bgra[y as usize];
                    let c = match m {
                        ImageMaskMode::A => c[3],
                        ImageMaskMode::B => c[0],
                        ImageMaskMode::G => c[1],
                        ImageMaskMode::R => c[2],
                        ImageMaskMode::Luminance => {
                            let hsla = Hsla::from(Rgba::new(c[2], c[1], c[0], c[3]));
                            (hsla.lightness * 255.0).round().clamp(0.0, 255.0) as u8
                        }
                        _ => unreachable!(),
                    };
                    for _x in 0..size.width.0 {
                        data.push(c);
                    }
                }

                Self::from_data(
                    IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes"),
                    ImageDataFormat::A8 { size },
                )
            }
            None => {
                let len = size.width.0 as usize * size.height.0 as usize * 4;
                let mut data = Vec::with_capacity(len);

                for y in 0..size.height.0 {
                    let c = bgra[y as usize];
                    for _x in 0..size.width.0 {
                        data.extend_from_slice(&c);
                    }
                }

                Self::from_data(
                    IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes"),
                    ImageDataFormat::Bgra8 {
                        size,
                        density,
                        original_color_type: ColorType::RGBA8,
                    },
                )
            }
        }
    }

    /// New image data from horizontal linear gradient.
    pub fn linear_horizontal(
        size: impl Into<PxSize>,
        stops: impl Into<GradientStops>,
        density: Option<PxDensity2d>,
        mask: Option<ImageMaskMode>,
    ) -> Self {
        Self::linear_horizontal_impl(size.into(), stops.into(), density, mask)
    }
    fn linear_horizontal_impl(size: PxSize, stops: GradientStops, density: Option<PxDensity2d>, mask: Option<ImageMaskMode>) -> Self {
        assert!(size.width > Px(0));
        assert!(size.height > Px(0));

        let mut line = PxLine::new(PxPoint::splat(Px(0)), PxPoint::new(size.width, Px(0)));
        let mut render_stops = vec![];
        LAYOUT.with_root_context(LayoutPassId::new(), LayoutMetrics::new(1.fct(), size, Px(14)), || {
            stops.layout_linear(LayoutAxis::Y, ExtendMode::Clamp, &mut line, &mut render_stops);
        });
        let line_a = line.start.x.0 as f32;
        let line_b = line.end.x.0 as f32;

        let mut bgra = Vec::with_capacity(size.width.0 as usize);
        let mut render_stops = render_stops.into_iter();
        let mut stop_a = render_stops.next().unwrap();
        let mut stop_b = render_stops.next().unwrap();
        'outer: for x in 0..size.width.0 {
            let xf = x as f32;
            let xf = (xf - line_a) / (line_b - line_a);
            if xf < stop_a.offset {
                // clamp start
                bgra.push(stop_a.color.to_bgra_bytes());
                continue;
            }
            while xf > stop_b.offset {
                if let Some(next_b) = render_stops.next() {
                    // advance
                    stop_a = stop_b;
                    stop_b = next_b;
                } else {
                    // clamp end
                    for _ in x..size.width.0 {
                        bgra.push(stop_b.color.to_bgra_bytes());
                    }
                    break 'outer;
                }
            }

            // lerp a-b
            let xf = (xf - stop_a.offset) / (stop_b.offset - stop_a.offset);
            let sample = stop_a.color.lerp(&stop_b.color, xf.fct());
            bgra.push(sample.to_bgra_bytes());
        }

        match mask {
            Some(m) => {
                let len = size.width.0 as usize * size.height.0 as usize;
                let mut data = Vec::with_capacity(len);

                for _y in 0..size.height.0 {
                    for c in &bgra {
                        let c = match m {
                            ImageMaskMode::A => c[3],
                            ImageMaskMode::B => c[0],
                            ImageMaskMode::G => c[1],
                            ImageMaskMode::R => c[2],
                            ImageMaskMode::Luminance => {
                                let hsla = Hsla::from(Rgba::new(c[2], c[1], c[0], c[3]));
                                (hsla.lightness * 255.0).round().clamp(0.0, 255.0) as u8
                            }
                            _ => unreachable!(),
                        };
                        data.push(c);
                    }
                }

                Self::from_data(
                    IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes"),
                    ImageDataFormat::A8 { size },
                )
            }
            None => {
                let len = size.width.0 as usize * size.height.0 as usize * 4;
                let mut data = Vec::with_capacity(len);

                for _y in 0..size.height.0 {
                    for c in &bgra {
                        data.extend_from_slice(c);
                    }
                }

                Self::from_data(
                    IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes"),
                    ImageDataFormat::Bgra8 {
                        size,
                        density,
                        original_color_type: ColorType::RGBA8,
                    },
                )
            }
        }
    }
}

impl PartialEq for ImageSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Read(l), Self::Read(r)) => l == r,
            #[cfg(feature = "http")]
            (Self::Download(lu, la), Self::Download(ru, ra)) => lu == ru && la == ra,
            (Self::Render(lf, la), Self::Render(rf, ra)) => Arc::ptr_eq(lf, rf) && la == ra,
            (Self::Image(l), Self::Image(r)) => l.var_eq(r),
            (l, r) => {
                let l_hash = match l {
                    ImageSource::Data(h, _, _) => h,
                    _ => return false,
                };
                let r_hash = match r {
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
            #[cfg(feature = "http")]
            ImageSource::Download(u, a) => f.debug_tuple("Download").field(u).field(a).finish(),
            ImageSource::Data(key, bytes, fmt) => f.debug_tuple("Data").field(key).field(bytes).field(fmt).finish(),

            ImageSource::Render(_, _) => write!(f, "Render(_, _)"),
            ImageSource::Image(_) => write!(f, "Image(_)"),
        }
    }
}

#[cfg(feature = "http")]
impl_from_and_into_var! {
    fn from(uri: crate::task::http::Uri) -> ImageSource {
        ImageSource::Download(uri, None)
    }
    /// From (URI, HTTP-ACCEPT).
    fn from((uri, accept): (crate::task::http::Uri, &'static str)) -> ImageSource {
        ImageSource::Download(uri, Some(accept.into()))
    }

    /// Converts `http://` and `https://` to [`Download`], `file://` to
    /// [`Read`] the path component, and the rest to [`Read`] the string as a path.
    ///
    /// [`Download`]: ImageSource::Download
    /// [`Read`]: ImageSource::Read
    fn from(s: &str) -> ImageSource {
        use crate::task::http::*;
        if let Ok(uri) = Uri::try_from(s)
            && let Some(scheme) = uri.scheme()
        {
            if scheme == &uri::Scheme::HTTPS || scheme == &uri::Scheme::HTTP {
                return ImageSource::Download(uri, None);
            } else if scheme.as_str() == "file" {
                return PathBuf::from(uri.path()).into();
            }
        }
        PathBuf::from(s).into()
    }
}

#[cfg(not(feature = "http"))]
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
    fn from(s: Txt) -> ImageSource {
        s.as_str().into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: &[u8]) -> ImageSource {
        ImageSource::Data(
            ImageHash::compute(data),
            IpcBytes::from_slice_blocking(data).expect("cannot allocate IpcBytes"),
            ImageDataFormat::Unknown,
        )
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from<const N: usize>(data: &[u8; N]) -> ImageSource {
        (&data[..]).into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: IpcBytes) -> ImageSource {
        ImageSource::Data(ImageHash::compute(&data[..]), data, ImageDataFormat::Unknown)
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: Vec<u8>) -> ImageSource {
        IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes").into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat>>((data, format): (&[u8], F)) -> ImageSource {
        ImageSource::Data(
            ImageHash::compute(data),
            IpcBytes::from_slice_blocking(data).expect("cannot allocate IpcBytes"),
            format.into(),
        )
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat>, const N: usize>((data, format): (&[u8; N], F)) -> ImageSource {
        (&data[..], format).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat>>((data, format): (Vec<u8>, F)) -> ImageSource {
        (IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes"), format).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat>>((data, format): (IpcBytes, F)) -> ImageSource {
        ImageSource::Data(ImageHash::compute(&data[..]), data, format.into())
    }
}

/// Cache mode of [`IMAGES`].
///
/// [`IMAGES`]: super::IMAGES
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
impl_from_and_into_var! {
    fn from(cache: bool) -> ImageCacheMode {
        if cache { ImageCacheMode::Cache } else { ImageCacheMode::Ignore }
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
    Custom(Arc<dyn Fn(&U) -> bool + Send + Sync>),
}
impl<U> ImageSourceFilter<U> {
    /// New [`Custom`] filter.
    ///
    /// [`Custom`]: Self::Custom
    pub fn custom(allow: impl Fn(&U) -> bool + Send + Sync + 'static) -> Self {
        Self::Custom(Arc::new(allow))
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
            (Custom(c0), Custom(c1)) => Custom(Arc::new(move |u| c0(u) && c1(u))),
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
            (Custom(c0), Custom(c1)) => Custom(Arc::new(move |u| c0(u) || c1(u))),
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
impl<U> PartialEq for ImageSourceFilter<U> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(l0), Self::Custom(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// Represents a [`ImageSource::Read`] path request filter.
///
/// Only absolute, normalized paths are shared with the [`Custom`] filter, there is no relative paths or `..` components.
///
/// The paths are **not** canonicalized and existence is not verified, no system requests are made with unfiltered paths.
///
/// See [`ImageLimits::allow_path`] for more information.
///
/// [`Custom`]: ImageSourceFilter::Custom
pub type PathFilter = ImageSourceFilter<PathBuf>;
impl PathFilter {
    /// Allow any file inside `dir` or sub-directories of `dir`.
    pub fn allow_dir(dir: impl AsRef<Path>) -> Self {
        let dir = crate::absolute_path(dir.as_ref(), || env::current_dir().expect("could not access current dir"), true);
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
        PathFilter::custom(|r| env::current_dir().map(|d| r.starts_with(d)).unwrap_or(false))
    }

    /// Allow any file inside the current executable directory or sub-directories.
    pub fn allow_exe_dir() -> Self {
        if let Ok(mut p) = env::current_exe().and_then(dunce::canonicalize)
            && p.pop()
        {
            return Self::allow_dir(p);
        }

        // not `BlockAll` so this can still be composed using `or`.
        Self::custom(|_| false)
    }

    /// Allow any file inside the [`zng::env::res`] directory or sub-directories.
    ///
    /// [`zng::env::res`]: zng_env::res
    pub fn allow_res() -> Self {
        Self::allow_dir(zng_env::res(""))
    }
}

/// Represents a [`ImageSource::Download`] path request filter.
///
/// See [`ImageLimits::allow_uri`] for more information.
#[cfg(feature = "http")]
pub type UriFilter = ImageSourceFilter<crate::task::http::Uri>;
#[cfg(feature = "http")]
impl UriFilter {
    /// Allow any file from the `host` site.
    pub fn allow_host(host: impl Into<Txt>) -> Self {
        let host = host.into();
        UriFilter::custom(move |u| u.authority().map(|a| a.host() == host).unwrap_or(false))
    }
}

impl<F: Fn(&PathBuf) -> bool + Send + Sync + 'static> From<F> for PathFilter {
    fn from(custom: F) -> Self {
        PathFilter::custom(custom)
    }
}

#[cfg(feature = "http")]
impl<F: Fn(&task::http::Uri) -> bool + Send + Sync + 'static> From<F> for UriFilter {
    fn from(custom: F) -> Self {
        UriFilter::custom(custom)
    }
}

/// Limits for image loading and decoding.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ImageLimits {
    /// Maximum encoded file size allowed.
    ///
    /// An error is returned if the file size surpasses this value. If the size can read before
    /// read/download the validation happens before download starts, otherwise the error happens when this limit
    /// is reached and all already downloaded bytes are dropped.
    ///
    /// The default is `100mb`.
    pub max_encoded_len: ByteLength,
    /// Maximum decoded file size allowed.
    ///
    /// An error is returned if the decoded image memory would surpass the `width * height * 4`
    pub max_decoded_len: ByteLength,

    /// Filter for [`ImageSource::Read`] paths.
    ///
    /// Only paths allowed by this filter are loaded
    pub allow_path: PathFilter,

    /// Filter for [`ImageSource::Download`] URIs.
    #[cfg(feature = "http")]
    pub allow_uri: UriFilter,
}
impl ImageLimits {
    /// No size limits, allow all paths and URIs.
    pub fn none() -> Self {
        ImageLimits {
            max_encoded_len: ByteLength::MAX,
            max_decoded_len: ByteLength::MAX,
            allow_path: PathFilter::AllowAll,
            #[cfg(feature = "http")]
            allow_uri: UriFilter::AllowAll,
        }
    }

    /// Set the [`max_encoded_len`].
    ///
    /// [`max_encoded_len`]: Self::max_encoded_len
    pub fn with_max_encoded_len(mut self, max_encoded_len: impl Into<ByteLength>) -> Self {
        self.max_encoded_len = max_encoded_len.into();
        self
    }

    /// Set the [`max_decoded_len`].
    ///
    /// [`max_decoded_len`]: Self::max_encoded_len
    pub fn with_max_decoded_len(mut self, max_decoded_len: impl Into<ByteLength>) -> Self {
        self.max_decoded_len = max_decoded_len.into();
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
    #[cfg(feature = "http")]
    pub fn with_allow_uri(mut self, allow_url: impl Into<UriFilter>) -> Self {
        self.allow_uri = allow_url.into();
        self
    }
}
impl Default for ImageLimits {
    /// 100 megabytes encoded and 4096 megabytes decoded (BMP max).
    ///
    /// Allows only paths in `zng::env::res`, blocks all downloads.
    fn default() -> Self {
        Self {
            max_encoded_len: 100.megabytes(),
            max_decoded_len: 4096.megabytes(),
            allow_path: PathFilter::allow_res(),
            #[cfg(feature = "http")]
            allow_uri: UriFilter::BlockAll,
        }
    }
}
impl_from_and_into_var! {
    fn from(some: ImageLimits) -> Option<ImageLimits>;
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
