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

use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard};
use zng_app_context::app_local;
use zng_layout::unit::{DipPoint, DipRect, DipSideOffsets, DipSize, Factor, Px, PxPoint, PxRect};
use zng_task::channel::{self, ChannelError, IpcBytes, IpcReceiver, Receiver};
use zng_txt::Txt;
use zng_var::{ResponderVar, Var, VarHandle};
use zng_view_api::{
    self, DeviceEventsFilter, DragDropId, Event, FocusResult, ViewProcessGen, ViewProcessInfo,
    api_extension::{ApiExtensionId, ApiExtensionName, ApiExtensionPayload, ApiExtensionRecvError},
    audio::{
        AudioDecoded, AudioId, AudioMetadata, AudioMix, AudioOutputId, AudioOutputRequest, AudioPlayId, AudioPlayRequest, AudioRequest,
    },
    dialog::{FileDialog, FileDialogResponse, MsgDialog, MsgDialogResponse, Notification, NotificationResponse},
    drag_drop::{DragDropData, DragDropEffect, DragDropError},
    font::{FontOptions, IpcFontBytes},
    image::{ImageDecoded, ImageEncodeId, ImageEncodeRequest, ImageMaskMode, ImageMetadata, ImageRequest, ImageTextureId},
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
    image::ImageId,
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

    info: ViewProcessInfo,

    loading_images: Vec<sync::Weak<ViewImageHandleData>>,
    encoding_images: Vec<EncodeRequest>,

    loading_audios: Vec<sync::Weak<ViewAudioHandleData>>,

    pending_frames: usize,

    message_dialogs: Vec<(zng_view_api::dialog::DialogId, ResponderVar<MsgDialogResponse>)>,
    file_dialogs: Vec<(zng_view_api::dialog::DialogId, ResponderVar<FileDialogResponse>)>,
    notifications: Vec<(zng_view_api::dialog::DialogId, VarHandle, ResponderVar<NotificationResponse>)>,

    ping_count: u16,
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
        if let Some(w) = &*vp
            && w.process.is_connected()
        {
            return Ok(MappedRwLockWriteGuard::map(vp, |w| w.as_mut().unwrap()));
        }
        Err(ChannelError::disconnected())
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

    /// Read lock view-process and reference current generation info.
    ///
    /// Strongly recommend to clone/copy the info required, the entire service is locked until the return value is dropped.
    pub fn info(&self) -> impl std::ops::Deref<Target = ViewProcessInfo> {
        MappedRwLockReadGuard::map(self.read(), |p| &p.info)
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

    /// Send a request to open a connection to an audio output device.
    ///
    /// A [`RAW_AUDIO_OUTPUT_OPEN_EVENT`] or [`RAW_AUDIO_OUTPUT_ERROR_EVENT`]
    ///
    /// [`RAW_AUDIO_OUTPUT_OPEN_EVENT`]: crate::view_process::raw_events::RAW_AUDIO_OUTPUT_OPEN_EVENT
    /// [`RAW_AUDIO_OUTPUT_ERROR_EVENT`]: crate::view_process::raw_events::RAW_AUDIO_OUTPUT_ERROR_EVENT
    pub fn open_audio_output(&self, request: AudioOutputRequest) -> Result<()> {
        self.write().process.open_audio_output(request)
    }

    /// Send an image for decoding and caching.
    ///
    /// This function returns immediately, the handle must be held and compared with the [`RAW_IMAGE_METADATA_DECODED_EVENT`],
    /// [`RAW_IMAGE_DECODED_EVENT`] and [`RAW_IMAGE_DECODE_ERROR_EVENT`] events to receive the data.
    ///
    /// [`RAW_IMAGE_METADATA_DECODED_EVENT`]: crate::view_process::raw_events::RAW_IMAGE_METADATA_DECODED_EVENT
    /// [`RAW_IMAGE_DECODED_EVENT`]: crate::view_process::raw_events::RAW_IMAGE_DECODED_EVENT
    /// [`RAW_IMAGE_DECODE_ERROR_EVENT`]: crate::view_process::raw_events::RAW_IMAGE_DECODE_ERROR_EVENT
    pub fn add_image(&self, request: ImageRequest<IpcBytes>) -> Result<ViewImageHandle> {
        let mut app = self.write();

        let id = app.process.add_image(request)?;

        let handle = Arc::new((APP.id().unwrap(), app.process.generation(), id));
        app.loading_images.push(Arc::downgrade(&handle));

        Ok(ViewImageHandle(Some(handle)))
    }

    /// Starts sending an image for *progressive* decoding and caching.
    ///
    /// This function returns immediately, the handle must be held and compared with the [`RAW_IMAGE_METADATA_DECODED_EVENT`],
    /// [`RAW_IMAGE_DECODED_EVENT`] and [`RAW_IMAGE_DECODE_ERROR_EVENT`] events to receive the data.
    ///
    /// [`RAW_IMAGE_METADATA_DECODED_EVENT`]: crate::view_process::raw_events::RAW_IMAGE_METADATA_DECODED_EVENT
    /// [`RAW_IMAGE_DECODED_EVENT`]: crate::view_process::raw_events::RAW_IMAGE_DECODED_EVENT
    /// [`RAW_IMAGE_DECODE_ERROR_EVENT`]: crate::view_process::raw_events::RAW_IMAGE_DECODE_ERROR_EVENT
    pub fn add_image_pro(&self, request: ImageRequest<IpcReceiver<IpcBytes>>) -> Result<ViewImageHandle> {
        let mut app = self.write();

        let id = app.process.add_image_pro(request)?;

        let handle = Arc::new((APP.id().unwrap(), app.process.generation(), id));
        app.loading_images.push(Arc::downgrade(&handle));

        Ok(ViewImageHandle(Some(handle)))
    }

    /// Starts encoding an image.
    ///
    /// The returned channel will update once with the result.
    pub fn encode_image(&self, request: ImageEncodeRequest) -> Receiver<std::result::Result<IpcBytes, EncodeError>> {
        let (sender, receiver) = channel::bounded(1);

        if request.id == ImageId::INVALID {
            let mut app = VIEW_PROCESS.write();

            match app.process.encode_image(request) {
                Ok(r) => {
                    app.encoding_images.push(EncodeRequest {
                        task_id: r,
                        listener: sender,
                    });
                }
                Err(_) => {
                    let _ = sender.send_blocking(Err(EncodeError::Disconnected));
                }
            }
        } else {
            let _ = sender.send_blocking(Err(EncodeError::Dummy));
        }

        receiver
    }

    /// Send an audio for decoding and caching.
    ///
    /// Depending on the request the audio may be decoded entirely or it may be decoded on demand.
    ///
    /// This function returns immediately, the handle must be held and compared with the [`RAW_AUDIO_METADATA_DECODED_EVENT`],
    /// [`RAW_AUDIO_DECODED_EVENT`] and [`RAW_AUDIO_DECODE_ERROR_EVENT`] events to receive the metadata and data.
    ///
    /// [`RAW_IMAGE_METADATA_DECODED_EVENT`]: crate::view_process::raw_events::RAW_AUDIO_METADATA_DECODED_EVENT
    /// [`RAW_IMAGE_DECODED_EVENT`]: crate::view_process::raw_events::RAW_AUDIO_DECODED_EVENT
    /// [`RAW_IMAGE_DECODE_ERROR_EVENT`]: crate::view_process::raw_events::RAW_AUDIO_DECODE_ERROR_EVENT
    pub fn add_audio(&self, request: AudioRequest<IpcBytes>) -> Result<ViewAudioHandle> {
        let mut app = self.write();

        let id = app.process.add_audio(request)?;

        let handle = Arc::new((APP.id().unwrap(), app.process.generation(), id));
        app.loading_audios.push(Arc::downgrade(&handle));

        Ok(ViewAudioHandle(Some(handle)))
    }

    /// Starts sending an audio for decoding and caching.
    ///
    /// Depending on the request the audio may be decoded as it is received or it may be decoded on demand.
    ///
    /// This function returns immediately, the handle must be held and compared with the [`RAW_AUDIO_METADATA_DECODED_EVENT`],
    /// [`RAW_AUDIO_DECODED_EVENT`] and [`RAW_AUDIO_DECODE_ERROR_EVENT`] events to receive the metadata and data.
    ///
    /// [`RAW_IMAGE_METADATA_DECODED_EVENT`]: crate::view_process::raw_events::RAW_AUDIO_METADATA_DECODED_EVENT
    /// [`RAW_IMAGE_DECODED_EVENT`]: crate::view_process::raw_events::RAW_AUDIO_DECODED_EVENT
    /// [`RAW_IMAGE_DECODE_ERROR_EVENT`]: crate::view_process::raw_events::RAW_AUDIO_DECODE_ERROR_EVENT
    pub fn add_audio_pro(&self, request: AudioRequest<IpcReceiver<IpcBytes>>) -> Result<ViewAudioHandle> {
        let mut app = self.write();

        let id = app.process.add_audio_pro(request)?;

        let handle = Arc::new((APP.id().unwrap(), app.process.generation(), id));
        app.loading_audios.push(Arc::downgrade(&handle));

        Ok(ViewAudioHandle(Some(handle)))
    }

    /// View-process clipboard methods.
    pub fn clipboard(&self) -> Result<&ViewClipboard> {
        if VIEW_PROCESS.is_connected() {
            Ok(&ViewClipboard {})
        } else {
            Err(ChannelError::disconnected())
        }
    }

    /// Register a native notification, either a popup or an entry in the system notifications list.
    ///
    /// If the `notification` var updates the notification content updates or closes.
    ///
    /// If the notification is responded the `responder` variable is set.
    pub fn notification_dialog(&self, notification: Var<Notification>, responder: ResponderVar<NotificationResponse>) -> Result<()> {
        let mut app = self.write();
        let dlg_id = app.process.notification_dialog(notification.get())?;
        let handle = notification.hook(move |n| {
            let mut app = VIEW_PROCESS.write();
            let retain = app.notifications.iter().any(|(id, _, _)| *id == dlg_id);
            if retain {
                app.process.update_notification(dlg_id, n.value().clone()).ok();
            }
            retain
        });
        app.notifications.push((dlg_id, handle, responder));
        Ok(())
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
            Ok(me.info.extensions.id(&extension_name.into()))
        } else {
            Err(ChannelError::disconnected())
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
            loading_audios: vec![],
            pending_frames: 0,
            message_dialogs: vec![],
            file_dialogs: vec![],
            notifications: vec![],
            info: ViewProcessInfo::new(ViewProcessGen::INVALID, false),
            ping_count: 0,
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
    pub(super) fn handle_inited(&self, inited: &zng_view_api::ViewProcessInfo) {
        let mut me = self.write();
        me.info = inited.clone();
        me.process.handle_inited(inited.generation);
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

    pub(super) fn on_image_metadata(&self, meta: &ImageMetadata) -> Option<ViewImageHandle> {
        let mut app = self.write();

        let mut found = None;
        app.loading_images.retain(|i| {
            if let Some(h) = i.upgrade() {
                if found.is_none() && h.2 == meta.id {
                    found = Some(h);
                }
                // retain
                true
            } else {
                false
            }
        });

        // Best effort avoid tracking handles already dropped,
        // the VIEW_PROCESS handles all image requests so we
        // can track all primary requests, only entry images are send without
        // knowing so we can skip all not found without parent.
        //
        // This could potentially restart tracking an entry that was dropped, but
        // all that does is generate a no-op event and a second `forget_image` requests for the view-process.

        if found.is_none() && meta.parent.is_some() {
            // start tracking entry image

            let handle = Arc::new((APP.id().unwrap(), app.process.generation(), meta.id));
            app.loading_images.push(Arc::downgrade(&handle));

            return Some(ViewImageHandle(Some(handle)));
        }

        found.map(|h| ViewImageHandle(Some(h)))
    }

    pub(super) fn on_image_decoded(&self, data: &ImageDecoded) -> Option<ViewImageHandle> {
        let mut app = self.write();

        // retain loading handle only for partial decode, cleanup for full decode.
        //
        // All valid not dropped requests are already in `loading_images` because they are
        // either primary requests or are entries (view-process always sends metadata decoded first for entries).

        let mut found = None;
        app.loading_images.retain(|i| {
            if let Some(h) = i.upgrade() {
                if found.is_none() && h.2 == data.meta.id {
                    found = Some(h);
                    return data.partial.is_some();
                }
                true
            } else {
                false
            }
        });

        found.map(|h| ViewImageHandle(Some(h)))
    }

    pub(super) fn on_image_error(&self, id: ImageId) -> Option<ViewImageHandle> {
        let mut app = self.write();

        let mut found = None;
        app.loading_images.retain(|i| {
            if let Some(h) = i.upgrade() {
                if found.is_none() && h.2 == id {
                    found = Some(h);
                    return false;
                }
                true
            } else {
                false
            }
        });

        // error images should already be removed from view-process, handle will request a removal anyway

        found.map(|h| ViewImageHandle(Some(h)))
    }

    pub(super) fn on_audio_metadata(&self, meta: &AudioMetadata) -> Option<ViewAudioHandle> {
        // this is very similar to `on_image_metadata`

        let mut app = self.write();

        let mut found = None;
        app.loading_audios.retain(|i| {
            if let Some(h) = i.upgrade() {
                if found.is_none() && h.2 == meta.id {
                    found = Some(h);
                }
                // retain
                true
            } else {
                false
            }
        });

        if found.is_none() && meta.parent.is_some() {
            // start tracking entry track

            let handle = Arc::new((APP.id().unwrap(), app.process.generation(), meta.id));
            app.loading_audios.push(Arc::downgrade(&handle));

            return Some(ViewAudioHandle(Some(handle)));
        }

        found.map(|h| ViewAudioHandle(Some(h)))
    }

    pub(super) fn on_audio_decoded(&self, audio: &AudioDecoded) -> Option<ViewAudioHandle> {
        // this is very similar to `on_image_decoded`, the big difference is that
        // partial decodes represent the latest decoded chunk, not all the previous decoded data,
        // and it may never finish decoding too, the progressive source can never end or the request
        // configured it to always decode on demand and drop the buffer as it is played.

        let mut app = self.write();

        let mut found = None;
        app.loading_audios.retain(|i| {
            if let Some(h) = i.upgrade() {
                if found.is_none() && h.2 == audio.id {
                    found = Some(h);
                    return !audio.is_full;
                }
                true
            } else {
                false
            }
        });

        found.map(|h| ViewAudioHandle(Some(h)))
    }

    pub(super) fn on_audio_error(&self, id: AudioId) -> Option<ViewAudioHandle> {
        let mut app = self.write();

        let mut found = None;
        app.loading_audios.retain(|i| {
            if let Some(h) = i.upgrade() {
                if found.is_none() && h.2 == id {
                    found = Some(h);
                    return false;
                }
                true
            } else {
                false
            }
        });

        // error audios should already be removed from view-process, handle will request a removal anyway

        found.map(|h| ViewAudioHandle(Some(h)))
    }

    pub(crate) fn on_frame_rendered(&self, _id: WindowId) {
        let mut vp = self.write();
        vp.pending_frames = vp.pending_frames.saturating_sub(1);
    }

    pub(crate) fn on_frame_image(&self, data: &ImageDecoded) -> ViewImageHandle {
        ViewImageHandle(Some(Arc::new((APP.id().unwrap(), self.generation(), data.meta.id))))
    }

    pub(super) fn on_image_encoded(&self, task_id: ImageEncodeId, data: IpcBytes) {
        self.on_image_encode_result(task_id, Ok(data));
    }
    pub(super) fn on_image_encode_error(&self, task_id: ImageEncodeId, error: Txt) {
        self.on_image_encode_result(task_id, Err(EncodeError::Encode(error)));
    }
    fn on_image_encode_result(&self, task_id: ImageEncodeId, result: std::result::Result<IpcBytes, EncodeError>) {
        let mut app = self.write();
        app.encoding_images.retain(move |r| {
            let done = r.task_id == task_id;
            if done {
                let _ = r.listener.send_blocking(result.clone());
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

    pub(crate) fn on_notification_dlg_response(&self, id: zng_view_api::dialog::DialogId, response: NotificationResponse) {
        let mut app = self.write();
        if let Some(i) = app.notifications.iter().position(|(i, _, _)| *i == id) {
            let (_, _, r) = app.notifications.swap_remove(i);
            r.respond(response);
        }
    }

    pub(super) fn on_respawned(&self, _gen: ViewProcessGen) {
        let mut app = self.write();
        app.pending_frames = 0;
        for (_, r) in app.message_dialogs.drain(..) {
            r.respond(MsgDialogResponse::Error(Txt::from_static("respawn")));
        }
        for (_, r) in app.file_dialogs.drain(..) {
            r.respond(FileDialogResponse::Error(Txt::from_static("respawn")));
        }
        for (_, _, r) in app.notifications.drain(..) {
            r.respond(NotificationResponse::Error(Txt::from_static("respawn")));
        }
    }

    pub(crate) fn exit(&self) {
        *VIEW_PROCESS_SV.write() = None;
    }

    pub(crate) fn ping(&self) {
        let mut app = self.write();
        let count = app.ping_count.wrapping_add(1);
        if let Ok(c) = app.process.ping(count)
            && c != count
        {
            tracing::error!("incorrect ping response, expected {count}, was {c}");
        }
        app.ping_count = count;
    }

    pub(crate) fn on_pong(&self, count: u16) {
        let expected = self.read().ping_count;
        if expected != count {
            // this could indicates a severe slowdown in the event pump
            tracing::warn!("unexpected pong event, expected {expected}, was {count}");
        }
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
        /// View-process implementation info.
        pub info: zng_view_api::ViewProcessInfo,

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
impl std::ops::Deref for ViewProcessInitedArgs {
    type Target = zng_view_api::ViewProcessInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
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
    pub fn set_icon(&self, icon: Option<&ViewImageHandle>) -> Result<()> {
        self.0.call(|id, p| {
            if let Some(icon) = icon.and_then(|i| i.0.as_ref()) {
                if p.generation() == icon.1 {
                    p.set_icon(id, Some(icon.2))
                } else {
                    Err(ChannelError::disconnected())
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
    pub fn set_cursor_image(&self, cursor: Option<&ViewImageHandle>, hotspot: PxPoint) -> Result<()> {
        self.0.call(|id, p| {
            if let Some(cur) = cursor.and_then(|i| i.0.as_ref()) {
                if p.generation() == cur.1 {
                    p.set_cursor_image(id, Some(zng_view_api::window::CursorImage::new(cur.2, hotspot)))
                } else {
                    Err(ChannelError::disconnected())
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
struct ViewAudioOutputData {
    app_id: AppId,
    id: AudioOutputId,
    generation: ViewProcessGen,
}
impl ViewAudioOutputData {
    fn call<R>(&self, f: impl FnOnce(AudioOutputId, &mut Controller) -> Result<R>) -> Result<R> {
        let mut app = VIEW_PROCESS.handle_write(self.app_id);
        if app.check_generation() {
            Err(ChannelError::disconnected())
        } else {
            f(self.id, &mut app.process)
        }
    }
}
impl Drop for ViewAudioOutputData {
    fn drop(&mut self) {
        if VIEW_PROCESS.is_available() {
            let mut app = VIEW_PROCESS.handle_write(self.app_id);
            if self.generation == app.process.generation() {
                let _ = app.process.close_audio_output(self.id);
            }
        }
    }
}

/// Handle to an audio output stream in the View Process.
///
/// The stream is disposed when all clones of the handle are dropped.
#[derive(Clone, Debug)]
#[must_use = "the audio output is disposed when all clones of the handle are dropped"]
pub struct ViewAudioOutput(Arc<ViewAudioOutputData>);
impl ViewAudioOutput {
    /// Play or enqueue audio.
    pub fn cue(&self, mix: AudioMix) -> Result<AudioPlayId> {
        self.0.call(|id, p| p.cue_audio(AudioPlayRequest::new(id, mix)))
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
            Err(ChannelError::disconnected())
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

type Result<T> = std::result::Result<T, ChannelError>;

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
/// This is only a weak reference, every method returns [`ChannelError::disconnected`] if the
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
            Err(ChannelError::disconnected())
        }
    }

    /// Returns the view-process generation on which the renderer was created.
    pub fn generation(&self) -> Result<ViewProcessGen> {
        self.0.upgrade().map(|c| c.generation).ok_or(ChannelError::disconnected())
    }

    /// Use an image resource in the window renderer.
    ///
    /// Returns the image texture ID.
    pub fn use_image(&self, image: &ViewImageHandle) -> Result<ImageTextureId> {
        self.call(|id, p| {
            if let Some(img) = &image.0 {
                if p.generation() == img.1 {
                    p.use_image(id, img.2)
                } else {
                    Err(ChannelError::disconnected())
                }
            } else {
                Ok(ImageTextureId::INVALID)
            }
        })
    }

    /// Replace the image resource in the window renderer.
    ///
    /// The new `image` handle must represent an image with same dimensions and format as the previous. If the
    /// image cannot be updated an error is logged and `false` is returned.
    ///
    /// The `dirty_rect` can be set to optimize texture upload to the GPU, if not set the entire image region updates.
    pub fn update_image_use(&mut self, tex_id: ImageTextureId, image: &ViewImageHandle, dirty_rect: Option<PxRect>) -> Result<bool> {
        self.call(|id, p| {
            if let Some(img) = &image.0 {
                if p.generation() == img.1 {
                    p.update_image_use(id, tex_id, img.2, dirty_rect)
                } else {
                    Err(ChannelError::disconnected())
                }
            } else {
                Ok(false)
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
    pub fn add_font_face(&self, bytes: IpcFontBytes, index: u32) -> Result<FontFaceId> {
        self.call(|id, p| p.add_font_face(id, bytes, index))
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
    pub fn frame_image(&self, mask: Option<ImageMaskMode>) -> Result<ViewImageHandle> {
        if let Some(c) = self.0.upgrade() {
            let id = c.call(|id, p| p.frame_image(id, mask))?;
            Ok(Self::add_frame_image(c.app_id, id))
        } else {
            Err(ChannelError::disconnected())
        }
    }

    /// Create a new image resource from a selection of the current rendered frame.
    pub fn frame_image_rect(&self, rect: PxRect, mask: Option<ImageMaskMode>) -> Result<ViewImageHandle> {
        if let Some(c) = self.0.upgrade() {
            let id = c.call(|id, p| p.frame_image_rect(id, rect, mask))?;
            Ok(Self::add_frame_image(c.app_id, id))
        } else {
            Err(ChannelError::disconnected())
        }
    }

    fn add_frame_image(app_id: AppId, id: ImageId) -> ViewImageHandle {
        if id == ImageId::INVALID {
            ViewImageHandle::dummy()
        } else {
            let mut app = VIEW_PROCESS.handle_write(app_id);
            let handle = Arc::new((APP.id().unwrap(), app.process.generation(), id));
            app.loading_images.push(Arc::downgrade(&handle));

            ViewImageHandle(Some(handle))
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
            Err(ChannelError::disconnected())
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
            Err(ChannelError::disconnected())
        }
    }

    /// Call a render extension with custom encoded payload.
    pub fn render_extension_raw(&self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> Result<ApiExtensionPayload> {
        if let Some(w) = self.0.upgrade() {
            w.call(|id, p| p.render_extension(id, extension_id, request))
        } else {
            Err(ChannelError::disconnected())
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

type ViewImageHandleData = (AppId, ViewProcessGen, ImageId);

/// Handle to an image loading or loaded in the View Process.
///
/// The image is disposed when all clones of the handle are dropped.
#[must_use = "the image is disposed when all clones of the handle are dropped"]
#[derive(Clone, Debug)]
pub struct ViewImageHandle(Option<Arc<ViewImageHandleData>>);
impl PartialEq for ViewImageHandle {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}
impl Eq for ViewImageHandle {}
impl ViewImageHandle {
    /// New handle to nothing.
    pub fn dummy() -> Self {
        ViewImageHandle(None)
    }

    /// Is handle to nothing.
    pub fn is_dummy(&self) -> bool {
        self.0.is_none()
    }

    /// Image ID.
    ///
    /// Is [`ImageId::INVALID`] for dummy.
    pub fn image_id(&self) -> ImageId {
        self.0.as_ref().map(|h| h.2).unwrap_or(ImageId::INVALID)
    }

    /// Application that requested this image.
    ///
    /// Images can only be used in the same app.
    ///
    /// Is `None` for dummy.
    pub fn app_id(&self) -> Option<AppId> {
        self.0.as_ref().map(|h| h.0)
    }

    /// View-process generation that provided this image.
    ///
    /// Images can only be used in the same view-process instance.
    ///
    /// Is [`ViewProcessGen::INVALID`] for dummy.
    pub fn view_process_gen(&self) -> ViewProcessGen {
        self.0.as_ref().map(|h| h.1).unwrap_or(ViewProcessGen::INVALID)
    }
}
impl Drop for ViewImageHandle {
    fn drop(&mut self) {
        if let Some(h) = self.0.take()
            && Arc::strong_count(&h) == 1
            && let Some(app) = APP.id()
        {
            if h.0 != app {
                tracing::error!("image from app `{:?}` dropped in app `{:?}`", h.0, app);
                return;
            }

            if VIEW_PROCESS.is_available() && VIEW_PROCESS.generation() == h.1 {
                let _ = VIEW_PROCESS.write().process.forget_image(h.2);
            }
        }
    }
}
/// Connection to an image loading or loaded in the View Process.
///
/// The image is removed from the View Process cache when all clones of [`ViewImageHandle`] drops, but
/// if there is another image pointer holding the image, this weak pointer can be upgraded back
/// to a strong connection to the image.
///
/// Dummy handles never upgrade back.
#[derive(Clone)]
pub struct WeakViewImageHandle(sync::Weak<ViewImageHandleData>);
impl WeakViewImageHandle {
    /// Attempt to upgrade the weak pointer to the image to a full image.
    ///
    /// Returns `Some` if the is at least another [`ViewImageHandle`] holding the image alive.
    pub fn upgrade(&self) -> Option<ViewImageHandle> {
        self.0.upgrade().map(|h| ViewImageHandle(Some(h)))
    }
}

/// Error returned by [`VIEW_PROCESS::encode_image`].
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
    /// Image is still loading, await it first.
    Loading,
    /// The View-Process disconnected or has not finished initializing yet, try again after [`VIEW_PROCESS_INITED_EVENT`].
    Disconnected,
}
impl From<Txt> for EncodeError {
    fn from(e: Txt) -> Self {
        EncodeError::Encode(e)
    }
}
impl From<ChannelError> for EncodeError {
    fn from(_: ChannelError) -> Self {
        EncodeError::Disconnected
    }
}
impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::Encode(e) => write!(f, "{e}"),
            EncodeError::Dummy => write!(f, "cannot encode dummy image"),
            EncodeError::Loading => write!(f, "cannot encode, image is still loading"),
            EncodeError::Disconnected => write!(f, "{}", ChannelError::disconnected()),
        }
    }
}
impl std::error::Error for EncodeError {}

struct EncodeRequest {
    task_id: ImageEncodeId,
    listener: channel::Sender<std::result::Result<IpcBytes, EncodeError>>,
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
        match VIEW_PROCESS
            .try_write()?
            .process
            .read_clipboard(vec![ClipboardType::Text], true)?
            .map(|mut r| r.pop())
        {
            Ok(Some(ClipboardData::Text(t))) => Ok(Ok(t)),
            Err(e) => Ok(Err(e)),
            _ => Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned incorrect type")))),
        }
    }

    /// Write [`ClipboardType::Text`].
    ///
    /// [`ClipboardType::Text`]: zng_view_api::clipboard::ClipboardType::Text
    pub fn write_text(&self, txt: Txt) -> Result<ClipboardResult<()>> {
        VIEW_PROCESS
            .try_write()?
            .process
            .write_clipboard(vec![ClipboardData::Text(txt)])
            .map(|r| r.map(|_| ()))
    }

    /// Read [`ClipboardType::Image`].
    ///
    /// [`ClipboardType::Image`]: zng_view_api::clipboard::ClipboardType::Image
    pub fn read_image(&self) -> Result<ClipboardResult<ViewImageHandle>> {
        let mut app = VIEW_PROCESS.try_write()?;
        match app.process.read_clipboard(vec![ClipboardType::Image], true)?.map(|mut r| r.pop()) {
            Ok(Some(ClipboardData::Image(id))) => {
                if id == ImageId::INVALID {
                    Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned invalid image"))))
                } else {
                    let handle = Arc::new((APP.id().unwrap(), app.process.generation(), id));
                    app.loading_images.push(Arc::downgrade(&handle));
                    Ok(Ok(ViewImageHandle(Some(handle))))
                }
            }
            Err(e) => Ok(Err(e)),
            _ => Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned incorrect type")))),
        }
    }

    /// Write [`ClipboardType::Image`].
    ///
    /// [`ClipboardType::Image`]: zng_view_api::clipboard::ClipboardType::Image
    pub fn write_image(&self, img: &ViewImageHandle) -> Result<ClipboardResult<()>> {
        return VIEW_PROCESS
            .try_write()?
            .process
            .write_clipboard(vec![ClipboardData::Image(img.image_id())])
            .map(|r| r.map(|_| ()));
    }

    /// Read [`ClipboardType::Paths`].
    ///
    /// [`ClipboardType::Paths`]: zng_view_api::clipboard::ClipboardType::Paths
    pub fn read_paths(&self) -> Result<ClipboardResult<Vec<PathBuf>>> {
        match VIEW_PROCESS
            .try_write()?
            .process
            .read_clipboard(vec![ClipboardType::Paths], true)?
            .map(|mut r| r.pop())
        {
            Ok(Some(ClipboardData::Paths(f))) => Ok(Ok(f)),
            Err(e) => Ok(Err(e)),
            _ => Ok(Err(ClipboardError::Other(Txt::from_static("view-process returned incorrect type")))),
        }
    }

    /// Write [`ClipboardType::Paths`].
    ///
    /// [`ClipboardType::Paths`]: zng_view_api::clipboard::ClipboardType::Paths
    pub fn write_paths(&self, list: Vec<PathBuf>) -> Result<ClipboardResult<()>> {
        VIEW_PROCESS
            .try_write()?
            .process
            .write_clipboard(vec![ClipboardData::Paths(list)])
            .map(|r| r.map(|_| ()))
    }

    /// Read [`ClipboardType::Extension`].
    ///
    /// [`ClipboardType::Extension`]: zng_view_api::clipboard::ClipboardType::Extension
    pub fn read_extension(&self, data_type: Txt) -> Result<ClipboardResult<IpcBytes>> {
        match VIEW_PROCESS
            .try_write()?
            .process
            .read_clipboard(vec![ClipboardType::Extension(data_type.clone())], true)?
            .map(|mut r| r.pop())
        {
            Ok(Some(ClipboardData::Extension { data_type: rt, data })) if rt == data_type => Ok(Ok(data)),
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
            .write_clipboard(vec![ClipboardData::Extension { data_type, data }])
            .map(|r| r.map(|_| ()))
    }
}

type ViewAudioHandleData = (AppId, ViewProcessGen, AudioId);

/// Handle to an audio loading or loaded in the View Process.
///
/// The audio is disposed when all clones of the handle are dropped.
#[must_use = "the audio is disposed when all clones of the handle are dropped"]
#[derive(Clone, Debug)]
pub struct ViewAudioHandle(Option<Arc<ViewAudioHandleData>>);
impl PartialEq for ViewAudioHandle {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}
impl Eq for ViewAudioHandle {}
impl ViewAudioHandle {
    /// New handle to nothing.
    pub fn dummy() -> Self {
        ViewAudioHandle(None)
    }

    /// Is handle to nothing.
    pub fn is_dummy(&self) -> bool {
        self.0.is_none()
    }

    /// Audio ID.
    ///
    /// Is [`AudioId::INVALID`] for dummy.
    pub fn audio_id(&self) -> AudioId {
        self.0.as_ref().map(|h| h.2).unwrap_or(AudioId::INVALID)
    }

    /// Application that requested this image.
    ///
    /// Audios can only be used in the same app.
    ///
    /// Is `None` for dummy.
    pub fn app_id(&self) -> Option<AppId> {
        self.0.as_ref().map(|h| h.0)
    }

    /// View-process generation that provided this image.
    ///
    /// Audios can only be used in the same view-process instance.
    ///
    /// Is [`ViewProcessGen::INVALID`] for dummy.
    pub fn view_process_gen(&self) -> ViewProcessGen {
        self.0.as_ref().map(|h| h.1).unwrap_or(ViewProcessGen::INVALID)
    }
}
impl Drop for ViewAudioHandle {
    fn drop(&mut self) {
        if let Some(h) = self.0.take()
            && Arc::strong_count(&h) == 1
            && let Some(app) = APP.id()
        {
            if h.0 != app {
                tracing::error!("audio from app `{:?}` dropped in app `{:?}`", h.0, app);
                return;
            }

            if VIEW_PROCESS.is_available() && VIEW_PROCESS.generation() == h.1 {
                let _ = VIEW_PROCESS.write().process.forget_audio(h.2);
            }
        }
    }
}
/// Connection to an audio loading or loaded in the View Process.
///
/// The audio is removed from the View Process cache when all clones of [`ViewAudioHandle`] drops, but
/// if there is another audio pointer holding it, this weak pointer can be upgraded back
/// to a strong connection to the audio.
///
/// Dummy handles never upgrade back.
#[derive(Clone)]
pub struct WeakViewAudioHandle(sync::Weak<ViewAudioHandleData>);
impl WeakViewAudioHandle {
    /// Attempt to upgrade the weak pointer to the audio to a full audio.
    ///
    /// Returns `Some` if the is at least another [`ViewAudioHandle`] holding the audio alive.
    pub fn upgrade(&self) -> Option<ViewAudioHandle> {
        self.0.upgrade().map(|h| ViewAudioHandle(Some(h)))
    }
}
