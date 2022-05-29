use std::{cell::Cell, collections::VecDeque, fmt, mem, rc::Rc, sync::Arc};

use glutin::{
    event_loop::EventLoopWindowTarget,
    monitor::{MonitorHandle, VideoMode as GVideoMode},
    window::{Fullscreen, Icon, Window as GWindow, WindowBuilder},
};
use tracing::span::EnteredSpan;
use webrender::{
    api::{
        ApiHitTester, BuiltDisplayList, ColorF, DisplayListPayload, DocumentId, DynamicProperties, FontInstanceKey, FontInstanceOptions,
        FontInstancePlatformOptions, FontKey, FontVariation, HitTestResult, HitTesterRequest, IdNamespace, ImageKey, PipelineId,
        SampledScrollOffset,
    },
    RenderApi, Renderer, RendererOptions, Transaction, UploadMethod, VertexUsageHint,
};
use zero_ui_view_api::{
    units::*, CursorIcon, DeviceId, FocusRequest, FrameId, FrameRequest, FrameUpdateRequest, ImageId, ImageLoadedData, RenderMode,
    VideoMode, ViewProcessGen, WindowId, WindowRequest, WindowState, WindowStateAll,
};

#[cfg(windows)]
use zero_ui_view_api::{Event, Key, KeyState, ScanCode};

use crate::{
    gl::{GlContext, GlContextManager},
    image_cache::{Image, ImageCache, ImageUseMap, WrImageCache},
    util::{CursorToWinit, DipToWinit, WinitToDip, WinitToPx},
    AppEvent, AppEventSender, FrameReadyMsg, WrNotifier,
};

enum HitTester {
    Ready(Arc<dyn ApiHitTester>),
    Request(HitTesterRequest),
    Busy,
}
impl HitTester {
    pub fn new(api: &RenderApi, document_id: DocumentId) -> Self {
        HitTester::Request(api.request_hit_tester(document_id))
    }

    pub fn hit_test(&mut self, point: PxPoint) -> HitTestResult {
        match mem::replace(self, HitTester::Busy) {
            HitTester::Ready(tester) => {
                let result = tester.hit_test(point.to_wr_world());
                *self = HitTester::Ready(tester);
                result
            }
            HitTester::Request(request) => {
                let tester = request.resolve();
                let result = tester.hit_test(point.to_wr_world());
                *self = HitTester::Ready(tester);
                result
            }
            HitTester::Busy => panic!("hit-test must be synchronous"),
        }
    }
}

/// A headed window.
pub(crate) struct Window {
    id: WindowId,
    pipeline_id: PipelineId,
    document_id: DocumentId,

    api: RenderApi,
    image_use: ImageUseMap,

    window: GWindow,
    context: GlContext,
    renderer: Option<Renderer>,
    capture_mode: bool,

    pending_frames: VecDeque<(FrameId, bool, Option<EnteredSpan>)>,
    rendered_frame_id: FrameId,
    kiosk: bool,

    resized: bool,

    video_mode: VideoMode,

    state: WindowStateAll,

    prev_pos: PxPoint,
    prev_size: PxSize,

    prev_monitor: Option<MonitorHandle>,

    visible: bool,
    waiting_first_frame: bool,
    steal_init_focus: bool,
    init_focus_request: Option<FocusRequest>,

    allow_alt_f4: Rc<Cell<bool>>,
    taskbar_visible: bool,

    movable: bool, // TODO

    cursor_pos: DipPoint,
    cursor_device: DeviceId,
    cursor_over: bool,
    hit_tester: HitTester,

    focused: bool,

    render_mode: RenderMode,
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
    pub fn open(
        gen: ViewProcessGen,
        icon: Option<Icon>,
        req: WindowRequest,
        window_target: &EventLoopWindowTarget<AppEvent>,
        gl_manager: &mut GlContextManager,
        event_sender: AppEventSender,
    ) -> Self {
        let id = req.id;

        let window_scope = tracing::trace_span!("glutin").entered();

        // create window and OpenGL context
        let mut winit = WindowBuilder::new()
            .with_title(req.title)
            .with_resizable(req.resizable)
            .with_transparent(req.transparent)
            .with_always_on_top(req.always_on_top)
            .with_window_icon(icon);

        let mut s = req.state;
        s.clamp_size();

        if let WindowState::Normal = s.state {
            winit = winit
                .with_min_inner_size(s.min_size.to_winit())
                .with_max_inner_size(s.max_size.to_winit())
                .with_inner_size(s.restore_rect.size.to_winit());

            #[cfg(target_os = "linux")]
            if req.default_position {
                // default X11 position is outer zero.
                winit = winit.with_position(DipPoint::new(Dip::new(120), Dip::new(80)).to_winit());
            }
        } else if req.default_position {
            if let Some(screen) = window_target.primary_monitor() {
                // fallback to center.
                let screen_size = screen.size().to_px().to_dip(screen.scale_factor() as f32);
                s.restore_rect.origin.x = (screen_size.width - s.restore_rect.size.width) / 2.0;
                s.restore_rect.origin.y = (screen_size.height - s.restore_rect.size.height) / 2.0;
            }
        }

        if let WindowState::Normal | WindowState::Minimized = s.state {
            winit = winit
                .with_decorations(s.chrome_visible)
                // we wait for the first frame to show the window,
                // so that there is no white frame when it's opening.
                //
                // unless its "kiosk" mode.
                .with_visible(req.kiosk);
        } else {
            // Maximized/Fullscreen Flickering Workaround Part 1
            //
            // We can't start maximized or fullscreen with visible=false because
            // that causes a white rectangle over a black background to flash on open.
            // The white rectangle is probably the window in Normal mode, not sure if its caused by winit or glutin.
            //
            // For some reason disabling the window chrome, then enabling it again after opening the window removes
            // the white rectangle, the black background still flashes when transparent=false, but at least its just
            // a solid fill.
            winit = winit.with_decorations(false);
        }

        winit = match s.state {
            WindowState::Normal | WindowState::Minimized => winit,
            WindowState::Maximized => winit.with_maximized(true),
            WindowState::Fullscreen | WindowState::Exclusive => winit.with_fullscreen(Some(Fullscreen::Borderless(None))),
        };

        let mut render_mode = req.render_mode;
        if !cfg!(software) && render_mode == RenderMode::Software {
            tracing::warn!("ignoring `RenderMode::Software` because did not build with \"software\" feature");
            render_mode = RenderMode::Integrated;
        }

        let (context, winit_window) = gl_manager.create_headed(id, winit, window_target, render_mode);
        render_mode = context.render_mode();

        // extend the winit Windows window to only block the Alt+F4 key press if we want it to.
        let allow_alt_f4 = Rc::new(Cell::new(req.allow_alt_f4));
        #[cfg(windows)]
        {
            let allow_alt_f4 = allow_alt_f4.clone();
            let event_sender = event_sender.clone();
            use glutin::platform::windows::WindowExtWindows;

            let hwnd = windows::Win32::Foundation::HWND(winit_window.hwnd() as _);
            crate::util::set_raw_windows_event_handler(hwnd, u32::from_ne_bytes(*b"alf4") as _, move |_, msg, wparam, _| {
                if msg == windows::Win32::UI::WindowsAndMessaging::WM_SYSKEYDOWN
                    && windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(wparam.0 as u16)
                        == windows::Win32::UI::Input::KeyboardAndMouse::VK_F4
                    && allow_alt_f4.get()
                {
                    let device = 0; // TODO recover actual ID

                    let _ = event_sender.send(AppEvent::Notify(Event::KeyboardInput {
                        window: id,
                        device,
                        scan_code: wparam.0 as ScanCode,
                        state: KeyState::Pressed,
                        key: Some(Key::F4),
                    }));
                    return Some(windows::Win32::Foundation::LRESULT(0));
                }
                None
            });
        }

        drop(window_scope);
        let wr_scope = tracing::trace_span!("webrender").entered();

        // create renderer and start the first frame.

        let device_size = winit_window.inner_size().to_px().to_wr_device();

        let opts = RendererOptions {
            // text-aa config from Firefox.
            enable_aa: true,
            force_subpixel_aa: false,
            enable_subpixel_aa: cfg!(not(target_os = "android")),

            renderer_id: Some((gen as u64) << 32 | id as u64),

            // this clear color paints over the one set using `Renderer::set_clear_color`.
            clear_color: ColorF::new(0.0, 0.0, 0.0, 0.0),

            allow_advanced_blend_equation: context.is_software(),
            clear_caches_with_quads: !context.is_software(),
            enable_gpu_markers: !context.is_software(),

            // best for GL
            upload_method: UploadMethod::PixelBuffer(VertexUsageHint::Dynamic),

            //panic_on_gl_error: true,
            ..Default::default()
        };

        let (mut renderer, sender) =
            webrender::Renderer::new(context.gl().clone(), WrNotifier::create(id, event_sender), opts, None).unwrap();
        renderer.set_external_image_handler(WrImageCache::new_boxed());

        let api = sender.create_api();
        let document_id = api.add_document(device_size);

        drop(wr_scope);

        let pipeline_id = webrender::api::PipelineId(gen, id);

        let hit_tester = HitTester::new(&api, document_id);

        let mut win = Self {
            id,
            image_use: ImageUseMap::default(),
            prev_pos: winit_window.inner_position().unwrap_or_default().to_px(),
            prev_size: winit_window.inner_size().to_px(),
            prev_monitor: winit_window.current_monitor(),
            state: s,
            kiosk: req.kiosk,
            window: winit_window,
            context,
            capture_mode: req.capture_mode,
            renderer: Some(renderer),
            video_mode: req.video_mode,
            api,
            document_id,
            pipeline_id,
            resized: true,
            waiting_first_frame: true,
            steal_init_focus: req.focus,
            init_focus_request: req.focus_request,
            visible: req.visible,
            allow_alt_f4,
            taskbar_visible: true,
            movable: req.movable,
            pending_frames: VecDeque::new(),
            rendered_frame_id: FrameId::INVALID,
            cursor_pos: DipPoint::zero(),
            cursor_device: 0,
            cursor_over: false,
            focused: false,
            hit_tester,
            render_mode,
        };

        if !req.default_position && win.state.state == WindowState::Normal {
            win.set_inner_position(win.state.restore_rect.origin);
        }

        // Maximized/Fullscreen Flickering Workaround Part 2
        if win.state.state != WindowState::Normal && win.state.state != WindowState::Minimized {
            win.window.set_decorations(win.state.chrome_visible);
            let _ = win.set_state(win.state.clone());

            // Prevents a false resize event that would have blocked
            // the process while waiting a second frame.
            win.prev_size = win.window.inner_size().to_px();
        }

        if win.state.state == WindowState::Normal && req.default_position {
            // system position.
            win.state.restore_rect.origin = win.window.inner_position().unwrap_or_default().to_px().to_dip(win.scale_factor());
        }

        #[cfg(windows)]
        if win.state.state != WindowState::Normal {
            win.windows_set_restore();
        }

        win.set_cursor(req.cursor);
        win.set_taskbar_visible(req.taskbar_visible);
        win
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn monitor(&self) -> Option<glutin::monitor::MonitorHandle> {
        self.window.current_monitor()
    }

    pub fn window_id(&self) -> glutin::window::WindowId {
        self.window.id()
    }

    pub fn id_namespace(&self) -> IdNamespace {
        self.api.get_namespace_id()
    }

    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Root document ID.
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }

    /// Latest rendered frame.
    pub fn frame_id(&self) -> FrameId {
        self.rendered_frame_id
    }

    pub fn set_title(&self, title: String) {
        self.window.set_title(&title);
    }

    /// Returns `true` if the cursor actually moved.
    pub fn cursor_moved(&mut self, pos: DipPoint, device: DeviceId) -> bool {
        let moved = self.cursor_pos != pos || self.cursor_device != device;

        if moved {
            self.cursor_pos = pos;
            self.cursor_device = device;
        }

        moved && self.cursor_over
    }

    /// Returns `true` if the previous focused status is different from `focused`.
    ///
    /// Sets the `focused` to if the window is actually the foreground keyboard focused window.
    pub fn focused_changed(&mut self, focused: &mut bool) -> bool {
        #[cfg(windows)]
        {
            use glutin::platform::windows::WindowExtWindows;

            let foreground = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
            *focused = foreground.0 == self.window.hwnd() as _;
        }

        let focused = *focused;

        let changed = self.focused != focused;
        if changed {
            self.focused = focused;
        }
        changed
    }

    /// Returns the last cursor moved data.
    pub fn last_cursor_pos(&self) -> (DipPoint, DeviceId) {
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
        let changed = self.cursor_over;
        self.cursor_over = false;
        changed
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
        self.window.set_always_on_top(always_on_top);
    }

    pub fn set_movable(&mut self, movable: bool) {
        self.movable = movable;
    }

    pub fn set_resizable(&mut self, resizable: bool) {
        self.window.set_resizable(resizable)
    }

    pub fn set_parent(&mut self, parent: Option<glutin::window::WindowId>, modal: bool) {
        todo!("implement parent & modal: {:?}", (parent, modal));
    }

    /// Returns `Some(new_pos)` if the window position is different from the previous call to this function.
    pub fn moved(&mut self) -> Option<DipPoint> {
        if !self.visible {
            return None;
        }

        let new_pos = self.window.inner_position().unwrap().to_px();
        if self.prev_pos != new_pos {
            self.prev_pos = new_pos;

            Some(new_pos.to_dip(self.scale_factor()))
        } else {
            None
        }
    }

    /// Returns `Some(new_size)` if the window size is different from the previous call to this function.
    pub fn resized(&mut self) -> Option<DipSize> {
        if !self.visible {
            return None;
        }

        let new_size = self.window.inner_size().to_px();
        if self.prev_size != new_size {
            self.prev_size = new_size;
            self.resized = true;

            Some(new_size.to_dip(self.scale_factor()))
        } else {
            None
        }
    }

    /// Returns `Some(new_monitor)` if the parent monitor changed from the previous call to this function.
    pub fn monitor_change(&mut self) -> Option<MonitorHandle> {
        let handle = self.window.current_monitor();
        if self.prev_monitor != handle {
            self.prev_monitor = handle.clone();
            handle
        } else {
            None
        }
    }

    #[cfg(windows)]
    fn windows_set_restore(&self) {
        use glutin::platform::windows::{MonitorHandleExtWindows, WindowExtWindows};
        use windows::Win32::Foundation::{BOOL, HWND};
        use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO, MONITORINFOEXW};
        use windows::Win32::{
            Foundation::{POINT, RECT},
            UI::WindowsAndMessaging::*,
        };

        if let Some(monitor) = self.window.current_monitor() {
            let hwnd = HWND(self.window.hwnd() as _);
            let mut placement = WINDOWPLACEMENT {
                length: mem::size_of::<WINDOWPLACEMENT>() as _,
                flags: WINDOWPLACEMENT_FLAGS(0),
                showCmd: SHOW_WINDOW_CMD(0),
                ptMinPosition: POINT { x: 0, y: 0 },
                ptMaxPosition: POINT { x: 0, y: 0 },
                rcNormalPosition: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
            };
            if unsafe { GetWindowPlacement(hwnd, &mut placement) } != BOOL(0) {
                let scale_factor = self.scale_factor();
                let mut left_top = self.state.restore_rect.origin.to_px(scale_factor);

                // placement is in "workspace", window is in "virtual screen space".
                let hmonitor = HMONITOR(monitor.hmonitor() as _);
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
                if unsafe { GetMonitorInfoW(hmonitor, &mut monitor_info as *mut MONITORINFOEXW as *mut MONITORINFO) } != BOOL(0) {
                    left_top.x.0 -= monitor_info.monitorInfo.rcWork.left;
                    left_top.y.0 -= monitor_info.monitorInfo.rcWork.top;
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

    /// Set cursor icon and visibility.
    pub fn set_cursor(&mut self, icon: Option<CursorIcon>) {
        if let Some(icon) = icon {
            self.window.set_cursor_icon(icon.to_winit());
            self.window.set_cursor_visible(true);
        } else {
            self.window.set_cursor_visible(false);
        }
    }

    /// Sets the focus request indicator.
    pub fn set_focus_request(&mut self, request: Option<FocusRequest>) {
        if self.waiting_first_frame {
            self.init_focus_request = request;
        } else {
            self.window.request_user_attention(request.map(|r| match r {
                FocusRequest::Critical => glutin::window::UserAttentionType::Critical,
                FocusRequest::Info => glutin::window::UserAttentionType::Informational,
            }));
        }
    }

    /// Steal input focus.
    pub fn focus(&mut self) {
        if self.waiting_first_frame {
            self.steal_init_focus = true;
        } else {
            self.window.focus_window();
        }
    }

    /// Gets the current Maximized status as early as possible.
    fn is_maximized(&self) -> bool {
        #[cfg(windows)]
        {
            let hwnd = glutin::platform::windows::WindowExtWindows::hwnd(&self.window);
            // SAFETY: function does not fail.
            return unsafe { windows::Win32::UI::WindowsAndMessaging::IsZoomed(windows::Win32::Foundation::HWND(hwnd as _)) }
                != windows::Win32::Foundation::BOOL(0);
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
            let hwnd = glutin::platform::windows::WindowExtWindows::hwnd(&self.window);
            // SAFETY: function does not fail.
            return unsafe { windows::Win32::UI::WindowsAndMessaging::IsIconic(windows::Win32::Foundation::HWND(hwnd as _)) }
                != windows::Win32::Foundation::BOOL(0);
        }

        #[allow(unreachable_code)]
        false
    }

    fn probe_state(&self) -> WindowStateAll {
        let mut state = self.state.clone();

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

            state.restore_rect = DipRect::new(
                self.window.inner_position().unwrap().to_px().to_dip(scale_factor),
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
            self.window.set_inner_size(new_state.restore_rect.size.to_winit());

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
                    && m.refresh_rate() <= mode.refresh_rate
                {
                    // select closest match to the requested video mode
                    if let Some(c) = &candidate {
                        if m.size().width >= c.size().width
                            && m.size().height >= c.size().height as u32
                            && m.bit_depth() >= c.bit_depth()
                            && m.refresh_rate() >= c.refresh_rate()
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
        tracing::error!("`set_taskbar_visible` not implemented for this OS");
    }

    #[cfg(windows)]
    pub fn set_taskbar_visible(&mut self, visible: bool) {
        if visible == self.taskbar_visible {
            return;
        }
        self.taskbar_visible = visible;

        use glutin::platform::windows::WindowExtWindows;
        use windows::Win32::Foundation::*;
        use windows::Win32::System::Com::*;
        use windows::Win32::UI::Shell::*;

        // winit already initializes COM

        unsafe {
            let result: Result<ITaskbarList, _> = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER);
            match result {
                Ok(tb) => {
                    let result = if visible {
                        tb.AddTab(HWND(self.window.hwnd() as _))
                    } else {
                        tb.DeleteTab(HWND(self.window.hwnd() as _))
                    };
                    match result {
                        Ok(_) => {}
                        Err(error) => {
                            let mtd_name = if visible { "AddTab" } else { "DeleteTab" };
                            tracing::error!(
                                target: "window",
                                "cannot set `taskbar_visible`, `ITaskbarList::{mtd_name}` failed, error: {error}",
                            )
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        target: "window",
                        "cannot set `taskbar_visible`, failed to create instance of `ITaskbarList`, error: {e}",
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
        let outer_pos = self.window.outer_position().unwrap_or_default();
        let inner_pos = self.window.inner_position().unwrap_or_default();
        let inner_offset = PxVector::new(Px(outer_pos.x - inner_pos.x), Px(outer_pos.y - inner_pos.y)).to_dip(self.scale_factor());
        let pos = pos + inner_offset;
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
                    if !new_state.state.is_fullscreen() {
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
            self.set_inner_position(self.state.restore_rect.origin);
            self.window.set_inner_size(self.state.restore_rect.size.to_winit());

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

    pub fn use_image(&mut self, image: &Image) -> ImageKey {
        self.image_use.new_use(image, self.document_id, &mut self.api)
    }

    pub fn update_image(&mut self, key: ImageKey, image: &Image) {
        self.image_use.update_use(key, image, self.document_id, &mut self.api);
    }

    pub fn delete_image(&mut self, key: ImageKey) {
        self.image_use.delete(key, self.document_id, &mut self.api);
    }

    pub fn add_font(&mut self, font: Vec<u8>, index: u32) -> FontKey {
        let key = self.api.generate_font_key();
        let mut txn = webrender::Transaction::new();
        txn.add_raw_font(key, font, index);
        self.api.send_transaction(self.document_id, txn);
        key
    }

    pub fn delete_font(&mut self, key: FontKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font(key);
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn add_font_instance(
        &mut self,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<FontInstanceOptions>,
        plataform_options: Option<FontInstancePlatformOptions>,
        variations: Vec<FontVariation>,
    ) -> FontInstanceKey {
        let key = self.api.generate_font_instance_key();
        let mut txn = webrender::Transaction::new();
        txn.add_font_instance(key, font_key, glyph_size.to_wr().get(), options, plataform_options, variations);
        self.api.send_transaction(self.document_id, txn);
        key
    }

    pub fn delete_font_instance(&mut self, instance_key: FontInstanceKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font_instance(instance_key);
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn set_allow_alt_f4(&mut self, allow: bool) {
        self.allow_alt_f4.set(allow);
    }

    pub fn set_capture_mode(&mut self, enabled: bool) {
        self.capture_mode = enabled;
    }

    /// Start rendering a new frame.
    ///
    /// The [callback](#callback) will be called when the frame is ready to be [presented](Self::present).
    pub fn render(&mut self, frame: FrameRequest) {
        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color);

        let size = self.window.inner_size();
        let viewport_size = size.to_px().to_wr();

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        self.push_resize(&mut txn);
        txn.generate_frame(frame.id.get(), frame.render_reasons());

        let display_list = BuiltDisplayList::from_data(
            DisplayListPayload {
                items_data: frame.display_list.0.to_vec(),
                cache_data: frame.display_list.1.to_vec(),
                spatial_tree: frame.display_list.2.to_vec(),
            },
            frame.display_list.3,
        );
        txn.reset_dynamic_properties();
        txn.append_dynamic_properties(DynamicProperties {
            transforms: vec![],
            floats: vec![],
            colors: vec![],
        });

        txn.set_display_list(
            frame.id.epoch(),
            Some(frame.clear_color),
            viewport_size,
            (frame.pipeline_id, display_list),
        );

        for (scroll_id, offset) in frame.scrolls {
            txn.set_scroll_offsets(
                scroll_id,
                vec![SampledScrollOffset {
                    offset: offset.to_wr(),
                    generation: 0,
                }],
            );
        }

        let frame_scope =
            tracing::trace_span!("<frame>", ?frame.id, capture_image = ?frame.capture_image, thread = "<webrender>").entered();

        self.pending_frames.push_back((frame.id, frame.capture_image, Some(frame_scope)));

        self.api.send_transaction(self.document_id, txn);
    }

    /// Start rendering a new frame based on the data of the last frame.
    pub fn render_update(&mut self, frame: FrameUpdateRequest) {
        let render_reasons = frame.render_reasons();

        if let Some(color) = frame.clear_color {
            self.renderer.as_mut().unwrap().set_clear_color(color);
        }

        let mut txn = Transaction::new();

        txn.set_root_pipeline(self.pipeline_id);
        txn.append_dynamic_properties(frame.updates);
        for (scroll_id, offset) in frame.scroll_updates {
            txn.set_scroll_offsets(
                scroll_id,
                vec![SampledScrollOffset {
                    offset: offset.to_wr(),
                    generation: 0,
                }],
            );
        }

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id().get(), render_reasons);
        self.api.send_transaction(self.document_id, txn);
    }

    /// Returns info for `FrameRendered` and if this is the first frame.
    #[must_use = "events must be generated from the result"]
    pub fn on_frame_ready(&mut self, msg: FrameReadyMsg, images: &mut ImageCache) -> FrameReadyResult {
        debug_assert_eq!(self.document_id, msg.document_id);

        let (frame_id, capture, _) = self.pending_frames.pop_front().unwrap_or((self.rendered_frame_id, false, None));
        self.rendered_frame_id = frame_id;

        let first_frame = self.waiting_first_frame;

        if self.waiting_first_frame {
            let _s = tracing::trace_span!("Window.first-draw").entered();
            debug_assert!(msg.composite_needed);

            self.waiting_first_frame = false;
            let s = self.window.inner_size();
            self.context.make_current();
            self.context.resize(s.width as i32, s.height as i32);
            self.redraw();
            if self.kiosk {
                self.window.request_redraw();
            } else if self.visible {
                self.set_visible(true);

                if mem::take(&mut self.steal_init_focus) {
                    self.window.focus_window();
                }
                if let Some(r) = self.init_focus_request.take() {
                    self.set_focus_request(Some(r));
                }
            }
        } else if msg.composite_needed {
            self.window.request_redraw();
        }

        let scale_factor = self.scale_factor();

        let image = if capture {
            let _s = tracing::trace_span!("capture_image").entered();
            if msg.composite_needed {
                self.redraw();
            }
            let renderer = self.renderer.as_mut().unwrap();
            Some(images.frame_image_data(renderer, PxRect::from_size(self.window.inner_size().to_px()), true, scale_factor))
        } else {
            None
        };

        let cursor_hits = if self.cursor_over {
            let pos = self.cursor_pos.to_px(scale_factor);
            (pos, self.hit_tester.hit_test(pos))
        } else {
            (PxPoint::new(Px(-1), Px(-1)), HitTestResult::default())
        };

        FrameReadyResult {
            frame_id,
            image,
            cursor_hits,
            first_frame,
        }
    }

    pub fn redraw(&mut self) {
        let _s = tracing::trace_span!("Window.redraw").entered();

        self.context.make_current();

        let renderer = self.renderer.as_mut().unwrap();
        renderer.update();
        let s = self.window.inner_size();
        renderer.render(s.to_px().to_wr_device(), 0).unwrap();
        let _ = renderer.flush_pipeline_info();

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
            self.context.resize(size.width as i32, size.height as i32);

            let size = self.window.inner_size();
            txn.set_document_view(PxRect::from_size(size.to_px()).to_wr_device());
        }
    }

    pub fn frame_image(&mut self, images: &mut ImageCache) -> ImageId {
        let scale_factor = self.scale_factor();
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            PxRect::from_size(self.window.inner_size().to_px()),
            self.capture_mode,
            self.id,
            self.rendered_frame_id,
            scale_factor,
        )
    }

    pub fn frame_image_rect(&mut self, images: &mut ImageCache, rect: PxRect) -> ImageId {
        // TODO check any frame rendered
        let scale_factor = self.scale_factor();
        let rect = PxRect::from_size(self.window.inner_size().to_px())
            .intersection(&rect)
            .unwrap_or_default();
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            rect,
            self.capture_mode,
            self.id,
            self.rendered_frame_id,
            scale_factor,
        )
    }

    pub fn inner_position(&self) -> DipPoint {
        self.window
            .inner_position()
            .unwrap_or_default()
            .to_logical(self.window.scale_factor())
            .to_dip()
    }

    pub fn size(&self) -> DipSize {
        self.window.inner_size().to_logical(self.window.scale_factor()).to_dip()
    }

    pub fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    /// Does a hit-test on the current frame.
    ///
    /// Returns all hits from front-to-back.
    pub fn hit_test(&mut self, point: DipPoint) -> (FrameId, PxPoint, HitTestResult) {
        let _p = tracing::trace_span!("hit_test").entered();
        let point = point.to_px(self.scale_factor());
        (self.rendered_frame_id, point, self.hit_tester.hit_test(point))
    }

    /// Window actual render mode.
    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        self.api.stop_render_backend();
        self.api.shut_down(true);

        // webrender deinit panics if the context is not current.
        self.context.make_current();
        self.renderer.take().unwrap().deinit();

        // context must be dropped before the winit window (glutin requirement).
        self.context.drop_before_winit();

        // the winit window will be dropped normally after this.
    }
}

pub(crate) struct FrameReadyResult {
    pub frame_id: FrameId,
    pub image: Option<ImageLoadedData>,
    pub cursor_hits: (PxPoint, HitTestResult),
    pub first_frame: bool,
}
