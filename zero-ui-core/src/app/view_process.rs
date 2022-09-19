//! View process controller types.

use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};
use std::{fmt, rc};

use linear_map::LinearMap;
use once_cell::unsync::OnceCell;

use super::DeviceId;
use crate::event::{event, event_args};
use crate::mouse::MultiClickConfig;
use crate::service::Service;
use crate::task::SignalOnce;
use crate::text::FontAntiAliasing;
use crate::units::{DipPoint, DipSize, Factor, Px, PxRect, PxSize};
use crate::window::{MonitorId, WindowId};
use zero_ui_view_api::webrender_api::{
    FontInstanceKey, FontInstanceOptions, FontInstancePlatformOptions, FontKey, FontVariation, IdNamespace, ImageKey, PipelineId,
};
pub use zero_ui_view_api::{
    bytes_channel, ColorScheme, CursorIcon, Event, EventCause, FocusIndicator, FrameRequest, FrameUpdateRequest, FrameWaitId,
    HeadlessOpenData, HeadlessRequest, ImageDataFormat, ImagePpi, IpcBytes, IpcBytesReceiver, IpcBytesSender, MonitorInfo, RenderMode,
    VideoMode, ViewProcessGen, ViewProcessOffline, WindowRequest, WindowState, WindowStateAll,
};
use zero_ui_view_api::{Controller, DeviceId as ApiDeviceId, ImageId, ImageLoadedData, MonitorId as ApiMonitorId, WindowId as ApiWindowId};

type Result<T> = std::result::Result<T, ViewProcessOffline>;

struct EncodeRequest {
    image_id: ImageId,
    format: String,
    listeners: Vec<flume::Sender<std::result::Result<Arc<Vec<u8>>, EncodeError>>>,
}

/// Reference to the running View Process.
///
/// This is the lowest level API, used for implementing fundamental services and is a service available
/// in headed apps or headless apps with renderer.
///
/// This is a strong reference to the view process. The process shuts down when all clones of this struct drops.
#[derive(Service, Clone)]
pub struct ViewProcess(Rc<RefCell<ViewApp>>);
struct ViewApp {
    process: zero_ui_view_api::Controller,
    device_ids: LinearMap<ApiDeviceId, DeviceId>,
    monitor_ids: LinearMap<ApiMonitorId, MonitorId>,

    data_generation: ViewProcessGen,

    loading_images: Vec<rc::Weak<ImageConnection>>,
    frame_images: Vec<rc::Weak<ImageConnection>>,
    encoding_images: Vec<EncodeRequest>,

    pending_frames: usize,
}
impl fmt::Debug for ViewApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewApp").finish_non_exhaustive()
    }
}
impl ViewApp {
    #[must_use = "if `true` all current WinId, DevId and MonId are invalid"]
    fn check_generation(&mut self) -> bool {
        let gen = self.process.generation();
        let invalid = gen != self.data_generation;
        if invalid {
            self.data_generation = gen;
            self.device_ids.clear();
            self.monitor_ids.clear();
        }
        invalid
    }
}
impl ViewProcess {
    /// Spawn the View Process.
    pub(super) fn start<F>(view_process_exe: Option<PathBuf>, device_events: bool, headless: bool, on_event: F) -> Self
    where
        F: FnMut(Event) + Send + 'static,
    {
        let _s = tracing::debug_span!("ViewProcess::start").entered();

        let process = zero_ui_view_api::Controller::start(view_process_exe, device_events, headless, on_event);
        Self(Rc::new(RefCell::new(ViewApp {
            data_generation: process.generation(),
            process,
            device_ids: LinearMap::default(),
            monitor_ids: LinearMap::default(),
            loading_images: vec![],
            encoding_images: vec![],
            frame_images: vec![],
            pending_frames: 0,
        })))
    }

    /// View-process connected and ready.
    pub fn online(&self) -> bool {
        self.0.borrow().process.online()
    }

    /// If is running in headless renderer mode.
    pub fn headless(&self) -> bool {
        self.0.borrow().process.headless()
    }

    /// If is running both view and app in the same process.
    pub fn same_process(&self) -> bool {
        self.0.borrow().process.same_process()
    }

    /// Sends a request to open a window and associate it with the `window_id`.
    ///
    /// A [`RawWindowOpenEvent`] or [`RawWindowOrHeadlessOpenErrorEvent`] will be received in response to this request.
    ///
    /// [`RawWindowOpenEvent`]: crate::app::raw_events::RawWindowOpenEvent
    /// [`RawWindowOrHeadlessOpenErrorEvent`]: crate::app::raw_events::RawWindowOrHeadlessOpenErrorEvent
    pub fn open_window(&self, config: WindowRequest) -> Result<()> {
        let _s = tracing::debug_span!("ViewProcess.open_window").entered();
        self.0.borrow_mut().process.open_window(config)
    }

    pub(crate) fn on_window_opened(&self, window_id: WindowId, data: zero_ui_view_api::WindowOpenData) -> (ViewWindow, WindowOpenData) {
        let mut app = self.0.borrow_mut();
        let _ = app.check_generation();

        let win = ViewWindow(Rc::new(WindowConnection {
            id: window_id.get(),
            app: self.0.clone(),
            id_namespace: data.id_namespace,
            pipeline_id: data.pipeline_id,
            generation: app.data_generation,
        }));
        drop(app);

        let data = WindowOpenData::new(data, |id| self.monitor_id(id));

        (win, data)
    }

    /// Sends a request to open a headless renderer and associate it with the `window_id`.
    ///
    /// Note that no actual window is created, only the renderer, the use of window-ids to identify
    /// this renderer is only for convenience.
    ///
    /// A [`RawHeadlessOpenEvent`] or [`RawWindowOrHeadlessOpenErrorEvent`] will be received in response to this request.
    ///
    /// [`RawHeadlessOpenEvent`]: crate::app::raw_events::RawHeadlessOpenEvent
    /// [`RawWindowOrHeadlessOpenErrorEvent`]: crate::app::raw_events::RawWindowOrHeadlessOpenErrorEvent
    pub fn open_headless(&self, config: HeadlessRequest) -> Result<()> {
        let _s = tracing::debug_span!("ViewProcess.open_headless").entered();
        self.0.borrow_mut().process.open_headless(config)
    }

    pub(crate) fn on_headless_opened(&self, id: WindowId, data: zero_ui_view_api::HeadlessOpenData) -> (ViewHeadless, HeadlessOpenData) {
        let mut app = self.0.borrow_mut();
        let _ = app.check_generation();

        let surf = ViewHeadless(Rc::new(WindowConnection {
            id: id.get(),
            app: self.0.clone(),
            id_namespace: data.id_namespace,
            pipeline_id: data.pipeline_id,
            generation: app.data_generation,
        }));

        (surf, data)
    }

    /// Translate `DevId` to `DeviceId`, generates a device id if it was unknown.
    pub(super) fn device_id(&self, id: ApiDeviceId) -> DeviceId {
        *self.0.borrow_mut().device_ids.entry(id).or_insert_with(DeviceId::new_unique)
    }

    /// Translate `MonId` to `MonitorId`, generates a monitor id if it was unknown.
    pub(super) fn monitor_id(&self, id: ApiMonitorId) -> MonitorId {
        *self.0.borrow_mut().monitor_ids.entry(id).or_insert_with(MonitorId::new_unique)
    }

    /// Reopen the view-process, causing another [`Event::Inited`].
    pub fn respawn(&self) {
        self.0.borrow_mut().process.respawn()
    }

    /// Causes a panic in the view-process to test respawn.
    #[cfg(debug_assertions)]
    pub fn crash_view_process(&self) {
        self.0.borrow_mut().process.crash().unwrap();
    }

    /// Handle an [`Event::Inited`].
    ///
    /// The view-process becomes online only after this call.
    pub(super) fn handle_inited(&self, gen: ViewProcessGen) {
        self.0.borrow_mut().process.handle_inited(gen);
    }

    /// Handle an [`Event::Disconnected`].
    ///
    /// The process will exit if the view-process was killed by the user.
    pub fn handle_disconnect(&mut self, gen: ViewProcessGen) {
        self.0.borrow_mut().process.handle_disconnect(gen)
    }

    /// Gets the current view-process generation.
    pub fn generation(&self) -> ViewProcessGen {
        self.0.borrow().process.generation()
    }

    /// Send an image for decoding.
    ///
    /// This function returns immediately, the [`ViewImage`] will update when
    /// [`Event::ImageMetadataLoaded`], [`Event::ImageLoaded`] and [`Event::ImageLoadError`] events are received.
    pub fn add_image(&self, format: ImageDataFormat, data: IpcBytes, max_decoded_size: u64) -> Result<ViewImage> {
        let mut app = self.0.borrow_mut();
        let id = app.process.add_image(format, data, max_decoded_size)?;
        let img = ViewImage(Rc::new(ImageConnection {
            id,
            generation: app.process.generation(),
            app: Some(self.0.clone()),
            size: Cell::new(PxSize::zero()),
            partial_size: Cell::new(PxSize::zero()),
            ppi: Cell::new(None),
            opaque: Cell::new(false),
            partial_bgra8: RefCell::new(None),
            bgra8: OnceCell::new(),
            done_signal: SignalOnce::new(),
        }));
        app.loading_images.push(Rc::downgrade(&img.0));
        Ok(img)
    }

    /// Starts sending an image for *progressive* decoding.
    ///
    /// This function returns immediately, the [`ViewImage`] will update when
    /// [`Event::ImageMetadataLoaded`], [`Event::ImagePartiallyLoaded`],
    /// [`Event::ImageLoaded`] and [`Event::ImageLoadError`] events are received.
    pub fn add_image_pro(&self, format: ImageDataFormat, data: IpcBytesReceiver, max_decoded_size: u64) -> Result<ViewImage> {
        let mut app = self.0.borrow_mut();
        let id = app.process.add_image_pro(format, data, max_decoded_size)?;
        let img = ViewImage(Rc::new(ImageConnection {
            id,
            generation: app.process.generation(),
            app: Some(self.0.clone()),
            size: Cell::new(PxSize::zero()),
            partial_size: Cell::new(PxSize::zero()),
            ppi: Cell::new(None),
            opaque: Cell::new(false),
            partial_bgra8: RefCell::new(None),
            bgra8: OnceCell::new(),
            done_signal: SignalOnce::new(),
        }));
        app.loading_images.push(Rc::downgrade(&img.0));
        Ok(img)
    }

    /// Returns a list of image decoders supported by the view-process backend.
    ///
    /// Each string is the lower-case file extension.
    pub fn image_decoders(&self) -> Result<Vec<String>> {
        self.0.borrow_mut().process.image_decoders()
    }

    /// Returns a list of image encoders supported by the view-process backend.
    ///
    /// Each string is the lower-case file extension.
    pub fn image_encoders(&self) -> Result<Vec<String>> {
        self.0.borrow_mut().process.image_encoders()
    }

    /// Number of frame send that have not finished rendering.
    ///
    /// This is the sum of pending frames for all renderers.
    pub fn pending_frames(&self) -> usize {
        self.0.borrow().pending_frames
    }

    fn loading_image_index(&self, id: ImageId) -> Option<usize> {
        let mut app = self.0.borrow_mut();

        // cleanup
        app.loading_images.retain(|i| i.strong_count() > 0);

        app.loading_images.iter().position(|i| i.upgrade().unwrap().id == id)
    }

    pub(super) fn on_image_metadata_loaded(&self, id: ImageId, size: PxSize, ppi: ImagePpi) -> Option<ViewImage> {
        if let Some(i) = self.loading_image_index(id) {
            let app = self.0.borrow();
            let img = app.loading_images[i].upgrade().unwrap();
            img.size.set(size);
            img.ppi.set(ppi);
            Some(ViewImage(img))
        } else {
            None
        }
    }

    pub(super) fn on_image_partially_loaded(
        &self,
        id: ImageId,
        partial_size: PxSize,
        ppi: ImagePpi,
        opaque: bool,
        partial_bgra8: IpcBytes,
    ) -> Option<ViewImage> {
        if let Some(i) = self.loading_image_index(id) {
            let app = self.0.borrow();
            let img = app.loading_images[i].upgrade().unwrap();
            img.partial_size.set(partial_size);
            img.ppi.set(ppi);
            img.opaque.set(opaque);
            *img.partial_bgra8.borrow_mut() = Some(partial_bgra8);
            Some(ViewImage(img))
        } else {
            None
        }
    }

    pub(super) fn on_image_loaded(&self, data: ImageLoadedData) -> Option<ViewImage> {
        if let Some(i) = self.loading_image_index(data.id) {
            let mut app = self.0.borrow_mut();
            let img = app.loading_images.swap_remove(i).upgrade().unwrap();
            img.size.set(data.size);
            img.partial_size.set(data.size);
            img.ppi.set(data.ppi);
            img.opaque.set(data.opaque);
            img.bgra8.set(Ok(data.bgra8)).unwrap();
            *img.partial_bgra8.borrow_mut() = None;
            img.done_signal.set();
            Some(ViewImage(img))
        } else {
            None
        }
    }

    pub(super) fn on_image_error(&self, id: ImageId, error: String) -> Option<ViewImage> {
        if let Some(i) = self.loading_image_index(id) {
            let mut app = self.0.borrow_mut();
            let img = app.loading_images.swap_remove(i).upgrade().unwrap();
            img.bgra8.set(Err(error)).unwrap();
            img.done_signal.set();
            Some(ViewImage(img))
        } else {
            None
        }
    }

    pub(crate) fn on_frame_rendered(&self, _id: WindowId) {
        let mut vp = self.0.borrow_mut();
        vp.pending_frames = vp.pending_frames.saturating_sub(1);
    }

    pub(crate) fn on_frame_image(&self, data: ImageLoadedData) -> ViewImage {
        let bgra8 = OnceCell::new();
        let _ = bgra8.set(Ok(data.bgra8));
        ViewImage(Rc::new(ImageConnection {
            id: data.id,
            generation: self.generation(),
            app: Some(self.0.clone()),
            size: Cell::new(data.size),
            partial_size: Cell::new(data.size),
            ppi: Cell::new(data.ppi),
            opaque: Cell::new(data.opaque),
            partial_bgra8: RefCell::new(None),
            bgra8,
            done_signal: SignalOnce::new_set(),
        }))
    }

    pub(super) fn on_frame_image_ready(&self, id: ImageId) -> Option<ViewImage> {
        let mut app = self.0.borrow_mut();

        // cleanup
        app.frame_images.retain(|i| i.strong_count() > 0);

        let i = app.frame_images.iter().position(|i| i.upgrade().unwrap().id == id);

        if let Some(i) = i {
            Some(ViewImage(app.frame_images.swap_remove(i).upgrade().unwrap()))
        } else {
            None
        }
    }

    pub(super) fn on_image_encoded(&self, id: ImageId, format: String, data: Vec<u8>) {
        self.on_image_encode_result(id, format, Ok(Arc::new(data)));
    }
    pub(super) fn on_image_encode_error(&self, id: ImageId, format: String, error: String) {
        self.on_image_encode_result(id, format, Err(EncodeError::Encode(error)));
    }
    fn on_image_encode_result(&self, id: ImageId, format: String, result: std::result::Result<Arc<Vec<u8>>, EncodeError>) {
        let mut app = self.0.borrow_mut();
        app.encoding_images.retain(move |r| {
            let done = r.image_id == id && r.format == format;
            if done {
                for sender in &r.listeners {
                    let _ = sender.send(result.clone());
                }
            }
            !done
        })
    }

    pub(super) fn on_respawed(&self, _gen: ViewProcessGen) {
        let mut vp = self.0.borrow_mut();
        vp.pending_frames = 0;
    }
}

struct ImageConnection {
    id: ImageId,
    generation: ViewProcessGen,
    app: Option<Rc<RefCell<ViewApp>>>,

    size: Cell<PxSize>,
    partial_size: Cell<PxSize>,
    ppi: Cell<ImagePpi>,
    opaque: Cell<bool>,

    partial_bgra8: RefCell<Option<IpcBytes>>,
    bgra8: OnceCell<std::result::Result<IpcBytes, String>>,

    done_signal: SignalOnce,
}
impl ImageConnection {
    fn online(&self) -> bool {
        if let Some(app) = &self.app {
            self.generation == app.borrow().process.generation()
        } else {
            true
        }
    }
}
impl Drop for ImageConnection {
    fn drop(&mut self) {
        if let Some(app) = self.app.take() {
            let mut app = app.borrow_mut();
            if app.process.generation() == self.generation {
                let _ = app.process.forget_image(self.id);
            }
        }
    }
}

/// Connection to an image loading or loaded in the View Process.
///
/// This is a strong reference to the image connection. The image is removed from the View Process cache
/// when all clones of this struct drops.
#[derive(Clone)]
pub struct ViewImage(Rc<ImageConnection>);
impl PartialEq for ViewImage {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ViewImage {}
impl std::hash::Hash for ViewImage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = Rc::as_ptr(&self.0) as usize;
        ptr.hash(state)
    }
}
impl fmt::Debug for ViewImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewImage")
            .field("loaded", &self.is_loaded())
            .field("error", &self.error())
            .field("size", &self.size())
            .field("dpi", &self.ppi())
            .field("opaque", &self.is_opaque())
            .field("generation", &self.generation())
            .field("alive", &self.online())
            .finish_non_exhaustive()
    }
}
impl ViewImage {
    /// Image id.
    pub fn id(&self) -> ImageId {
        self.0.id
    }

    /// If the image does not actually exists in the view-process.
    pub fn is_dummy(&self) -> bool {
        self.0.app.is_none()
    }

    /// Returns `true` if the image has successfully decoded.
    pub fn is_loaded(&self) -> bool {
        self.0.bgra8.get().map(|r| r.is_ok()).unwrap_or(false)
    }

    /// Returns `true` if the image is progressively decoding and has partially decoded.
    pub fn is_partially_loaded(&self) -> bool {
        self.0.partial_bgra8.borrow().is_some()
    }

    /// if [`error`] is `Some`.
    ///
    /// [`error`]: Self::error
    pub fn is_error(&self) -> bool {
        self.0.bgra8.get().map(|r| r.is_err()).unwrap_or(false)
    }

    /// Returns the load error if one happened.
    pub fn error(&self) -> Option<&str> {
        self.0.bgra8.get().and_then(|s| s.as_ref().err().map(|s| s.as_str()))
    }

    /// Returns the pixel size, or zero if is not loaded or error.
    pub fn size(&self) -> PxSize {
        self.0.size.get()
    }

    /// Actual size of the current pixels.
    ///
    /// Can be different from [`size`] if the image is progressively decoding.
    ///
    /// [`size`]: Self::size
    pub fn partial_size(&self) -> PxSize {
        self.0.partial_size.get()
    }

    /// Returns the "pixels-per-inch" metadata associated with the image, or `None` if not loaded or error or no
    /// metadata provided by decoder.
    pub fn ppi(&self) -> ImagePpi {
        self.0.ppi.get()
    }

    /// Returns if the image is fully opaque.
    pub fn is_opaque(&self) -> bool {
        self.0.opaque.get()
    }

    /// Copy the partially decoded pixels if the image is progressively decoding
    /// and has not finished decoding.
    pub fn partial_bgra8(&self) -> Option<Vec<u8>> {
        (*self.0.partial_bgra8.borrow()).as_ref().map(|r| r[..].to_vec())
    }

    /// Reference the decoded and pre-multiplied BGRA8 bytes of the image.
    ///
    /// Returns `None` until the image is fully loaded. Use [`partial_bgra8`] to copy
    /// partially decoded bytes.
    ///
    /// [`partial_bgra8`]: Self::partial_bgra8
    pub fn bgra8(&self) -> Option<&[u8]> {
        self.0.bgra8.get().and_then(|r| r.as_ref().ok()).map(|m| &m[..])
    }

    /// Clone the reference to the inter-process shared memory that contains
    /// the image BGRA8 pixel buffer.
    pub fn shared_bgra8(&self) -> Option<IpcBytes> {
        self.0.bgra8.get().and_then(|r| r.as_ref().ok()).cloned()
    }

    /// Returns the view-process generation on which the image is loaded.
    pub fn generation(&self) -> ViewProcessGen {
        self.0.generation
    }

    /// Returns `true` if this window connection is still valid.
    ///
    /// The connection can be permanently lost in case the "view-process" respawns, in this
    /// case all methods will return [`ViewProcessOffline`], and you must discard this connection and
    /// create a new one.
    pub fn online(&self) -> bool {
        self.0.online()
    }

    /// Creates a [`WeakViewImage`].
    pub fn downgrade(&self) -> WeakViewImage {
        WeakViewImage(Rc::downgrade(&self.0))
    }

    /// Create a dummy image in the loaded or error state.
    pub fn dummy(error: Option<String>) -> Self {
        let bgra8 = OnceCell::new();

        if let Some(e) = error {
            bgra8.set(Err(e)).unwrap();
        } else {
            bgra8.set(Ok(IpcBytes::from_slice(&[]))).unwrap();
        }

        ViewImage(Rc::new(ImageConnection {
            id: 0,
            generation: 0,
            app: None,
            size: Cell::new(PxSize::zero()),
            partial_size: Cell::new(PxSize::zero()),
            ppi: Cell::new(None),
            opaque: Cell::new(true),
            partial_bgra8: RefCell::new(None),
            bgra8,
            done_signal: SignalOnce::new_set(),
        }))
    }

    /// Returns a future that awaits until this image is loaded or encountered an error.
    pub fn awaiter(&self) -> impl std::future::Future<Output = ()> + Send + Sync + 'static {
        self.0.done_signal.clone()
    }

    /// Tries to encode the image to the format.
    ///
    /// The `format` must be one of the [`image_encoders`] supported by the view-process backend.
    ///
    /// [`image_encoders`]: View::image_encoders.
    #[allow(clippy::await_holding_refcell_ref)] // false positive
    pub async fn encode(&self, format: String) -> std::result::Result<Arc<Vec<u8>>, EncodeError> {
        self.awaiter().await;

        if let Some(e) = self.error() {
            return Err(EncodeError::Encode(e.to_owned()));
        }

        if let Some(app) = &self.0.app {
            let mut app = app.borrow_mut();
            app.process.encode_image(self.0.id, format.clone())?;

            let (sender, receiver) = flume::bounded(1);
            if let Some(entry) = app
                .encoding_images
                .iter_mut()
                .find(|r| r.image_id == self.0.id && r.format == format)
            {
                entry.listeners.push(sender);
            } else {
                app.encoding_images.push(EncodeRequest {
                    image_id: self.0.id,
                    format,
                    listeners: vec![sender],
                });
            }
            drop(app);
            receiver.recv_async().await?
        } else {
            Err(EncodeError::Dummy)
        }
    }

    pub(crate) fn done_signal(&self) -> SignalOnce {
        self.0.done_signal.clone()
    }
}

/// Error returned by [`ViewImage::encode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// Encode error.
    Encode(String),
    /// Attempted to encode dummy image.
    ///
    /// In a headless-app without renderer all images are dummy because there is no
    /// view-process backend running.
    Dummy,
    /// The View-Process disconnected or has not finished initializing yet, try again after [`ViewProcessInitedEvent`].
    ViewProcessOffline,
}
impl From<String> for EncodeError {
    fn from(e: String) -> Self {
        EncodeError::Encode(e)
    }
}
impl From<ViewProcessOffline> for EncodeError {
    fn from(_: ViewProcessOffline) -> Self {
        EncodeError::ViewProcessOffline
    }
}
impl From<flume::RecvError> for EncodeError {
    fn from(_: flume::RecvError) -> Self {
        EncodeError::ViewProcessOffline
    }
}
impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::Encode(e) => write!(f, "{e}"),
            EncodeError::Dummy => write!(f, "cannot encode dummy image"),
            EncodeError::ViewProcessOffline => write!(f, "{}", ViewProcessOffline),
        }
    }
}
impl std::error::Error for EncodeError {}

/// Connection to an image loading or loaded in the View Process.
///
/// The image is removed from the View Process cache when all clones of [`ViewImage`] drops, but
/// if there is another image pointer holding the image, this weak pointer can be upgraded back
/// to a strong connection to the image.
#[derive(Clone)]
pub struct WeakViewImage(rc::Weak<ImageConnection>);
impl WeakViewImage {
    /// Attempt to upgrade the weak pointer to the image to a full image.
    ///
    /// Returns `Some` if the is at least another [`ViewImage`] holding the image alive.
    pub fn upgrade(&self) -> Option<ViewImage> {
        self.0.upgrade().map(ViewImage)
    }
}

#[derive(Debug)]
struct WindowConnection {
    id: ApiWindowId,
    id_namespace: IdNamespace,
    pipeline_id: PipelineId,
    generation: ViewProcessGen,
    app: Rc<RefCell<ViewApp>>,
}
impl WindowConnection {
    fn online(&self) -> bool {
        let vp = self.app.borrow();
        vp.process.online() && self.generation == vp.process.generation()
    }

    fn call<R>(&self, f: impl FnOnce(ApiWindowId, &mut Controller) -> Result<R>) -> Result<R> {
        let mut app = self.app.borrow_mut();
        if app.check_generation() {
            Err(ViewProcessOffline)
        } else {
            f(self.id, &mut app.process)
        }
    }
}
impl Drop for WindowConnection {
    fn drop(&mut self) {
        let mut app = self.app.borrow_mut();
        if self.generation == app.process.generation() {
            let _ = app.process.close_window(self.id);
        }
    }
}

/// Connection to a window open in the View Process.
///
/// This is a strong reference to the window connection. The window closes when all clones of this struct drops.
#[derive(Clone, Debug)]
pub struct ViewWindow(Rc<WindowConnection>);
impl PartialEq for ViewWindow {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id && self.0.generation == other.0.generation
    }
}
impl Eq for ViewWindow {}
impl ViewWindow {
    /// Returns `true` if this window connection is still valid.
    ///
    /// The connection can be permanently lost in case the "view-process" respawns, in this
    /// case all methods will return [`ViewProcessOffline`], and you must discard this connection and
    /// create a new one.
    pub fn online(&self) -> bool {
        self.0.online()
    }

    /// Returns the view-process generation on which the window was open.
    pub fn generation(&self) -> ViewProcessGen {
        self.0.generation
    }

    /// Set the window title.
    pub fn set_title(&self, title: String) -> Result<()> {
        self.0.call(|id, p| p.set_title(id, title))
    }

    /// Set the window visibility.
    pub fn set_visible(&self, visible: bool) -> Result<()> {
        self.0.call(|id, p| p.set_visible(id, visible))
    }

    /// Set if the window is "top-most".
    pub fn set_always_on_top(&self, always_on_top: bool) -> Result<()> {
        self.0.call(|id, p| p.set_always_on_top(id, always_on_top))
    }

    /// Set if the user can drag-move the window.
    pub fn set_movable(&self, movable: bool) -> Result<()> {
        self.0.call(|id, p| p.set_movable(id, movable))
    }

    /// Set if the user can resize the window.
    pub fn set_resizable(&self, resizable: bool) -> Result<()> {
        self.0.call(|id, p| p.set_resizable(id, resizable))
    }

    /// Set the window icon.
    pub fn set_icon(&self, icon: Option<&ViewImage>) -> Result<()> {
        self.0.call(|id, p| {
            if let Some(icon) = icon {
                if p.generation() == icon.0.generation {
                    p.set_icon(id, Some(icon.0.id))
                } else {
                    Err(ViewProcessOffline)
                }
            } else {
                p.set_icon(id, None)
            }
        })
    }

    /// Set the window cursor icon and visibility.
    pub fn set_cursor(&self, icon: Option<CursorIcon>) -> Result<()> {
        self.0.call(|id, p| p.set_cursor(id, icon))
    }

    /// Set the window icon visibility in the taskbar.
    pub fn set_taskbar_visible(&self, visible: bool) -> Result<()> {
        self.0.call(|id, p| p.set_taskbar_visible(id, visible))
    }

    /// Set the window parent and if `self` has a modal connection to it.
    ///
    /// The `parent` window must be already open or this returns `WindowNotFound(0)`.
    pub fn set_parent(&self, parent: Option<WindowId>, modal: bool) -> Result<()> {
        self.0.call(|id, p| p.set_parent(id, parent.map(WindowId::get), modal))
    }

    /// Set the window state.
    pub fn set_state(&self, state: WindowStateAll) -> Result<()> {
        self.0.call(|id, p| p.set_state(id, state))
    }

    /// Set video mode used in exclusive fullscreen.
    pub fn set_video_mode(&self, mode: VideoMode) -> Result<()> {
        self.0.call(|id, p| p.set_video_mode(id, mode))
    }

    /// Reference the window renderer.
    pub fn renderer(&self) -> ViewRenderer {
        ViewRenderer(Rc::downgrade(&self.0))
    }

    /// Sets if the headed window is in *capture-mode*. If `true` the resources used to capture
    /// a screenshot are kept in memory to be reused in the next screenshot capture.
    pub fn set_capture_mode(&self, enabled: bool) -> Result<()> {
        self.0.call(|id, p| p.set_capture_mode(id, enabled))
    }

    /// Brings the window to the front and sets input focus.
    ///
    /// This request can steal focus from other apps disrupting the user, be careful with it.
    pub fn focus(&self) -> Result<()> {
        self.0.call(|id, p| p.focus_window(id))
    }

    /// Sets the user attention request indicator, the indicator is cleared when the window is focused or
    /// if canceled by setting to `None`.
    pub fn set_focus_indicator(&self, indicator: Option<FocusIndicator>) -> Result<()> {
        self.0.call(|id, p| p.set_focus_indicator(id, indicator))
    }

    /// Drop `self`.
    pub fn close(self) {
        drop(self)
    }
}

/// Connection to a headless surface/document open in the View Process.
///
/// This is a strong reference to the window connection. The view is disposed when every reference drops.
#[derive(Clone, Debug)]
pub struct ViewHeadless(Rc<WindowConnection>);
impl PartialEq for ViewHeadless {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id && self.0.generation == other.0.generation
    }
}
impl Eq for ViewHeadless {}
impl ViewHeadless {
    /// Resize the headless surface.
    pub fn set_size(&self, size: DipSize, scale_factor: Factor) -> Result<()> {
        self.0.call(|id, p| p.set_headless_size(id, size, scale_factor.0))
    }

    /// Reference the window renderer.
    pub fn renderer(&self) -> ViewRenderer {
        ViewRenderer(Rc::downgrade(&self.0))
    }
}

/// Connection to a renderer in the View Process.
///
/// This is only a weak reference, every method returns [`ViewProcessOffline`] if the
/// renderer has been dropped.
#[derive(Clone, Debug)]
pub struct ViewRenderer(rc::Weak<WindowConnection>);
impl PartialEq for ViewRenderer {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(s), Some(o)) = (self.0.upgrade(), other.0.upgrade()) {
            s.id == o.id && s.generation == o.generation
        } else {
            false
        }
    }
}
impl ViewRenderer {
    fn call<R>(&self, f: impl FnOnce(ApiWindowId, &mut Controller) -> Result<R>) -> Result<R> {
        if let Some(c) = self.0.upgrade() {
            c.call(f)
        } else {
            Err(ViewProcessOffline)
        }
    }

    /// Returns the view-process generation on which the renderer was created.
    pub fn generation(&self) -> Result<ViewProcessGen> {
        self.0.upgrade().map(|c| c.generation).ok_or(ViewProcessOffline)
    }

    /// Returns `true` if the renderer is still alive.
    ///
    /// The renderer is dropped when the window closes or the view-process respawns.
    pub fn online(&self) -> bool {
        self.0.upgrade().map(|c| c.online()).unwrap_or(false)
    }

    /// Pipeline ID.
    ///
    /// This value is cached locally (not an IPC call).
    pub fn pipeline_id(&self) -> Result<PipelineId> {
        if let Some(c) = self.0.upgrade() {
            if c.online() {
                return Ok(c.pipeline_id);
            }
        }
        Err(ViewProcessOffline)
    }

    /// Resource namespace.
    ///
    /// This value is cached locally (not an IPC call).
    pub fn namespace_id(&self) -> Result<IdNamespace> {
        if let Some(c) = self.0.upgrade() {
            if c.online() {
                return Ok(c.id_namespace);
            }
        }
        Err(ViewProcessOffline)
    }

    /// Use an image resource in the window renderer.
    ///
    /// Returns the image key.
    pub fn use_image(&self, image: &ViewImage) -> Result<ImageKey> {
        self.call(|id, p| {
            if p.generation() == image.0.generation {
                p.use_image(id, image.0.id)
            } else {
                Err(ViewProcessOffline)
            }
        })
    }

    /// Replace the image resource in the window renderer.
    pub fn update_image_use(&mut self, key: ImageKey, image: &ViewImage) -> Result<()> {
        self.call(|id, p| {
            if p.generation() == image.0.generation {
                p.update_image_use(id, key, image.0.id)
            } else {
                Err(ViewProcessOffline)
            }
        })
    }

    /// Delete the image resource in the window renderer.
    pub fn delete_image_use(&mut self, key: ImageKey) -> Result<()> {
        self.call(|id, p| p.delete_image_use(id, key))
    }

    /// Add a raw font resource to the window renderer.
    ///
    /// Returns the new font key.
    pub fn add_font(&self, bytes: Vec<u8>, index: u32) -> Result<FontKey> {
        self.call(|id, p| p.add_font(id, IpcBytes::from_vec(bytes), index))
    }

    /// Delete the font resource in the window renderer.
    pub fn delete_font(&self, key: FontKey) -> Result<()> {
        self.call(|id, p| p.delete_font(id, key))
    }

    /// Add a font instance to the window renderer.
    ///
    /// Returns the new instance key.
    pub fn add_font_instance(
        &self,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<FontInstanceOptions>,
        plataform_options: Option<FontInstancePlatformOptions>,
        variations: Vec<FontVariation>,
    ) -> Result<FontInstanceKey> {
        self.call(|id, p| p.add_font_instance(id, font_key, glyph_size, options, plataform_options, variations))
    }

    /// Delete the font instance.
    pub fn delete_font_instance(&self, key: FontInstanceKey) -> Result<()> {
        self.call(|id, p| p.delete_font_instance(id, key))
    }

    /// Create a new image resource from the current rendered frame.
    pub fn frame_image(&self) -> Result<ViewImage> {
        if let Some(c) = self.0.upgrade() {
            let id = c.call(|id, p| p.frame_image(id))?;
            Ok(Self::add_frame_image(&c.app, id))
        } else {
            Err(ViewProcessOffline)
        }
    }

    /// Create a new image resource from a selection of the current rendered frame.
    pub fn frame_image_rect(&self, rect: PxRect) -> Result<ViewImage> {
        if let Some(c) = self.0.upgrade() {
            let id = c.call(|id, p| p.frame_image_rect(id, rect))?;
            Ok(Self::add_frame_image(&c.app, id))
        } else {
            Err(ViewProcessOffline)
        }
    }

    fn add_frame_image(app: &Rc<RefCell<ViewApp>>, id: ImageId) -> ViewImage {
        if id == 0 {
            ViewImage::dummy(None)
        } else {
            let mut app_mut = app.borrow_mut();
            let img = ViewImage(Rc::new(ImageConnection {
                id,
                generation: app_mut.process.generation(),
                app: Some(app.clone()),
                size: Cell::new(PxSize::zero()),
                partial_size: Cell::new(PxSize::zero()),
                ppi: Cell::new(None),
                opaque: Cell::new(false),
                partial_bgra8: RefCell::new(None),
                bgra8: OnceCell::new(),
                done_signal: SignalOnce::new(),
            }));

            app_mut.loading_images.push(Rc::downgrade(&img.0));
            app_mut.frame_images.push(Rc::downgrade(&img.0));

            img
        }
    }

    /// Render a new frame.
    pub fn render(&self, frame: FrameRequest) -> Result<()> {
        let _s = tracing::debug_span!("ViewRenderer.render").entered();

        if let Some(w) = self.0.upgrade() {
            w.call(|id, p| p.render(id, frame))?;
            w.app.borrow_mut().pending_frames += 1;
            Ok(())
        } else {
            Err(ViewProcessOffline)
        }
    }

    /// Update the current frame and re-render it.
    pub fn render_update(&self, frame: FrameUpdateRequest) -> Result<()> {
        let _s = tracing::debug_span!("ViewRenderer.render_update").entered();

        if let Some(w) = self.0.upgrade() {
            w.call(|id, p| p.render_update(id, frame))?;
            w.app.borrow_mut().pending_frames += 1;
            Ok(())
        } else {
            Err(ViewProcessOffline)
        }
    }
}

event_args! {
    /// Arguments for the [`VIEW_PROCESS_INITED_EVENT`].
    pub struct ViewProcessInitedArgs {
        /// View-process generation.
        pub generation: ViewProcessGen,

        /// If this is not the first time a view-process was inited. If `true`
        /// all resources created in a previous generation must be rebuilt.
        pub is_respawn: bool,

        /// Monitors list.
        pub available_monitors: Vec<(MonitorId, MonitorInfo)>,

        /// System multi-click config.
        pub multi_click_config: MultiClickConfig,

        /// System keyboard pressed repeat delay config.
        pub key_repeat_delay: Duration,

        /// System font font-aliasing config.
        pub font_aa: FontAntiAliasing,

        /// System animations config.
        ///
        /// People with photosensitive epilepsy usually disable animations system wide.
        pub animations_enabled: bool,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self) -> EventDeliveryList {
            EventDeliveryList::all()
        }
    }
}

event! {
    /// View Process finished initializing and is now online.
    pub static VIEW_PROCESS_INITED_EVENT: ViewProcessInitedArgs;
}

/// Information about a successfully opened window.
#[derive(Debug, Clone)]
pub struct WindowOpenData {
    /// Window complete state.
    pub state: WindowStateAll,

    /// Monitor that contains the window.
    pub monitor: Option<MonitorId>,

    /// Final top-left offset of the window (excluding outer chrome).
    ///
    /// The position is relative to the monitor.
    pub position: DipPoint,
    /// Final dimensions of the client area of the window (excluding outer chrome).
    pub size: DipSize,

    /// Final scale factor.
    pub scale_factor: f32,

    /// Actual render mode, can be different from the requested mode if it is not available.
    pub render_mode: RenderMode,

    /// Preferred color scheme.
    pub color_scheme: ColorScheme,
}
impl WindowOpenData {
    fn new(data: zero_ui_view_api::WindowOpenData, map_monitor: impl FnOnce(zero_ui_view_api::MonitorId) -> MonitorId) -> Self {
        WindowOpenData {
            state: data.state,
            monitor: data.monitor.map(map_monitor),
            position: data.position,
            size: data.size,
            scale_factor: data.scale_factor,
            render_mode: data.render_mode,
            color_scheme: data.color_scheme,
        }
    }
}
