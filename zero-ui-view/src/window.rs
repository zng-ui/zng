use std::{
    cell::Cell,
    fmt,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use gleam::gl;
use glutin::{
    dpi::PhysicalSize,
    event_loop::EventLoopWindowTarget,
    window::{Fullscreen, Window as GWindow, WindowBuilder},
    Api as GApi, ContextBuilder, CreationError, GlRequest,
};
use webrender::{
    api::{
        self as webrender_api, BuiltDisplayList, ColorF, DisplayListPayload, DocumentId, DynamicProperties, Epoch, FontInstanceKey,
        FontInstanceOptions, FontInstancePlatformOptions, FontKey, FontVariation, HitTestResult, IdNamespace, ImageDescriptor, ImageKey,
        PipelineId, RenderNotifier,
    },
    RenderApi, Renderer, RendererOptions, Transaction,
};
use zero_ui_view_api::{
    units::{PxToDip, *},
    ByteBuf, Event, FramePixels, FrameRequest, Key, KeyState, ScanCode, TextAntiAliasing, ViewProcessGen, WinId, WindowConfig, WindowState,
};

use crate::{
    config,
    util::{self, DipToWinit, GlContext, GlContextManager, WinitToPx},
    AppEvent, AppEventSender,
};

/// A headed window.
pub(crate) struct Window {
    id: WinId,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    api: RenderApi,

    window: GWindow,
    context: GlContext,
    gl: Rc<dyn gl::Gl>,
    renderer: Option<Renderer>,

    redirect_frame: Arc<AtomicBool>,
    redirect_frame_recv: flume::Receiver<()>,

    frame_id: Epoch,
    resized: bool,

    prev_pos: DipPoint,
    prev_size: DipSize,
    state: WindowState,

    visible: bool,
    waiting_first_frame: bool,

    allow_alt_f4: Rc<Cell<bool>>,
    taskbar_visible: bool,

    movable: bool, // TODO
    transparent: bool,
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
        id: WinId,
        gen: ViewProcessGen,
        cfg: WindowConfig,
        window_target: &EventLoopWindowTarget<AppEvent>,
        gl_manager: &mut GlContextManager,
        event_sender: impl AppEventSender,
    ) -> Self {
        // create window and OpenGL context
        let mut winit = WindowBuilder::new()
            .with_title(cfg.title)
            .with_inner_size(cfg.size.to_winit())
            .with_decorations(cfg.chrome_visible)
            .with_resizable(cfg.resizable)
            .with_transparent(cfg.transparent)
            .with_min_inner_size(cfg.min_size.to_winit())
            .with_max_inner_size(cfg.max_size.to_winit())
            .with_always_on_top(cfg.always_on_top)
            .with_window_icon(
                cfg.icon
                    .and_then(|i| glutin::window::Icon::from_rgba(i.rgba.into_vec(), i.width, i.height).ok()),
            )
            .with_visible(false); // we wait for the first frame to show the window.

        if let Some(pos) = cfg.pos {
            winit = winit.with_position(pos.to_winit());
        }

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
            let event_sender = event_sender.clone_();

            util::set_raw_windows_event_handler(&winit_window, u32::from_ne_bytes(*b"alf4") as _, move |_, msg, wparam, _| {
                if msg == winapi::um::winuser::WM_SYSKEYDOWN && wparam as i32 == winapi::um::winuser::VK_F4 && allow_alt_f4.get() {
                    let device_id = 0; // TODO recover actual ID

                    let _ = event_sender.send(AppEvent::Notify(Event::KeyboardInput(
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

        let (renderer, sender) = webrender::Renderer::new(
            Rc::clone(&gl),
            Box::new(Notifier {
                window_id: id,
                sender: event_sender.clone_(),
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
            visible: cfg.visible,
            allow_alt_f4,
            taskbar_visible: true,
            movable: cfg.movable,
            transparent: cfg.transparent,
            frame_id: Epoch::invalid(),
            state: WindowState::Normal,
        };
        win.state_change(); // update

        win.set_taskbar_visible(cfg.taskbar_visible);
        win.set_state(cfg.state);

        win
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

    pub fn video_mode(&self) -> Option<glutin::monitor::VideoMode> {
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
}

struct Notifier<S> {
    window_id: WinId,
    sender: S,
    redirect: Arc<AtomicBool>,
    redirect_sender: flume::Sender<()>,
}
impl<S: AppEventSender> RenderNotifier for Notifier<S> {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            window_id: self.window_id,
            sender: self.sender.clone_(),
            redirect: self.redirect.clone(),
            redirect_sender: self.redirect_sender.clone(),
        })
    }

    fn wake_up(&self, _: bool) {}

    fn new_frame_ready(&self, _: DocumentId, _: bool, _: bool, _: Option<u64>) {
        if self.redirect.load(Ordering::Relaxed) {
            let _ = self.redirect_sender.send(());
        } else {
            let _ = self.sender.send(AppEvent::FrameReady(self.window_id));
        }
    }
}
