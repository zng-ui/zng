use std::{
    cell::Cell,
    collections::VecDeque,
    fmt, mem,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use glutin::{
    event_loop::EventLoopWindowTarget,
    monitor::VideoMode as GVideoMode,
    window::{Fullscreen, Icon, Window as GWindow, WindowBuilder},
    ContextBuilder, CreationError, GlRequest,
};
use tracing::span::EnteredSpan;
use webrender::{
    api::{
        ApiHitTester, BuiltDisplayList, DisplayListPayload, DocumentId, DynamicProperties, FontInstanceKey, FontInstanceOptions,
        FontInstancePlatformOptions, FontKey, FontVariation, HitTestResult, HitTesterRequest, IdNamespace, ImageKey, PipelineId,
        RenderNotifier, ScrollClamping,
    },
    RenderApi, Renderer, RendererOptions, Transaction,
};
use zero_ui_view_api::{
    units::*, CursorIcon, Event, FrameId, FrameRequest, FrameUpdateRequest, HeadlessOpenData, ImageId, ImageLoadedData, Key, KeyState,
    ScanCode, TextAntiAliasing, VideoMode, ViewProcessGen, WindowId, WindowRequest, WindowState,
};

use crate::{
    config,
    image_cache::{Image, ImageCache, ImageUseMap, WrImageCache},
    util::{self, CursorToWinit, DipToWinit, GlContext, GlContextManager, WinitToDip, WinitToPx},
    AppEvent, AppEventSender, FrameReadyMsg,
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
    documents: Vec<DocumentId>,
    api: RenderApi,
    image_use: ImageUseMap,

    window: GWindow,
    context: GlContext,
    renderer: Option<Renderer>,
    capture_mode: bool,

    redirect_frame: Arc<AtomicBool>,
    redirect_frame_recv: flume::Receiver<FrameReadyMsg>,

    pending_frames: VecDeque<(FrameId, bool, Option<EnteredSpan>)>,
    rendered_frame_id: FrameId,

    resized: bool,

    video_mode: VideoMode,

    prev_pos: PxPoint,
    prev_size: PxSize,
    state: WindowState,

    visible: bool,
    waiting_first_frame: bool,

    allow_alt_f4: Rc<Cell<bool>>,
    taskbar_visible: bool,

    movable: bool, // TODO

    cursor_pos: PxPoint,
    hit_tester: HitTester,
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

        let glutin_scope = tracing::trace_span!("open/glutin").entered();

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
            // We can't start maximized or fullscreen with visible=false because
            // that causes a white rectangle over a black background to flash on open.
            // The white rectangle is probably the window in Normal mode, not sure if its caused by winit or glutin.
            //
            // For some reason disabling the window chrome, then enabling it again after opening the window removes
            // the white rectangle, the black background still flashes when transparent=false, but at least its just
            // a solid fill.
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

        #[cfg(software)]
        let context = gl_manager.manage_headed(id, context, None);
        #[cfg(not(software))]
        let context = gl_manager.manage_headed(id, context);

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

        drop(glutin_scope);
        let wr_scope = tracing::trace_span!("open/webrender").entered();

        // create renderer and start the first frame.

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
            context.share_gl(),
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

        drop(wr_scope);

        let pipeline_id = webrender::api::PipelineId(gen, id);

        let hit_tester = HitTester::new(&api, document_id);

        let mut win = Self {
            id,
            image_use: ImageUseMap::default(),
            prev_pos: winit_window.outer_position().unwrap_or_default().to_px(),
            prev_size: winit_window.inner_size().to_px(),
            window: winit_window,
            context,
            capture_mode: cfg.capture_mode,
            renderer: Some(renderer),
            redirect_frame,
            redirect_frame_recv,
            video_mode: cfg.video_mode,
            api,
            document_id,
            documents: vec![],
            pipeline_id,
            resized: true,
            waiting_first_frame: true,
            visible: cfg.visible,
            allow_alt_f4,
            taskbar_visible: true,
            movable: cfg.movable,
            pending_frames: VecDeque::new(),
            rendered_frame_id: FrameId::INVALID,
            state: cfg.state,
            cursor_pos: PxPoint::zero(),
            hit_tester,
        };

        // Maximized/Fullscreen Flickering Workaround Part 2
        if cfg.state != WindowState::Normal && cfg.state != WindowState::Minimized {
            win.window.set_decorations(cfg.chrome_visible);
            let _ = win.set_state(cfg.state);

            // Prevents a false resize event that would have blocked
            // the process while waiting a second frame.
            win.prev_size = win.window.inner_size().to_px();
        }

        win.set_cursor(cfg.cursor);
        win.set_taskbar_visible(cfg.taskbar_visible);
        win
    }

    pub fn open_document(&mut self, scale_factor: f32, initial_size: DipSize) -> HeadlessOpenData {
        let document_id = self.api.add_document(initial_size.to_px(scale_factor).to_wr_device());
        self.documents.push(document_id);
        HeadlessOpenData {
            id_namespace: self.id_namespace(),
            pipeline_id: self.pipeline_id,
            document_id,
        }
    }

    pub fn close_document(&mut self, document_id: DocumentId) {
        if let Some(i) = self.documents.iter().position(|&d| d == document_id) {
            self.documents.swap_remove(i);
            self.api.delete_document(document_id);
        }
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

    pub fn set_cursor_pos(&mut self, pos: PxPoint) {
        self.cursor_pos = pos;
    }

    pub fn set_visible(&mut self, visible: bool) {
        if !self.waiting_first_frame {
            self.visible = visible;
            self.window.set_visible(visible);

            // state changes when not visible only set `self.state`.
            let state = self.state;
            if self.state_change().is_some() {
                self.state = state;
                let _changed = self.set_state(state);
                debug_assert!(_changed);
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

    pub fn set_chrome_visible(&mut self, visible: bool) {
        self.window.set_decorations(visible);
    }

    /// Returns `true` if the `new_pos` is actually different then the previous or init position
    /// and is the current position.
    pub fn moved(&mut self, new_pos: PxPoint) -> bool {
        let moved = self.prev_pos != new_pos && self.window.outer_position().unwrap_or_default().to_px() == new_pos;
        if moved {
            self.prev_pos = new_pos;
        }
        moved
    }

    /// Returns `true` if the `new_size` is actually different then the previous or init size and
    /// is the current size.
    pub fn resized(&mut self, new_size: PxSize) -> bool {
        let resized = self.prev_size != new_size && self.window.inner_size().to_px() == new_size;
        if resized {
            self.prev_size = new_size;
        }
        resized
    }

    /// window.inner_size maybe new.
    pub fn on_resized(&mut self) {
        self.context.make_current();
        self.context.resize(self.window.inner_size());
        self.resized = true;
    }

    /// Move window, returns `true` if actually moved.
    #[must_use = "must send an event if the return is `true`"]
    pub fn set_outer_pos(&mut self, pos: DipPoint) -> bool {
        let pos_px = pos.to_px(self.scale_factor());
        if self.window.outer_position().unwrap_or_default().to_px() != pos_px {
            self.window.set_outer_position(pos.to_winit());
            self.moved(pos_px)
        } else {
            false
        }
    }

    /// Resize window, returns `true` if actually resized
    #[must_use = "must send a resized event if the return true"]
    pub fn set_inner_size(&mut self, size: DipSize) -> bool {
        let size_px = size.to_px(self.scale_factor());

        if self.window.inner_size().to_px() != size_px {
            self.window.set_inner_size(size.to_winit());
            let r = self.resized(size_px);
            self.on_resized();
            r
        } else {
            false
        }
    }

    pub fn set_document_size(&mut self, document_id: DocumentId, size: DipSize, scale_factor: f32) {
        todo!("doc-resize: {:?}", (document_id, size, scale_factor))
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
            if self.state.is_fullscreen() {
                // restore rect becomes the fullscreen size if we don't do this.
                self.window.set_fullscreen(None);
            }

            if let Some(mode) = self.video_mode() {
                self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
            } else {
                tracing::error!("failed to determinate exclusive video mode, will use windowed fullscreen");
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

        if let Some(s) = self.state_change() {
            tracing::error!("window state was out of sync in `set_state`, corrected to `{:?}`", s);
        }

        if self.state == state {
            return false;
        }

        // clear previous state.
        match self.state {
            WindowState::Minimized => self.window.set_minimized(false),
            WindowState::Maximized => self.window.set_maximized(false),
            WindowState::Fullscreen | WindowState::Exclusive => self.window.set_fullscreen(None),
            WindowState::Normal => {}
        }

        // set new state.
        if state.is_fullscreen() {
            match state {
                WindowState::Fullscreen => self.window.set_fullscreen(Some(Fullscreen::Borderless(None))),
                WindowState::Exclusive => {
                    if let Some(mode) = self.video_mode() {
                        self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
                    } else {
                        tracing::error!("failed to determinate exclusive video mode, will use windowed fullscreen");
                        self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }
                }
                _ => unreachable!(),
            }
        } else if let WindowState::Maximized = state {
            self.window.set_maximized(true);
        } else if let WindowState::Minimized = state {
            self.window.set_minimized(true);
        } else {
            debug_assert_eq!(state, WindowState::Normal);
        }

        if let Some(s) = self.state_change() {
            if s != state {
                tracing::error!("window state not set correctly, expected `{:?}` but was `{:?}`", state, s);
            }
            true
        } else {
            tracing::error!("window state did not change, expected `{:?}`", state);
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
                            tracing::error!(
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
                    tracing::error!(
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

        txn.reset_dynamic_properties();

        txn.set_root_pipeline(self.pipeline_id);
        txn.append_dynamic_properties(frame.updates);
        for (scroll_id, offset) in frame.scroll_updates {
            txn.scroll_node_with_id(offset.to_point().to_wr(), scroll_id, ScrollClamping::NoClamping);
        }

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id().get(), render_reasons);
        self.api.send_transaction(self.document_id, txn);
    }

    /// Returns info for `FrameRendered` and if this is the first frame.
    #[must_use = "events must be generated from the result"]
    pub fn on_frame_ready<S: AppEventSender>(
        &mut self,
        msg: FrameReadyMsg,
        images: &mut ImageCache<S>,
    ) -> ((FrameId, Option<ImageLoadedData>, HitTestResult), bool) {
        debug_assert!(self.document_id == msg.document_id || self.documents.contains(&msg.document_id));

        //println!("{:#?}", msg);

        if self.document_id != msg.document_id {
            todo!("document rendering is not implemented in WR");
        }

        let (frame_id, capture, _) = self.pending_frames.pop_front().unwrap_or((self.rendered_frame_id, false, None));
        self.rendered_frame_id = frame_id;

        let first_frame = self.waiting_first_frame;

        if self.waiting_first_frame {
            let _s = tracing::trace_span!("Window.first-draw").entered();
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
            let _s = tracing::trace_span!("capture_image").entered();
            if msg.composite_needed {
                self.redraw();
            }
            let scale_factor = self.scale_factor();
            let renderer = self.renderer.as_mut().unwrap();
            Some(images.frame_image_data(renderer, PxRect::from_size(self.window.inner_size().to_px()), true, scale_factor))
        } else {
            None
        };

        let hits = self.hit_tester.hit_test(self.cursor_pos);

        ((frame_id, data, hits), first_frame)
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
            Some(self.on_frame_ready(msg, images).0)
        } else {
            None
        }
    }

    pub fn is_maximized(&self) -> bool {
        self.state == WindowState::Maximized
    }

    #[cfg(windows)]
    pub fn is_active_window(&self) -> bool {
        let hwnd = glutin::platform::windows::WindowExtWindows::hwnd(&self.window);
        // SAFETY: `GetActiveWindow` does not have an error state.
        unsafe { winapi::um::winuser::GetActiveWindow() == hwnd as _ }
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
    pub fn hit_test(&mut self, point: DipPoint) -> (FrameId, HitTestResult) {
        let _p = tracing::trace_span!("hit_test").entered();
        let point = point.to_px(self.scale_factor());
        (self.rendered_frame_id, self.hit_tester.hit_test(point))
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
            // scrolled,
        };
        if self.redirect.load(Ordering::Relaxed) {
            let _ = self.redirect_sender.send(msg);
        } else {
            let _ = self.sender.send(AppEvent::FrameReady(self.window_id, msg));
        }
    }
}
