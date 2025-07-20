//! View process connection and other types.

use std::{
    collections::HashMap,
    fmt,
    path::PathBuf,
    sync::{self, Arc},
};

pub mod raw_device_events;
pub mod raw_events;

use crate::{
    event::{event, event_args},
    window::{MonitorId, WindowId},
};

use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock};
use zng_app_context::app_local;
use zng_layout::unit::{DipPoint, DipRect, DipSideOffsets, DipSize, Factor, Px, PxPoint, PxRect, PxSize};
use zng_task::SignalOnce;
use zng_txt::Txt;
use zng_var::ResponderVar;
use zng_view_api::{
    self, DeviceEventsFilter, DragDropId, Event, FocusResult, ViewProcessGen,
    api_extension::{ApiExtensionId, ApiExtensionName, ApiExtensionPayload, ApiExtensionRecvError, ApiExtensions},
    config::{AnimationsConfig, ChromeConfig, ColorsConfig, FontAntiAliasing, LocaleConfig, TouchConfig},
    dialog::{FileDialog, FileDialogResponse, MsgDialog, MsgDialogResponse},
    drag_drop::{DragDropData, DragDropEffect, DragDropError},
    font::FontOptions,
    image::{ImageMaskMode, ImagePpi, ImageRequest, ImageTextureId},
    ipc::{IpcBytes, IpcBytesReceiver, ViewChannelError},
    window::{
        CursorIcon, FocusIndicator, FrameRequest, FrameUpdateRequest, HeadlessOpenData, HeadlessRequest, RenderMode, ResizeDirection,
        VideoMode, WindowButton, WindowRequest, WindowStateAll,
    },
};

pub(crate) use zng_view_api::{
    Controller, raw_input::InputDeviceId as ApiDeviceId, window::MonitorId as ApiMonitorId, window::WindowId as ApiWindowId,
};
use zng_view_api::{
    clipboard::{ClipboardData, ClipboardError, ClipboardType},
    font::{FontFaceId, FontId, FontVariationName},
    image::{ImageId, ImageLoadedData},
};

use self::raw_device_events::InputDeviceId;

use super::{APP, AppId};

/// Connection to the running view-process for the context app.
#[expect(non_camel_case_types)]
pub struct VIEW_PROCESS;
struct ViewProcessService {
    process: zng_view_api::Controller,
    input_device_ids: HashMap<ApiDeviceId, InputDeviceId>,
    monitor_ids: HashMap<ApiMonitorId, MonitorId>,

    data_generation: ViewProcessGen,

    extensions: ApiExtensions,

    loading_images: Vec<sync::Weak<RwLock<ViewImageData>>>,
    frame_images: Vec<sync::Weak<RwLock<ViewImageData>>>,
    encoding_images: Vec<EncodeRequest>,

    pending_frames: usize,

    message_dialogs: Vec<(zng_view_api::dialog::DialogId, ResponderVar<MsgDialogResponse>)>,
    file_dialogs: Vec<(zng_view_api::dialog::DialogId, ResponderVar<FileDialogResponse>)>,
}
app_local! {
    static VIEW_PROCESS_SV: Option<ViewProcessService> = None;
}
impl VIEW_PROCESS {
    /// If the `VIEW_PROCESS` can be used, this is only true in app threads for apps with render, all other
    /// methods will panic if called when this is not true.
    pub fn is_available(&self) -> bool {
        APP.is_running() && VIEW_PROCESS_SV.read().is_some()
    }

    fn read(&self) -> MappedRwLockReadGuard<'_, ViewProcessService> {
        VIEW_PROCESS_SV.read_map(|e| e.as_ref().expect("VIEW_PROCESS not available"))
    }

    fn write(&self) -> MappedRwLockWriteGuard<'_, ViewProcessService> {
        VIEW_PROCESS_SV.write_map(|e| e.as_mut().expect("VIEW_PROCESS not available"))
    }

    fn try_write(&self) -> Result<MappedRwLockWriteGuard<'_, ViewProcessService>> {
        let vp = VIEW_PROCESS_SV.write();
        if let Some(w) = &*vp {
            if w.process.is_connected() {
                return Ok(MappedRwLockWriteGuard::map(vp, |w| w.as_mut().unwrap()));
            }
        }
        Err(ViewChannelError::Disconnected)
    }

    fn check_app(&self, id: AppId) {
        let actual = APP.id();
        if Some(id) != actual {
            panic!("cannot use view handle from app `{id:?}` in app `{actual:?}`");
        }
    }

    fn handle_write(&self, id: AppId) -> MappedRwLockWriteGuard<'_, ViewProcessService> {
        self.check_app(id);
        self.write()
    }

    /// View-process running, connected and ready.
    pub fn is_connected(&self) -> bool {
        self.read().process.is_connected()
    }

    /// If is running in headless renderer mode.
    pub fn is_headless_with_render(&self) -> bool {
        self.read().process.headless()
    }

    /// If is running both view and app in the same process.
    pub fn is_same_process(&self) -> bool {
        self.read().process.same_process()
    }

    /// Gets the current view-process generation.
    pub fn generation(&self) -> ViewProcessGen {
        self.read().process.generation()
    }

    /// Enable/disable global device events.
    ///
    /// This filter affects device events not targeted at windows, such as mouse move outside windows or
    /// key presses when the app has no focused window.
    pub fn set_device_events_filter(&self, filter: DeviceEventsFilter) -> Result<()> {
        self.write().process.set_device_events_filter(filter)
    }

    /// Sends a request to open a window and associate it with the `window_id`.
    ///
    /// A [`RAW_WINDOW_OPEN_EVENT`] or [`RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT`] will be received in response to this request.
    ///
    /// [`RAW_WINDOW_OPEN_EVENT`]: crate::view_process::raw_events::RAW_WINDOW_OPEN_EVENT
    /// [`RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT`]: crate::view_process::raw_events::RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT
    pub fn open_window(&self, config: WindowRequest) -> Result<()> {
        let _s = tracing::debug_span!("VIEW_PROCESS.open_window").entered();
        self.write().process.open_window(config)
    }

    /// Sends a request to open a headless renderer and associate it with the `window_id`.
    ///
    /// Note that no actual window is created, only the renderer, the use of window-ids to identify
    /// this renderer is only for convenience.
    ///
    /// A [`RAW_HEADLESS_OPEN_EVENT`] or [`RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT`] will be received in response to this request.
    ///
    /// [`RAW_HEADLESS_OPEN_EVENT`]: crate::view_process::raw_events::RAW_HEADLESS_OPEN_EVENT
    /// [`RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT`]: crate::view_process::raw_events::RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT
    pub fn open_headless(&self, config: HeadlessRequest) -> Result<()> {
        let _s = tracing::debug_span!("VIEW_PROCESS.open_headless").entered();
        self.write().process.open_headless(config)
    }

    /// Send an image for decoding.
    ///
    /// This function returns immediately, the [`ViewImage`] will update when
    /// [`Event::ImageMetadataLoaded`], [`Event::ImageLoaded`] and [`Event::ImageLoadError`] events are received.
    ///
    /// [`Event::ImageMetadataLoaded`]: zng_view_api::Event::ImageMetadataLoaded
    /// [`Event::ImageLoaded`]: zng_view_api::Event::ImageLoaded
    /// [`Event::ImageLoadError`]: zng_view_api::Event::ImageLoadError
    pub fn add_image(&self, request: ImageRequest<IpcBytes>) -> Result<ViewImage> {
        let mut app = self.write();
        let id = app.process.add_image(request)?;
        let img = ViewImage(Arc::new(RwLock::new(ViewImageData {
            id: Some(id),
            app_id: APP.id(),
            generation: app.process.generation(),
            size: PxSize::zero(),
            partial_size: PxSize::zero(),
            ppi: None,
            is_opaque: false,
            partial_pixels: None,
            pixels: None,
            is_mask: false,
            done_signal: SignalOnce::new(),
        })));
        app.loading_images.push(Arc::downgrade(&img.0));
        Ok(img)
    }

    /// Starts sending an image for *progressive* decoding.
    ///
    /// This function returns immediately, the [`ViewImage`] will update when
    /// [`Event::ImageMetadataLoaded`], [`Event::ImagePartiallyLoaded`],
    /// [`Event::ImageLoaded`] and [`Event::ImageLoadError`] events are received.
    ///
    /// [`Event::ImageMetadataLoaded`]: zng_view_api::Event::ImageMetadataLoaded
    /// [`Event::ImageLoaded`]: zng_view_api::Event::ImageLoaded
    /// [`Event::ImageLoadError`]: zng_view_api::Event::ImageLoadError
    /// [`Event::ImagePartiallyLoaded`]: zng_view_api::Event::ImagePartiallyLoaded
    pub fn add_image_pro(&self, request: ImageRequest<IpcBytesReceiver>) -> Result<ViewImage> {
        let mut app = self.write();
        let id = app.process.add_image_pro(request)?;
        let img = ViewImage(Arc::new(RwLock::new(ViewImageData {
            id: Some(id),
            app_id: APP.id(),
            generation: app.process.generation(),
            size: PxSize::zero(),
            partial_size: PxSize::zero(),
            ppi: None,
            is_opaque: false,
            partial_pixels: None,
            pixels: None,
            is_mask: false,
            done_signal: SignalOnce::new(),
        })));
        app.loading_images.push(Arc::downgrade(&img.0));
        Ok(img)
    }

    /// View-process clipboard methods.
    pub fn clipboard(&self) -> Result<&ViewClipboard> {
        if VIEW_PROCESS.is_connected() {
            Ok(&ViewClipboard {})
        } else {
            Err(ViewChannelError::Disconnected)
        }
    }

    /// Returns a list of image decoders supported by the view-process backend.
    ///
    /// Each text is the lower-case file extension, without the dot.
    pub fn image_decoders(&self) -> Result<Vec<Txt>> {
        self.write().process.image_decoders()
    }

    /// Returns a list of image encoders supported by the view-process backend.
    ///
    /// Each text is the lower-case file extension, without the dot.
    pub fn image_encoders(&self) -> Result<Vec<Txt>> {
        self.write().process.image_encoders()
    }

    /// Number of frame send that have not finished rendering.
    ///
    /// This is the sum of pending frames for all renderers.
    pub fn pending_frames(&self) -> usize {
        self.write().pending_frames
    }

    /// Reopen the view-process, causing another [`Event::Inited`].
    ///
    /// [`Event::Inited`]: zng_view_api::Event::Inited
    pub fn respawn(&self) {
        self.write().process.respawn()
    }

    /// Gets the ID for the `extension_name` in the current view-process.
    ///
    /// The ID can change for every view-process instance, you must subscribe to the
    /// [`VIEW_PROCESS_INITED_EVENT`] to refresh the ID. The view-process can respawn
    /// at any time in case of error.
    pub fn extension_id(&self, extension_name: impl Into<ApiExtensionName>) -> Result<Option<ApiExtensionId>> {
        let me = self.read();
        if me.process.is_connected() {
            Ok(me.extensions.id(&extension_name.into()))
        } else {
            Err(ViewChannelError::Disconnected)
        }
    }

    /// Licenses that may be required to be displayed in the app about screen.
    ///
    /// This is specially important for prebuilt view users, as the tools that scrap licenses
    /// may not find the prebuilt dependencies.
    pub fn third_party_licenses(&self) -> Result<Vec<crate::third_party::LicenseUsed>> {
        self.write().process.third_party_licenses()
    }

    /// Call an extension with custom encoded payload.
    pub fn app_extension_raw(&self, extension_id: ApiExtensionId, extension_request: ApiExtensionPayload) -> Result<ApiExtensionPayload> {
        self.write().process.app_extension(extension_id, extension_request)
    }

    /// Call an extension with payload `request`.
    pub fn app_extension<I, O>(&self, extension_id: ApiExtensionId, request: &I) -> Result<std::result::Result<O, ApiExtensionRecvError>>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let payload = ApiExtensionPayload::serialize(&request).unwrap();
        let response = self.write().process.app_extension(extension_id, payload)?;
        Ok(response.deserialize::<O>())
    }

    /// Handle an [`Event::Disconnected`].
    ///
    /// The process will exit if the view-process was killed by the user.
    ///
    /// [`Event::Disconnected`]: zng_view_api::Event::Disconnected
    pub fn handle_disconnect(&self, vp_gen: ViewProcessGen) {
        self.write().process.handle_disconnect(vp_gen)
    }

    /// Spawn the View Process.
    pub(super) fn start<F>(&self, view_process_exe: PathBuf, view_process_env: HashMap<Txt, Txt>, headless: bool, on_event: F)
    where
        F: FnMut(Event) + Send + 'static,
    {
        let _s = tracing::debug_span!("VIEW_PROCESS.start").entered();

        let process = zng_view_api::Controller::start(view_process_exe, view_process_env, headless, on_event);
        *VIEW_PROCESS_SV.write() = Some(ViewProcessService {
            data_generation: process.generation(),
            process,
            input_device_ids: HashMap::default(),
            monitor_ids: HashMap::default(),
            loading_images: vec![],
            encoding_images: vec![],
            frame_images: vec![],
            pending_frames: 0,
            message_dialogs: vec![],
            file_dialogs: vec![],
            extensions: ApiExtensions::default(),
        });
    }

    pub(crate) fn on_window_opened(&self, window_id: WindowId, data: zng_view_api::window::WindowOpenData) -> (ViewWindow, WindowOpenData) {
        let mut app = self.write();
        let _ = app.check_generation();

        let win = ViewWindow(Arc::new(ViewWindowData {
            app_id: APP.id().unwrap(),
            id: ApiWindowId::from_raw(window_id.get()),
            generation: app.data_generation,
        }));
        drop(app);

        let data = WindowOpenData::new(data, |id| self.monitor_id(id));

        (win, data)
    }
    /// Translate input device ID, generates a device id if it was unknown.
    pub(super) fn input_device_id(&self, id: ApiDeviceId) -> InputDeviceId {
        *self.write().input_device_ids.entry(id).or_insert_with(InputDeviceId::new_unique)
    }

    /// Translate `MonId` to `MonitorId`, generates a monitor id if it was unknown.
    pub(super) fn monitor_id(&self, id: ApiMonitorId) -> MonitorId {
        *self.write().monitor_ids.entry(id).or_insert_with(MonitorId::new_unique)
    }

    /// Handle an [`Event::Inited`].
    ///
    /// The view-process becomes "connected" only after this call.
    ///
    /// [`Event::Inited`]: zng_view_api::Event::Inited
    pub(super) fn handle_inited(&self, vp_gen: ViewProcessGen, extensions: ApiExtensions) {
        let mut me = self.write();
        me.extensions = extensions;
        me.process.handle_inited(vp_gen);
    }

    pub(super) fn handle_suspended(&self) {
        self.write().process.handle_suspended();
    }

    pub(crate) fn on_headless_opened(
        &self,
        id: WindowId,
        data: zng_view_api::window::HeadlessOpenData,
    ) -> (ViewHeadless, HeadlessOpenData) {
        let mut app = self.write();
        let _ = app.check_generation();

        let surf = ViewHeadless(Arc::new(ViewWindowData {
            app_id: APP.id().unwrap(),
            id: ApiWindowId::from_raw(id.get()),
            generation: app.data_generation,
        }));

        (surf, data)
    }

    fn loading_image_index(&self, id: ImageId) -> Option<usize> {
        let mut app = self.write();

        // cleanup
        app.loading_images.retain(|i| i.strong_count() > 0);

        app.loading_images.iter().position(|i| i.upgrade().unwrap().read().id == Some(id))
    }

    pub(super) fn on_image_metadata_loaded(&self, id: ImageId, size: PxSize, ppi: Option<ImagePpi>, is_mask: bool) -> Option<ViewImage> {
        if let Some(i) = self.loading_image_index(id) {
            let img = self.read().loading_images[i].upgrade().unwrap();
            {
                let mut img = img.write();
                img.size = size;
                img.ppi = ppi;
                img.is_mask = is_mask;
            }
            Some(ViewImage(img))
        } else {
            None
        }
    }

    pub(super) fn on_image_partially_loaded(
        &self,
        id: ImageId,
        partial_size: PxSize,
        ppi: Option<ImagePpi>,
        is_opaque: bool,
        is_mask: bool,
        partial_pixels: IpcBytes,
    ) -> Option<ViewImage> {
        if let Some(i) = self.loading_image_index(id) {
            let img = self.read().loading_images[i].upgrade().unwrap();
            {
                let mut img = img.write();
                img.partial_size = partial_size;
                img.ppi = ppi;
                img.is_opaque = is_opaque;
                img.partial_pixels = Some(partial_pixels);
                img.is_mask = is_mask;
            }
            Some(ViewImage(img))
        } else {
            None
        }
    }

    pub(super) fn on_image_loaded(&self, data: ImageLoadedData) -> Option<ViewImage> {
        if let Some(i) = self.loading_image_index(data.id) {
            let img = self.write().loading_images.swap_remove(i).upgrade().unwrap();
            {
                let mut img = img.write();
                img.size = data.size;
                img.partial_size = data.size;
                img.ppi = data.ppi;
                img.is_opaque = data.is_opaque;
                img.pixels = Some(Ok(data.pixels));
                img.partial_pixels = None;
                img.is_mask = data.is_mask;
                img.done_signal.set();
            }
            Some(ViewImage(img))
        } else {
            None
        }
    }

    pub(super) fn on_image_error(&self, id: ImageId, error: Txt) -> Option<ViewImage> {
        if let Some(i) = self.loading_image_index(id) {
            let img = self.write().loading_images.swap_remove(i).upgrade().unwrap();
            {
                let mut img = img.write();
                img.pixels = Some(Err(error));
                img.done_signal.set();
            }
            Some(ViewImage(img))
        } else {
            None
        }
    }

    pub(crate) fn on_frame_rendered(&self, _id: WindowId) {
        let mut vp = self.write();
        vp.pending_frames = vp.pending_frames.saturating_sub(1);
    }

    pub(crate) fn on_frame_image(&self, data: ImageLoadedData) -> ViewImage {
        ViewImage(Arc::new(RwLock::new(ViewImageData {
            app_id: APP.id(),
            id: Some(data.id),
            generation: self.generation(),
            size: data.size,
            partial_size: data.size,
            ppi: data.ppi,
            is_opaque: data.is_opaque,
            partial_pixels: None,
            pixels: Some(Ok(data.pixels)),
            is_mask: data.is_mask,
            done_signal: SignalOnce::new_set(),
        })))
    }

    pub(super) fn on_frame_image_ready(&self, id: ImageId) -> Option<ViewImage> {
        let mut app = self.write();

        // cleanup
        app.frame_images.retain(|i| i.strong_count() > 0);

        let i = app.frame_images.iter().position(|i| i.upgrade().unwrap().read().id == Some(id));

        i.map(|i| ViewImage(app.frame_images.swap_remove(i).upgrade().unwrap()))
    }

    pub(super) fn on_image_encoded(&self, id: ImageId, format: Txt, data: IpcBytes) {
        self.on_image_encode_result(id, format, Ok(data));
    }
    pub(super) fn on_image_encode_error(&self, id: ImageId, format: Txt, error: Txt) {
        self.on_image_encode_result(id, format, Err(EncodeError::Encode(error)));
    }
    fn on_image_encode_result(&self, id: ImageId, format: Txt, result: std::result::Result<IpcBytes, EncodeError>) {
        let mut app = self.write();
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

    pub(crate) fn on_message_dlg_response(&self, id: zng_view_api::dialog::DialogId, response: MsgDialogResponse) {
        let mut app = self.write();
        if let Some(i) = app.message_dialogs.iter().position(|(i, _)| *i == id) {
            let (_, r) = app.message_dialogs.swap_remove(i);
            r.respond(response);
        }
    }

    pub(crate) fn on_file_dlg_response(&self, id: zng_view_api::dialog::DialogId, response: FileDialogResponse) {
        let mut app = self.write();
        if let Some(i) = app.file_dialogs.iter().position(|(i, _)| *i == id) {
            let (_, r) = app.file_dialogs.swap_remove(i);
            r.respond(response);
        }
    }

    pub(super) fn on_respawned(&self, _gen: ViewProcessGen) {
        let mut app = self.write();
        app.pending_frames = 0;
        for (_, r) in app.message_dialogs.drain(..) {
            r.respond(MsgDialogResponse::Error(Txt::from_static("respawn")));
        }
    }

    pub(crate) fn exit(&self) {
        *VIEW_PROCESS_SV.write() = None;
    }
}
impl ViewProcessService {
    #[must_use = "if `true` all current WinId, DevId and MonId are invalid"]
    fn check_generation(&mut self) -> bool {
        let vp_gen = self.process.generation();
        let invalid = vp_gen != self.data_generation;
        if invalid {
            self.data_generation = vp_gen;
            self.input_device_ids.clear();
            self.monitor_ids.clear();
        }
        invalid
    }
}

event_args! {
    /// Arguments for the [`VIEW_PROCESS_INITED_EVENT`].
    pub struct ViewProcessInitedArgs {
        /// View-process generation.
        pub generation: ViewProcessGen,

        /// If this is not the first time a view-process was inited. If `true`
        /// all resources created in a previous generation must be rebuilt.
        ///
        /// This can happen after a view-process crash or app suspension.
        pub is_respawn: bool,

        /// System touch config.
        pub touch_config: TouchConfig,

        /// System font font-aliasing config.
        pub font_aa: FontAntiAliasing,

        /// System animations config.
        pub animations_config: AnimationsConfig,

        /// System locale config.
        pub locale_config: LocaleConfig,

        /// System preferred color scheme and accent color.
        pub colors_config: ColorsConfig,

        /// System window chrome preferences.
        pub chrome_config: ChromeConfig,

        /// API extensions implemented by the view-process.
        ///
        /// The extension IDs will stay valid for the duration of the view-process.
        pub extensions: ApiExtensions,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`VIEW_PROCESS_SUSPENDED_EVENT`].
    pub struct ViewProcessSuspendedArgs {

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}

event! {
    /// View-Process finished initializing and is now connected and ready.
    pub static VIEW_PROCESS_INITED_EVENT: ViewProcessInitedArgs;
    /// View-Process suspended, all resources dropped.
    ///
    /// The view-process will only be available if the app resumes. On resume [`VIEW_PROCESS_INITED_EVENT`]
    /// notify a view-process respawn.
    pub static VIEW_PROCESS_SUSPENDED_EVENT: ViewProcessSuspendedArgs;
}

/// Information about a successfully opened window.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WindowOpenData {
    /// Window complete state.
    pub state: WindowStateAll,

    /// Monitor that contains the window.
    pub monitor: Option<MonitorId>,

    /// Final top-left offset of the window (excluding outer chrome).
    ///
    /// The values are the global position and the position in the monitor.
    pub position: (PxPoint, DipPoint),
    /// Final dimensions of the client area of the window (excluding outer chrome).
    pub size: DipSize,

    /// Final scale factor.
    pub scale_factor: Factor,

    /// Actual render mode, can be different from the requested mode if it is not available.
    pub render_mode: RenderMode,

    /// Padding that must be applied to the window content so that it stays clear of screen obstructions
    /// such as a camera notch cutout.
    ///
    /// Note that the *unsafe* area must still be rendered as it may be partially visible, just don't place nay
    /// interactive or important content outside of this padding.
    pub safe_padding: DipSideOffsets,
}
impl WindowOpenData {
    pub(crate) fn new(data: zng_view_api::window::WindowOpenData, map_monitor: impl FnOnce(ApiMonitorId) -> MonitorId) -> Self {
        WindowOpenData {
            state: data.state,
            monitor: data.monitor.map(map_monitor),
            position: data.position,
            size: data.size,
            scale_factor: data.scale_factor,
            render_mode: data.render_mode,
            safe_padding: data.safe_padding,
        }
    }
}

/// Handle to a window open in the view-process.
///
/// The window is closed when all clones of the handle are dropped.
#[derive(Debug, Clone)]
#[must_use = "the window is closed when all clones of the handle are dropped"]
pub struct ViewWindow(Arc<ViewWindowData>);
impl PartialEq for ViewWindow {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ViewWindow {}

impl ViewWindow {
    /// Returns the view-process generation on which the window was open.
    pub fn generation(&self) -> ViewProcessGen {
        self.0.generation
    }

    /// Set the window title.
    pub fn set_title(&self, title: Txt) -> Result<()> {
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
                let icon = icon.0.read();
                if p.generation() == icon.generation {
                    p.set_icon(id, icon.id)
                } else {
                    Err(ViewChannelError::Disconnected)
                }
            } else {
                p.set_icon(id, None)
            }
        })
    }

    /// Set the window cursor icon and visibility.
    pub fn set_cursor(&self, cursor: Option<CursorIcon>) -> Result<()> {
        self.0.call(|id, p| p.set_cursor(id, cursor))
    }

    /// Set the window cursor to a custom image.
    ///
    /// Falls back to cursor icon if set to `None`.
    ///
    /// The `hotspot` value is an exact point in the image that is the mouse position. This value is only used if
    /// the image format does not contain a hotspot.
    pub fn set_cursor_image(&self, cursor: Option<&ViewImage>, hotspot: PxPoint) -> Result<()> {
        self.0.call(|id, p| {
            if let Some(cur) = cursor {
                let cur = cur.0.read();
                if p.generation() == cur.generation {
                    p.set_cursor_image(id, cur.id.map(|img| zng_view_api::window::CursorImage::new(img, hotspot)))
                } else {
                    Err(ViewChannelError::Disconnected)
                }
            } else {
                p.set_cursor_image(id, None)
            }
        })
    }

    /// Set the window icon visibility in the taskbar.
    pub fn set_taskbar_visible(&self, visible: bool) -> Result<()> {
        self.0.call(|id, p| p.set_taskbar_visible(id, visible))
    }

    /// Bring the window the z top.
    pub fn bring_to_top(&self) -> Result<()> {
        self.0.call(|id, p| p.bring_to_top(id))
    }

    /// Set the window state.
    pub fn set_state(&self, state: WindowStateAll) -> Result<()> {
        self.0.call(|id, p| p.set_state(id, state))
    }

    /// Set video mode used in exclusive fullscreen.
    pub fn set_video_mode(&self, mode: VideoMode) -> Result<()> {
        self.0.call(|id, p| p.set_video_mode(id, mode))
    }

    /// Set enabled window chrome buttons.
    pub fn set_enabled_buttons(&self, buttons: WindowButton) -> Result<()> {
        self.0.call(|id, p| p.set_enabled_buttons(id, buttons))
    }

    /// Reference the window renderer.
    pub fn renderer(&self) -> ViewRenderer {
        ViewRenderer(Arc::downgrade(&self.0))
    }

    /// Sets if the headed window is in *capture-mode*. If `true` the resources used to capture
    /// a screenshot may be kept in memory to be reused in the next screenshot capture.
    ///
    /// Note that capture must still be requested in each frame request.
    pub fn set_capture_mode(&self, enabled: bool) -> Result<()> {
        self.0.call(|id, p| p.set_capture_mode(id, enabled))
    }

    /// Brings the window to the front and sets input focus.
    ///
    /// This request can steal focus from other apps disrupting the user, be careful with it.
    pub fn focus(&self) -> Result<FocusResult> {
        self.0.call(|id, p| p.focus(id))
    }

    /// Sets the user attention request indicator, the indicator is cleared when the window is focused or
    /// if canceled by setting to `None`.
    pub fn set_focus_indicator(&self, indicator: Option<FocusIndicator>) -> Result<()> {
        self.0.call(|id, p| p.set_focus_indicator(id, indicator))
    }

    /// Moves the window with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed immediately before this function is called.
    pub fn drag_move(&self) -> Result<()> {
        self.0.call(|id, p| p.drag_move(id))
    }

    /// Resizes the window with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed immediately before this function is called.
    pub fn drag_resize(&self, direction: ResizeDirection) -> Result<()> {
        self.0.call(|id, p| p.drag_resize(id, direction))
    }

    /// Start a drag and drop operation, if the window is pressed.
    ///
    /// A [`RAW_APP_DRAG_ENDED_EVENT`] will be received when the operation finishes.
    ///
    /// [`RAW_APP_DRAG_ENDED_EVENT`]: raw_events::RAW_APP_DRAG_ENDED_EVENT
    pub fn start_drag_drop(
        &self,
        data: Vec<DragDropData>,
        allowed_effects: DragDropEffect,
    ) -> Result<std::result::Result<DragDropId, DragDropError>> {
        self.0.call(|id, p| p.start_drag_drop(id, data, allowed_effects))
    }

    /// Notify the drag source of what effect was applied for a received drag&drop.
    pub fn drag_dropped(&self, drop_id: DragDropId, applied: DragDropEffect) -> Result<()> {
        self.0.call(|id, p| p.drag_dropped(id, drop_id, applied))
    }

    /// Open system title bar context menu.
    pub fn open_title_bar_context_menu(&self, position: DipPoint) -> Result<()> {
        self.0.call(|id, p| p.open_title_bar_context_menu(id, position))
    }

    /// Shows a native message dialog for the window.
    ///
    /// The window is not interactive while the dialog is visible and the dialog may be modal in the view-process.
    /// In the app-process this is always async, and the response var will update once when the user responds.
    pub fn message_dialog(&self, dlg: MsgDialog, responder: ResponderVar<MsgDialogResponse>) -> Result<()> {
        let dlg_id = self.0.call(|id, p| p.message_dialog(id, dlg))?;
        VIEW_PROCESS.handle_write(self.0.app_id).message_dialogs.push((dlg_id, responder));
        Ok(())
    }

    /// Shows a native file/folder dialog for the window.
    ///
    /// The window is not interactive while the dialog is visible and the dialog may be modal in the view-process.
    /// In the app-process this is always async, and the response var will update once when the user responds.
    pub fn file_dialog(&self, dlg: FileDialog, responder: ResponderVar<FileDialogResponse>) -> Result<()> {
        let dlg_id = self.0.call(|id, p| p.file_dialog(id, dlg))?;
        VIEW_PROCESS.handle_write(self.0.app_id).file_dialogs.push((dlg_id, responder));
        Ok(())
    }

    /// Update the window's accessibility info tree.
    pub fn access_update(&self, update: zng_view_api::access::AccessTreeUpdate) -> Result<()> {
        self.0.call(|id, p| p.access_update(id, update))
    }

    /// Enable or disable IME by setting a cursor area.
    ///
    /// In mobile platforms also shows the software keyboard for `Some(_)` and hides it for `None`.
    pub fn set_ime_area(&self, area: Option<DipRect>) -> Result<()> {
        self.0.call(|id, p| p.set_ime_area(id, area))
    }

    /// Attempt to set a system wide shutdown warning associated with the window.
    ///
    /// Operating systems that support this show the `reason` in a warning for the user, it must be a short text
    /// that identifies the critical operation that cannot be cancelled.
    ///
    /// Note that there is no guarantee that the view-process or operating system will actually set a block, there
    /// is no error result because operating systems can silently ignore block requests at any moment, even after
    /// an initial successful block.
    ///
    /// Set to an empty text to remove the warning.
    pub fn set_system_shutdown_warn(&self, reason: Txt) -> Result<()> {
        self.0.call(move |id, p| p.set_system_shutdown_warn(id, reason))
    }

    /// Drop `self`.
    pub fn close(self) {
        drop(self)
    }

    /// Call a window extension with custom encoded payload.
    pub fn window_extension_raw(&self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> Result<ApiExtensionPayload> {
        self.0.call(|id, p| p.window_extension(id, extension_id, request))
    }

    /// Call a window extension with serialized payload.
    pub fn window_extension<I, O>(&self, extension_id: ApiExtensionId, request: &I) -> Result<std::result::Result<O, ApiExtensionRecvError>>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let r = self.window_extension_raw(extension_id, ApiExtensionPayload::serialize(&request).unwrap())?;
        Ok(r.deserialize())
    }
}

/// View window or headless surface.
#[derive(Clone, Debug)]
pub enum ViewWindowOrHeadless {
    /// Headed window view.
    Window(ViewWindow),
    /// Headless surface view.
    Headless(ViewHeadless),
}
impl ViewWindowOrHeadless {
    /// Reference the window or surface renderer.
    pub fn renderer(&self) -> ViewRenderer {
        match self {
            ViewWindowOrHeadless::Window(w) => w.renderer(),
            ViewWindowOrHeadless::Headless(h) => h.renderer(),
        }
    }

    /// Call a window extension with custom encoded payload.
    pub fn window_extension_raw(&self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> Result<ApiExtensionPayload> {
        match self {
            ViewWindowOrHeadless::Window(w) => w.window_extension_raw(extension_id, request),
            ViewWindowOrHeadless::Headless(h) => h.window_extension_raw(extension_id, request),
        }
    }

    /// Call a window extension with serialized payload.
    pub fn window_extension<I, O>(&self, extension_id: ApiExtensionId, request: &I) -> Result<std::result::Result<O, ApiExtensionRecvError>>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        match self {
            ViewWindowOrHeadless::Window(w) => w.window_extension(extension_id, request),
            ViewWindowOrHeadless::Headless(h) => h.window_extension(extension_id, request),
        }
    }
}
impl From<ViewWindow> for ViewWindowOrHeadless {
    fn from(w: ViewWindow) -> Self {
        ViewWindowOrHeadless::Window(w)
    }
}
impl From<ViewHeadless> for ViewWindowOrHeadless {
    fn from(w: ViewHeadless) -> Self {
        ViewWindowOrHeadless::Headless(w)
    }
}

#[derive(Debug)]
struct ViewWindowData {
    app_id: AppId,
    id: ApiWindowId,
    generation: ViewProcessGen,
}
impl ViewWindowData {
    fn call<R>(&self, f: impl FnOnce(ApiWindowId, &mut Controller) -> Result<R>) -> Result<R> {
        let mut app = VIEW_PROCESS.handle_write(self.app_id);
        if app.check_generation() {
            Err(ViewChannelError::Disconnected)
        } else {
            f(self.id, &mut app.process)
        }
    }
}
impl Drop for ViewWindowData {
    fn drop(&mut self) {
        if VIEW_PROCESS.is_available() {
            let mut app = VIEW_PROCESS.handle_write(self.app_id);
            if self.generation == app.process.generation() {
                let _ = app.process.close(self.id);
            }
        }
    }
}
type Result<T> = std::result::Result<T, ViewChannelError>;

/// Handle to a headless surface/document open in the View Process.
///
/// The view is disposed when all clones of the handle are dropped.
#[derive(Clone, Debug)]
#[must_use = "the view is disposed when all clones of the handle are dropped"]
pub struct ViewHeadless(Arc<ViewWindowData>);
impl PartialEq for ViewHeadless {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ViewHeadless {}
impl ViewHeadless {
    /// Resize the headless surface.
    pub fn set_size(&self, size: DipSize, scale_factor: Factor) -> Result<()> {
        self.0.call(|id, p| p.set_headless_size(id, size, scale_factor))
    }

    /// Reference the window renderer.
    pub fn renderer(&self) -> ViewRenderer {
        ViewRenderer(Arc::downgrade(&self.0))
    }

    /// Call a window extension with custom encoded payload.
    pub fn window_extension_raw(&self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> Result<ApiExtensionPayload> {
        self.0.call(|id, p| p.window_extension(id, extension_id, request))
    }

    /// Call a window extension with serialized payload.
    pub fn window_extension<I, O>(&self, extension_id: ApiExtensionId, request: &I) -> Result<std::result::Result<O, ApiExtensionRecvError>>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let r = self.window_extension_raw(extension_id, ApiExtensionPayload::serialize(&request).unwrap())?;
        Ok(r.deserialize())
    }
}

/// Weak handle to a window or view.
///
/// This is only a weak reference, every method returns [`ViewChannelError::Disconnected`] if the
/// window is closed or view is disposed.
#[derive(Clone, Debug)]
pub struct ViewRenderer(sync::Weak<ViewWindowData>);
impl PartialEq for ViewRenderer {
    fn eq(&self, other: &Self) -> bool {
        if let Some(s) = self.0.upgrade()
            && let Some(o) = other.0.upgrade()
        {
            Arc::ptr_eq(&s, &o)
        } else {
            false
        }
    }
}
impl Eq for ViewRenderer {}

impl ViewRenderer {
    fn call<R>(&self, f: impl FnOnce(ApiWindowId, &mut Controller) -> Result<R>) -> Result<R> {
        if let Some(c) = self.0.upgrade() {
            c.call(f)
        } else {
            Err(ViewChannelError::Disconnected)
        }
    }

    /// Returns the view-process generation on which the renderer was created.
    pub fn generation(&self) -> Result<ViewProcessGen> {
        self.0.upgrade().map(|c| c.generation).ok_or(ViewChannelError::Disconnected)
    }

    /// Use an image resource in the window renderer.
    ///
    /// Returns the image texture ID.
    pub fn use_image(&self, image: &ViewImage) -> Result<ImageTextureId> {
        self.call(|id, p| {
            let image = image.0.read();
            if p.generation() == image.generation {
                p.use_image(id, image.id.unwrap_or(ImageId::INVALID))
            } else {
                Err(ViewChannelError::Disconnected)
            }
        })
    }

    /// Replace the image resource in the window renderer.
    pub fn update_image_use(&mut self, tex_id: ImageTextureId, image: &ViewImage) -> Result<()> {
        self.call(|id, p| {
            let image = image.0.read();
            if p.generation() == image.generation {
                p.update_image_use(id, tex_id, image.id.unwrap_or(ImageId::INVALID))
            } else {
                Err(ViewChannelError::Disconnected)
            }
        })
    }

    /// Delete the image resource in the window renderer.
    pub fn delete_image_use(&mut self, tex_id: ImageTextureId) -> Result<()> {
        self.call(|id, p| p.delete_image_use(id, tex_id))
    }

    /// Add a raw font resource to the window renderer.
    ///
    /// Returns the new font face ID, unique for this renderer.
    pub fn add_font_face(&self, bytes: Vec<u8>, index: u32) -> Result<FontFaceId> {
        self.call(|id, p| p.add_font_face(id, IpcBytes::from_vec(bytes), index))
    }

    /// Delete the font resource in the window renderer.
    pub fn delete_font_face(&self, font_face_id: FontFaceId) -> Result<()> {
        self.call(|id, p| p.delete_font_face(id, font_face_id))
    }

    /// Add a sized font to the window renderer.
    ///
    /// Returns the new font ID, unique for this renderer.
    pub fn add_font(
        &self,
        font_face_id: FontFaceId,
        glyph_size: Px,
        options: FontOptions,
        variations: Vec<(FontVariationName, f32)>,
    ) -> Result<FontId> {
        self.call(|id, p| p.add_font(id, font_face_id, glyph_size, options, variations))
    }

    /// Delete the sized font.
    pub fn delete_font(&self, font_id: FontId) -> Result<()> {
        self.call(|id, p| p.delete_font(id, font_id))
    }

    /// Create a new image resource from the current rendered frame.
    pub fn frame_image(&self, mask: Option<ImageMaskMode>) -> Result<ViewImage> {
        if let Some(c) = self.0.upgrade() {
            let id = c.call(|id, p| p.frame_image(id, mask))?;
            Ok(Self::add_frame_image(c.app_id, id))
        } else {
            Err(ViewChannelError::Disconnected)
        }
    }

    /// Create a new image resource from a selection of the current rendered frame.
    pub fn frame_image_rect(&self, rect: PxRect, mask: Option<ImageMaskMode>) -> Result<ViewImage> {
        if let Some(c) = self.0.upgrade() {
            let id = c.call(|id, p| p.frame_image_rect(id, rect, mask))?;
            Ok(Self::add_frame_image(c.app_id, id))
        } else {
            Err(ViewChannelError::Disconnected)
        }
    }

    fn add_frame_image(app_id: AppId, id: ImageId) -> ViewImage {
        if id == ImageId::INVALID {
            ViewImage::dummy(None)
        } else {
            let mut app = VIEW_PROCESS.handle_write(app_id);
            let img = ViewImage(Arc::new(RwLock::new(ViewImageData {
                app_id: Some(app_id),
                id: Some(id),
                generation: app.process.generation(),
                size: PxSize::zero(),
                partial_size: PxSize::zero(),
                ppi: None,
                is_opaque: false,
                partial_pixels: None,
                pixels: None,
                is_mask: false,
                done_signal: SignalOnce::new(),
            })));

            app.loading_images.push(Arc::downgrade(&img.0));
            app.frame_images.push(Arc::downgrade(&img.0));

            img
        }
    }

    /// Render a new frame.
    pub fn render(&self, frame: FrameRequest) -> Result<()> {
        let _s = tracing::debug_span!("ViewRenderer.render").entered();

        if let Some(w) = self.0.upgrade() {
            w.call(|id, p| p.render(id, frame))?;
            VIEW_PROCESS.handle_write(w.app_id).pending_frames += 1;
            Ok(())
        } else {
            Err(ViewChannelError::Disconnected)
        }
    }

    /// Update the current frame and re-render it.
    pub fn render_update(&self, frame: FrameUpdateRequest) -> Result<()> {
        let _s = tracing::debug_span!("ViewRenderer.render_update").entered();

        if let Some(w) = self.0.upgrade() {
            w.call(|id, p| p.render_update(id, frame))?;
            VIEW_PROCESS.handle_write(w.app_id).pending_frames += 1;
            Ok(())
        } else {
            Err(ViewChannelError::Disconnected)
        }
    }

    /// Call a render extension with custom encoded payload.
    pub fn render_extension_raw(&self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> Result<ApiExtensionPayload> {
        if let Some(w) = self.0.upgrade() {
            w.call(|id, p| p.render_extension(id, extension_id, request))
        } else {
            Err(ViewChannelError::Disconnected)
        }
    }

    /// Call a render extension with serialized payload.
    pub fn render_extension<I, O>(&self, extension_id: ApiExtensionId, request: &I) -> Result<std::result::Result<O, ApiExtensionRecvError>>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let r = self.render_extension_raw(extension_id, ApiExtensionPayload::serialize(&request).unwrap())?;
        Ok(r.deserialize())
    }
}

/// Handle to an image loading or loaded in the View Process.
///
/// The image is disposed when all clones of the handle are dropped.
#[must_use = "the image is disposed when all clones of the handle are dropped"]
#[derive(Clone)]
pub struct ViewImage(Arc<RwLock<ViewImageData>>);
impl PartialEq for ViewImage {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ViewImage {}
impl std::hash::Hash for ViewImage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.0) as usize;
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
            .field("is_opaque", &self.is_opaque())
            .field("is_mask", &self.is_mask())
            .field("generation", &self.generation())
            .finish_non_exhaustive()
    }
}

struct ViewImageData {
    app_id: Option<AppId>,
    id: Option<ImageId>,
    generation: ViewProcessGen,

    size: PxSize,
    partial_size: PxSize,
    ppi: Option<ImagePpi>,
    is_opaque: bool,

    partial_pixels: Option<IpcBytes>,
    pixels: Option<std::result::Result<IpcBytes, Txt>>,
    is_mask: bool,

    done_signal: SignalOnce,
}
impl Drop for ViewImageData {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            let app_id = self.app_id.unwrap();
            if let Some(app) = APP.id() {
                if app_id != app {
                    tracing::error!("image from app `{:?}` dropped in app `{:?}`", app_id, app);
                }

                if VIEW_PROCESS.is_available() && VIEW_PROCESS.generation() == self.generation {
                    let _ = VIEW_PROCESS.write().process.forget_image(id);
                }
            }
        }
    }
}

impl ViewImage {
    /// Image id.
    pub fn id(&self) -> Option<ImageId> {
        self.0.read().id
    }

    /// If the image does not actually exists in the view-process.
    pub fn is_dummy(&self) -> bool {
        self.0.read().id.is_none()
    }

    /// Returns `true` if the image has successfully decoded.
    pub fn is_loaded(&self) -> bool {
        self.0.read().pixels.as_ref().map(|r| r.is_ok()).unwrap_or(false)
    }

    /// Returns `true` if the image is progressively decoding and has partially decoded.
    pub fn is_partially_loaded(&self) -> bool {
        self.0.read().partial_pixels.is_some()
    }

    /// if [`error`] is `Some`.
    ///
    /// [`error`]: Self::error
    pub fn is_error(&self) -> bool {
        self.0.read().pixels.as_ref().map(|r| r.is_err()).unwrap_or(false)
    }

    /// Returns the load error if one happened.
    pub fn error(&self) -> Option<Txt> {
        self.0.read().pixels.as_ref().and_then(|s| s.as_ref().err().cloned())
    }

    /// Returns the pixel size, or zero if is not loaded or error.
    pub fn size(&self) -> PxSize {
        self.0.read().size
    }

    /// Actual size of the current pixels.
    ///
    /// Can be different from [`size`] if the image is progressively decoding.
    ///
    /// [`size`]: Self::size
    pub fn partial_size(&self) -> PxSize {
        self.0.read().partial_size
    }

    /// Returns the "pixels-per-inch" metadata associated with the image, or `None` if not loaded or error or no
    /// metadata provided by decoder.
    pub fn ppi(&self) -> Option<ImagePpi> {
        self.0.read().ppi
    }

    /// Returns if the image is fully opaque.
    pub fn is_opaque(&self) -> bool {
        self.0.read().is_opaque
    }

    /// Returns if the image is a single channel mask (A8).
    pub fn is_mask(&self) -> bool {
        self.0.read().is_mask
    }

    /// Copy the partially decoded pixels if the image is progressively decoding
    /// and has not finished decoding.
    ///
    /// Format is BGRA8 for normal images or A8 if [`is_mask`].
    ///
    /// [`is_mask`]: Self::is_mask
    pub fn partial_pixels(&self) -> Option<Vec<u8>> {
        self.0.read().partial_pixels.as_ref().map(|r| r[..].to_vec())
    }

    /// Reference the decoded pixels of image.
    ///
    /// Returns `None` until the image is fully loaded. Use [`partial_pixels`] to copy
    /// partially decoded bytes.
    ///
    /// Format is pre-multiplied BGRA8 for normal images or A8 if [`is_mask`].
    ///
    /// [`is_mask`]: Self::is_mask
    ///
    /// [`partial_pixels`]: Self::partial_pixels
    pub fn pixels(&self) -> Option<IpcBytes> {
        self.0.read().pixels.as_ref().and_then(|r| r.as_ref().ok()).cloned()
    }

    /// Returns the app that owns the view-process that is handling this image.
    pub fn app_id(&self) -> Option<AppId> {
        self.0.read().app_id
    }

    /// Returns the view-process generation on which the image is loaded.
    pub fn generation(&self) -> ViewProcessGen {
        self.0.read().generation
    }

    /// Creates a [`WeakViewImage`].
    pub fn downgrade(&self) -> WeakViewImage {
        WeakViewImage(Arc::downgrade(&self.0))
    }

    /// Create a dummy image in the loaded or error state.
    pub fn dummy(error: Option<Txt>) -> Self {
        ViewImage(Arc::new(RwLock::new(ViewImageData {
            app_id: None,
            id: None,
            generation: ViewProcessGen::INVALID,
            size: PxSize::zero(),
            partial_size: PxSize::zero(),
            ppi: None,
            is_opaque: true,
            partial_pixels: None,
            pixels: if let Some(e) = error {
                Some(Err(e))
            } else {
                Some(Ok(IpcBytes::from_slice(&[])))
            },
            is_mask: false,
            done_signal: SignalOnce::new_set(),
        })))
    }

    /// Returns a future that awaits until this image is loaded or encountered an error.
    pub fn awaiter(&self) -> SignalOnce {
        self.0.read().done_signal.clone()
    }

    /// Tries to encode the image to the format.
    ///
    /// The `format` must be one of the [`image_encoders`] supported by the view-process backend.
    ///
    /// [`image_encoders`]: VIEW_PROCESS::image_encoders
    pub async fn encode(&self, format: Txt) -> std::result::Result<IpcBytes, EncodeError> {
        self.awaiter().await;

        if let Some(e) = self.error() {
            return Err(EncodeError::Encode(e));
        }

        let receiver = {
            let img = self.0.read();
            if let Some(id) = img.id {
                let mut app = VIEW_PROCESS.handle_write(img.app_id.unwrap());

                app.process.encode_image(id, format.clone())?;

                let (sender, receiver) = flume::bounded(1);
                if let Some(entry) = app.encoding_images.iter_mut().find(|r| r.image_id == id && r.format == format) {
                    entry.listeners.push(sender);
                } else {
                    app.encoding_images.push(EncodeRequest {
                        image_id: id,
                        format,
                        listeners: vec![sender],
                    });
                }
                receiver
            } else {
                return Err(EncodeError::Dummy);
            }
        };

        receiver.recv_async().await?
    }
}

/// Error returned by [`ViewImage::encode`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeError {
    /// Encode error.
    Encode(Txt),
    /// Attempted to encode dummy image.
    ///
    /// In a headless-app without renderer all images are dummy because there is no
    /// view-process backend running.
    Dummy,
    /// The View-Process disconnected or has not finished initializing yet, try again after [`VIEW_PROCESS_INITED_EVENT`].
    Disconnected,
}
impl From<Txt> for EncodeError {
    fn from(e: Txt) -> Self {
        EncodeError::Encode(e)
    }
}
impl From<ViewChannelError> for EncodeError {
    fn from(_: ViewChannelError) -> Self {
        EncodeError::Disconnected
    }
}
impl From<flume::RecvError> for EncodeError {
    fn from(_: flume::RecvError) -> Self {
        EncodeError::Disconnected
    }
}
impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::Encode(e) => write!(f, "{e}"),
            EncodeError::Dummy => write!(f, "cannot encode dummy image"),
            EncodeError::Disconnected => write!(f, "{}", ViewChannelError::Disconnected),
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
pub struct WeakViewImage(sync::Weak<RwLock<ViewImageData>>);
impl WeakViewImage {
    /// Attempt to upgrade the weak pointer to the image to a full image.
    ///
    /// Returns `Some` if the is at least another [`ViewImage`] holding the image alive.
    pub fn upgrade(&self) -> Option<ViewImage> {
        self.0.upgrade().map(ViewImage)
    }
}

struct EncodeRequest {
    image_id: ImageId,
    format: Txt,
    listeners: Vec<flume::Sender<std::result::Result<IpcBytes, EncodeError>>>,
}

type ClipboardResult<T> = std::result::Result<T, ClipboardError>;

/// View-process clipboard methods.
#[non_exhaustive]
pub struct ViewClipboard {}
impl ViewClipboard {
    /// Read [`ClipboardType::Text`].
    ///
    /// [`ClipboardType::Text`]: zng_view_api::clipboard::ClipboardType::Text
    pub fn read_text(&self) -> Result<ClipboardResult<Txt>> {
        match VIEW_PROCESS.try_write()?.process.read_clipboard(ClipboardType::Text)? {
            Ok(ClipboardData::Text(t)) => Ok(Ok(t)),
            Err(e) => Ok(Err(e)),
            _ => Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned incorrect type")))),
        }
    }

    /// Write [`ClipboardType::Text`].
    ///
    /// [`ClipboardType::Text`]: zng_view_api::clipboard::ClipboardType::Text
    pub fn write_text(&self, txt: Txt) -> Result<ClipboardResult<()>> {
        VIEW_PROCESS.try_write()?.process.write_clipboard(ClipboardData::Text(txt))
    }

    /// Read [`ClipboardType::Image`].
    ///
    /// [`ClipboardType::Image`]: zng_view_api::clipboard::ClipboardType::Image
    pub fn read_image(&self) -> Result<ClipboardResult<ViewImage>> {
        let mut app = VIEW_PROCESS.try_write()?;
        match app.process.read_clipboard(ClipboardType::Image)? {
            Ok(ClipboardData::Image(id)) => {
                if id == ImageId::INVALID {
                    Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned invalid image"))))
                } else {
                    let img = ViewImage(Arc::new(RwLock::new(ViewImageData {
                        id: Some(id),
                        app_id: APP.id(),
                        generation: app.process.generation(),
                        size: PxSize::zero(),
                        partial_size: PxSize::zero(),
                        ppi: None,
                        is_opaque: false,
                        partial_pixels: None,
                        pixels: None,
                        is_mask: false,
                        done_signal: SignalOnce::new(),
                    })));
                    app.loading_images.push(Arc::downgrade(&img.0));
                    Ok(Ok(img))
                }
            }
            Err(e) => Ok(Err(e)),
            _ => Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned incorrect type")))),
        }
    }

    /// Write [`ClipboardType::Image`].
    ///
    /// [`ClipboardType::Image`]: zng_view_api::clipboard::ClipboardType::Image
    pub fn write_image(&self, img: &ViewImage) -> Result<ClipboardResult<()>> {
        if img.is_loaded() {
            if let Some(id) = img.id() {
                return VIEW_PROCESS.try_write()?.process.write_clipboard(ClipboardData::Image(id));
            }
        }
        Ok(Err(ClipboardError::Other(Txt::from_static("image not loaded"))))
    }

    /// Read [`ClipboardType::FileList`].
    ///
    /// [`ClipboardType::FileList`]: zng_view_api::clipboard::ClipboardType::FileList
    pub fn read_file_list(&self) -> Result<ClipboardResult<Vec<PathBuf>>> {
        match VIEW_PROCESS.try_write()?.process.read_clipboard(ClipboardType::FileList)? {
            Ok(ClipboardData::FileList(f)) => Ok(Ok(f)),
            Err(e) => Ok(Err(e)),
            _ => Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned incorrect type")))),
        }
    }

    /// Write [`ClipboardType::FileList`].
    ///
    /// [`ClipboardType::FileList`]: zng_view_api::clipboard::ClipboardType::FileList
    pub fn write_file_list(&self, list: Vec<PathBuf>) -> Result<ClipboardResult<()>> {
        VIEW_PROCESS.try_write()?.process.write_clipboard(ClipboardData::FileList(list))
    }

    /// Read [`ClipboardType::Extension`].
    ///
    /// [`ClipboardType::Extension`]: zng_view_api::clipboard::ClipboardType::Extension
    pub fn read_extension(&self, data_type: Txt) -> Result<ClipboardResult<IpcBytes>> {
        match VIEW_PROCESS
            .try_write()?
            .process
            .read_clipboard(ClipboardType::Extension(data_type.clone()))?
        {
            Ok(ClipboardData::Extension { data_type: rt, data }) if rt == data_type => Ok(Ok(data)),
            Err(e) => Ok(Err(e)),
            _ => Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned incorrect type")))),
        }
    }

    /// Write [`ClipboardType::Extension`].
    ///
    /// [`ClipboardType::Extension`]: zng_view_api::clipboard::ClipboardType::Extension
    pub fn write_extension(&self, data_type: Txt, data: IpcBytes) -> Result<ClipboardResult<()>> {
        VIEW_PROCESS
            .try_write()?
            .process
            .write_clipboard(ClipboardData::Extension { data_type, data })
    }
}
