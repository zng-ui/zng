use std::{
    env, fmt, fs, io, mem, ops,
    path::{Path, PathBuf},
    sync::Arc,
};

use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use zng_app::{
    view_process::{EncodeError, ViewImage, ViewRenderer},
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
    self as task, SignalOnce,
    channel::{IpcBytes, IpcBytesMut},
};
use zng_txt::Txt;
use zng_var::{Var, animation::Transitionable, impl_from_and_into_var};
use zng_view_api::image::ImageTextureId;

use crate::render::ImageRenderWindowRoot;

pub use zng_view_api::image::{ColorType, ImageDataFormat, ImageDownscaleMode, ImageEntriesMode, ImageEntryKind, ImageMaskMode};

/// A custom proxy in [`IMAGES`].
///
/// Implementers can intercept cache requests and redirect to another cache request or returns an image directly.
///
/// The methods on this API are synchronous, implementers that do any potential slow processing must output
/// a *loading* [`ImageVar`] immediately and update it with the finished pixels when ready.
///
/// [`IMAGES`]: super::IMAGES
pub trait ImageCacheProxy: Send + Sync {
    /// Intercept a get request.
    fn get(
        &mut self,
        key: &ImageHash,
        source: &ImageSource,
        mode: ImageCacheMode,
        downscale: Option<&ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ProxyGetResult {
        let r = match source {
            ImageSource::Data(_, data, image_format) => self.data(key, data, image_format, mode, downscale, mask, entries, false),
            _ => return ProxyGetResult::None,
        };
        match r {
            Some(img) => ProxyGetResult::Image(img),
            None => ProxyGetResult::None,
        }
    }

    /// Intercept a [`Data`] request.
    ///
    /// If [`is_data_proxy`] also intercept the [`Read`] or [`Download`] data.
    ///
    /// If `is_loaded` is `true` the data was read or downloaded and the return var will be bound to an existing var that may already be cached.
    /// If it is `false` the data was already loaded on the source and the return var will be returned directly, without caching.
    ///
    /// [`Data`]: ImageSource::Data
    /// [`is_data_proxy`]: ImageCacheProxy::is_data_proxy
    /// [`Read`]: ImageSource::Read
    /// [`Download`]: ImageSource::Download
    #[allow(clippy::too_many_arguments)]
    fn data(
        &mut self,
        key: &ImageHash,
        data: &[u8],
        image_format: &ImageDataFormat,
        mode: ImageCacheMode,
        downscale: Option<&ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
        is_loaded: bool,
    ) -> Option<ImageVar> {
        let _ = (key, data, image_format, mode, downscale, mask, entries, is_loaded);
        None
    }

    /// Intercept a remove request.
    fn remove(&mut self, key: &ImageHash, purge: bool) -> ProxyRemoveResult {
        let _ = (key, purge);
        ProxyRemoveResult::None
    }

    /// Called when the cache is cleaned or purged.
    fn clear(&mut self, purge: bool) {
        let _ = purge;
    }

    /// If this proxy only handles [`Data`] sources.
    ///
    /// When this is `true` the [`get`] call is delayed to after [`Read`] and [`Download`] have loaded the data
    /// and is skipped for [`Render`] and [`Image`].
    ///
    /// This is `false` by default.
    ///
    /// [`get`]: ImageCacheProxy::get
    /// [`Data`]: ImageSource::Data
    /// [`Read`]: ImageSource::Read
    /// [`Download`]: ImageSource::Download
    /// [`Render`]: ImageSource::Render
    /// [`Image`]: ImageSource::Image
    fn is_data_proxy(&self) -> bool {
        false
    }
}

/// Result of an [`ImageCacheProxy`] *get* redirect.
pub enum ProxyGetResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Load and cache using the replacement source.
    Cache(
        ImageSource,
        ImageCacheMode,
        Option<ImageDownscaleMode>,
        Option<ImageMaskMode>,
        ImageEntriesMode,
    ),
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

/// Represents an [`Img`] tracked by the [`IMAGES`] cache.
///
/// The variable updates when the image updates.
///
/// [`IMAGES`]: super::IMAGES
pub type ImageVar = Var<Img>;

#[derive(Default, Debug)]
struct ImgMut {
    render_ids: Vec<RenderImage>,
    entries: Vec<ImageVar>,
}

/// State of an [`ImageVar`].
///
/// Each instance of this struct represent a single state,
#[derive(Debug, Clone)]
pub struct Img {
    // use inner_set_or_replace to set
    pub(super) view: OnceCell<ViewImage>,
    pub(super) done_signal: SignalOnce,
    pub(super) cache_key: Option<ImageHash>,

    img_mut: Arc<Mutex<ImgMut>>,
}
impl PartialEq for Img {
    fn eq(&self, other: &Self) -> bool {
        self.view == other.view
    }
}
impl Img {
    pub(super) fn new_none(cache_key: Option<ImageHash>) -> Self {
        Img {
            view: OnceCell::new(),
            done_signal: SignalOnce::new(),
            cache_key,
            img_mut: Arc::default(),
        }
    }

    /// New from existing `ViewImage`.
    pub fn new(view: ViewImage) -> Self {
        let sig = view.awaiter();
        let v = OnceCell::new();
        let _ = v.set(view);
        Img {
            view: v,
            done_signal: sig,
            cache_key: None,
            img_mut: Arc::default(),
        }
    }

    /// Create a dummy image in the loaded or error state.
    pub fn dummy(error: Option<Txt>) -> Self {
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
    pub fn error(&self) -> Option<Txt> {
        match self.view.get() {
            Some(v) => v.error(),
            None => None,
        }
    }

    /// Returns a future that awaits until this image is loaded or encountered an error.
    pub fn wait_done(&self) -> impl Future<Output = ()> + Send + Sync + 'static {
        self.done_signal.clone()
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
        self.view.get().map(|v| v.size()).unwrap_or_else(PxSize::zero)
    }

    /// Size of [`partial_pixels`].
    ///
    /// Can be different from [`size`] if the image is progressively decoding.
    ///
    /// [`size`]: Self::size
    /// [`partial_pixels`]: Self::partial_pixels
    pub fn partial_size(&self) -> Option<PxSize> {
        self.view.get().and_then(|v| v.partial_size())
    }

    /// Returns the image pixel density metadata if the image is loaded and the
    /// metadata was retrieved.
    pub fn density(&self) -> Option<PxDensity2d> {
        self.view.get().and_then(|v| v.density())
    }

    /// Image color type before it was converted to BGRA8 or A8.
    pub fn original_color_type(&self) -> ColorType {
        self.view
            .get()
            .map(|v| v.original_color_type())
            .unwrap_or(ColorType::new(Txt::from_static(""), 0, 0))
    }

    /// Gets A8 for masks and BGRA8 for others.
    pub fn color_type(&self) -> ColorType {
        if self.is_mask() { ColorType::A8 } else { ColorType::BGRA8 }
    }

    /// Returns `true` if the image is fully opaque or it is not loaded.
    pub fn is_opaque(&self) -> bool {
        self.view.get().map(|v| v.is_opaque()).unwrap_or(true)
    }

    /// Returns `true` if the image pixels are a single channel (A8).
    pub fn is_mask(&self) -> bool {
        self.view.get().map(|v| v.is_mask()).unwrap_or(false)
    }

    /// Other images from the same container that reference back to this image as parent.
    pub fn entries(&self) -> Vec<ImageVar> {
        self.img_mut.lock().entries.iter().map(|e| e.read_only()).collect()
    }

    /// Kind of image container entry this image was decoded from.
    pub fn entry_kind(&self) -> ImageEntryKind {
        self.view.get().map(|v| v.entry_kind()).unwrap_or(ImageEntryKind::Page)
    }

    /// Calls `visit` with the image or [`ImageEntryKind::Reduced`] entry that is nearest to `size` and greater or equal to it.
    ///
    /// Does not call `visit` if none of the images are loaded, returns `None` in that case.
    pub fn with_best_reduce<R>(&self, size: PxSize, visit: impl FnOnce(&Img) -> R) -> Option<R> {
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

        let img_mut = self.img_mut.lock();

        if self.is_loaded() {
            best_i = img_mut.entries.len();
            best_size = self.size();
        }

        for (i, entry) in img_mut.entries.iter().enumerate() {
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
        } else if best_i == img_mut.entries.len() {
            drop(img_mut);
            Some(visit(self))
        } else {
            let entry = img_mut.entries[best_i].clone();
            drop(img_mut);
            Some(entry.with(visit))
        }
    }

    /// Connection to the image resource.
    pub fn view(&self) -> Option<&ViewImage> {
        self.view.get()
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
        self.view.get().and_then(|v| v.pixels())
    }

    /// Reference the partially decoded pixels if the image is progressively decoding
    /// and has not finished decoding.
    ///
    /// Format is BGRA8 for normal images or A8 if [`is_mask`].
    ///
    /// [`is_mask`]: Self::is_mask
    pub fn partial_pixels(&self) -> Option<IpcBytes> {
        self.view.get().and_then(|v| v.partial_pixels())
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
    pub async fn encode_with_entries(&self, entries: &[(Img, ImageEntryKind)], format: Txt) -> std::result::Result<IpcBytes, EncodeError> {
        self.done_signal.clone().await;
        if let Some(e) = self.error().or_else(|| entries.iter().filter_map(|i| i.0.error()).next()) {
            Err(EncodeError::Encode(e))
        } else {
            if self.view().is_none() || entries.iter().any(|v| v.0.view().is_none()) {
                return Err(EncodeError::Dummy);
            }
            let entries = entries.iter().map(|(img, kind)| (img.view().unwrap().id().unwrap(), kind.clone()));
            self.view().unwrap().encode(entries.collect(), format).await
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
        entries: &[(Img, ImageEntryKind)],
        format: impl Into<Txt>,
        path: impl Into<PathBuf>,
    ) -> io::Result<()> {
        self.save_impl(entries, format.into(), path.into()).await
    }

    async fn save_impl(&self, entries: &[(Img, ImageEntryKind)], format: Txt, path: PathBuf) -> io::Result<()> {
        let data = self.encode_with_entries(entries, format).await.map_err(io::Error::other)?;
        task::wait(move || fs::write(path, &data[..])).await
    }

    pub(crate) fn inner_set_or_replace(&mut self, img: ViewImage, done: bool) {
        match self.view.set(img) {
            Ok(()) => {
                if done {
                    self.done_signal.set();
                }
            }
            Err(img) => {
                // this can happen on reload
                let cache_key = self.cache_key;
                *self = Self {
                    view: OnceCell::with_value(img),
                    done_signal: if done { SignalOnce::new_set() } else { SignalOnce::new() },
                    cache_key,
                    img_mut: Arc::default(),
                };
            }
        }
    }

    pub(crate) fn find_entry(&self, id: zng_view_api::image::ImageId) -> Option<ImageVar> {
        self.img_mut.lock().entries.iter().find_map(|v| {
            if v.with(|i| i.view().map(|i| i.id() == Some(id))).unwrap_or(false) {
                Some(v.clone())
            } else {
                v.with(|i| i.find_entry(id))
            }
        })
    }

    pub(crate) fn has_loading_entries(&self) -> bool {
        self.img_mut
            .lock()
            .entries
            .iter()
            .any(|e| e.with(|e| e.is_loading() || e.has_loading_entries()))
    }

    /// Insert `entry` in [`entries`].
    ///
    /// Note that `Img` is a shared reference, the new entry will be inserted for all clones, this method is
    /// `&mut self` to ensure variable updates when set in a [`ImageVar`], the most common way of handling images.
    ///
    /// [`entries`]: Self::entries
    pub fn register_entry(&mut self, entry: ViewImage) -> Var<Img> {
        // takes &mut self to force ImageVar updates
        let mut self_ = self.img_mut.lock();
        let i = entry.entry_index();
        let i = self_
            .entries
            .iter()
            .position(|v| {
                let entry_i = v.with(|i| i.view().map(|v| v.entry_index())).unwrap_or(0);
                entry_i > i
            })
            .unwrap_or(self_.entries.len());
        let entry = zng_var::var(Self::new(entry));
        self_.entries.insert(i, entry.clone());
        entry
    }
}
impl zng_app::render::Img for Img {
    fn renderer_id(&self, renderer: &ViewRenderer) -> ImageTextureId {
        if self.is_loaded() {
            let mut img = self.img_mut.lock();
            let rms = &mut img.render_ids;
            if let Some(rm) = rms.iter().find(|k| &k.renderer == renderer) {
                return rm.image_id;
            }

            let key = match renderer.use_image(self.view.get().unwrap()) {
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
    pub fn hash128(
        &self,
        downscale: Option<&ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> Option<ImageHash> {
        match self {
            ImageSource::Read(p) => Some(Self::hash128_read(p, downscale, mask, entries)),
            #[cfg(feature = "http")]
            ImageSource::Download(u, a) => Some(Self::hash128_download(u, a, downscale, mask, entries)),
            ImageSource::Data(h, _, _) => Some(Self::hash128_data(*h, downscale, mask, entries)),
            ImageSource::Render(rfn, args) => Some(Self::hash128_render(rfn, args, downscale, mask, entries)),
            ImageSource::Image(_) => None,
        }
    }

    /// Compute hash for a borrowed [`Data`] image.
    ///
    /// [`Data`]: Self::Data
    pub fn hash128_data(
        data_hash: ImageHash,
        downscale: Option<&ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageHash {
        if downscale.is_some() || mask.is_some() {
            use std::hash::Hash;
            let mut h = ImageHash::hasher();
            data_hash.0.hash(&mut h);
            downscale.hash(&mut h);
            mask.hash(&mut h);
            entries.hash(&mut h);
            h.finish()
        } else {
            data_hash
        }
    }

    /// Compute hash for a borrowed [`Read`] path.
    ///
    /// [`Read`]: Self::Read
    pub fn hash128_read(
        path: &Path,
        downscale: Option<&ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        0u8.hash(&mut h);
        path.hash(&mut h);
        downscale.hash(&mut h);
        mask.hash(&mut h);
        entries.hash(&mut h);
        h.finish()
    }

    /// Compute hash for a borrowed [`Download`] URI and HTTP-ACCEPT.
    ///
    /// [`Download`]: Self::Download
    #[cfg(feature = "http")]
    pub fn hash128_download(
        uri: &crate::task::http::Uri,
        accept: &Option<Txt>,
        downscale: Option<&ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        1u8.hash(&mut h);
        uri.hash(&mut h);
        accept.hash(&mut h);
        downscale.hash(&mut h);
        mask.hash(&mut h);
        entries.hash(&mut h);
        h.finish()
    }

    /// Compute hash for a borrowed [`Render`] source.
    ///
    /// Pointer equality is used to identify the node closure.
    ///
    /// [`Render`]: Self::Render
    pub fn hash128_render(
        rfn: &RenderFn,
        args: &Option<ImageRenderArgs>,
        downscale: Option<&ImageDownscaleMode>,
        mask: Option<ImageMaskMode>,
        entries: ImageEntriesMode,
    ) -> ImageHash {
        use std::hash::Hash;
        let mut h = ImageHash::hasher();
        2u8.hash(&mut h);
        (Arc::as_ptr(rfn) as usize).hash(&mut h);
        args.hash(&mut h);
        downscale.hash(&mut h);
        mask.hash(&mut h);
        entries.hash(&mut h);
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
    pub fn with_max_encoded_len(mut self, max_encoded_size: impl Into<ByteLength>) -> Self {
        self.max_encoded_len = max_encoded_size.into();
        self
    }

    /// Set the [`max_decoded_len`].
    ///
    /// [`max_decoded_len`]: Self::max_encoded_len
    pub fn with_max_decoded_len(mut self, max_decoded_size: impl Into<ByteLength>) -> Self {
        self.max_decoded_len = max_decoded_size.into();
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
