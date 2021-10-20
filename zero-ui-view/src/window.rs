use std::{
    cell::Cell,
    collections::VecDeque,
    fmt,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use gleam::gl;
use glutin::{
    event_loop::EventLoopWindowTarget,
    monitor::VideoMode as GVideoMode,
    window::{Fullscreen, Icon, Window as GWindow, WindowBuilder},
    ContextBuilder, CreationError, GlRequest,
};
use webrender::{
    api::{
        BuiltDisplayList, DisplayListPayload, DocumentId, FontInstanceKey, FontInstanceOptions, FontInstancePlatformOptions, FontKey,
        FontVariation, HitTestResult, IdNamespace, ImageKey, PipelineId, RenderNotifier,
    },
    RenderApi, Renderer, RendererOptions, Transaction,
};
use zero_ui_view_api::{
    units::{PxToDip, *},
    Event, FrameId, FrameRequest, FrameUpdateRequest, ImageId, ImageLoadedData, Key, KeyState, ScanCode, TextAntiAliasing, VideoMode,
    ViewProcessGen, WindowId, WindowRequest, WindowState,
};

use crate::{
    config,
    image_cache::{Image, ImageCache, ImageUseMap, WrImageCache},
    util::{self, DipToWinit, GlContext, GlContextManager, WinitToDip, WinitToPx},
    AppEvent, AppEventSender, FrameReadyMsg,
};

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

    redirect_frame: Arc<AtomicBool>,
    redirect_frame_recv: flume::Receiver<FrameReadyMsg>,

    pending_frames: VecDeque<(FrameId, bool)>,
    rendered_frame_id: FrameId,

    resized: bool,

    video_mode: VideoMode,

    prev_pos: DipPoint,
    prev_size: DipSize,
    state: WindowState,

    visible: bool,
    waiting_first_frame: bool,

    allow_alt_f4: Rc<Cell<bool>>,
    taskbar_visible: bool,

    movable: bool, // TODO
    transparent: bool,

    cursor_pos: PxPoint,
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
        cfg: WindowRequest,
        window_target: &EventLoopWindowTarget<AppEvent>,
        gl_manager: &mut GlContextManager,
        event_sender: impl AppEventSender,
    ) -> Self {
        let id = cfg.id;

        // create window and OpenGL context
        let mut winit = WindowBuilder::new()
            .with_title(cfg.title)
            .with_inner_size(cfg.size.to_winit())
            .with_resizable(cfg.resizable)
            .with_transparent(cfg.transparent)
            .with_min_inner_size(cfg.min_size.to_winit())
            .with_max_inner_size(cfg.max_size.to_winit())
            .with_always_on_top(cfg.always_on_top)
            .with_window_icon(icon);

        if let WindowState::Normal | WindowState::Minimized = cfg.state {
            winit = winit
                .with_decorations(cfg.chrome_visible)
                // we wait for the first frame to show the window,
                // so that there is no white frame when it's opening.
                .with_visible(false);
        } else {
            // Maximized/Fullscreen Flickering Workaround Part 1
            // 
            // TODO: explain the problem this workaround is solving.
            winit = winit.with_decorations(false);
        }

        if let Some(pos) = cfg.pos {
            winit = winit.with_position(pos.to_winit());
        }

        winit = match cfg.state {
            WindowState::Normal | WindowState::Minimized => winit,
            WindowState::Maximized => winit.with_maximized(true),
            WindowState::Fullscreen | WindowState::Exclusive => winit.with_fullscreen(Some(Fullscreen::Borderless(None))),
        };

        let glutin = match ContextBuilder::new()
            .with_hardware_acceleration(None)
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(winit, window_target)
        {
            Ok(c) => c,
            Err(
                CreationError::NoAvailablePixelFormat | CreationError::NoBackendAvailable(_) | CreationError::OpenGlVersionNotSupported,
            ) => {
                panic!("software rendering is not implemented");
            }
            Err(e) => panic!("failed to create OpenGL context, {:?}", e),
        };
        // SAFETY: we drop the context before the window (or panic if we don't).
        let (context, winit_window) = unsafe { glutin.split() };
        let mut context = gl_manager.manage_headed(id, context);

        // extend the winit Windows window to only block the Alt+F4 key press if we want it to.
        let allow_alt_f4 = Rc::new(Cell::new(cfg.allow_alt_f4));
        #[cfg(windows)]
        {
            let allow_alt_f4 = allow_alt_f4.clone();
            let event_sender = event_sender.clone();

            util::set_raw_windows_event_handler(&winit_window, u32::from_ne_bytes(*b"alf4") as _, move |_, msg, wparam, _| {
                if msg == winapi::um::winuser::WM_SYSKEYDOWN && wparam as i32 == winapi::um::winuser::VK_F4 && allow_alt_f4.get() {
                    let device = 0; // TODO recover actual ID

                    let _ = event_sender.send(AppEvent::Notify(Event::KeyboardInput {
                        window: id,
                        device,
                        scan_code: wparam as ScanCode,
                        state: KeyState::Pressed,
                        key: Some(Key::F4),
                    }));
                    return Some(0);
                }
                None
            });
        }

        // create renderer and start the first frame.
        let gl_ctx = context.make_current();

        let gl = match gl_ctx.get_api() {
            glutin::Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| gl_ctx.get_proc_address(symbol) as *const _) },
            glutin::Api::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| gl_ctx.get_proc_address(symbol) as *const _) },
            glutin::Api::WebGl => panic!("WebGl is not supported"),
        };

        let device_size = winit_window.inner_size().to_px().to_wr_device();

        let mut text_aa = cfg.text_aa;
        if let TextAntiAliasing::Default = cfg.text_aa {
            text_aa = config::text_aa();
        }

        let opts = RendererOptions {
            enable_aa: text_aa != TextAntiAliasing::Mono,
            enable_subpixel_aa: text_aa == TextAntiAliasing::Subpixel,
            renderer_id: Some((gen as u64) << 32 | id as u64),
            //panic_on_gl_error: true,
            // TODO expose more options to the user.
            ..Default::default()
        };

        let redirect_frame = Arc::new(AtomicBool::new(false));
        let (rf_sender, redirect_frame_recv) = flume::unbounded();

        let (mut renderer, sender) = webrender::Renderer::new(
            gl,
            Box::new(Notifier {
                window_id: id,
                sender: event_sender,
                redirect: redirect_frame.clone(),
                redirect_sender: rf_sender,
            }),
            opts,
            None,
        )
        .unwrap();
        renderer.set_external_image_handler(WrImageCache::new_boxed());

        let api = sender.create_api();
        let document_id = api.add_document(device_size);

        let pipeline_id = webrender::api::PipelineId(gen, id);

        let scale_factor = winit_window.scale_factor() as f32;

        let mut win = Self {
            id,
            image_use: ImageUseMap::default(),
            prev_pos: winit_window.outer_position().unwrap_or_default().to_px().to_dip(scale_factor),
            prev_size: winit_window.inner_size().to_px().to_dip(scale_factor),
            window: winit_window,
            context,
            capture_mode: cfg.capture_mode,
            renderer: Some(renderer),
            redirect_frame,
            redirect_frame_recv,
            video_mode: cfg.video_mode,
            api,
            document_id,
            pipeline_id,
            resized: true,
            waiting_first_frame: true,
            visible: cfg.visible,
            allow_alt_f4,
            taskbar_visible: true,
            movable: cfg.movable,
            transparent: cfg.transparent,
            pending_frames: VecDeque::new(),
            rendered_frame_id: FrameId::INVALID,
            state: cfg.state,
            cursor_pos: PxPoint::zero(),
        };

        // Maximized/Fullscreen Flickering Workaround Part 2
        if cfg.state != WindowState::Normal && cfg.state != WindowState::Minimized {
            win.window.set_decorations(cfg.chrome_visible);
            let _ = win.set_state(cfg.state);

            // Prevents a false resize event that would have blocked
            // the process while waiting a second frame.
            win.prev_size = win.window.inner_size().to_px().to_dip(scale_factor);
        }

        win.set_taskbar_visible(cfg.taskbar_visible);

        win
    }

    pub fn id(&self) -> WindowId {
        self.id
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

    /// Latest rendered frame.
    pub fn frame_id(&self) -> FrameId {
        self.rendered_frame_id
    }

    pub fn set_title(&self, title: String) {
        self.window.set_title(&title);
    }

    pub fn set_cursor_pos(&mut self, pos: PxPoint) {
        self.cursor_pos = pos;
    }

    pub fn set_visible(&mut self, visible: bool) {
        if !self.waiting_first_frame {
            self.visible = visible;
            if visible {
                self.window.set_visible(true);
                let _ = self.set_state(self.state);
            } else {
                self.window.set_fullscreen(None);
                self.window.set_maximized(false);
                self.window.set_minimized(false);
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

    pub fn set_transparent(&mut self, transparent: bool) {
        if self.transparent != transparent {
            self.transparent = transparent;
            todo!("respawn just the window?")
        }
    }

    pub fn set_chrome_visible(&mut self, visible: bool) {
        self.window.set_decorations(visible);
    }

    /// Returns `true` if the `new_pos` is actually different then the previous or init position.
    pub fn moved(&mut self, new_pos: DipPoint) -> bool {
        let moved = self.prev_pos != new_pos;
        self.prev_pos = new_pos;
        moved
    }

    /// Returns `true` if the `new_size` is actually different then the previous or init size.
    pub fn resized(&mut self, new_size: DipSize) -> bool {
        let resized = self.prev_size != new_size;
        self.prev_size = new_size;
        resized
    }

    /// window.inner_size maybe new.
    pub fn on_resized(&mut self) {
        let ctx = self.context.make_current();
        ctx.resize(self.window.inner_size());
        self.resized = true;
    }

    /// Move window, returns `true` if actually moved.
    #[must_use = "must send an event if the return is `true`"]
    pub fn set_outer_pos(&mut self, pos: DipPoint) -> bool {
        let moved = self.moved(pos);
        if moved {
            let new_pos = pos.to_winit();
            self.window.set_outer_position(new_pos);
        }
        moved
    }

    /// Resize window, returns `Some(new_size)` if actually resized and `Some(<frame>)` if the frame rendered within 300m.
    #[must_use = "must send an event if the return is `Some(_)`"]
    #[allow(clippy::type_complexity)]
    pub fn set_inner_size<S: AppEventSender>(
        &mut self,
        size: DipSize,
        frame: FrameRequest,
        images: &mut ImageCache<S>,
    ) -> Option<(DipSize, Option<(FrameId, Option<ImageLoadedData>, HitTestResult)>)> {
        if self.resized(size) {
            let new_size = size.to_winit();
            self.render(frame);
            let render = self.wait_frame_ready(Instant::now() + Duration::from_millis(300), images);

            self.window.set_inner_size(new_size);
            self.on_resized();

            if render.is_some() {
                self.redraw();
            }
            Some((self.size(), render))
        } else {
            None
        }
    }

    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.window.set_window_icon(icon);
    }

    /// Probe state, returns `Some(new_state)`
    pub fn state_change(&mut self) -> Option<WindowState> {
        if !self.visible {
            return None;
        }

        let state = if self.window.inner_size().width == 0 {
            WindowState::Minimized
        } else if let Some(h) = self.window.fullscreen() {
            match h {
                Fullscreen::Exclusive(_) => WindowState::Exclusive,
                Fullscreen::Borderless(_) => WindowState::Fullscreen,
            }
        } else if self.window.is_maximized() {
            WindowState::Maximized
        } else {
            WindowState::Normal
        };

        if self.state != state {
            self.state = state;
            Some(state)
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
        if let WindowState::Exclusive = self.state {
            if let Some(mode) = self.video_mode() {
                self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
            } else {
                log::error!("failed to determinate exclusive video mode, will use windowed fullscreen");
                self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }
    }

    /// Apply the new state, returns `true` if the state changed.
    #[must_use = "must send an event if the return is `true`"]
    pub fn set_state(&mut self, state: WindowState) -> bool {
        if !self.visible {
            // will apply when set to visible.
            self.state = state;
            return false;
        }

        if state.is_fullscreen() {
            self.window.set_minimized(false);

            match state {
                WindowState::Fullscreen => self.window.set_fullscreen(Some(Fullscreen::Borderless(None))),
                WindowState::Exclusive => {
                    if let Some(mode) = self.video_mode() {
                        self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
                    } else {
                        log::error!("failed to determinate exclusive video mode, will use windowed fullscreen");
                        self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }
                }
                _ => unreachable!(),
            }
        } else if let WindowState::Maximized = state {
            self.window.set_minimized(false);
            self.window.set_fullscreen(None);
            self.window.set_maximized(true);
        } else if let WindowState::Normal = state {
            self.window.set_minimized(false);
            self.window.set_fullscreen(None);
            self.window.set_maximized(false);
        } else {
            self.window.set_minimized(true);
        }

        if let Some(s) = self.state_change() {
            debug_assert_eq!(s, state);
            true
        } else {
            false
        }
    }

    #[cfg(windows)]
    pub fn set_taskbar_visible(&mut self, visible: bool) {
        if visible == self.taskbar_visible {
            return;
        }
        self.taskbar_visible = visible;

        use glutin::platform::windows::WindowExtWindows;
        use std::ptr;
        use winapi::shared::winerror;
        use winapi::um::combaseapi;
        use winapi::um::shobjidl_core::ITaskbarList;
        use winapi::Interface;

        // winit already initializes COM

        unsafe {
            let mut tb_ptr: *mut ITaskbarList = ptr::null_mut();
            let result = combaseapi::CoCreateInstance(
                &winapi::um::shobjidl_core::CLSID_TaskbarList,
                ptr::null_mut(),
                winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER,
                &ITaskbarList::uuidof(),
                &mut tb_ptr as *mut _ as *mut _,
            );
            match result {
                winerror::S_OK => {
                    let tb = tb_ptr.as_ref().unwrap();
                    let result = if visible {
                        tb.AddTab(self.window.hwnd() as winapi::shared::windef::HWND)
                    } else {
                        tb.DeleteTab(self.window.hwnd() as winapi::shared::windef::HWND)
                    };
                    match result {
                        winerror::S_OK => {}
                        error => {
                            let mtd_name = if visible { "AddTab" } else { "DeleteTab" };
                            log::error!(
                                target: "window",
                                "cannot set `taskbar_visible`, `ITaskbarList::{}` failed, error: {:X}",
                                mtd_name,
                                error
                            )
                        }
                    }
                    tb.Release();
                }
                error => {
                    log::error!(
                        target: "window",
                        "cannot set `taskbar_visible`, failed to create instance of `ITaskbarList`, error: {:X}",
                        error
                    )
                }
            }
        }
    }

    pub fn set_min_inner_size(&mut self, min_size: DipSize) {
        self.window.set_min_inner_size(Some(min_size.to_winit()))
    }

    pub fn set_max_inner_size(&mut self, max_size: DipSize) {
        self.window.set_max_inner_size(Some(max_size.to_winit()))
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

    pub fn set_text_aa(&mut self, aa: TextAntiAliasing) {
        todo!("need to rebuild the renderer? {:?}", aa)
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
        self.pending_frames.push_back((frame.id, frame.capture_image));
        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color);

        let size = self.window.inner_size();
        let viewport_size = size.to_px().to_wr();

        let mut txn = Transaction::new();
        let display_list = BuiltDisplayList::from_data(
            DisplayListPayload {
                data: frame.display_list.0.into_vec(),
            },
            frame.display_list.1,
        );
        txn.set_display_list(
            frame.id.epoch(),
            Some(frame.clear_color),
            viewport_size,
            (frame.pipeline_id, display_list),
            true,
        );
        txn.set_root_pipeline(self.pipeline_id);

        self.push_resize(&mut txn);

        txn.generate_frame(frame.id.get());
        self.api.send_transaction(self.document_id, txn);
    }

    /// Start rendering a new frame based on the data of the last frame.
    pub fn render_update(&mut self, frame: FrameUpdateRequest) {
        if let Some(color) = frame.clear_color {
            self.renderer.as_mut().unwrap().set_clear_color(color);
        }

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        txn.update_dynamic_properties(frame.updates);

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id().get());
        self.api.send_transaction(self.document_id, txn);
    }

    /// Returns if it is the first frame.
    #[must_use = "if `true` must notify the initial Resized event"]
    pub fn on_frame_ready<S: AppEventSender>(
        &mut self,
        msg: FrameReadyMsg,
        images: &mut ImageCache<S>,
    ) -> (FrameId, Option<ImageLoadedData>, HitTestResult) {
        debug_assert_eq!(self.document_id, msg.document_id);

        let (frame_id, capture) = self.pending_frames.pop_front().unwrap_or_else(|| {
            debug_assert!(!msg.composite_needed);
            (self.rendered_frame_id, false)
        });
        self.rendered_frame_id = frame_id;

        if self.waiting_first_frame {
            debug_assert!(msg.composite_needed);

            self.waiting_first_frame = false;
            self.redraw();
            if self.visible {
                self.set_visible(true);
            }
        } else if msg.composite_needed {
            self.window.request_redraw();
        }

        let data = if capture {
            if msg.composite_needed {
                self.redraw();
            }
            let scale_factor = self.scale_factor();
            let renderer = self.renderer.as_mut().unwrap();
            Some(images.frame_image_data(renderer, PxRect::from_size(self.window.inner_size().to_px()), true, scale_factor))
        } else {
            None
        };

        let (_hits_frame_id, hits) = self.hit_test(self.cursor_pos);
        debug_assert_eq!(_hits_frame_id, frame_id);

        (frame_id, data, hits)
    }

    pub fn redraw(&mut self) {
        let ctx = self.context.make_current();
        let renderer = self.renderer.as_mut().unwrap();
        renderer.update();
        let s = self.window.inner_size();
        renderer.render(s.to_px().to_wr_device(), 0).unwrap();
        let _ = renderer.flush_pipeline_info();
        ctx.swap_buffers().unwrap();
    }

    /// Capture the next frame-ready event.
    ///
    /// Returns `Some` if received before `deadline`, if `Some` already redraw too.
    pub fn wait_frame_ready<S: AppEventSender>(
        &mut self,
        deadline: Instant,
        images: &mut ImageCache<S>,
    ) -> Option<(FrameId, Option<ImageLoadedData>, HitTestResult)> {
        self.redirect_frame.store(true, Ordering::Relaxed);
        let stop_redirect = util::RunOnDrop::new(|| self.redirect_frame.store(false, Ordering::Relaxed));

        let received = self.redirect_frame_recv.recv_deadline(deadline);

        drop(stop_redirect);

        if let Ok(msg) = received {
            Some(self.on_frame_ready(msg, images))
        } else {
            None
        }
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;
            let size = self.window.inner_size();
            txn.set_document_view(PxRect::from_size(size.to_px()).to_wr_device());
        }
    }

    pub fn frame_image<S: AppEventSender>(&mut self, images: &mut ImageCache<S>) -> ImageId {
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

    pub fn frame_image_rect<S: AppEventSender>(&mut self, images: &mut ImageCache<S>, rect: PxRect) -> ImageId {
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

    pub fn outer_position(&self) -> DipPoint {
        self.window
            .outer_position()
            .ok()
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
    pub fn hit_test(&self, point: PxPoint) -> (FrameId, HitTestResult) {
        (
            self.rendered_frame_id,
            self.api.hit_test(self.document_id, Some(self.pipeline_id), point.to_wr_world()),
        )
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        // webrender deinit panics if the context is not current.
        let _ctx = self.context.make_current();
        self.renderer.take().unwrap().deinit();

        // context must be dropped before the winit window (glutin requirement).
        self.context.drop_before_winit();

        // the winit window will be dropped normally after this.
    }
}

struct Notifier<S> {
    window_id: WindowId,
    sender: S,
    redirect: Arc<AtomicBool>,
    redirect_sender: flume::Sender<FrameReadyMsg>,
}
impl<S: AppEventSender> RenderNotifier for Notifier<S> {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            window_id: self.window_id,
            sender: self.sender.clone(),
            redirect: self.redirect.clone(),
            redirect_sender: self.redirect_sender.clone(),
        })
    }

    fn wake_up(&self, _: bool) {}

    fn new_frame_ready(&self, document_id: DocumentId, _scrolled: bool, composite_needed: bool, _render_time_ns: Option<u64>) {
        let msg = FrameReadyMsg {
            document_id,
            composite_needed,
        };
        if self.redirect.load(Ordering::Relaxed) {
            let _ = self.redirect_sender.send(msg);
        } else {
            let _ = self.sender.send(AppEvent::FrameReady(self.window_id, msg));
        }
    }
}
