use std::{
    collections::HashMap,
    fmt, mem,
    path::PathBuf,
    sync::Arc,
    task::Waker,
    time::{Duration, Instant},
};

use crate::{Deadline, event::EventArgs as _, handler::HandlerExt as _, window::WINDOWS_APP};
use parking_lot::Mutex;
use zng_app_context::{AppScope, app_local};
use zng_task::DEADLINE_APP;
use zng_task::channel::{self, ChannelError};
use zng_time::{INSTANT_APP, InstantMode};
use zng_txt::Txt;
use zng_var::{ResponderVar, ResponseVar, VARS_APP, Var, expr_var, response_var};
use zng_view_api::{DeviceEventsFilter, raw_input::InputDeviceEvent};

use crate::{
    APP, AppControlFlow, DInstant, INSTANT,
    event::{CommandInfoExt, CommandNameExt, command, event},
    event_args,
    shortcut::CommandShortcutExt,
    shortcut::shortcut,
    timer::TimersService,
    update::{ContextUpdates, InfoUpdates, LayoutUpdates, RenderUpdates, UPDATES, UpdateOp, UpdateTrace, UpdatesTrace, WidgetUpdates},
    view_process::{raw_device_events::InputDeviceId, *},
    widget::WidgetId,
    window::WindowId,
};

/// Represents a running app controlled by an external event loop.
pub(crate) struct RunningApp {
    receiver: channel::Receiver<AppEvent>,

    loop_timer: LoopTimer,
    loop_monitor: LoopMonitor,
    last_wait_event: Instant,

    pending_view_events: Vec<zng_view_api::Event>,
    pending_view_frame_events: Vec<zng_view_api::window::EventFrameRendered>,
    pending: ContextUpdates,

    exited: bool,

    // cleans on drop
    _scope: AppScope,
}
impl Drop for RunningApp {
    fn drop(&mut self) {
        let _s = tracing::debug_span!("exit").entered();
        APP.call_deinit_handlers();
        VIEW_PROCESS.exit();
    }
}
impl RunningApp {
    pub(crate) fn start(
        scope: AppScope,
        is_headed: bool,
        with_renderer: bool,
        view_process_exe: Option<PathBuf>,
        view_process_env: HashMap<Txt, Txt>,
    ) -> Self {
        let _s = tracing::debug_span!("APP::start").entered();

        let (sender, receiver) = AppEventSender::new();

        UPDATES.init(sender);

        fn app_waker() {
            UPDATES.update(None);
        }
        VARS_APP.init_app_waker(app_waker);
        VARS_APP.init_modify_trace(UpdatesTrace::log_var);
        DEADLINE_APP.init_deadline_service(crate::timer::deadline_service);
        zng_var::animation::TRANSITIONABLE_APP.init_rgba_lerp(zng_color::lerp_rgba);

        if with_renderer && view_process_exe.is_none() {
            zng_env::assert_inited();
        }

        #[cfg(not(target_arch = "wasm32"))]
        let view_process_exe = view_process_exe.unwrap_or_else(|| std::env::current_exe().expect("current_exe"));
        #[cfg(target_arch = "wasm32")]
        let view_process_exe = std::path::PathBuf::from("<wasm>");

        APP.pre_init(is_headed, with_renderer, view_process_exe, view_process_env);

        APP.call_init_handlers();

        RunningApp {
            receiver,

            loop_timer: LoopTimer::default(),
            loop_monitor: LoopMonitor::default(),
            last_wait_event: Instant::now(),

            pending_view_events: Vec::with_capacity(100),
            pending_view_frame_events: Vec::with_capacity(5),
            pending: ContextUpdates {
                update: false,
                info: false,
                layout: false,
                render: false,
                update_widgets: WidgetUpdates::default(),
                info_widgets: InfoUpdates::default(),
                layout_widgets: LayoutUpdates::default(),
                render_widgets: RenderUpdates::default(),
                render_update_widgets: RenderUpdates::default(),
            },
            exited: false,

            _scope: scope,
        }
    }

    pub fn has_exited(&self) -> bool {
        self.exited
    }

    fn input_device_id(&mut self, id: zng_view_api::raw_input::InputDeviceId) -> InputDeviceId {
        VIEW_PROCESS.input_device_id(id)
    }

    /// Process a View Process event.
    fn on_view_event(&mut self, ev: zng_view_api::Event) {
        use crate::view_process::raw_device_events::*;
        use crate::view_process::raw_events::*;
        use zng_view_api::Event;

        fn window_id(id: zng_view_api::window::WindowId) -> WindowId {
            WindowId::from_raw(id.get())
        }
        fn audio_output_id(id: zng_view_api::audio::AudioOutputId) -> AudioOutputId {
            AudioOutputId::from_raw(id.get())
        }

        match ev {
            Event::MouseMoved {
                window: w_id,
                device: d_id,
                coalesced_pos,
                position,
            } => {
                let args = RawMouseMovedArgs::now(window_id(w_id), self.input_device_id(d_id), coalesced_pos, position);
                RAW_MOUSE_MOVED_EVENT.notify(args);
            }
            Event::MouseEntered {
                window: w_id,
                device: d_id,
            } => {
                let args = RawMouseArgs::now(window_id(w_id), self.input_device_id(d_id));
                RAW_MOUSE_ENTERED_EVENT.notify(args);
            }
            Event::MouseLeft {
                window: w_id,
                device: d_id,
            } => {
                let args = RawMouseArgs::now(window_id(w_id), self.input_device_id(d_id));
                RAW_MOUSE_LEFT_EVENT.notify(args);
            }
            Event::WindowChanged(c) => {
                let monitor_id = c.monitor.map(|id| VIEW_PROCESS.monitor_id(id));
                let args = RawWindowChangedArgs::now(
                    window_id(c.window),
                    c.state,
                    c.position,
                    monitor_id,
                    c.size,
                    c.safe_padding,
                    c.cause,
                    c.frame_wait_id,
                );
                RAW_WINDOW_CHANGED_EVENT.notify(args);
            }
            Event::DragHovered { window, data, allowed } => {
                let args = RawDragHoveredArgs::now(window_id(window), data, allowed);
                RAW_DRAG_HOVERED_EVENT.notify(args);
            }
            Event::DragMoved {
                window,
                coalesced_pos,
                position,
            } => {
                let args = RawDragMovedArgs::now(window_id(window), coalesced_pos, position);
                RAW_DRAG_MOVED_EVENT.notify(args);
            }
            Event::DragDropped {
                window,
                data,
                allowed,
                drop_id,
            } => {
                let args = RawDragDroppedArgs::now(window_id(window), data, allowed, drop_id);
                RAW_DRAG_DROPPED_EVENT.notify(args);
            }
            Event::DragCancelled { window } => {
                let args = RawDragCancelledArgs::now(window_id(window));
                RAW_DRAG_CANCELLED_EVENT.notify(args);
            }
            Event::AppDragEnded { window, drag, applied } => {
                let args = RawAppDragEndedArgs::now(window_id(window), drag, applied);
                RAW_APP_DRAG_ENDED_EVENT.notify(args);
            }
            Event::FocusChanged { prev, new } => {
                let args = RawWindowFocusArgs::now(prev.map(window_id), new.map(window_id));
                RAW_WINDOW_FOCUS_EVENT.notify(args);
            }
            Event::KeyboardInput {
                window: w_id,
                device: d_id,
                key_code,
                state,
                key,
                key_location,
                key_modified,
                text,
            } => {
                let args = RawKeyInputArgs::now(
                    window_id(w_id),
                    self.input_device_id(d_id),
                    key_code,
                    key_location,
                    state,
                    key,
                    key_modified,
                    text,
                );
                RAW_KEY_INPUT_EVENT.notify(args);
            }
            Event::Ime { window: w_id, ime } => {
                let args = RawImeArgs::now(window_id(w_id), ime);
                RAW_IME_EVENT.notify(args);
            }

            Event::MouseWheel {
                window: w_id,
                device: d_id,
                delta,
                phase,
            } => {
                let args = RawMouseWheelArgs::now(window_id(w_id), self.input_device_id(d_id), delta, phase);
                RAW_MOUSE_WHEEL_EVENT.notify(args);
            }
            Event::MouseInput {
                window: w_id,
                device: d_id,
                state,
                button,
            } => {
                let args = RawMouseInputArgs::now(window_id(w_id), self.input_device_id(d_id), state, button);
                RAW_MOUSE_INPUT_EVENT.notify(args);
            }
            Event::TouchpadPressure {
                window: w_id,
                device: d_id,
                pressure,
                stage,
            } => {
                let args = RawTouchpadPressureArgs::now(window_id(w_id), self.input_device_id(d_id), pressure, stage);
                RAW_TOUCHPAD_PRESSURE_EVENT.notify(args);
            }
            Event::AxisMotion {
                window: w_id,
                device: d_id,
                axis,
                value,
            } => {
                let args = RawAxisMotionArgs::now(window_id(w_id), self.input_device_id(d_id), axis, value);
                RAW_AXIS_MOTION_EVENT.notify(args);
            }
            Event::Touch {
                window: w_id,
                device: d_id,
                touches,
            } => {
                let args = RawTouchArgs::now(window_id(w_id), self.input_device_id(d_id), touches);
                RAW_TOUCH_EVENT.notify(args);
            }
            Event::ScaleFactorChanged {
                monitor: id,
                windows,
                scale_factor,
            } => {
                let monitor_id = VIEW_PROCESS.monitor_id(id);
                let windows: Vec<_> = windows.into_iter().map(window_id).collect();
                let args = RawScaleFactorChangedArgs::now(monitor_id, windows, scale_factor);
                RAW_SCALE_FACTOR_CHANGED_EVENT.notify(args);
            }
            Event::MonitorsChanged(monitors) => {
                let monitors: Vec<_> = monitors.into_iter().map(|(id, info)| (VIEW_PROCESS.monitor_id(id), info)).collect();
                let args = RawMonitorsChangedArgs::now(monitors);
                RAW_MONITORS_CHANGED_EVENT.notify(args);
            }
            Event::AudioDevicesChanged(_audio_devices) => {}
            Event::WindowCloseRequested(w_id) => {
                let args = RawWindowCloseRequestedArgs::now(window_id(w_id));
                RAW_WINDOW_CLOSE_REQUESTED_EVENT.notify(args);
            }
            Event::WindowOpened(w_id, data) => {
                let w_id = window_id(w_id);
                let (window, data) = VIEW_PROCESS.on_window_opened(w_id, data);
                let args = RawWindowOpenArgs::now(w_id, window, data);
                RAW_WINDOW_OPEN_EVENT.notify(args);
            }
            Event::HeadlessOpened(w_id, data) => {
                let w_id = window_id(w_id);
                let (surface, data) = VIEW_PROCESS.on_headless_opened(w_id, data);
                let args = RawHeadlessOpenArgs::now(w_id, surface, data);
                RAW_HEADLESS_OPEN_EVENT.notify(args);
            }
            Event::WindowOrHeadlessOpenError { id: w_id, error } => {
                let w_id = window_id(w_id);
                let args = RawWindowOrHeadlessOpenErrorArgs::now(w_id, error);
                RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT.notify(args);
            }
            Event::WindowClosed(w_id) => {
                let args = RawWindowCloseArgs::now(window_id(w_id));
                RAW_WINDOW_CLOSE_EVENT.notify(args);
            }
            Event::ImageMetadataDecoded(meta) => {
                if let Some(handle) = VIEW_PROCESS.on_image_metadata(&meta) {
                    let args = RawImageMetadataDecodedArgs::now(handle, meta);
                    RAW_IMAGE_METADATA_DECODED_EVENT.notify(args);
                } else {
                    tracing::warn!("received unknown image metadata {:?} ({:?}), ignoring", meta.id, meta.size);
                }
            }
            Event::ImageDecoded(img) => {
                if let Some(handle) = VIEW_PROCESS.on_image_decoded(&img) {
                    let args = RawImageDecodedArgs::now(handle, img);
                    RAW_IMAGE_DECODED_EVENT.notify(args);
                } else {
                    tracing::warn!("received unknown image metadata {:?} ({:?}), ignoring", img.meta.id, img.meta.size);
                }
            }
            Event::ImageDecodeError { image: id, error } => {
                if let Some(handle) = VIEW_PROCESS.on_image_error(id) {
                    let args = RawImageDecodeErrorArgs::now(handle, error);
                    RAW_IMAGE_DECODE_ERROR_EVENT.notify(args);
                }
            }
            Event::ImageEncoded { task, data } => VIEW_PROCESS.on_image_encoded(task, data),
            Event::ImageEncodeError { task, error } => {
                VIEW_PROCESS.on_image_encode_error(task, error);
            }

            Event::AudioMetadataDecoded(meta) => {
                if let Some(handle) = VIEW_PROCESS.on_audio_metadata(&meta) {
                    let args = RawAudioMetadataDecodedArgs::now(handle, meta);
                    RAW_AUDIO_METADATA_DECODED_EVENT.notify(args);
                } else {
                    tracing::warn!("received unknown audio metadata {:?}, ignoring", meta.id);
                }
            }
            Event::AudioDecoded(audio) => {
                if let Some(handle) = VIEW_PROCESS.on_audio_decoded(&audio) {
                    let args = RawAudioDecodedArgs::now(handle, audio);
                    RAW_AUDIO_DECODED_EVENT.notify(args);
                } else {
                    tracing::warn!("received unknown audio metadata {:?}, ignoring", audio.id);
                }
            }
            Event::AudioDecodeError { audio: id, error } => {
                if let Some(handle) = VIEW_PROCESS.on_audio_error(id) {
                    let args = RawAudioDecodeErrorArgs::now(handle, error);
                    RAW_AUDIO_DECODE_ERROR_EVENT.notify(args);
                }
            }

            Event::AudioOutputOpened(id, data) => {
                let a_id = audio_output_id(id);
                let output = VIEW_PROCESS.on_audio_output_opened(a_id, data);

                let args = RawAudioOutputOpenArgs::now(a_id, output);
                RAW_AUDIO_OUTPUT_OPEN_EVENT.notify(args);
            }
            Event::AudioOutputOpenError { id, error } => {
                let a_id = audio_output_id(id);

                let args = RawAudioOutputOpenErrorArgs::now(a_id, error);
                RAW_AUDIO_OUTPUT_OPEN_ERROR_EVENT.notify(args);
            }

            Event::AccessInit { window: w_id } => {
                crate::access::on_access_init(window_id(w_id));
            }
            Event::AccessCommand {
                window: win_id,
                target: wgt_id,
                command,
            } => {
                crate::access::on_access_command(window_id(win_id), WidgetId::from_raw(wgt_id.0), command);
            }
            Event::AccessDeinit { window: w_id } => {
                crate::access::on_access_deinit(window_id(w_id));
            }

            // native dialog responses
            Event::MsgDialogResponse(id, response) => {
                VIEW_PROCESS.on_message_dlg_response(id, response);
            }
            Event::FileDialogResponse(id, response) => {
                VIEW_PROCESS.on_file_dlg_response(id, response);
            }
            Event::NotificationResponse(id, response) => {
                VIEW_PROCESS.on_notification_dlg_response(id, response);
            }

            Event::MenuCommand { id } => {
                let _ = id;
            }

            // custom
            Event::ExtensionEvent(id, payload) => {
                let args = RawExtensionEventArgs::now(id, payload);
                RAW_EXTENSION_EVENT.notify(args);
            }

            // config events
            Event::FontsChanged => {
                let args = RawFontChangedArgs::now();
                RAW_FONT_CHANGED_EVENT.notify(args);
            }
            Event::FontAaChanged(aa) => {
                let args = RawFontAaChangedArgs::now(aa);
                RAW_FONT_AA_CHANGED_EVENT.notify(args);
            }
            Event::MultiClickConfigChanged(cfg) => {
                let args = RawMultiClickConfigChangedArgs::now(cfg);
                RAW_MULTI_CLICK_CONFIG_CHANGED_EVENT.notify(args);
            }
            Event::AnimationsConfigChanged(cfg) => {
                VARS_APP.set_sys_animations_enabled(cfg.enabled);
                let args = RawAnimationsConfigChangedArgs::now(cfg);
                RAW_ANIMATIONS_CONFIG_CHANGED_EVENT.notify(args);
            }
            Event::KeyRepeatConfigChanged(cfg) => {
                let args = RawKeyRepeatConfigChangedArgs::now(cfg);
                RAW_KEY_REPEAT_CONFIG_CHANGED_EVENT.notify(args);
            }
            Event::TouchConfigChanged(cfg) => {
                let args = RawTouchConfigChangedArgs::now(cfg);
                RAW_TOUCH_CONFIG_CHANGED_EVENT.notify(args);
            }
            Event::LocaleChanged(cfg) => {
                let args = RawLocaleChangedArgs::now(cfg);
                RAW_LOCALE_CONFIG_CHANGED_EVENT.notify(args);
            }
            Event::ColorsConfigChanged(cfg) => {
                let args = RawColorsConfigChangedArgs::now(cfg);
                RAW_COLORS_CONFIG_CHANGED_EVENT.notify(args);
            }
            Event::ChromeConfigChanged(cfg) => {
                let args = RawChromeConfigChangedArgs::now(cfg);
                RAW_CHROME_CONFIG_CHANGED_EVENT.notify(args);
            }

            // `device_events`
            Event::InputDevicesChanged(devices) => {
                let devices: HashMap<_, _> = devices.into_iter().map(|(d_id, info)| (self.input_device_id(d_id), info)).collect();
                INPUT_DEVICES.update(devices.clone());
                let args = InputDevicesChangedArgs::now(devices);
                INPUT_DEVICES_CHANGED_EVENT.notify(args);
            }
            Event::InputDeviceEvent { device, event } => {
                let d_id = self.input_device_id(device);
                match event {
                    InputDeviceEvent::PointerMotion { delta } => {
                        let args = PointerMotionArgs::now(d_id, delta);
                        POINTER_MOTION_EVENT.notify(args);
                    }
                    InputDeviceEvent::ScrollMotion { delta } => {
                        let args = ScrollMotionArgs::now(d_id, delta);
                        SCROLL_MOTION_EVENT.notify(args);
                    }
                    InputDeviceEvent::AxisMotion { axis, value } => {
                        let args = AxisMotionArgs::now(d_id, axis, value);
                        AXIS_MOTION_EVENT.notify(args);
                    }
                    InputDeviceEvent::Button { button, state } => {
                        let args = ButtonArgs::now(d_id, button, state);
                        BUTTON_EVENT.notify(args);
                    }
                    InputDeviceEvent::Key { key_code, state } => {
                        let args = KeyArgs::now(d_id, key_code, state);
                        KEY_EVENT.notify(args);
                    }
                    _ => {}
                }
            }

            Event::LowMemory => {
                LOW_MEMORY_EVENT.notify(LowMemoryArgs::now());
            }

            Event::RecoveredFromComponentPanic { component, recover, panic } => {
                tracing::error!(
                    "view-process recovered from internal component panic\n  component: {component}\n  recover: {recover}\n```panic\n{panic}\n```"
                );
            }

            // Others
            Event::Inited(zng_view_api::ViewProcessInfo { .. }) | Event::Suspended | Event::Disconnected(_) | Event::FrameRendered(_) => {
                unreachable!()
            } // handled before coalesce.

            _ => {}
        }
    }

    /// Process a [`Event::FrameRendered`] event.
    fn on_view_rendered_event(&mut self, ev: zng_view_api::window::EventFrameRendered) {
        debug_assert!(ev.window != zng_view_api::window::WindowId::INVALID);
        let window_id = WindowId::from_raw(ev.window.get());
        // view.on_frame_rendered(window_id); // already called in push_coalesce
        let image = ev.frame_image.map(|img| (VIEW_PROCESS.on_frame_image(&img), img));
        let args = crate::view_process::raw_events::RawFrameRenderedArgs::now(window_id, ev.frame, image);
        crate::view_process::raw_events::RAW_FRAME_RENDERED_EVENT.notify(args);
    }

    pub(crate) fn run_headed(mut self) {
        self.apply_updates();
        let mut wait = false;
        loop {
            wait = match self.poll(wait) {
                AppControlFlow::Poll => false,
                AppControlFlow::Wait => true,
                AppControlFlow::Exit => break,
            };
        }
    }

    fn push_coalesce(&mut self, ev: AppEvent) {
        match ev {
            AppEvent::ViewEvent(ev) => match ev {
                zng_view_api::Event::FrameRendered(ev) => {
                    if ev.window == zng_view_api::window::WindowId::INVALID {
                        tracing::error!("ignored rendered event for invalid window id, {ev:?}");
                        return;
                    }

                    let window = WindowId::from_raw(ev.window.get());

                    // update ViewProcess immediately.
                    {
                        if VIEW_PROCESS.is_available() {
                            VIEW_PROCESS.on_frame_rendered(window);
                        }
                    }

                    #[cfg(debug_assertions)]
                    if self.pending_view_frame_events.iter().any(|e| e.window == ev.window) {
                        tracing::warn!("window `{window:?}` probably sent a frame request without awaiting renderer idle");
                    }

                    self.pending_view_frame_events.push(ev);
                }
                zng_view_api::Event::Pong(count) => VIEW_PROCESS.on_pong(count),
                zng_view_api::Event::Inited(inited) => {
                    // notify immediately.
                    if inited.is_respawn {
                        VIEW_PROCESS.on_respawned(inited.generation);
                        APP_PROCESS_SV.read().is_suspended.set(false);
                    }

                    VIEW_PROCESS.handle_inited(&inited);

                    let args = crate::view_process::ViewProcessInitedArgs::now(inited);
                    VIEW_PROCESS_INITED_EVENT.notify(args);
                }
                zng_view_api::Event::Suspended => {
                    VIEW_PROCESS.handle_suspended();
                    let args = crate::view_process::ViewProcessSuspendedArgs::now();
                    VIEW_PROCESS_SUSPENDED_EVENT.notify(args);
                    APP_PROCESS_SV.read().is_suspended.set(true);
                }
                zng_view_api::Event::Disconnected(vp_gen) => {
                    // update ViewProcess immediately.
                    VIEW_PROCESS.handle_disconnect(vp_gen);
                }
                ev => {
                    if let Some(last) = self.pending_view_events.last_mut() {
                        match last.coalesce(ev) {
                            Ok(()) => {}
                            Err(ev) => self.pending_view_events.push(ev),
                        }
                    } else {
                        self.pending_view_events.push(ev);
                    }
                }
            },
            AppEvent::Update(op, target) => {
                UPDATES.update_op(op, target);
            }
            AppEvent::CheckUpdate => {}
            AppEvent::ResumeUnwind(p) => std::panic::resume_unwind(p),
        }
    }

    fn has_pending_updates(&mut self) -> bool {
        !self.pending_view_events.is_empty() || self.pending.has_updates() || UPDATES.has_pending_updates() || !self.receiver.is_empty()
    }

    pub(crate) fn poll(&mut self, wait_app_event: bool) -> AppControlFlow {
        let mut disconnected = false;

        if self.exited {
            return AppControlFlow::Exit;
        }

        if wait_app_event {
            let idle = tracing::debug_span!("<idle>", ended_by = tracing::field::Empty).entered();

            const PING_TIMER: Duration = Duration::from_secs(2);

            let ping_timer = Deadline::timeout(PING_TIMER);
            let timer = if self.view_is_busy() {
                None
            } else {
                self.loop_timer.poll().map(|t| t.min(ping_timer))
            };
            match self.receiver.recv_deadline_blocking(timer.unwrap_or(ping_timer)) {
                Ok(ev) => {
                    idle.record("ended_by", "event");
                    drop(idle);
                    self.last_wait_event = Instant::now();
                    self.push_coalesce(ev)
                }
                Err(e) => match e {
                    ChannelError::Timeout => {
                        if timer.is_none() {
                            idle.record("ended_by", "timeout (ping)");
                        } else {
                            idle.record("ended_by", "timeout");
                        }
                        if self.last_wait_event.elapsed() >= PING_TIMER && !VIEW_PROCESS.is_same_process() && VIEW_PROCESS.is_connected() {
                            VIEW_PROCESS.ping();
                        }
                    }
                    ChannelError::Disconnected { .. } => {
                        idle.record("ended_by", "disconnected");
                        disconnected = true
                    }
                },
            }
        }
        loop {
            match self.receiver.try_recv() {
                Ok(ev) => match ev {
                    Some(ev) => self.push_coalesce(ev),
                    None => break,
                },
                Err(e) => match e {
                    ChannelError::Disconnected { .. } => {
                        disconnected = true;
                        break;
                    }
                    _ => unreachable!(),
                },
            }
        }
        if disconnected {
            panic!("app events channel disconnected");
        }

        if self.view_is_busy() {
            return AppControlFlow::Wait;
        }

        UPDATES.on_app_awake();

        // clear timers.
        let updated_timers = self.loop_timer.awake();
        if updated_timers {
            // tick timers and collect not elapsed timers.
            UPDATES.update_timers(&mut self.loop_timer);
            self.apply_updates();
        }

        let mut events = mem::take(&mut self.pending_view_events);
        for ev in events.drain(..) {
            self.on_view_event(ev);
            self.apply_updates();
        }
        debug_assert!(self.pending_view_events.is_empty());
        self.pending_view_events = events; // reuse capacity

        let mut events = mem::take(&mut self.pending_view_frame_events);
        for ev in events.drain(..) {
            self.on_view_rendered_event(ev);
        }
        self.pending_view_frame_events = events;

        if self.has_pending_updates() {
            self.apply_updates();
        }

        if self.view_is_busy() {
            return AppControlFlow::Wait;
        }

        self.finish_frame();

        UPDATES.next_deadline(&mut self.loop_timer);

        if APP_PROCESS_SV.read().exit {
            UPDATES.on_app_sleep();
            self.exited = true;
            AppControlFlow::Exit
        } else if self.has_pending_updates() || UPDATES.has_pending_layout_or_render() {
            AppControlFlow::Poll
        } else {
            UPDATES.on_app_sleep();
            AppControlFlow::Wait
        }
    }

    /// Does updates, collects pending update generated events and layout + render.
    fn apply_updates(&mut self) {
        let _s = tracing::debug_span!("apply_updates").entered();

        let mut run = true;
        while run {
            run = self.loop_monitor.update(|| {
                let mut any = false;

                self.pending |= UPDATES.apply_info();
                if mem::take(&mut self.pending.info) {
                    any = true;
                    let _s = tracing::debug_span!("info").entered();

                    let mut info_widgets = mem::take(&mut self.pending.info_widgets);

                    let _t = INSTANT_APP.pause_for_update();

                    WINDOWS_APP.update_info(&mut info_widgets);
                }

                self.pending |= UPDATES.apply_updates();
                TimersService::notify();
                if mem::take(&mut self.pending.update) {
                    any = true;
                    let _s = tracing::debug_span!("update").entered();

                    let mut update_widgets = mem::take(&mut self.pending.update_widgets);

                    let _t = INSTANT_APP.pause_for_update();

                    UPDATES.on_pre_updates();

                    WINDOWS_APP.update_widgets(&mut update_widgets);

                    UPDATES.on_updates();
                }

                any
            });
        }
    }

    fn view_is_busy(&mut self) -> bool {
        VIEW_PROCESS.is_available() && VIEW_PROCESS.pending_frames() > 0
    }

    // apply pending layout & render if the view-process is not already rendering.
    fn finish_frame(&mut self) {
        debug_assert!(!self.view_is_busy());

        self.pending |= UPDATES.apply_layout_render();

        while mem::take(&mut self.pending.layout) {
            let _s = tracing::debug_span!("apply_layout").entered();

            let mut layout_widgets = mem::take(&mut self.pending.layout_widgets);

            self.loop_monitor.maybe_trace(|| {
                let _t = INSTANT_APP.pause_for_update();

                WINDOWS_APP.update_layout(&mut layout_widgets);
            });

            self.apply_updates();
            self.pending |= UPDATES.apply_layout_render();
        }

        if mem::take(&mut self.pending.render) {
            let _s = tracing::debug_span!("apply_render").entered();

            let mut render_widgets = mem::take(&mut self.pending.render_widgets);
            let mut render_update_widgets = mem::take(&mut self.pending.render_update_widgets);

            let _t = INSTANT_APP.pause_for_update();

            WINDOWS_APP.update_render(&mut render_widgets, &mut render_update_widgets);
        }

        self.loop_monitor.finish_frame();
    }
}

/// Arguments for [`APP.on_init`] handlers.
///
/// No args as of this release. The handler is called in the new app context, so you can access any service inside.
///
/// [`APP.on_init`]: APP::on_init
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AppInitArgs {}

/// Arguments for [`APP.on_deinit`] handlers.
///
/// No args as of this release. The handler is called in the app context.
///
/// [`APP.on_deinit`]: APP::on_deinit
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AppDeinitArgs {}

impl APP {
    /// Register a handler to be called when the app starts.
    ///
    /// In single app builds (without `"multi_app"` feature) the `handler` is called only once and dropped.
    ///
    /// In `"multi_app"` builds the `handler` can be called more than once. The handler is called in the new app context, but
    /// it lives in the app process lifetime, you can unsubscribe from the inside or just use `hn_once!` to drop on init.
    ///
    /// This method must be called before any other `APP` method, the `handler` is not called for an already running app.
    ///
    /// Async handlers are fully supported, the code before the first `.await` runs blocking the rest runs in the `UPDATES` service.
    pub fn on_init(&self, handler: crate::handler::Handler<AppInitArgs>) {
        zng_unique_id::hot_static_ref!(ON_APP_INIT).lock().push(handler);
    }

    /// Register a handler to be called when the app exits.
    ///
    /// The `handler` is called only once, it runs in the app context and is dropped, any async tasks or requests that require app updates will
    /// not work, the app will exit just after calling the handler.
    ///
    /// This method must be called in the app context.
    pub fn on_deinit(&self, handler: impl FnOnce(&AppDeinitArgs) + Send + 'static) {
        ON_APP_DEINIT.write().get_mut().push(Box::new(handler));
    }

    fn call_init_handlers(&self) {
        let mut handlers = mem::take(&mut *zng_unique_id::hot_static_ref!(ON_APP_INIT).lock());
        let args = AppInitArgs {};
        handlers.retain_mut(|h| {
            let (owner, handle) = zng_handle::Handle::new(());
            h.app_event(Box::new(handle.downgrade()), true, &args);
            !owner.is_dropped()
        });

        let mut s = zng_unique_id::hot_static_ref!(ON_APP_INIT).lock();
        handlers.extend(s.drain(..));
        *s = handlers;
    }

    fn call_deinit_handlers(&self) {
        let handlers = mem::take(&mut *ON_APP_DEINIT.write().get_mut());
        let args = AppDeinitArgs {};
        for h in handlers {
            h(&args);
        }
    }
}
zng_unique_id::hot_static! {
    static ON_APP_INIT: Mutex<Vec<crate::handler::Handler<AppInitArgs>>> = Mutex::new(vec![]);
}
app_local! {
    // Mutex for Sync only
    static ON_APP_DEINIT: Mutex<Vec<Box<dyn FnOnce(&AppDeinitArgs) + Send + 'static>>> = const { Mutex::new(vec![]) };
}

/// App main loop timer.
#[derive(Debug)]
pub(crate) struct LoopTimer {
    now: DInstant,
    deadline: Option<Deadline>,
}
impl Default for LoopTimer {
    fn default() -> Self {
        Self {
            now: INSTANT.now(),
            deadline: None,
        }
    }
}
impl LoopTimer {
    /// Returns `true` if the `deadline` has elapsed, `false` if the `deadline` was
    /// registered for future waking.
    pub fn elapsed(&mut self, deadline: Deadline) -> bool {
        if deadline.0 <= self.now {
            true
        } else {
            self.register(deadline);
            false
        }
    }

    /// Register the future `deadline`.
    pub fn register(&mut self, deadline: Deadline) {
        if let Some(d) = &mut self.deadline {
            if deadline < *d {
                *d = deadline;
            }
        } else {
            self.deadline = Some(deadline)
        }
    }

    /// Get next recv deadline.
    pub(crate) fn poll(&mut self) -> Option<Deadline> {
        self.deadline
    }

    /// Maybe awake timer.
    pub(crate) fn awake(&mut self) -> bool {
        self.now = INSTANT.now();
        if let Some(d) = self.deadline
            && d.0 <= self.now
        {
            self.deadline = None;
            return true;
        }
        false
    }

    /// Awake timestamp.
    pub fn now(&self) -> DInstant {
        self.now
    }
}
impl zng_var::animation::AnimationTimer for LoopTimer {
    fn elapsed(&mut self, deadline: Deadline) -> bool {
        self.elapsed(deadline)
    }

    fn register(&mut self, deadline: Deadline) {
        self.register(deadline)
    }

    fn now(&self) -> DInstant {
        self.now()
    }
}

#[derive(Default)]
struct LoopMonitor {
    update_count: u16,
    skipped: bool,
    trace: Vec<UpdateTrace>,
}
impl LoopMonitor {
    /// Returns `false` if the loop should break.
    pub fn update(&mut self, update_once: impl FnOnce() -> bool) -> bool {
        self.update_count += 1;

        if self.update_count < 500 {
            update_once()
        } else if self.update_count < 1000 {
            UpdatesTrace::collect_trace(&mut self.trace, update_once)
        } else if self.update_count == 1000 {
            self.skipped = true;
            let trace = UpdatesTrace::format_trace(mem::take(&mut self.trace));
            tracing::error!(
                "updated 1000 times without rendering, probably stuck in an infinite loop\n\
                 will start skipping updates to render and poll system events\n\
                 top 20 most frequent update requests (in 500 cycles):\n\
                 {trace}\n\
                    you can use `UpdatesTraceUiNodeExt` and `updates_trace_event` to refine the trace"
            );
            false
        } else if self.update_count == 1500 {
            self.update_count = 1001;
            false
        } else {
            update_once()
        }
    }

    pub fn maybe_trace(&mut self, notify_once: impl FnOnce()) {
        if (500..1000).contains(&self.update_count) {
            UpdatesTrace::collect_trace(&mut self.trace, notify_once);
        } else {
            notify_once();
        }
    }

    pub fn finish_frame(&mut self) {
        if !self.skipped {
            self.skipped = false;
            self.update_count = 0;
            self.trace = vec![];
        }
    }
}

impl APP {
    /// Pre-init intrinsic services and commands, must be called before extensions init.
    pub(super) fn pre_init(&self, is_headed: bool, with_renderer: bool, view_process_exe: PathBuf, view_process_env: HashMap<Txt, Txt>) {
        // apply `pause_time_for_updates`
        let s = APP_PROCESS_SV.read();
        s.pause_time_for_updates
            .hook(|a| {
                if !matches!(INSTANT.mode(), zng_time::InstantMode::Manual) {
                    if *a.value() {
                        INSTANT_APP.set_mode(InstantMode::UpdatePaused);
                    } else {
                        INSTANT_APP.set_mode(InstantMode::Now);
                    }
                }
                true
            })
            .perm();

        // (re)apply `device_events_filter` on process init.
        VIEW_PROCESS_INITED_EVENT
            .hook(|_| {
                let filter = APP_PROCESS_SV.read().device_events_filter.get();
                if !filter.is_empty()
                    && let Err(e) = VIEW_PROCESS.set_device_events_filter(filter)
                {
                    tracing::error!("cannot set device events on the view-process, {e}");
                }
                true
            })
            .perm();

        // implement `EXIT_CMD`, let any other handler intercept it first
        EXIT_CMD
            .on_event(
                true,
                crate::hn!(|a| {
                    if !a.propagation().is_stopped() {
                        a.propagation().stop();
                        APP.exit();
                    }
                }),
            )
            .perm();

        // apply `device_events_filter`
        s.device_events_filter
            .hook(|a| {
                if let Err(e) = VIEW_PROCESS.set_device_events_filter(a.value().clone()) {
                    tracing::error!("cannot set device events on the view-process, {e}");
                }
                true
            })
            .perm();

        // spawn view-process
        if is_headed {
            debug_assert!(with_renderer);

            let view_evs_sender = UPDATES.sender();
            VIEW_PROCESS.start(view_process_exe, view_process_env, false, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
        } else if with_renderer {
            let view_evs_sender = UPDATES.sender();
            VIEW_PROCESS.start(view_process_exe, view_process_env, true, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
        }
    }
}

impl APP {
    /// Register a request for process exit with code `0` in the next update.
    ///
    /// The [`EXIT_REQUESTED_EVENT`] will notify, and if propagation is not cancelled the app process will exit.
    ///
    /// Returns a response variable that is updated once with the unit value [`ExitCancelled`]
    /// if the exit operation is cancelled.
    ///
    /// See also the [`EXIT_CMD`].
    pub fn exit(&self) -> ResponseVar<ExitCancelled> {
        let mut s = APP_PROCESS_SV.write();
        if let Some(r) = &s.exit_requests {
            r.response_var()
        } else {
            let (responder, response) = response_var();
            s.exit_requests = Some(responder);
            EXIT_REQUESTED_EVENT.notify(ExitRequestedArgs::now());
            EXIT_REQUESTED_EVENT
                .on_event(crate::hn_once!(|args: &ExitRequestedArgs| {
                    let mut s = APP_PROCESS_SV.write();
                    if args.propagation().is_stopped() {
                        s.exit = true;
                    } else {
                        s.exit_requests.take().unwrap().respond(ExitCancelled);
                    }
                }))
                .perm();
            response
        }
    }

    /// Gets a variable that tracks if the app is suspended by the operating system.
    ///
    /// Suspended apps cannot create graphics contexts and are likely to be killed if the user does not
    /// return. Operations that persist data should flush on suspension.
    ///
    /// App suspension is controlled by the view-process, the [`VIEW_PROCESS_SUSPENDED_EVENT`] notifies
    /// on suspension and the [`VIEW_PROCESS_INITED_EVENT`] notifies a "respawn" on resume.
    pub fn is_suspended(&self) -> Var<bool> {
        expr_var! {
            let inited = #{VIEW_PROCESS_INITED_EVENT.var_latest()};
            let sus = #{VIEW_PROCESS_SUSPENDED_EVENT.var_latest()};

            match (sus, inited) {
                (_, None) => true,                               // never inited
                (None, Some(_)) => false,                        // inited, never suspended
                (Some(s), Some(i)) => s.timestamp > i.timestamp, // if suspended after last init
            }
        }
    }
}

/// App time control.
///
/// The manual time methods are only recommended for headless apps. These methods apply immediately, there are not like service methods that
/// only apply after current update.
impl APP {
    /// Gets a variable that configures if [`INSTANT.now`] is the same exact value during each update, info, layout or render pass.
    ///
    /// Time is paused by default, setting this to `false` will cause [`INSTANT.now`] to read the system time for every call.
    ///
    /// [`INSTANT.now`]: crate::INSTANT::now
    pub fn pause_time_for_update(&self) -> Var<bool> {
        APP_PROCESS_SV.read().pause_time_for_updates.clone()
    }

    /// Pause the [`INSTANT.now`] value, after this call it must be updated manually using
    /// [`advance_manual_time`] or [`set_manual_time`]. To resume normal time use [`end_manual_time`].
    ///
    /// [`INSTANT.now`]: crate::INSTANT::now
    /// [`advance_manual_time`]: Self::advance_manual_time
    /// [`set_manual_time`]: Self::set_manual_time
    /// [`end_manual_time`]: Self::end_manual_time
    pub fn start_manual_time(&self) {
        INSTANT_APP.set_mode(InstantMode::Manual);
        INSTANT_APP.set_now(INSTANT.now());
        UPDATES.update(None);
    }

    /// Adds the `advance` to the current manual time.
    ///
    /// Note that you must ensure an update reaches the code that controls manual time, otherwise
    /// the app loop may end-up stuck on idle or awaiting a timer that never elapses.
    ///
    /// # Panics
    ///
    /// Panics if called before [`start_manual_time`].
    ///
    /// [`start_manual_time`]: Self::start_manual_time
    pub fn advance_manual_time(&self, advance: Duration) {
        INSTANT_APP.advance_now(advance);
        UPDATES.update(None);
    }

    /// Set the current [`INSTANT.now`].
    ///
    /// # Panics
    ///
    /// Panics if called before [`start_manual_time`].
    ///
    /// [`INSTANT.now`]: crate::INSTANT::now
    /// [`start_manual_time`]: Self::start_manual_time
    pub fn set_manual_time(&self, now: DInstant) {
        INSTANT_APP.set_now(now);
        UPDATES.update(None);
    }

    /// Resumes normal time.
    pub fn end_manual_time(&self) {
        INSTANT_APP.set_mode(match APP.pause_time_for_update().get() {
            true => InstantMode::UpdatePaused,
            false => InstantMode::Now,
        });
        UPDATES.update(None);
    }
}

command! {
    /// Represents the app process [`exit`] request.
    ///
    /// [`exit`]: APP::exit
    pub static EXIT_CMD = {
        l10n!: true,
        name: "Exit",
        info: "Close all windows and exit",
        shortcut: shortcut!(Exit),
    };
}

/// Cancellation message of an [exit request].
///
/// [exit request]: APP::exit
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitCancelled;
impl fmt::Display for ExitCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exit request cancelled")
    }
}

pub(crate) fn assert_not_view_process() {
    if zng_view_api::ViewConfig::from_env().is_some() {
        panic!("cannot start App in view-process");
    }
}
/// When compiled with `"deadlock_detection"` spawns a thread that monitors for `parking_lot` deadlocks.
///
/// Note that this method is already called on app scope spawn.
/// You can call it before `zng::env::init!` to detect deadlocks in other processes too.
#[cfg(feature = "deadlock_detection")]
pub fn spawn_deadlock_detection() {
    use parking_lot::deadlock;
    use std::{
        sync::atomic::{self, AtomicBool},
        thread,
        time::*,
    };

    static CHECK_RUNNING: AtomicBool = AtomicBool::new(false);

    if CHECK_RUNNING.swap(true, atomic::Ordering::SeqCst) {
        return;
    }

    thread::Builder::new()
        .name("deadlock_detection".into())
        .stack_size(256 * 1024)
        .spawn(|| {
            loop {
                thread::sleep(Duration::from_secs(10));

                let deadlocks = deadlock::check_deadlock();
                if deadlocks.is_empty() {
                    continue;
                }

                use std::fmt::Write;
                let mut msg = String::new();

                let _ = writeln!(&mut msg, "{} deadlocks detected", deadlocks.len());
                for (i, threads) in deadlocks.iter().enumerate() {
                    let _ = writeln!(&mut msg, "Deadlock #{}, {} threads", i, threads.len());
                    for t in threads {
                        let _ = writeln!(&mut msg, "Thread Id {:#?}", t.thread_id());
                        let _ = writeln!(&mut msg, "{:#?}", t.backtrace());
                    }
                }

                #[cfg(not(feature = "test_util"))]
                eprint!("{msg}");

                #[cfg(feature = "test_util")]
                {
                    // test runner captures output and ignores panics in background threads, so
                    // we write directly to stderr and exit the process.
                    use std::io::Write;
                    let _ = write!(&mut std::io::stderr(), "{msg}");
                    zng_env::exit(-1);
                }
            }
        })
        .expect("failed to spawn thread");
}
/// When compiled with `"deadlock_detection"` spawns a thread that monitors for `parking_lot` deadlocks.
///
/// Note that this method is already called on app scope spawn.
/// You can call it before `zng::env::init!` to detect deadlocks in other processes too.
#[cfg(not(feature = "deadlock_detection"))]
pub fn spawn_deadlock_detection() {}

app_local! {
    pub(super) static APP_PROCESS_SV: AppProcessService = AppProcessService {
        exit_requests: None,
        exit: false,
        device_events_filter: zng_var::var(Default::default()),
        pause_time_for_updates: zng_var::var(true),
        is_suspended: zng_var::var(false),
    };
}

pub(super) struct AppProcessService {
    exit_requests: Option<ResponderVar<ExitCancelled>>,
    pub(crate) exit: bool,
    pub(crate) device_events_filter: Var<DeviceEventsFilter>,
    pause_time_for_updates: Var<bool>,
    is_suspended: Var<bool>,
}

/// App events.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // Event is the most used variant
pub(crate) enum AppEvent {
    /// Event from the View Process.
    ViewEvent(zng_view_api::Event),
    /// Do an update cycle.
    Update(UpdateOp, Option<WidgetId>),
    /// Resume a panic in the app main thread.
    ResumeUnwind(PanicPayload),
    /// Check for pending updates.
    CheckUpdate,
}

/// A sender that can awake apps and insert events into the main loop.
///
/// A Clone of the sender is available in [`UPDATES.sender`].
///
/// [`UPDATES.sender`]: crate::update::UPDATES::sender
#[derive(Clone)]
pub struct AppEventSender(channel::Sender<AppEvent>);
impl AppEventSender {
    pub(crate) fn new() -> (Self, channel::Receiver<AppEvent>) {
        let (sender, receiver) = channel::unbounded();
        (Self(sender), receiver)
    }

    #[allow(clippy::result_large_err)] // error does not move far up the stack
    fn send_app_event(&self, event: AppEvent) -> Result<(), ChannelError> {
        self.0.send_blocking(event)
    }

    #[allow(clippy::result_large_err)]
    fn send_view_event(&self, event: zng_view_api::Event) -> Result<(), ChannelError> {
        self.0.send_blocking(AppEvent::ViewEvent(event))
    }

    /// Causes an update cycle to happen in the app.
    pub fn send_update(&self, op: UpdateOp, target: impl Into<Option<WidgetId>>) -> Result<(), ChannelError> {
        UpdatesTrace::log_update();
        self.send_app_event(AppEvent::Update(op, target.into()))
    }

    /// Resume a panic in the app main loop thread.
    pub fn send_resume_unwind(&self, payload: PanicPayload) -> Result<(), ChannelError> {
        self.send_app_event(AppEvent::ResumeUnwind(payload))
    }

    /// [`UPDATES`] util.
    pub(crate) fn send_check_update(&self) -> Result<(), ChannelError> {
        self.send_app_event(AppEvent::CheckUpdate)
    }

    /// Create an [`Waker`] that causes a [`send_update`](Self::send_update).
    pub fn waker(&self, target: impl Into<Option<WidgetId>>) -> Waker {
        Arc::new(AppWaker(self.0.clone(), target.into())).into()
    }
}

struct AppWaker(channel::Sender<AppEvent>, Option<WidgetId>);
impl std::task::Wake for AppWaker {
    fn wake(self: std::sync::Arc<Self>) {
        self.wake_by_ref()
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let _ = self.0.send_blocking(AppEvent::Update(UpdateOp::Update, self.1));
    }
}

type PanicPayload = Box<dyn std::any::Any + Send + 'static>;

event_args! {
    /// Arguments for [`EXIT_REQUESTED_EVENT`].
    ///
    /// Requesting `propagation().stop()` on this event cancels the exit.
    pub struct ExitRequestedArgs {

        ..

        /// Broadcast to all.
        fn is_in_target(&self, _id: WidgetId) -> bool {
            true
        }
    }
}

event! {
    /// Cancellable event raised when app process exit is requested.
    ///
    /// App exit can be requested using the [`APP`] service or the [`EXIT_CMD`], some extensions
    /// also request exit if some conditions are met, for example, `WindowManager` requests it after the last window
    /// is closed.
    ///
    /// Requesting `propagation().stop()` on this event cancels the exit.
    pub static EXIT_REQUESTED_EVENT: ExitRequestedArgs;
}
