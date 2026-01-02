use std::{
    collections::VecDeque,
    fmt, mem,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tracing::span::EnteredSpan;
use webrender::{
    RenderApi, Renderer, Transaction, UploadMethod, VertexUsageHint,
    api::{DocumentId, DynamicProperties, FontInstanceKey, FontKey, FontVariation, PipelineId},
};

use winit::{
    event_loop::ActiveEventLoop,
    monitor::{MonitorHandle, VideoModeHandle as GVideoMode},
    window::{CustomCursor, Fullscreen, Icon, Window as GWindow, WindowAttributes},
};
use zng_txt::{ToTxt, Txt};
use zng_unit::{DipPoint, DipRect, DipSideOffsets, DipSize, DipToPx, Factor, Px, PxPoint, PxRect, PxToDip, PxVector, Rgba};
use zng_view_api::{
    Event, ViewProcessGen,
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    font::{FontFaceId, FontId, FontOptions, FontVariationName, IpcFontBytes},
    image::{ImageDecoded, ImageId, ImageMaskMode, ImageTextureId},
    raw_input::InputDeviceId,
    window::{
        CursorIcon, FocusIndicator, FrameCapture, FrameId, FrameRequest, FrameUpdateRequest, RenderMode, ResizeDirection, VideoMode,
        WindowButton, WindowId, WindowRequest, WindowState, WindowStateAll,
    },
};

use zng_view_api::dialog as dlg_api;

#[cfg(windows)]
use zng_view_api::keyboard::{Key, KeyCode, KeyState};

use crate::{
    AppEvent, AppEventSender, FrameReadyMsg, WrNotifier,
    display_list::{DisplayListCache, display_list_to_webrender},
    extensions::{
        self, BlobExtensionsImgHandler, DisplayListExtAdapter, FrameReadyArgs, RedrawArgs, RendererCommandArgs, RendererConfigArgs,
        RendererDeinitedArgs, RendererExtension, RendererInitedArgs, WindowCommandArgs, WindowConfigArgs, WindowDeinitedArgs,
        WindowExtension, WindowInitedArgs,
    },
    gl::{GlContext, GlContextManager},
    image_cache::{Image, ImageCache, ImageUseMap, WrImageCache},
    px_wr::PxToWr as _,
    util::{
        CursorToWinit, DipToWinit, PxToWinit, ResizeDirectionToWinit as _, WindowButtonsToWinit as _, WinitToDip, WinitToPx,
        frame_render_reasons, frame_update_render_reasons,
    },
};

/// A headed window.
pub(crate) struct Window {
    id: WindowId,
    pipeline_id: PipelineId,
    document_id: DocumentId,

    api: RenderApi,
    image_use: ImageUseMap,

    display_list_cache: DisplayListCache,
    clear_color: Option<Rgba>,

    context: GlContext, // context must be dropped before window.
    window: GWindow,
    renderer: Option<Renderer>,
    window_exts: Vec<(ApiExtensionId, Box<dyn WindowExtension>)>,
    renderer_exts: Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
    external_images: extensions::ExternalImages,
    capture_mode: bool,

    pending_frames: VecDeque<(FrameId, FrameCapture, Option<EnteredSpan>)>,
    rendered_frame_id: FrameId,
    kiosk: bool,

    resized: bool,

    video_mode: VideoMode,

    state: WindowStateAll,

    prev_pos: PxPoint, // in the global space
    prev_size: DipSize,

    prev_monitor: Option<MonitorHandle>,

    visible: bool,
    is_always_on_top: bool,
    waiting_first_frame: bool,
    steal_init_focus: bool,
    init_focus_request: Option<FocusIndicator>,

    taskbar_visible: bool,

    movable: bool,

    cursor_pos: DipPoint,
    cursor_device: InputDeviceId,
    cursor_over: bool,

    touch_pos: Vec<((InputDeviceId, u64), DipPoint)>,

    focused: Option<bool>,

    render_mode: RenderMode,

    modal_dialog_active: Arc<AtomicBool>,

    access: Option<accesskit_winit::Adapter>, // None if has panicked

    ime_area: Option<DipRect>,
    #[cfg(windows)]
    has_shutdown_warn: bool,

    cursor: Option<CursorIcon>,
    cursor_img: Option<CustomCursor>,

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    xlib_maximize: bool,
}
impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .field("document_id", &self.document_id)
            .finish_non_exhaustive()
    }
}
impl Window {
    #[expect(clippy::too_many_arguments)]
    pub fn open(
        vp_gen: ViewProcessGen,
        cfg_icon: Option<Icon>,
        cfg_cursor_image: Option<CustomCursor>,
        cfg: WindowRequest,
        winit_loop: &ActiveEventLoop,
        gl_manager: &mut GlContextManager,
        mut window_exts: Vec<(ApiExtensionId, Box<dyn WindowExtension>)>,
        mut renderer_exts: Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
        event_sender: AppEventSender,
    ) -> Self {
        let id = cfg.id;

        let window_scope = tracing::trace_span!("glutin").entered();

        // create window and OpenGL context
        let mut winit = WindowAttributes::default()
            .with_title(cfg.title)
            .with_resizable(cfg.resizable)
            .with_transparent(cfg.transparent && cfg!(not(target_os = "android")))
            .with_window_icon(cfg_icon);

        let mut s = cfg.state;
        s.clamp_size();

        let mut monitor = winit_loop.primary_monitor();
        let mut monitor_is_primary = true;
        for m in winit_loop.available_monitors() {
            let pos = m.position();
            let size = m.size();
            let rect = PxRect::new(pos.to_px(), size.to_px());

            if rect.contains(s.global_position) {
                let m = Some(m);
                monitor_is_primary = m == monitor;
                monitor = m;
                break;
            }
        }
        let monitor = monitor;
        let monitor_is_primary = monitor_is_primary;

        if let WindowState::Normal = s.state {
            winit = winit
                .with_min_inner_size(s.min_size.to_winit())
                .with_max_inner_size(s.max_size.to_winit())
                .with_inner_size(s.restore_rect.size.to_winit());

            if let Some(m) = monitor {
                if cfg.default_position {
                    if (cfg!(windows) && !monitor_is_primary) || cfg!(target_os = "linux") {
                        // default Windows position is in the primary only.
                        // default X11 position is outer zero.

                        let mut pos = m.position();
                        pos.x += 120;
                        pos.y += 80;
                        winit = winit.with_position(pos);
                    }
                } else {
                    let mut pos_in_monitor = s.restore_rect.origin.to_px(Factor(m.scale_factor() as _));

                    let monitor_size = m.size();
                    if pos_in_monitor.x.0 > monitor_size.width as _ {
                        pos_in_monitor.x.0 = 120;
                    }
                    if pos_in_monitor.y.0 > monitor_size.height as _ {
                        pos_in_monitor.y.0 = 80;
                    }

                    let mut pos = m.position();
                    pos.x += pos_in_monitor.x.0;
                    pos.y += pos_in_monitor.y.0;

                    winit = winit.with_position(pos);
                }
            }
        } else if let Some(m) = monitor {
            // fallback to center.
            let screen_size = m.size().to_px().to_dip(Factor(m.scale_factor() as _));
            s.restore_rect.origin.x = (screen_size.width - s.restore_rect.size.width) / 2.0;
            s.restore_rect.origin.y = (screen_size.height - s.restore_rect.size.height) / 2.0;

            // place on monitor
            winit = winit.with_position(m.position());
        }

        winit = winit
            .with_decorations(s.chrome_visible)
            // we wait for the first frame to show the window,
            // so that there is no white frame when it's opening.
            //
            // unless its "kiosk" mode.
            .with_visible(cfg!(target_os = "android") || cfg.kiosk || matches!(s.state, WindowState::Exclusive));

        let mut render_mode = cfg.render_mode;
        if !cfg!(feature = "software") && render_mode == RenderMode::Software {
            tracing::warn!("ignoring `RenderMode::Software` because did not build with \"software\" feature");
            render_mode = RenderMode::Integrated;
        }

        #[cfg(windows)]
        let mut prefer_egl = false;
        #[cfg(not(windows))]
        let prefer_egl = false;

        for (id, ext) in &mut window_exts {
            ext.configure(&mut WindowConfigArgs {
                config: cfg.extensions.iter().find(|(k, _)| k == id).map(|(_, p)| p),
                window: Some(&mut winit),
            });

            #[cfg(windows)]
            if let Some(ext) = ext.as_any().downcast_ref::<crate::extensions::PreferAngleExt>() {
                prefer_egl = ext.prefer_egl;
            }
        }

        let (winit_window, mut context) = gl_manager.create_headed(id, winit, winit_loop, render_mode, &event_sender, prefer_egl);

        render_mode = context.render_mode();

        window_exts.retain_mut(|(_, ext)| {
            ext.window_inited(&mut WindowInitedArgs {
                window: &winit_window,
                context: &mut context,
            });
            !ext.is_init_only()
        });

        // * Extend the winit Windows window to not block the Alt+F4 key press.
        // * Check if the window is actually keyboard focused until first focus.
        // * Block system shutdown if a block is set.
        #[cfg(windows)]
        {
            let event_sender = event_sender.clone();

            let mut first_focus = false;

            let window_id = winit_window.id();
            let hwnd = crate::util::winit_to_hwnd(&winit_window);
            crate::util::set_raw_windows_event_handler(hwnd as _, u32::from_ne_bytes(*b"alf4") as _, move |_, msg, wparam, _| {
                if !first_focus && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() } == hwnd as _ {
                    // Windows sends a `WM_SETFOCUS` when the window open, even if the user changed focus to something
                    // else before the process opens the window so that the window title bar shows the unfocused visual and
                    // we are not actually keyboard focused. We block this in `focused_changed` but then become out-of-sync
                    // with the native window state, to recover from this we check the system wide foreground window at every
                    // opportunity until we actually become the keyboard focus, at that point we can stop checking because we are in sync with
                    // the native window state and the native window state is in sync with the system wide state.
                    first_focus = true;
                    let _ = event_sender.send(AppEvent::WinitFocused(window_id, true));
                }

                match msg {
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_SYSKEYDOWN => {
                        if wparam as windows_sys::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY
                            == windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F4
                        {
                            // winit always blocks ALT+F4 we want to allow it so that the shortcut is handled in the same way as other commands.

                            let _ = event_sender.send(AppEvent::Notify(Event::KeyboardInput {
                                window: id,
                                device: InputDeviceId::INVALID, // same as winit
                                key_code: KeyCode::F4,
                                state: KeyState::Pressed,
                                key: Key::F4,
                                key_modified: Key::F4,
                                text: Txt::from_static(""),
                                key_location: zng_view_api::keyboard::KeyLocation::Standard,
                            }));
                            return Some(0);
                        }
                    }
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_QUERYENDSESSION => {
                        let mut reason = [0u16; 256];
                        let mut reason_size = reason.len() as u32;
                        let ok = unsafe {
                            windows_sys::Win32::System::Shutdown::ShutdownBlockReasonQuery(hwnd as _, reason.as_mut_ptr(), &mut reason_size)
                        };
                        if ok != 0 {
                            let s = windows::core::HSTRING::from_wide(&reason);
                            tracing::warn!("blocked system shutdown, reason: {}", s);
                            // send a close requested to hopefully cause the normal close/cancel dialog to appear.
                            let _ = event_sender.send(AppEvent::Notify(Event::WindowCloseRequested(id)));
                            return Some(0);
                        }
                    }
                    _ => {}
                }

                None
            });
        }

        drop(window_scope);
        let wr_scope = tracing::trace_span!("webrender").entered();

        // create renderer and start the first frame.

        let device_size = winit_window.inner_size().to_px().to_wr_device();

        let mut opts = webrender::WebRenderOptions {
            // text-aa config from Firefox.
            enable_aa: true,
            enable_subpixel_aa: cfg!(not(target_os = "android")),

            renderer_id: Some(((vp_gen.get() as u64) << 32) | id.get() as u64),

            // this clear color paints over the one set using `Renderer::set_clear_color`.
            clear_color: webrender::api::ColorF::new(0.0, 0.0, 0.0, 0.0),

            allow_advanced_blend_equation: context.is_software(),
            clear_caches_with_quads: !context.is_software(),
            enable_gpu_markers: !context.is_software(),

            // best for GL
            upload_method: UploadMethod::PixelBuffer(VertexUsageHint::Dynamic),

            // extensions expect this to be set.
            workers: Some(crate::util::wr_workers()),
            // optimize memory usage
            chunk_pool: Some(crate::util::wr_chunk_pool()),

            // rendering is broken on Android emulators with unoptimized shaders.
            // see: https://github.com/servo/servo/pull/31727
            // webrender issue: https://bugzilla.mozilla.org/show_bug.cgi?id=1887337
            #[cfg(target_os = "android")]
            use_optimized_shaders: true,

            //panic_on_gl_error: true,
            ..Default::default()
        };
        let mut blobs = BlobExtensionsImgHandler(vec![]);
        for (id, ext) in &mut renderer_exts {
            ext.configure(&mut RendererConfigArgs {
                config: cfg.extensions.iter().find(|(k, _)| k == id).map(|(_, p)| p),
                options: &mut opts,
                blobs: &mut blobs.0,
                window: Some(&winit_window),
                context: &mut context,
            });
        }
        if !opts.enable_multithreading {
            for b in &mut blobs.0 {
                b.enable_multithreading(false);
            }
        }
        opts.blob_image_handler = Some(Box::new(blobs));

        let (mut renderer, sender) =
            webrender::create_webrender_instance(context.gl().clone(), WrNotifier::create(id, event_sender.clone()), opts, None).unwrap();
        renderer.set_external_image_handler(WrImageCache::new_boxed());

        let mut external_images = extensions::ExternalImages::default();

        let mut api = sender.create_api();
        let document_id = api.add_document(device_size);
        let pipeline_id = webrender::api::PipelineId(vp_gen.get(), id.get());

        renderer_exts.retain_mut(|(_, ext)| {
            ext.renderer_inited(&mut RendererInitedArgs {
                renderer: &mut renderer,
                external_images: &mut external_images,
                api_sender: &sender,
                api: &mut api,
                document_id,
                pipeline_id,
                window: Some(&winit_window),
                context: &mut context,
            });
            !ext.is_init_only()
        });

        drop(wr_scope);

        let access = accesskit_winit::Adapter::with_direct_handlers(
            winit_loop,
            &winit_window,
            AccessActivateHandler {
                id,
                event_sender: event_sender.clone(),
            },
            AccessActionSender {
                id,
                event_sender: event_sender.clone(),
            },
            AccessDeactivateHandler { id, event_sender },
        );

        let mut win = Self {
            id,
            image_use: ImageUseMap::new(),
            prev_pos: winit_window.inner_position().unwrap_or_default().to_px(),
            prev_size: winit_window.inner_size().to_px().to_dip(Factor(winit_window.scale_factor() as _)),
            prev_monitor: winit_window.current_monitor(),
            state: s,
            kiosk: cfg.kiosk,
            window: winit_window,
            context,
            capture_mode: cfg.capture_mode,
            renderer: Some(renderer),
            window_exts,
            renderer_exts,
            external_images,
            video_mode: cfg.video_mode,
            display_list_cache: DisplayListCache::new(pipeline_id, api.get_namespace_id()),
            api,
            document_id,
            pipeline_id,
            resized: true,
            waiting_first_frame: true,
            steal_init_focus: cfg.focus,
            init_focus_request: cfg.focus_indicator,
            visible: cfg.visible,
            is_always_on_top: false,
            taskbar_visible: true,
            movable: cfg.movable,
            pending_frames: VecDeque::new(),
            rendered_frame_id: FrameId::INVALID,
            cursor_pos: DipPoint::zero(),
            touch_pos: vec![],
            cursor_device: InputDeviceId::INVALID,
            cursor_over: false,
            clear_color: None,
            focused: None,
            modal_dialog_active: Arc::new(AtomicBool::new(false)),
            render_mode,
            access: Some(access),
            ime_area: cfg.ime_area,
            #[cfg(windows)]
            has_shutdown_warn: false,
            cursor: None,
            cursor_img: None,

            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            xlib_maximize: false,
        };

        if !cfg.default_position && win.state.state == WindowState::Normal {
            win.set_inner_position(win.state.restore_rect.origin);
        }

        if cfg.always_on_top {
            win.set_always_on_top(true);
        }

        win.cursor = cfg.cursor;
        win.cursor_img = cfg_cursor_image;
        win.update_cursor();

        win.set_taskbar_visible(cfg.taskbar_visible);

        win.set_enabled_buttons(cfg.enabled_buttons);

        if !cfg.system_shutdown_warn.is_empty() {
            win.set_system_shutdown_warn(cfg.system_shutdown_warn);
        }

        if win.ime_area.is_some() {
            win.window.set_ime_allowed(true);
        }

        // settings these in the builder causes flickering
        match win.state.state {
            WindowState::Normal | WindowState::Minimized => {}
            WindowState::Maximized => win.window.set_maximized(true),
            WindowState::Fullscreen => win.window.set_fullscreen(Some(Fullscreen::Borderless(None))),
            WindowState::Exclusive => win.window.set_fullscreen(Some(if let Some(mode) = win.video_mode() {
                Fullscreen::Exclusive(mode)
            } else {
                Fullscreen::Borderless(None)
            })),
        }

        win.state.global_position = win.window.inner_position().unwrap_or_default().to_px();
        let monitor_offset = if let Some(m) = win.window.current_monitor() {
            m.position().to_px().to_vector()
        } else {
            PxVector::zero()
        };

        if win.state.state == WindowState::Normal && cfg.default_position {
            // system position.
            win.state.restore_rect.origin = (win.state.global_position - monitor_offset).to_dip(win.scale_factor());
        }

        win
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        self.window.current_monitor()
    }

    pub fn window_id(&self) -> winit::window::WindowId {
        self.window.id()
    }

    /// Latest rendered frame.
    pub fn frame_id(&self) -> FrameId {
        self.rendered_frame_id
    }

    pub fn set_title(&self, title: Txt) {
        self.window.set_title(&title);
    }

    /// Window event should ignore interaction events.
    ///
    /// Dialogs are already modal in Windows and Mac, but Linux
    pub fn modal_dialog_active(&self) -> bool {
        self.modal_dialog_active.load(Ordering::Relaxed)
    }

    /// Returns `true` if the cursor actually moved.
    pub fn cursor_moved(&mut self, pos: DipPoint, device: InputDeviceId) -> bool {
        if !self.cursor_over {
            // this can happen on X11
            return false;
        }

        let moved = self.cursor_pos != pos || self.cursor_device != device;

        if moved {
            self.cursor_pos = pos;
            self.cursor_device = device;
        }

        moved
    }

    /// Returns `true` if the touch actually moved.
    pub fn touch_moved(&mut self, pos: DipPoint, device: InputDeviceId, touch: u64) -> bool {
        if let Some(p) = self.touch_pos.iter_mut().find(|p| p.0 == (device, touch)) {
            let moved = p.1 != pos;
            p.1 = pos;
            moved
        } else {
            self.touch_pos.push(((device, touch), pos));
            true
        }
    }

    /// Clear touch position.
    pub fn touch_end(&mut self, device: InputDeviceId, touch: u64) {
        if let Some(i) = self.touch_pos.iter().position(|p| p.0 == (device, touch)) {
            self.touch_pos.swap_remove(i);
        }
    }

    #[cfg(windows)]
    fn windows_is_foreground(&self) -> bool {
        let foreground = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        foreground == crate::util::winit_to_hwnd(&self.window) as _
    }

    pub fn is_focused(&self) -> bool {
        self.focused.unwrap_or(false)
    }

    /// Returns `true` if the previous focused status is different from `focused`.
    ///
    /// Sets the `focused` to if the window is actually the foreground keyboard focused window.
    pub fn focused_changed(&mut self, focused: &mut bool) -> bool {
        #[cfg(windows)]
        if self.focused.is_none() {
            *focused = self.windows_is_foreground();
        }

        let focused = Some(*focused);

        let changed = self.focused != focused;
        if changed {
            self.focused = focused;
        }
        changed
    }

    /// Returns the last cursor moved data.
    pub fn last_cursor_pos(&self) -> (DipPoint, InputDeviceId) {
        (self.cursor_pos, self.cursor_device)
    }

    /// Returns `true` if the cursor was not over the window.
    pub fn cursor_entered(&mut self) -> bool {
        let changed = !self.cursor_over;
        self.cursor_over = true;
        changed
    }

    /// Returns `true` if the cursor was over the window.
    pub fn cursor_left(&mut self) -> bool {
        #[cfg(windows)]
        if Self::has_mouse_capture(&self.window) {
            // winit sends a CursorLeft event while capture in Windows
            return false;
        }

        let changed = self.cursor_over;
        self.cursor_over = false;
        changed
    }

    #[cfg(windows)]
    fn has_mouse_capture<W: raw_window_handle::HasWindowHandle>(window: &W) -> bool {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetCapture;

        if let Ok(handle) = window.window_handle() {
            if let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() {
                let hwnd = h.hwnd.get();
                // SAFETY: function can be called at any time
                unsafe { GetCapture() == hwnd as _ }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        if self.kiosk && !self.visible {
            tracing::error!("window in `kiosk` mode cannot be hidden");
        }

        if !self.waiting_first_frame {
            let _s = tracing::trace_span!("set_visible", %visible).entered();

            self.visible = visible;

            if visible {
                if self.state.state != WindowState::Minimized {
                    self.window.set_minimized(false);
                }

                self.window.set_visible(true);
                self.apply_state(self.state.clone(), true);
            } else {
                if self.state.state != WindowState::Minimized {
                    // if the state is maximized or fullscreen the window is not hidden, a white
                    // "restored" window is shown instead.
                    self.window.set_minimized(true);
                }

                self.window.set_visible(false);
            }
        }
    }

    pub fn set_always_on_top(&mut self, always_on_top: bool) {
        self.window.set_window_level(if always_on_top {
            winit::window::WindowLevel::AlwaysOnTop
        } else {
            winit::window::WindowLevel::Normal
        });
        self.is_always_on_top = always_on_top;
    }

    pub fn set_movable(&mut self, movable: bool) {
        self.movable = movable;
    }

    pub fn set_resizable(&mut self, resizable: bool) {
        self.window.set_resizable(resizable)
    }

    #[cfg(windows)]
    pub fn bring_to_top(&mut self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;

        if !self.is_always_on_top {
            let hwnd = crate::util::winit_to_hwnd(&self.window);

            unsafe {
                let _ = SetWindowPos(
                    hwnd as _,
                    HWND_TOP,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );
            }
        }
    }

    #[cfg(not(windows))]
    pub fn bring_to_top(&mut self) {
        if !self.is_always_on_top {
            self.set_always_on_top(true);
            self.set_always_on_top(false);
        }
    }

    /// Returns `Some((new_global_pos, new_pos))` if the window position is different from the previous call to this function.
    pub fn moved(&mut self) -> Option<(PxPoint, DipPoint)> {
        if !self.visible {
            return None;
        }

        let new_pos = match self.window.inner_position() {
            Ok(p) => p.to_px(),
            Err(e) => {
                tracing::error!("cannot get inner_position, {e}");
                PxPoint::zero()
            }
        };
        if self.prev_pos != new_pos {
            self.prev_pos = new_pos;

            let monitor_offset = if let Some(m) = self.window.current_monitor() {
                m.position().to_px().to_vector()
            } else {
                PxVector::zero()
            };

            Some((new_pos, (new_pos - monitor_offset).to_dip(self.scale_factor())))
        } else {
            None
        }
    }

    /// Returns `Some(new_size)` if the window size is different from the previous call to this function.
    pub fn resized(&mut self) -> Option<DipSize> {
        if !self.visible {
            return None;
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        if std::mem::take(&mut self.xlib_maximize) {
            // X11 does not open maximized
            // to work we need to set inner_size (after first frame) and request maximized here after xlib resizes.
            self.window.set_maximized(true);
            return None;
        }

        let new_size = self.window.inner_size().to_px().to_dip(self.scale_factor());
        if self.prev_size != new_size {
            #[cfg(windows)]
            if matches!(self.state.state, WindowState::Maximized | WindowState::Fullscreen)
                && self.window.current_monitor() != self.window.primary_monitor()
            {
                // workaround issue when opening a window maximized in a non-primary monitor
                // causes it to use the maximized style, but not the size.

                match self.state.state {
                    WindowState::Maximized => {
                        self.window.set_maximized(false);
                        self.window.set_maximized(true);
                    }
                    WindowState::Fullscreen => {
                        self.window.set_fullscreen(None);
                        self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }
                    _ => unreachable!(),
                }

                let new_size = self.window.inner_size().to_px().to_dip(self.scale_factor());
                return if self.prev_size != new_size {
                    self.prev_size = new_size;
                    self.resized = true;

                    Some(new_size)
                } else {
                    None
                };
            }

            self.prev_size = new_size;
            self.resized = true;

            Some(new_size)
        } else {
            None
        }
    }

    /// Returns `Some(new_monitor)` if the parent monitor changed from the previous call to this function.
    pub fn monitor_change(&mut self) -> Option<MonitorHandle> {
        let handle = self.window.current_monitor();
        if self.prev_monitor != handle {
            self.prev_monitor.clone_from(&handle);
            handle
        } else {
            None
        }
    }

    #[cfg(windows)]
    fn windows_set_restore(&self) {
        use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO, MONITORINFOEXW};
        use windows_sys::Win32::{
            Foundation::{POINT, RECT},
            UI::WindowsAndMessaging::*,
        };
        use winit::platform::windows::MonitorHandleExtWindows;

        if let Some(monitor) = self.window.current_monitor() {
            let hwnd = crate::util::winit_to_hwnd(&self.window) as _;
            let mut placement = WINDOWPLACEMENT {
                length: mem::size_of::<WINDOWPLACEMENT>() as _,
                flags: 0,
                showCmd: 0,
                ptMinPosition: POINT { x: 0, y: 0 },
                ptMaxPosition: POINT { x: 0, y: 0 },
                rcNormalPosition: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
            };
            if unsafe { GetWindowPlacement(hwnd, &mut placement) } != 0 {
                let scale_factor = self.scale_factor();
                let mut left_top = self.state.restore_rect.origin.to_px(scale_factor);

                // placement is in "workspace", window is in "virtual screen space".
                let hmonitor = monitor.hmonitor() as _;
                let mut monitor_info = MONITORINFOEXW {
                    monitorInfo: MONITORINFO {
                        cbSize: mem::size_of::<MONITORINFOEXW>() as _,
                        rcMonitor: RECT {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        },
                        rcWork: RECT {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        },
                        dwFlags: 0,
                    },
                    szDevice: [0; 32],
                };
                if unsafe { GetMonitorInfoW(hmonitor, &mut monitor_info as *mut MONITORINFOEXW as *mut MONITORINFO) } != 0 {
                    left_top.x.0 += monitor_info.monitorInfo.rcWork.left;
                    left_top.y.0 += monitor_info.monitorInfo.rcWork.top;
                }

                // placement includes the non-client area.
                let outer_offset =
                    self.window.outer_position().unwrap_or_default().to_px() - self.window.inner_position().unwrap_or_default().to_px();
                let size_offset = self.window.outer_size().to_px() - self.window.inner_size().to_px();

                left_top += outer_offset;
                let bottom_right = left_top + self.state.restore_rect.size.to_px(scale_factor) + size_offset;

                placement.rcNormalPosition.top = left_top.y.0;
                placement.rcNormalPosition.left = left_top.x.0;
                placement.rcNormalPosition.bottom = bottom_right.y.0;
                placement.rcNormalPosition.right = bottom_right.x.0;

                let _ = unsafe { SetWindowPlacement(hwnd, &placement) };
            }
        }
    }

    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.window.set_window_icon(icon);
    }

    /// Set named cursor.
    pub fn set_cursor(&mut self, icon: Option<CursorIcon>) {
        self.cursor = icon;
        self.update_cursor();
    }

    /// Set custom cursor.
    pub fn set_cursor_image(&mut self, img: Option<CustomCursor>) {
        self.cursor_img = img;
        self.update_cursor();
    }

    fn update_cursor(&self) {
        match (&self.cursor_img, self.cursor) {
            (Some(i), _) => {
                self.window.set_cursor(i.clone());
                self.window.set_cursor_visible(true);
            }
            (None, Some(i)) => {
                self.window.set_cursor(i.to_winit());
                self.window.set_cursor_visible(true);
            }
            (None, None) => {
                self.window.set_cursor_visible(false);
                self.window.set_cursor(CursorIcon::Default.to_winit());
            }
        }
    }

    /// Sets the focus request indicator.
    pub fn set_focus_request(&mut self, request: Option<FocusIndicator>) {
        if self.waiting_first_frame {
            self.init_focus_request = request;
        } else {
            self.window.request_user_attention(request.map(|r| match r {
                FocusIndicator::Critical => winit::window::UserAttentionType::Critical,
                FocusIndicator::Info => winit::window::UserAttentionType::Informational,
                _ => winit::window::UserAttentionType::Informational,
            }));
        }
    }

    /// Steal input focus.
    #[cfg(not(windows))]
    pub fn focus(&mut self) -> zng_view_api::FocusResult {
        if self.waiting_first_frame {
            self.steal_init_focus = true;
        } else if !self.modal_dialog_active() {
            self.window.focus_window();
        }
        if self.window.has_focus() {
            zng_view_api::FocusResult::AlreadyFocused
        } else {
            zng_view_api::FocusResult::Requested
        }
    }

    /// Steal input focus.
    ///
    /// Returns if the next `RAlt` press and release key inputs must be ignored.
    #[cfg(windows)]
    #[must_use]
    pub fn focus(&mut self) -> (zng_view_api::FocusResult, bool) {
        let skip_ralt = if self.waiting_first_frame {
            self.steal_init_focus = true;
            false
        } else if !self.modal_dialog_active() && !self.windows_is_foreground() {
            // winit uses a hack to steal focus that causes a `RAlt` key press.
            self.window.focus_window();
            self.windows_is_foreground()
        } else {
            false
        };
        let r = if self.window.has_focus() {
            zng_view_api::FocusResult::AlreadyFocused
        } else {
            zng_view_api::FocusResult::Requested
        };
        (r, skip_ralt)
    }

    /// Gets the current Maximized status as early as possible.
    fn is_maximized(&self) -> bool {
        #[cfg(windows)]
        {
            let hwnd = crate::util::winit_to_hwnd(&self.window);
            // SAFETY: function does not fail.
            return unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsZoomed(hwnd as _) } != 0;
        }

        #[allow(unreachable_code)]
        {
            // this changes only after the Resized event, we want state change detection before the Moved also.
            self.window.is_maximized()
        }
    }

    /// Gets the current Maximized status.
    fn is_minimized(&self) -> bool {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return true;
        }

        #[cfg(windows)]
        {
            let hwnd = crate::util::winit_to_hwnd(&self.window);
            // SAFETY: function does not fail.
            return unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd as _) } != 0;
        }

        #[allow(unreachable_code)]
        false
    }

    fn probe_state(&self) -> WindowStateAll {
        let mut state = self.state.clone();

        state.global_position = match self.window.inner_position() {
            Ok(p) => p.to_px(),
            Err(e) => {
                tracing::error!("cannot get inner_position, {e}");
                PxPoint::zero()
            }
        };

        if self.is_minimized() {
            state.state = WindowState::Minimized;
        } else if let Some(h) = self.window.fullscreen() {
            state.state = match h {
                Fullscreen::Exclusive(_) => WindowState::Exclusive,
                Fullscreen::Borderless(_) => WindowState::Fullscreen,
            };
        } else if self.is_maximized() {
            state.state = WindowState::Maximized;
        } else {
            state.state = WindowState::Normal;

            let scale_factor = self.scale_factor();

            let monitor_offset = if let Some(monitor) = self.window.current_monitor() {
                monitor.position().to_px().to_vector()
            } else {
                PxVector::zero()
            };

            state.restore_rect = DipRect::new(
                (state.global_position - monitor_offset).to_dip(scale_factor),
                self.window.inner_size().to_px().to_dip(scale_factor),
            );
        }

        state
    }

    /// Probe state, returns `Some(new_state)`
    pub fn state_change(&mut self) -> Option<WindowStateAll> {
        if !self.visible {
            return None;
        }

        let mut new_state = self.probe_state();

        if self.state.state == WindowState::Minimized && self.state.restore_state == WindowState::Fullscreen {
            self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else if new_state.state == WindowState::Normal && self.state.state != WindowState::Normal {
            new_state.restore_rect = self.state.restore_rect;

            self.set_inner_position(new_state.restore_rect.origin);
            let new_size = new_state.restore_rect.size.to_winit();
            if let Some(immediate_new_size) = self.window.request_inner_size(new_size)
                && immediate_new_size == new_size.to_physical(self.window.scale_factor())
            {
                // size changed immediately, winit says: "resize event in such case may not be generated"
                // * Review of Windows and Linux shows that the resize event is send.
                tracing::debug!("immediate resize may not have notified, new size: {immediate_new_size:?}");
            }

            self.window.set_min_inner_size(Some(new_state.min_size.to_winit()));
            self.window.set_max_inner_size(Some(new_state.max_size.to_winit()));
        }

        new_state.set_restore_state_from(self.state.state);

        if new_state != self.state {
            self.state = new_state.clone();
            Some(new_state)
        } else {
            None
        }
    }

    fn video_mode(&self) -> Option<GVideoMode> {
        let mode = &self.video_mode;
        self.window.current_monitor().and_then(|m| {
            let mut candidate: Option<GVideoMode> = None;
            for m in m.video_modes() {
                // filter out video modes larger than requested
                if m.size().width <= mode.size.width.0 as u32
                    && m.size().height <= mode.size.height.0 as u32
                    && m.bit_depth() <= mode.bit_depth
                    && m.refresh_rate_millihertz() <= mode.refresh_rate
                {
                    // select closest match to the requested video mode
                    if let Some(c) = &candidate {
                        if m.size().width >= c.size().width
                            && m.size().height >= c.size().height
                            && m.bit_depth() >= c.bit_depth()
                            && m.refresh_rate_millihertz() >= c.refresh_rate_millihertz()
                        {
                            candidate = Some(m);
                        }
                    } else {
                        candidate = Some(m);
                    }
                }
            }
            candidate
        })
    }

    pub fn set_video_mode(&mut self, mode: VideoMode) {
        self.video_mode = mode;
        if let WindowState::Exclusive = self.state.state {
            self.window.set_fullscreen(None);

            if let Some(mode) = self.video_mode() {
                self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
            } else {
                self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }
    }

    #[cfg(not(windows))]
    pub fn set_taskbar_visible(&mut self, visible: bool) {
        if visible != self.taskbar_visible {
            return;
        }
        self.taskbar_visible = visible;
        tracing::warn!("`set_taskbar_visible` not implemented for {}", std::env::consts::OS);
    }

    #[cfg(windows)]
    pub fn set_taskbar_visible(&mut self, visible: bool) {
        if visible == self.taskbar_visible {
            return;
        }
        self.taskbar_visible = visible;

        use windows_sys::Win32::System::Com::*;

        use crate::util::taskbar_com;

        // winit already initializes COM

        unsafe {
            let mut taskbar_list2: *mut taskbar_com::ITaskbarList2 = std::ptr::null_mut();
            match CoCreateInstance(
                &taskbar_com::CLSID_TaskbarList,
                std::ptr::null_mut(),
                CLSCTX_ALL,
                &taskbar_com::IID_ITaskbarList2,
                &mut taskbar_list2 as *mut _ as *mut _,
            ) {
                0 => {
                    let result = if visible {
                        let add_tab = (*(*taskbar_list2).lpVtbl).parent.AddTab;
                        add_tab(taskbar_list2.cast(), crate::util::winit_to_hwnd(&self.window) as _)
                    } else {
                        let delete_tab = (*(*taskbar_list2).lpVtbl).parent.DeleteTab;
                        delete_tab(taskbar_list2.cast(), crate::util::winit_to_hwnd(&self.window) as _)
                    };
                    if result != 0 {
                        let mtd_name = if visible { "AddTab" } else { "DeleteTab" };
                        tracing::error!(
                            target: "window",
                            "cannot set `taskbar_visible`, `ITaskbarList::{mtd_name}` failed, error: 0x{result:x}",
                        )
                    }

                    let release = (*(*taskbar_list2).lpVtbl).parent.parent.Release;
                    let result = release(taskbar_list2.cast());
                    if result != 0 {
                        tracing::error!(
                            target: "window",
                            "failed to release `taskbar_list`, error: 0x{result:x}"
                        )
                    }
                }
                error => {
                    tracing::error!(
                        target: "window",
                        "cannot set `taskbar_visible`, failed to create instance of `ITaskbarList`, error: 0x{error:x}",
                    )
                }
            }
        }
    }

    /// Returns of the last update state.
    pub fn state(&self) -> WindowStateAll {
        self.state.clone()
    }

    fn set_inner_position(&self, pos: DipPoint) {
        let monitor_offset = if let Some(m) = self.window.current_monitor() {
            m.position().to_px().to_vector()
        } else {
            PxVector::zero()
        };

        let outer_pos = self.window.outer_position().unwrap_or_default();
        let inner_pos = self.window.inner_position().unwrap_or_default();
        let inner_offset = PxVector::new(Px(outer_pos.x - inner_pos.x), Px(outer_pos.y - inner_pos.y));
        let pos = pos.to_px(self.scale_factor()) + monitor_offset + inner_offset;
        self.window.set_outer_position(pos.to_winit());
    }

    /// Reset all window state.
    ///
    /// Returns `true` if the state changed.
    pub fn set_state(&mut self, new_state: WindowStateAll) -> bool {
        if self.state == new_state {
            return false;
        }

        if !self.visible {
            // will force apply when set to visible again.
            self.state = new_state;
            return true;
        }

        self.apply_state(new_state, false);

        true
    }

    /// Moves the window with the left mouse button until the button is released.
    pub fn drag_move(&self) {
        if let Err(e) = self.window.drag_window() {
            tracing::error!("failed to drag_move, {e}");
        }
    }

    /// Resizes the window with the left mouse button until the button is released.
    pub fn drag_resize(&self, direction: ResizeDirection) {
        if let Err(e) = self.window.drag_resize_window(direction.to_winit()) {
            tracing::error!("failed to drag_resize, {e}");
        }
    }

    /// Set enabled chrome buttons.
    pub fn set_enabled_buttons(&self, buttons: WindowButton) {
        self.window.set_enabled_buttons(buttons.to_winit());
    }

    /// Open windows title bar context menu.
    pub fn open_title_bar_context_menu(&self, pos: DipPoint) {
        self.window.show_window_menu(pos.to_winit())
    }

    fn apply_state(&mut self, new_state: WindowStateAll, force: bool) {
        if self.state.chrome_visible != new_state.chrome_visible {
            self.window.set_decorations(new_state.chrome_visible);
        }

        if self.state.state != new_state.state || force {
            // unset previous state.
            match self.state.state {
                WindowState::Normal => {}
                WindowState::Minimized => self.window.set_minimized(false),
                WindowState::Maximized => {
                    if !new_state.state.is_fullscreen() && new_state.state != WindowState::Minimized {
                        self.window.set_maximized(false);
                    }
                }
                WindowState::Fullscreen | WindowState::Exclusive => self.window.set_fullscreen(None),
            }

            // set new state.
            match new_state.state {
                WindowState::Normal => {}
                WindowState::Minimized => self.window.set_minimized(true),
                WindowState::Maximized => self.window.set_maximized(true),
                WindowState::Fullscreen => {
                    self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                }
                WindowState::Exclusive => {
                    if let Some(mode) = self.video_mode() {
                        self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
                    } else {
                        self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }
                }
            }
        }

        self.state = new_state;

        if self.state.state == WindowState::Normal {
            let _ = self.window.request_inner_size(self.state.restore_rect.size.to_winit());

            let outer_offset = match (self.window.outer_position(), self.window.inner_position()) {
                (Ok(o), Ok(i)) => (i.x - o.x, i.y - o.y),
                _ => (0, 0),
            };
            let mut origin = self
                .state
                .restore_rect
                .origin
                .to_winit()
                .to_physical::<i32>(self.window.scale_factor());
            origin.x -= outer_offset.0;
            origin.y -= outer_offset.1;
            self.window.set_outer_position(origin);

            self.window.set_min_inner_size(Some(self.state.min_size.to_winit()));
            self.window.set_max_inner_size(Some(self.state.max_size.to_winit()));

            // this can happen if minimized from "Task Manager"
            //
            // - Set to Fullscreen.
            // - Minimize from Windows Task Manager.
            // - Restore from Taskbar.
            // - Set the state to Normal.
            //
            // Without this hack the window stays minimized and then restores
            // Normal but at the fullscreen size.
            #[cfg(windows)]
            if self.is_minimized() {
                self.windows_set_restore();

                self.window.set_minimized(true);
                self.window.set_minimized(false);
            }
        }

        // Update restore placement for Windows to avoid rendering incorrect frame when the OS restores the window.
        //
        // Windows changes the size if it considers the window "restored", that is the case for `Normal` and `Borderless` fullscreen.
        #[cfg(windows)]
        if !matches!(self.state.state, WindowState::Normal | WindowState::Fullscreen) {
            self.windows_set_restore();
        }
    }

    pub fn use_image(&mut self, image: &Image) -> ImageTextureId {
        self.image_use.new_use(image, self.document_id, &mut self.api)
    }

    pub fn update_image(&mut self, texture_id: ImageTextureId, image: &Image, dirty_rect: Option<PxRect>) -> bool {
        self.image_use
            .update_use(texture_id, image, dirty_rect, self.document_id, &mut self.api)
    }

    pub fn delete_image(&mut self, texture_id: ImageTextureId) {
        self.image_use.delete(texture_id, self.document_id, &mut self.api);
    }

    pub fn add_font_face(&mut self, font: IpcFontBytes, index: u32) -> FontFaceId {
        #[cfg(target_os = "macos")]
        let index = {
            if index != 0 {
                tracing::error!("webrender does not support font index on macOS, ignoring `{index}` will use `0`");
            }
            0
        };
        let key = self.api.generate_font_key();
        let mut txn = webrender::Transaction::new();
        match font {
            IpcFontBytes::Bytes(b) => txn.add_raw_font(key, b.to_vec(), index),
            IpcFontBytes::System(p) => {
                #[cfg(not(any(target_os = "macos", target_os = "ios")))]
                txn.add_native_font(key, webrender::api::NativeFontHandle { path: p, index });

                #[cfg(any(target_os = "macos", target_os = "ios"))]
                match std::fs::read(p) {
                    Ok(d) => txn.add_raw_font(key, d, index),
                    Err(e) => {
                        tracing::error!("cannot load font, {e}");
                        return FontFaceId::INVALID;
                    }
                }
            }
        }
        self.api.send_transaction(self.document_id, txn);
        FontFaceId::from_raw(key.1)
    }

    pub fn delete_font_face(&mut self, font_face_id: FontFaceId) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font(FontKey(self.api.get_namespace_id(), font_face_id.get()));
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn add_font(
        &mut self,
        font_face_id: FontFaceId,
        glyph_size: Px,
        options: FontOptions,
        variations: Vec<(FontVariationName, f32)>,
    ) -> FontId {
        let key = self.api.generate_font_instance_key();
        let mut txn = webrender::Transaction::new();
        txn.add_font_instance(
            key,
            FontKey(self.api.get_namespace_id(), font_face_id.get()),
            glyph_size.to_wr().get(),
            options.to_wr(),
            None,
            variations
                .into_iter()
                .map(|(n, v)| FontVariation {
                    tag: u32::from_be_bytes(n),
                    value: v,
                })
                .collect(),
        );
        self.api.send_transaction(self.document_id, txn);
        FontId::from_raw(key.1)
    }

    pub fn delete_font(&mut self, font_id: FontId) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font_instance(FontInstanceKey(self.api.get_namespace_id(), font_id.get()));
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn set_capture_mode(&mut self, enabled: bool) {
        self.capture_mode = enabled;
    }

    /// Start rendering a new frame.
    ///
    /// The [callback](#callback) will be called when the frame is ready to be [presented](Self::present).
    pub fn render(&mut self, frame: FrameRequest) {
        let _scope = tracing::trace_span!("render", ?frame.id).entered();

        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color.to_wr());

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        self.push_resize(&mut txn);
        txn.generate_frame(frame.id.get(), true, false, frame_render_reasons(&frame));

        let display_list = display_list_to_webrender(
            frame.display_list,
            &mut DisplayListExtAdapter {
                frame_id: frame.id,
                extensions: &mut self.renderer_exts,
                transaction: &mut txn,
                document_id: self.document_id,
                renderer: self.renderer.as_mut().unwrap(),
                api: &mut self.api,
                external_images: &mut self.external_images,
            },
            &mut self.image_use,
            &mut self.display_list_cache,
        );

        txn.reset_dynamic_properties();
        txn.append_dynamic_properties(DynamicProperties {
            transforms: vec![],
            floats: vec![],
            colors: vec![],
        });

        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color.to_wr());
        self.clear_color = Some(frame.clear_color);

        txn.set_display_list(webrender::api::Epoch(frame.id.epoch()), (self.pipeline_id, display_list));

        let frame_scope =
            tracing::trace_span!("<frame>", ?frame.id, capture = ?frame.capture, from_update = false, thread = "<webrender>").entered();

        self.pending_frames.push_back((frame.id, frame.capture, Some(frame_scope)));

        self.api.send_transaction(self.document_id, txn);
    }

    /// Start rendering a new frame based on the data of the last frame.
    pub fn render_update(&mut self, frame: FrameUpdateRequest) {
        let _scope = tracing::trace_span!("render_update", ?frame.id).entered();

        let render_reasons = frame_update_render_reasons(&frame);

        if let Some(color) = frame.clear_color {
            self.clear_color = Some(color);
            self.renderer.as_mut().unwrap().set_clear_color(color.to_wr());
        }

        let resized = self.resized;

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        self.push_resize(&mut txn);
        txn.generate_frame(self.frame_id().get(), true, false, render_reasons);

        let frame_scope = match self.display_list_cache.update(
            &mut DisplayListExtAdapter {
                frame_id: self.frame_id(),
                extensions: &mut self.renderer_exts,
                transaction: &mut txn,
                document_id: self.document_id,
                renderer: self.renderer.as_mut().unwrap(),
                api: &mut self.api,
                external_images: &mut self.external_images,
            },
            &mut self.image_use,
            frame.transforms,
            frame.floats,
            frame.colors,
            frame.extensions,
            resized,
        ) {
            Ok(p) => {
                if let Some(p) = p {
                    txn.append_dynamic_properties(p);
                }

                tracing::trace_span!("<frame-update>", ?frame.id, capture = ?frame.capture, thread = "<webrender>")
            }
            Err(d) => {
                txn.reset_dynamic_properties();
                txn.append_dynamic_properties(DynamicProperties {
                    transforms: vec![],
                    floats: vec![],
                    colors: vec![],
                });

                txn.set_display_list(webrender::api::Epoch(frame.id.epoch()), (self.pipeline_id, d));

                tracing::trace_span!("<frame>", ?frame.id, capture = ?frame.capture, from_update = true, thread = "<webrender>")
            }
        };

        self.pending_frames
            .push_back((frame.id, frame.capture, Some(frame_scope.entered())));

        self.api.send_transaction(self.document_id, txn);
    }

    /// Returns info for `FrameRendered` and if this is the first frame.
    #[must_use = "events must be generated from the result"]
    pub fn on_frame_ready(&mut self, msg: FrameReadyMsg, images: &mut ImageCache) -> FrameReadyResult {
        let (frame_id, capture, _) = self
            .pending_frames
            .pop_front()
            .unwrap_or((self.rendered_frame_id, FrameCapture::None, None));
        self.rendered_frame_id = frame_id;

        let first_frame = self.waiting_first_frame;

        let mut ext_args = FrameReadyArgs {
            frame_id,
            redraw: msg.composite_needed || self.waiting_first_frame,
        };
        for (_, ext) in &mut self.renderer_exts {
            ext.frame_ready(&mut ext_args);
            ext_args.redraw |= msg.composite_needed || self.waiting_first_frame;
        }

        if self.waiting_first_frame {
            let _s = tracing::trace_span!("first-draw").entered();
            debug_assert!(msg.composite_needed);

            self.waiting_first_frame = false;
            let s = self.window.inner_size();
            self.context.make_current();
            self.context.resize(s);
            self.redraw();
            if self.kiosk {
                self.window.request_redraw();
            } else if self.visible {
                self.set_visible(true);

                // X11 does not open maximized
                // to work we need to set inner_size and after xlib resizes it request maximized...
                #[cfg(any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd"
                ))]
                if let raw_window_handle::RawWindowHandle::Xlib(_) =
                    raw_window_handle::HasWindowHandle::window_handle(&self.window).unwrap().as_raw()
                    && let WindowState::Maximized = self.state.state
                {
                    self.xlib_maximize = self.window.request_inner_size(self.window.inner_size()).is_none();
                }

                if mem::take(&mut self.steal_init_focus) {
                    self.window.focus_window();
                }
                if let Some(r) = self.init_focus_request.take() {
                    self.set_focus_request(Some(r));
                }
            }
        } else if ext_args.redraw || msg.composite_needed {
            self.window.request_redraw();
        }

        let scale_factor = self.scale_factor();

        let capture = match capture {
            FrameCapture::None => None,
            FrameCapture::Full => Some(None),
            FrameCapture::Mask(m) => Some(Some(m)),
            _ => None,
        };
        let image = if let Some(mask) = capture {
            let _s = tracing::trace_span!("capture_image").entered();
            if ext_args.redraw || msg.composite_needed {
                self.redraw();
            }
            images
                .frame_image_data(
                    &**self.context.gl(),
                    PxRect::from_size(self.window.inner_size().to_px()),
                    scale_factor,
                    mask,
                )
                .ok()
        } else {
            None
        };

        FrameReadyResult {
            frame_id,
            image,
            first_frame,
        }
    }

    pub fn redraw(&mut self) {
        let span = tracing::trace_span!("redraw", stats = tracing::field::Empty).entered();

        self.context.make_current();

        let scale_factor = self.scale_factor();
        let size = self.window.inner_size().to_px();

        let renderer = self.renderer.as_mut().unwrap();
        renderer.update();

        let r = renderer.render(size.to_wr_device(), 0).unwrap();
        span.record("stats", tracing::field::debug(&r.stats));

        for (_, ext) in &mut self.renderer_exts {
            ext.redraw(&mut RedrawArgs {
                scale_factor,
                size,
                context: &mut self.context,
            });
        }

        let _ = renderer.flush_pipeline_info();

        self.window.pre_present_notify();
        self.context.swap_buffers();
    }

    pub fn is_rendering_frame(&self) -> bool {
        !self.pending_frames.is_empty()
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;

            self.context.make_current();
            let size = self.window.inner_size();
            self.context.resize(size);
            txn.set_document_view(PxRect::from_size(size.to_px()).to_wr_device());
        }
    }

    pub fn frame_image(&mut self, images: &mut ImageCache, mask: Option<ImageMaskMode>) -> ImageId {
        let scale_factor = self.scale_factor();
        if !self.context.is_software() {
            self.redraw(); // refresh back buffer
        }
        images.frame_image(
            &**self.context.gl(),
            PxRect::from_size(self.window.inner_size().to_px()),
            self.id,
            self.rendered_frame_id,
            scale_factor,
            mask,
        )
    }

    pub fn frame_image_rect(&mut self, images: &mut ImageCache, rect: PxRect, mask: Option<ImageMaskMode>) -> ImageId {
        let scale_factor = self.scale_factor();
        let rect = PxRect::from_size(self.window.inner_size().to_px())
            .intersection(&rect)
            .unwrap_or_default();
        if !self.context.is_software() {
            self.redraw(); // refresh back buffer
        }
        images.frame_image(&**self.context.gl(), rect, self.id, self.rendered_frame_id, scale_factor, mask)
    }

    /// (global_position, monitor_position)
    pub fn inner_position(&self) -> (PxPoint, DipPoint) {
        let global_pos = self.window.inner_position().unwrap_or_default().to_px();
        let monitor_offset = if let Some(m) = self.window.current_monitor() {
            m.position().to_px().to_vector()
        } else {
            PxVector::zero()
        };

        (global_pos, (global_pos - monitor_offset).to_dip(self.scale_factor()))
    }

    pub fn size(&self) -> DipSize {
        self.window.inner_size().to_logical(self.window.scale_factor()).to_dip()
    }

    pub fn safe_padding(&self) -> DipSideOffsets {
        #[cfg(target_os = "android")]
        match self.try_get_insets() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("cannot get insets, {e}");
                DipSideOffsets::zero()
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            DipSideOffsets::zero()
        }
    }
    #[cfg(target_os = "android")]
    fn try_get_insets(&self) -> jni::errors::Result<DipSideOffsets> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as _) }?;

        let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };
        let mut env = vm.attach_current_thread()?;

        let jni_window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;

        let view = env.call_method(jni_window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;

        let insets = env
            .call_method(view, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?
            .l()?;
        let cutout = env
            .call_method(insets, "getDisplayCutout", "()Landroid/view/DisplayCutout;", &[])?
            .l()?;
        let top = env.call_method(&cutout, "getSafeInsetTop", "()I", &[])?.i()? as i32;
        let right = env.call_method(&cutout, "getSafeInsetRight", "()I", &[])?.i()? as i32;
        let bottom = env.call_method(&cutout, "getSafeInsetBottom", "()I", &[])?.i()? as i32;
        let left = env.call_method(&cutout, "getSafeInsetLeft", "()I", &[])?.i()? as i32;

        let offsets = zng_unit::PxSideOffsets::new(Px(top), Px(right), Px(bottom), Px(left));

        Ok(offsets.to_dip(self.scale_factor()))
    }

    pub fn scale_factor(&self) -> Factor {
        Factor(self.window.scale_factor() as f32)
    }

    /// Window actual render mode.
    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    /// Calls the window extension command.
    pub fn window_extension(&mut self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> ApiExtensionPayload {
        for (key, ext) in &mut self.window_exts {
            if *key == extension_id {
                return ext.command(&mut WindowCommandArgs {
                    window: &self.window,
                    context: &mut self.context,
                    request,
                });
            }
        }
        ApiExtensionPayload::unknown_extension(extension_id)
    }

    /// Calls the render extension command.
    pub fn render_extension(&mut self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> ApiExtensionPayload {
        for (key, ext) in &mut self.renderer_exts {
            if *key == extension_id {
                let mut redraw = false;
                let r = ext.command(&mut RendererCommandArgs {
                    renderer: self.renderer.as_mut().unwrap(),
                    api: &mut self.api,
                    request,
                    document_id: self.document_id,
                    window: Some(&self.window),
                    context: &mut self.context,
                    redraw: &mut redraw,
                });
                if redraw {
                    self.window.request_redraw();
                }

                return r;
            }
        }
        ApiExtensionPayload::unknown_extension(extension_id)
    }

    #[cfg(not(target_os = "android"))]
    fn enter_dialog(&self, id: dlg_api::DialogId, event_sender: &AppEventSender) -> bool {
        let already_open = self.modal_dialog_active.swap(true, Ordering::Acquire);
        if already_open {
            let _ = event_sender.send(AppEvent::Notify(Event::MsgDialogResponse(
                id,
                dlg_api::MsgDialogResponse::Error(Txt::from_static("dialog already open")),
            )));
        }
        already_open
    }

    #[cfg(target_os = "android")]
    pub(crate) fn message_dialog(&self, dialog: dlg_api::MsgDialog, id: dlg_api::DialogId, event_sender: AppEventSender) {
        let _ = dialog;
        let _ = event_sender.send(AppEvent::Notify(Event::MsgDialogResponse(
            id,
            dlg_api::MsgDialogResponse::Error(Txt::from_static("native dialogs not implemented for android")),
        )));
    }

    /// Shows a native message dialog.
    #[cfg(not(target_os = "android"))]
    pub(crate) fn message_dialog(&self, dialog: dlg_api::MsgDialog, id: dlg_api::DialogId, event_sender: AppEventSender) {
        if self.enter_dialog(id, &event_sender) {
            return;
        }

        let dlg = rfd::AsyncMessageDialog::new()
            .set_level(match dialog.icon {
                dlg_api::MsgDialogIcon::Info => rfd::MessageLevel::Info,
                dlg_api::MsgDialogIcon::Warn => rfd::MessageLevel::Warning,
                dlg_api::MsgDialogIcon::Error => rfd::MessageLevel::Error,
                _ => rfd::MessageLevel::Info,
            })
            .set_buttons(match dialog.buttons {
                dlg_api::MsgDialogButtons::Ok => rfd::MessageButtons::Ok,
                dlg_api::MsgDialogButtons::OkCancel => rfd::MessageButtons::OkCancel,
                dlg_api::MsgDialogButtons::YesNo => rfd::MessageButtons::YesNo,
                _ => rfd::MessageButtons::Ok,
            })
            .set_title(dialog.title.as_str())
            .set_description(dialog.message.as_str())
            .set_parent(&self.window);

        let modal_dialog_active = self.modal_dialog_active.clone();
        Self::run_dialog(async move {
            let r = dlg.show().await;

            let r = match dialog.buttons {
                dlg_api::MsgDialogButtons::Ok => dlg_api::MsgDialogResponse::Ok,
                dlg_api::MsgDialogButtons::OkCancel => match r {
                    rfd::MessageDialogResult::Yes => dlg_api::MsgDialogResponse::Ok,
                    rfd::MessageDialogResult::No => dlg_api::MsgDialogResponse::Cancel,
                    rfd::MessageDialogResult::Ok => dlg_api::MsgDialogResponse::Ok,
                    rfd::MessageDialogResult::Cancel => dlg_api::MsgDialogResponse::Cancel,
                    rfd::MessageDialogResult::Custom(_) => dlg_api::MsgDialogResponse::Cancel,
                },
                dlg_api::MsgDialogButtons::YesNo => match r {
                    rfd::MessageDialogResult::Yes => dlg_api::MsgDialogResponse::Yes,
                    rfd::MessageDialogResult::No => dlg_api::MsgDialogResponse::No,
                    rfd::MessageDialogResult::Ok => dlg_api::MsgDialogResponse::Yes,
                    rfd::MessageDialogResult::Cancel => dlg_api::MsgDialogResponse::No,
                    rfd::MessageDialogResult::Custom(_) => dlg_api::MsgDialogResponse::No,
                },
                _ => dlg_api::MsgDialogResponse::Ok,
            };
            modal_dialog_active.store(false, Ordering::Release);
            let _ = event_sender.send(AppEvent::Notify(Event::MsgDialogResponse(id, r)));
        });
    }

    #[cfg(target_os = "android")]
    pub(crate) fn file_dialog(&self, dialog: dlg_api::FileDialog, id: dlg_api::DialogId, event_sender: AppEventSender) {
        let _ = dialog;
        let _ = event_sender.send(AppEvent::Notify(Event::MsgDialogResponse(
            id,
            dlg_api::MsgDialogResponse::Error(Txt::from_static("native dialogs not implemented for android")),
        )));
    }

    /// Shows a native file dialog.
    #[cfg(not(target_os = "android"))]
    pub(crate) fn file_dialog(&self, dialog: dlg_api::FileDialog, id: dlg_api::DialogId, event_sender: AppEventSender) {
        if self.enter_dialog(id, &event_sender) {
            return;
        }

        let mut dlg = rfd::AsyncFileDialog::new()
            .set_title(dialog.title.as_str())
            .set_directory(&dialog.starting_dir)
            .set_file_name(dialog.starting_name.as_str())
            .set_parent(&self.window);
        for (name, patterns) in dialog.iter_filters() {
            dlg = dlg.add_filter(
                name,
                &patterns
                    .map(|s| {
                        let s = s.trim_start_matches(['*', '.']);
                        if s.is_empty() { "*" } else { s }
                    })
                    .collect::<Vec<_>>(),
            );
        }

        let modal_dialog_active = self.modal_dialog_active.clone();
        Self::run_dialog(async move {
            let selection: Vec<_> = match dialog.kind {
                dlg_api::FileDialogKind::OpenFile => dlg.pick_file().await.into_iter().map(Into::into).collect(),
                dlg_api::FileDialogKind::OpenFiles => dlg.pick_files().await.into_iter().flatten().map(Into::into).collect(),
                dlg_api::FileDialogKind::SelectFolder => dlg.pick_folder().await.into_iter().map(Into::into).collect(),
                dlg_api::FileDialogKind::SelectFolders => dlg.pick_folders().await.into_iter().flatten().map(Into::into).collect(),
                dlg_api::FileDialogKind::SaveFile => dlg.save_file().await.into_iter().map(Into::into).collect(),
                _ => vec![],
            };

            let r = if selection.is_empty() {
                dlg_api::FileDialogResponse::Cancel
            } else {
                dlg_api::FileDialogResponse::Selected(selection)
            };

            modal_dialog_active.store(false, Ordering::Release);
            let _ = event_sender.send(AppEvent::Notify(Event::FileDialogResponse(id, r)));
        });
    }
    /// Run dialog unblocked.
    #[cfg(not(target_os = "android"))]
    fn run_dialog(run: impl Future + Send + 'static) {
        let mut task = Box::pin(run);
        std::thread::Builder::new()
            .name("run_dialog".into())
            .spawn(move || {
                struct ThreadWaker(std::thread::Thread);
                impl std::task::Wake for ThreadWaker {
                    fn wake(self: std::sync::Arc<Self>) {
                        self.0.unpark();
                    }
                }
                let waker = Arc::new(ThreadWaker(std::thread::current())).into();
                let mut cx = std::task::Context::from_waker(&waker);
                loop {
                    match task.as_mut().poll(&mut cx) {
                        std::task::Poll::Ready(_) => return,
                        std::task::Poll::Pending => std::thread::park(),
                    }
                }
            })
            .expect("failed to spawn thread");
    }

    /// Pump the accessibility adapter and window extensions.
    pub fn on_window_event(&mut self, event: &winit::event::WindowEvent) {
        if let Some(a) = &mut self.access {
            a.process_event(&self.window, event);
        }
        for (_, ext) in &mut self.window_exts {
            ext.event(&mut extensions::WindowEventArgs {
                window: &self.window,
                context: &mut self.context,
                event,
            });
        }
    }

    /// Update the accessibility info.
    pub fn access_update(&mut self, update: zng_view_api::access::AccessTreeUpdate, event_sender: &AppEventSender) {
        if let Some(a) = &mut self.access {
            // SAFETY: we drop `access` in case of panic.
            let mut a = std::panic::AssertUnwindSafe(a);
            let panic = crate::util::catch_suppress(move || {
                a.update_if_active(|| crate::util::access_tree_update_to_kit(update));
            });
            if let Err(p) = panic {
                self.access = None;

                let _ = event_sender.send(AppEvent::Notify(Event::RecoveredFromComponentPanic {
                    component: Txt::from_static("accesskit_winit::Adapter::update_if_active"),
                    recover: Txt::from_static("accessibility disabled for this window instance"),
                    panic: p.to_txt(),
                }));
            }
        }
    }

    pub(crate) fn on_low_memory(&mut self) {
        self.api.notify_memory_pressure();

        for (_, ext) in &mut self.renderer_exts {
            ext.low_memory();
        }
    }

    pub(crate) fn set_ime_area(&mut self, area: Option<DipRect>) {
        if let Some(a) = area {
            if self.ime_area != Some(a) {
                if self.ime_area.is_none() {
                    self.window.set_ime_allowed(true);

                    #[cfg(target_os = "android")]
                    self.set_mobile_keyboard_vis(true);
                }

                self.ime_area = Some(a);
                self.window.set_ime_cursor_area(a.origin.to_winit(), a.size.to_winit());
            }
        } else if self.ime_area.is_some() {
            self.window.set_ime_allowed(false);
            self.ime_area = None;

            #[cfg(target_os = "android")]
            self.set_mobile_keyboard_vis(false);
        }
    }
    #[cfg(target_os = "android")]
    fn set_mobile_keyboard_vis(&self, visible: bool) {
        // this does not work
        //
        // let app = crate::platform::android::android_app();
        // if visible {
        //     app.show_soft_input(false);
        // } else {
        //     app.hide_soft_input(false);
        // }

        if let Err(e) = self.try_show_hide_soft_keyboard(visible) {
            tracing::error!("cannot {} mobile keyboard, {e}", if visible { "show" } else { "hide" });
        }
    }
    #[cfg(target_os = "android")]
    fn try_show_hide_soft_keyboard(&self, show: bool) -> jni::errors::Result<()> {
        use jni::objects::JValue;

        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as _) }?;

        let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };
        let mut env = vm.attach_current_thread()?;

        let class_ctx = env.find_class("android/content/Context")?;
        let ims = env.get_static_field(class_ctx, "INPUT_METHOD_SERVICE", "Ljava/lang/String;")?;

        let im_manager = env
            .call_method(
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[ims.borrow()],
            )?
            .l()?;

        let jni_window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;

        let view = env.call_method(jni_window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;

        if show {
            env.call_method(&view, "requestFocus", "()Z", &[])?;

            env.call_method(
                im_manager,
                "showSoftInput",
                "(Landroid/view/View;I)Z",
                &[JValue::Object(&view), 0i32.into()],
            )?;
        } else {
            let window_token = env.call_method(view, "getWindowToken", "()Landroid/os/IBinder;", &[])?.l()?;
            let jvalue_window_token = jni::objects::JValueGen::Object(&window_token);

            env.call_method(
                im_manager,
                "hideSoftInputFromWindow",
                "(Landroid/os/IBinder;I)Z",
                &[jvalue_window_token, 0i32.into()],
            )?;
        }

        Ok(())
    }

    #[cfg(windows)]
    pub(crate) fn set_system_shutdown_warn(&mut self, reason: Txt) {
        if !reason.is_empty() {
            self.has_shutdown_warn = true;
            let hwnd = crate::util::winit_to_hwnd(&self.window);
            let reason = windows::core::HSTRING::from(reason.as_str());
            // SAFETY: function return handled.
            let created = unsafe { windows_sys::Win32::System::Shutdown::ShutdownBlockReasonCreate(hwnd as _, reason.as_ptr()) } != 0;
            if !created {
                let error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                tracing::error!("failed to set system shutdown warn ({error:#X}), requested warn reason was: {reason}");
            }
        } else if mem::take(&mut self.has_shutdown_warn) {
            let hwnd = crate::util::winit_to_hwnd(&self.window);
            // SAFETY: function return handled.
            let destroyed = unsafe { windows_sys::Win32::System::Shutdown::ShutdownBlockReasonDestroy(hwnd as _) } != 0;
            if !destroyed {
                let error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                tracing::error!("failed to unset system shutdown warn ({error:#X})");
            }
        }
    }

    #[cfg(not(windows))]
    pub(crate) fn set_system_shutdown_warn(&mut self, reason: Txt) {
        if !reason.is_empty() {
            tracing::warn!("system shutdown warn not implemented on {}", std::env::consts::OS);
        }
    }

    pub(crate) fn drag_drop_cursor_pos(&self) -> Option<DipPoint> {
        #[cfg(windows)]
        {
            let mut pt = windows::Win32::Foundation::POINT::default();
            // SAFETY: normal call
            if unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) }.is_ok() {
                let cursor_pos = PxPoint::new(Px(pt.x), Px(pt.y));
                let win_pos = self.window.inner_position().unwrap_or_default().to_px();
                let pos = cursor_pos - win_pos.to_vector();
                if pos.x >= Px(0) && pos.y >= Px(0) {
                    let size = self.window.inner_size().to_px();
                    if pos.x <= size.width && pos.y <= size.height {
                        return Some(pos.to_dip(self.scale_factor()));
                    }
                }
            }
        }
        None
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        self.set_system_shutdown_warn(Txt::from(""));

        self.api.stop_render_backend();
        self.api.shut_down(true);

        // webrender deinit panics if the context is not current.
        self.context.make_current();
        self.renderer.take().unwrap().deinit();

        for (_, ext) in &mut self.renderer_exts {
            ext.renderer_deinited(&mut RendererDeinitedArgs {
                document_id: self.document_id,
                pipeline_id: self.pipeline_id,
                context: &mut self.context,
                window: Some(&self.window),
            })
        }
        for (_, ext) in &mut self.window_exts {
            ext.window_deinited(&mut WindowDeinitedArgs {
                window: &self.window,
                context: &mut self.context,
            });
        }
    }
}

pub(crate) struct FrameReadyResult {
    pub frame_id: FrameId,
    pub image: Option<ImageDecoded>,
    pub first_frame: bool,
}

struct AccessActivateHandler {
    id: WindowId,
    event_sender: AppEventSender,
}
impl accesskit::ActivationHandler for AccessActivateHandler {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        let _ = self.event_sender.send(AppEvent::Notify(Event::AccessInit { window: self.id }));
        None
    }
}

struct AccessDeactivateHandler {
    id: WindowId,
    event_sender: AppEventSender,
}

impl accesskit::DeactivationHandler for AccessDeactivateHandler {
    fn deactivate_accessibility(&mut self) {
        let _ = self.event_sender.send(AppEvent::Notify(Event::AccessDeinit { window: self.id }));
    }
}

struct AccessActionSender {
    id: WindowId,
    event_sender: AppEventSender,
}
impl accesskit::ActionHandler for AccessActionSender {
    fn do_action(&mut self, request: accesskit::ActionRequest) {
        if let Some(ev) = crate::util::accesskit_to_event(self.id, request) {
            let _ = self.event_sender.send(AppEvent::Notify(ev));
        }
    }
}
