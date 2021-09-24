use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use gleam::gl;
use glutin::{
    event_loop::EventLoopProxy,
    monitor::VideoMode,
    window::{Fullscreen, Window, WindowBuilder, WindowId},
    ContextBuilder, CreationError, GlRequest,
};
use webrender::{api::*, RenderApi, Renderer, RendererOptions, Transaction};

use crate::{
    config,
    types::{FramePixels, ScanCode},
    units::*,
    util::{self, GlContext, RunOnDrop},
    AppEvent, AppEventSender, Context, Ev, FrameRequest, Key, KeyState, TextAntiAliasing, ViewProcessGen, WinId, WindowConfig, WindowState,
};

pub(crate) struct ViewWindow {
    id: WinId,
    window: Window,
    state: WindowState,
    context: GlContext,
    gl: Rc<dyn gl::Gl>,
    renderer: Option<Renderer>,
    api: RenderApi,

    redirect_frame: Arc<AtomicBool>,
    redirect_frame_recv: flume::Receiver<()>,

    pipeline_id: PipelineId,
    document_id: DocumentId,

    resized: bool,

    visible: bool,
    waiting_first_frame: bool,

    prev_pos: DipPoint,
    prev_size: DipSize,

    allow_alt_f4: Rc<Cell<bool>>,
    taskbar_visible: bool,
    movable: bool, // TODO
    transparent: bool,

    frame_id: Epoch,
}

impl ViewWindow {
    #[allow(clippy::too_many_arguments)]
    pub fn new<E: AppEventSender>(ctx: &Context<E>, gen: ViewProcessGen, id: WinId, w: WindowConfig) -> Self {
        // create window and OpenGL context
        let mut winit = WindowBuilder::new()
            .with_title(w.title)
            .with_inner_size(w.size.to_winit())
            .with_decorations(w.chrome_visible)
            .with_resizable(w.resizable)
            .with_transparent(w.transparent)
            .with_min_inner_size(w.min_size.to_winit())
            .with_max_inner_size(w.max_size.to_winit())
            .with_always_on_top(w.always_on_top)
            .with_window_icon(
                w.icon
                    .and_then(|i| glutin::window::Icon::from_rgba(i.rgba.into_vec(), i.width, i.height).ok()),
            )
            .with_visible(false); // we wait for the first frame to show the window.

        if let Some(pos) = w.pos {
            winit = winit.with_position(pos.to_winit());
        }

        let glutin = match ContextBuilder::new()
            .with_hardware_acceleration(None)
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(winit, ctx.window_target)
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
        let mut context = ctx.gl_manager.manage_headed(id, context);

        // extend the winit Windows window to only block the Alt+F4 key press if we want it to.
        let allow_alt_f4 = Rc::new(Cell::new(w.allow_alt_f4));
        #[cfg(windows)]
        {
            let allow_alt_f4 = allow_alt_f4.clone();
            let event_loop = ctx.event_loop.clone();

            util::set_raw_windows_event_handler(&winit_window, u32::from_ne_bytes(*b"alf4") as _, move |_, msg, wparam, _| {
                if msg == winapi::um::winuser::WM_SYSKEYDOWN && wparam as i32 == winapi::um::winuser::VK_F4 && allow_alt_f4.get() {
                    let device_id = 0; // TODO recover actual ID

                    let _ = event_loop.send_event(AppEvent::Notify(Ev::KeyboardInput(
                        id,
                        device_id,
                        wparam as ScanCode,
                        KeyState::Pressed,
                        Some(Key::F4),
                    )));
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

        let mut text_aa = w.text_aa;
        if let TextAntiAliasing::Default = w.text_aa {
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

        let (renderer, sender) = webrender::Renderer::new(
            Rc::clone(&gl),
            Box::new(Notifier {
                window_id: winit_window.id(),
                sender: ctx.event_loop.clone(),
                redirect: redirect_frame.clone(),
                redirect_sender: rf_sender,
            }),
            opts,
            None,
        )
        .unwrap();

        let api = sender.create_api();
        let document_id = api.add_document(device_size);

        let pipeline_id = webrender::api::PipelineId(gen, id);

        let scale_factor = winit_window.scale_factor() as f32;

        let mut win = Self {
            id,
            prev_pos: winit_window.outer_position().unwrap_or_default().to_px().to_dip(scale_factor),
            prev_size: winit_window.inner_size().to_px().to_dip(scale_factor),
            window: winit_window,
            context,
            gl,
            renderer: Some(renderer),
            redirect_frame,
            redirect_frame_recv,
            api,
            document_id,
            pipeline_id,
            resized: true,
            waiting_first_frame: true,
            visible: w.visible,
            allow_alt_f4,
            taskbar_visible: true,
            movable: w.movable,
            transparent: w.transparent,
            frame_id: Epoch::invalid(),
            state: WindowState::Normal,
        };
        win.state_change(); // update

        win.set_taskbar_visible(w.taskbar_visible);
        win.set_state(w.state);

        win
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

    pub fn id(&self) -> WinId {
        self.id
    }

    /// Latest received frame.
    pub fn frame_id(&self) -> Epoch {
        self.frame_id
    }

    pub fn is_window(&self, window_id: WindowId) -> bool {
        self.window.id() == window_id
    }

    pub fn actual_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    /// Move window, returns `true` if actually moved.
    #[must_use = "an event must be send if returns `true`"]
    pub fn set_outer_pos(&mut self, pos: DipPoint) -> bool {
        let moved = self.moved(pos);
        if moved {
            let new_pos = pos.to_winit();
            self.window.set_outer_position(new_pos);
        }
        moved
    }

    /// Probe state, returns `Some(new_state)`
    pub fn state_change(&mut self) -> Option<WindowState> {
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

    pub fn video_mode(&self) -> Option<VideoMode> {
        // TODO configurable video mode.
        self.window.current_monitor().and_then(|m| m.video_modes().next())
    }

    /// Apply the new state, returns `true` if the state changed.
    pub fn set_state(&mut self, state: WindowState) -> bool {
        if state.is_fullscreen() {
            match state {
                WindowState::Fullscreen => self.window.set_fullscreen(Some(Fullscreen::Borderless(None))),
                WindowState::Exclusive => {
                    if let Some(mode) = self.video_mode() {
                        self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
                    } else {
                        todo!()
                    }
                }
                _ => unreachable!(),
            }
        } else {
            if self.window.fullscreen().is_some() {
                self.window.set_fullscreen(None);
            }
            match state {
                WindowState::Normal => self.window.set_maximized(false),
                WindowState::Minimized => self.window.set_minimized(true),
                WindowState::Maximized => self.window.set_maximized(true),
                _ => unreachable!(),
            }
        }

        if let Some(s) = self.state_change() {
            debug_assert_eq!(s, state);
            true
        } else {
            false
        }
    }

    /// Resize and render, returns `true` if actually resized.
    ///
    /// Returns (resized, rendered)
    #[must_use = "an event must be send if returns `true`"]
    pub fn resize_inner(&mut self, size: DipSize, frame: FrameRequest) -> (bool, bool) {
        let resized = self.resized(size);
        let mut rendered = false;
        if resized {
            let new_size = size.to_winit();
            self.window.set_inner_size(new_size);
            self.resized = true;
            self.render(frame);
            rendered = self.wait_frame_ready(Instant::now() + Duration::from_secs(1));
        }
        (resized, rendered)
    }

    pub fn set_min_inner_size(&mut self, min_size: DipSize) {
        self.window.set_min_inner_size(Some(min_size.to_winit()))
    }

    pub fn set_max_inner_size(&mut self, max_size: DipSize) {
        self.window.set_max_inner_size(Some(max_size.to_winit()))
    }

    /// window.inner_size maybe new.
    pub fn on_resized(&mut self) {
        let ctx = self.context.make_current();
        ctx.resize(self.window.inner_size());
        self.resized = true;
    }

    pub fn set_visible(&mut self, visible: bool) {
        if !self.waiting_first_frame {
            self.window.set_visible(visible);
        }
        self.visible = visible;
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

    pub fn set_icon(&mut self, icon: Option<crate::Icon>) {
        self.window
            .set_window_icon(icon.and_then(|i| glutin::window::Icon::from_rgba(i.rgba.into_vec(), i.width, i.height).ok()));
    }

    pub fn set_title(&self, title: String) {
        self.window.set_title(&title);
    }

    /// Start rendering a new frame.
    ///
    /// The [callback](#callback) will be called when the frame is ready to be [presented](Self::present).
    pub fn render(&mut self, frame: FrameRequest) {
        self.frame_id = frame.id;
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
            frame.id,
            Some(frame.clear_color),
            viewport_size,
            (frame.pipeline_id, display_list),
            true,
        );
        txn.set_root_pipeline(self.pipeline_id);

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id.0 as u64); // TODO review frame_id != Epoch?
        self.api.send_transaction(self.document_id, txn);
    }

    /// Start rendering a new frame based on the data of the last frame.
    pub fn render_update(&mut self, updates: DynamicProperties, clear_color: Option<ColorF>) {
        if let Some(color) = clear_color {
            self.renderer.as_mut().unwrap().set_clear_color(color);
        }

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        txn.update_dynamic_properties(updates);

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id.0 as u64);
        self.api.send_transaction(self.document_id, txn);
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;
            let size = self.window.inner_size();
            txn.set_document_view(PxRect::from_size(size.to_px()).to_wr_device());
        }
    }

    pub fn send_transaction(&mut self, txn: Transaction) {
        self.api.send_transaction(self.document_id, txn);
    }

    /// Capture the next frame-ready event.
    ///
    /// Returns `true` if received before `deadline`, if `true` already redraw too.
    pub fn wait_frame_ready(&mut self, deadline: Instant) -> bool {
        self.redirect_frame.store(true, Ordering::Relaxed);
        let stop_redirect = RunOnDrop::new(|| self.redirect_frame.store(false, Ordering::Relaxed));

        let received = self.redirect_frame_recv.recv_deadline(deadline).is_ok();

        drop(stop_redirect);

        if received {
            self.redraw();
        }
        received
    }

    /// Returns if it is the first frame.
    #[must_use = "if `true` must notify the initial Resized event"]
    pub fn request_redraw(&mut self) -> bool {
        if self.waiting_first_frame {
            self.waiting_first_frame = false;
            self.redraw();
            if self.visible {
                self.window.set_visible(true);
            }
            true
        } else {
            self.window.request_redraw();

            false
        }
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

    /// Does a hit-test on the current frame.
    ///
    /// Returns all hits from front-to-back.
    pub fn hit_test(&self, point: PxPoint) -> (Epoch, HitTestResult) {
        (
            self.frame_id,
            self.api.hit_test(self.document_id, Some(self.pipeline_id), point.to_wr_world()),
        )
    }

    pub fn set_text_aa(&self, aa: TextAntiAliasing) {
        todo!("need to rebuild the renderer? {:?}", aa)
    }

    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    pub fn namespace_id(&self) -> IdNamespace {
        self.api.get_namespace_id()
    }

    pub fn generate_image_key(&self) -> ImageKey {
        self.api.generate_image_key()
    }

    pub fn generate_font_key(&self) -> FontKey {
        self.api.generate_font_key()
    }

    pub fn generate_font_instance_key(&self) -> FontInstanceKey {
        self.api.generate_font_instance_key()
    }

    pub fn outer_position(&self) -> DipPoint {
        self.window
            .outer_position()
            .unwrap_or_default()
            .to_logical(self.window.scale_factor())
            .to_dip()
    }

    pub fn size(&self) -> DipSize {
        self.window.inner_size().to_logical(self.window.scale_factor()).to_dip()
    }

    pub fn read_pixels(&mut self) -> FramePixels {
        let px_size = self.window.inner_size().to_px();
        // `self.gl` is only valid if we are the current context.
        let _ctx = self.context.make_current();
        util::read_pixels_rect(&self.gl, px_size, PxRect::from_size(px_size), self.scale_factor())
    }

    pub fn read_pixels_rect(&mut self, rect: PxRect) -> FramePixels {
        // `self.gl` is only valid if we are the current context.
        let _ctx = self.context.make_current();
        util::read_pixels_rect(&self.gl, self.window.inner_size().to_px(), rect, self.scale_factor())
    }

    #[cfg(not(windows))]
    pub fn set_taskbar_visible(&mut self, visible: bool) {
        log::error!("taskbar_visible not implemented in this plataform");
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        if self.transparent != transparent {
            self.transparent = transparent;
            todo!("respawn just the window?")
        }
    }

    pub fn set_parent(&mut self, parent: Option<WindowId>, modal: bool) {
        todo!("implement parent & modal: {:?}", (parent, modal));
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

    pub fn set_chrome_visible(&mut self, visible: bool) {
        self.window.set_decorations(visible);
    }

    pub fn set_allow_alt_f4(&mut self, allow: bool) {
        self.allow_alt_f4.set(allow);
    }
}
impl Drop for ViewWindow {
    fn drop(&mut self) {
        // webrender deinit panics if the context is not current.
        let _ctx = self.context.make_current();
        self.renderer.take().unwrap().deinit();

        // context must be dropped before the winit window (glutin requirement).
        self.context.drop_before_winit();

        // the winit window will be dropped normally after this.
    }
}

struct Notifier {
    window_id: WindowId,
    sender: EventLoopProxy<AppEvent>,
    redirect: Arc<AtomicBool>,
    redirect_sender: flume::Sender<()>,
}
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            window_id: self.window_id,
            sender: self.sender.clone(),
            redirect: self.redirect.clone(),
            redirect_sender: self.redirect_sender.clone(),
        })
    }

    fn wake_up(&self, _: bool) {}

    fn new_frame_ready(&self, _: DocumentId, _: bool, _: bool, _: Option<u64>) {
        if self.redirect.load(Ordering::Relaxed) {
            let _ = self.redirect_sender.send(());
        } else {
            let _ = self.sender.send_event(AppEvent::FrameReady(self.window_id));
        }
    }
}
