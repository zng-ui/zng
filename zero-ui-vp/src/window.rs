use std::{cell::Cell, rc::Rc};

use gleam::gl;
use glutin::{ContextBuilder, ContextWrapper, NotCurrent, dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize}, event::{ElementState, KeyboardInput, ModifiersState, VirtualKeyCode}, event_loop::EventLoopProxy, window::{Window, WindowBuilder, WindowId}};
use webrender::{
    api::{units::*, *},
    euclid, Renderer, RendererKind, RendererOptions,
};

use crate::{
    config::{set_raw_windows_event_handler, text_aa},
    types::FramePixels,
    AppEvent, Context, Ev, FrameRequest, TextAntiAliasing, WinId, WindowConfig,
};

pub(crate) struct ViewWindow {
    id: WinId,
    window: Window,
    context: Option<ContextWrapper<NotCurrent, ()>>,
    gl: Rc<dyn gl::Gl>,
    renderer: Option<Renderer>,
    api: RenderApi,

    pipeline_id: PipelineId,
    document_id: DocumentId,
    clear_color: Option<ColorF>,

    resized: bool,

    visible: bool,
    waiting_first_frame: bool,

    prev_size: PhysicalSize<u32>,

    allow_alt_f4: Rc<Cell<bool>>,
    taskbar_visible: bool,
    movable: bool, // TODO
    transparent: bool,
}

impl ViewWindow {
    #[allow(clippy::too_many_arguments)]
    pub fn new(ctx: &Context, id: u32, w: WindowConfig) -> Self {
        // create window and OpenGL context
        let winit = WindowBuilder::new()
            .with_title(w.title)
            .with_position(LogicalPosition::new(w.pos.x, w.pos.y))
            .with_inner_size(LogicalSize::new(w.size.width, w.size.height))
            .with_decorations(w.chrome_visible)
            .with_resizable(w.resizable)
            .with_transparent(w.transparent)
            .with_min_inner_size(LogicalSize::new(w.min_size.width, w.min_size.height))
            .with_max_inner_size(LogicalSize::new(w.max_size.width, w.max_size.height))
            .with_window_icon(w.icon.and_then(|i| glutin::window::Icon::from_rgba(i.rgba, i.width, i.height).ok()))
            .with_visible(false); // we wait for the first frame to show the window.

        let glutin = ContextBuilder::new().build_windowed(winit, ctx.window_target).unwrap();
        // SAFETY: we drop the context before the window.
        let (context, winit_window) = unsafe { glutin.split() };

        // extend the winit Windows window to only block the Alt+F4 key press if we want it to.
        let allow_alt_f4 = Rc::new(Cell::new(w.allow_alt_f4));
        #[cfg(windows)]
        {
            let allow_alt_f4 = allow_alt_f4.clone();
            let event_loop = ctx.event_loop.clone();

            set_raw_windows_event_handler(&winit_window, u32::from_ne_bytes(*b"alf4") as _, move |_, msg, wparam, _| {
                if msg == winapi::um::winuser::WM_SYSKEYDOWN && wparam as i32 == winapi::um::winuser::VK_F4 && allow_alt_f4.get() {
                    let device_id = 0; // TODO recover actual ID

                    #[allow(deprecated)] // `modifiers` is deprecated but there is no other way to init a KeyboardInput
                    let _ = event_loop.send_event(AppEvent::Notify(Ev::KeyboardInput(
                        id,
                        device_id,
                        KeyboardInput {
                            scancode: wparam as u32,
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::F4),
                            modifiers: ModifiersState::ALT,
                        },
                    )));
                    return Some(0);
                }
                None
            });
        }

        // create renderer and start the first frame.
        let context = unsafe { context.make_current() }.unwrap();

        let gl = match context.get_api() {
            glutin::Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            glutin::Api::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            glutin::Api::WebGl => panic!("WebGl is not supported"),
        };

        let device_size = winit_window.inner_size();
        let device_size = DeviceIntSize::new(device_size.width as i32, device_size.height as i32);

        let mut text_aa = w.text_aa;
        if let TextAntiAliasing::Default = w.text_aa {
            text_aa = self::text_aa();
        }

        let opts = RendererOptions {
            device_pixel_ratio: winit_window.scale_factor() as f32,
            renderer_kind: RendererKind::Native,
            clear_color: w.clear_color,
            enable_aa: text_aa != TextAntiAliasing::Mono,
            enable_subpixel_aa: text_aa == TextAntiAliasing::Subpixel,
            //panic_on_gl_error: true,
            // TODO expose more options to the user.
            ..Default::default()
        };

        let (renderer, sender) = webrender::Renderer::new(
            Rc::clone(&gl),
            Box::new(Notifier(winit_window.id(), ctx.event_loop.clone())),
            opts,
            None,
            device_size,
        )
        .unwrap();

        let api = sender.create_api();
        let document_id = api.add_document(device_size, 0);

        let pipeline_id = webrender::api::PipelineId(1, 0);

        let context = unsafe { context.make_not_current() }.unwrap();

        let mut win = Self {
            id,
            prev_size: winit_window.inner_size(),
            window: winit_window,
            context: Some(context),
            gl,
            renderer: Some(renderer),
            api,
            document_id,
            pipeline_id,
            resized: true,
            clear_color: w.clear_color,
            waiting_first_frame: true,
            visible: w.visible,
            allow_alt_f4,
            taskbar_visible: true,
            movable: w.movable,
            transparent: w.transparent,
        };

        win.set_taskbar_visible(w.taskbar_visible);

        win
    }

    /// Returns `true` if the `new_size` is actually different then the previous or init size.
    pub fn resized(&mut self, new_size: PhysicalSize<u32>) -> bool {
        let resized = self.prev_size != new_size;
        self.prev_size = new_size;
        resized
    }

    pub fn id(&self) -> WinId {
        self.id
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

    pub fn set_outer_pos(&mut self, pos: LayoutPoint) {
        let s = self.scale_factor();
        let pos = PhysicalPosition::new((pos.x * s) as i32, (pos.y * s) as i32);
        self.window.set_outer_position(pos);
    }

    /// Resize and render, returns `true` if actually resized.
    #[must_use = "an event must be send if returns `true`"]
    pub fn resize_inner(&mut self, size: LayoutSize, frame: FrameRequest) -> bool {
        let new_size = LogicalSize::new(size.width, size.height).to_physical(self.window.scale_factor());
        let resized = self.resized(new_size);
        if resized {
            self.window.set_inner_size(new_size);
            self.resized = true;
            self.render(frame);
        }
        resized
    }

    pub fn set_min_inner_size(&mut self, min_size: LayoutSize) {
        self.window
            .set_min_inner_size(Some(LogicalSize::new(min_size.width, min_size.height)))
    }

    pub fn set_max_inner_size(&mut self, max_size: LayoutSize) {
        self.window
            .set_max_inner_size(Some(LogicalSize::new(max_size.width, max_size.height)))
    }

    /// window.inner_size maybe new.
    pub fn on_resized(&mut self) {
        let ctx = unsafe { self.context.take().unwrap().make_current().unwrap() };
        ctx.resize(self.window.inner_size());
        self.context = unsafe { Some(ctx.make_not_current().unwrap()) };
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
            .set_window_icon(icon.and_then(|i| glutin::window::Icon::from_rgba(i.rgba, i.width, i.height).ok()));
    }

    pub fn set_title(&self, title: String) {
        self.window.set_title(&title);
    }

    /// Start rendering a new frame.
    ///
    /// The [callback](#callback) will be called when the frame is ready to be [presented](Self::present).
    pub fn render(&mut self, frame: FrameRequest) {
        let scale_factor = self.window.scale_factor() as f32;
        let size = self.window.inner_size();
        let viewport_size = LayoutSize::new(size.width as f32 / scale_factor, size.height as f32 / scale_factor);

        let mut txn = Transaction::new();
        let display_list = BuiltDisplayList::from_data(frame.display_list.0, frame.display_list.1);
        txn.set_display_list(
            frame.id,
            self.clear_color,
            viewport_size,
            (frame.pipeline_id, frame.size, display_list),
            true,
        );
        txn.set_root_pipeline(self.pipeline_id);

        if self.resized {
            self.resized = false;
            txn.set_document_view(
                DeviceIntRect::new(euclid::point2(0, 0), euclid::size2(size.width as i32, size.height as i32)),
                scale_factor,
            );
        }

        txn.generate_frame();
        self.api.send_transaction(self.document_id, txn);
    }

    /// Start rendering a new frame based on the data of the last frame.
    pub fn render_update(&mut self, updates: DynamicProperties) {
        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        txn.update_dynamic_properties(updates);

        if self.resized {
            self.resized = false;
            let scale_factor = self.window.scale_factor() as f32;
            let size = self.window.inner_size();
            txn.set_document_view(
                DeviceIntRect::new(euclid::point2(0, 0), euclid::size2(size.width as i32, size.height as i32)),
                scale_factor,
            );
        }

        txn.generate_frame();
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn update_resources(&mut self, updates: Vec<ResourceUpdate>) {
        self.api.update_resources(updates);
    }

    pub fn request_redraw(&mut self) {
        if self.waiting_first_frame {
            self.waiting_first_frame = false;
            self.redraw();
            if self.visible {
                self.window.set_visible(true);
            }
        } else {
            self.window.request_redraw();
        }
    }

    pub fn redraw(&mut self) {
        let ctx = unsafe { self.context.take().unwrap().make_current() }.unwrap();
        let renderer = self.renderer.as_mut().unwrap();
        renderer.update();
        let s = self.window.inner_size();
        renderer.render(DeviceIntSize::new(s.width as i32, s.height as i32)).unwrap();
        ctx.swap_buffers().unwrap();
        self.context = Some(unsafe { ctx.make_not_current() }.unwrap());
    }

    /// Does a hit-test on the current frame.
    ///
    /// Returns all hits from front-to-back.
    pub fn hit_test(&self, point: LayoutPoint) -> HitTestResult {
        self.api.hit_test(
            self.document_id,
            Some(self.pipeline_id),
            units::WorldPoint::new(point.x, point.y),
            HitTestFlags::all(),
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

    pub fn inner_size(&self) -> LayoutSize {
        let size = self.window.inner_size();
        let scale = self.scale_factor();
        LayoutSize::new(size.width as f32 / scale, size.height as f32 / scale)
    }

    /// Read all pixels of the current frame.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels(&mut self) -> FramePixels {
        self.read_pixels_rect(LayoutRect::from_size(self.inner_size()))
    }

    /// Read a selection of pixels of the current frame.
    ///
    /// This is a call to `glReadPixels`, the pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels_rect(&mut self, rect: LayoutRect) -> FramePixels {
        let max = LayoutRect::from_size(self.inner_size());
        let rect = rect.intersection(&max).unwrap_or_default();

        let scale_factor = self.scale_factor();
        let x = rect.origin.x * scale_factor;
        let y = rect.origin.y * scale_factor;
        let width = rect.size.width * scale_factor;
        let height = rect.size.height * scale_factor;

        if width < 1.0 || height < 1.0 {
            return FramePixels {
                width: 0,
                height: 0,
                bgra: vec![],
                scale_factor,
                opaque: true,
            };
        }

        let ctx = unsafe { self.context.take().unwrap().make_current() }.unwrap();

        let bgra = self
            .gl
            .read_pixels(x as _, (y + height) as _, width as _, height as _, gl::BGRA, gl::UNSIGNED_BYTE);
        assert!(self.gl.get_error() == 0);

        self.context = Some(unsafe { ctx.make_not_current() }.unwrap());

        FramePixels {
            width: width as u32,
            height: height as u32,
            bgra,
            scale_factor,
            opaque: true,
        }
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
        // context must be dropped before the winit window and webrender deinit needs an active context.

        let ctx = self.context.take().unwrap();
        if let Ok(ctx) = unsafe { ctx.make_current() } {
            self.renderer.take().unwrap().deinit();
            let _ = unsafe { ctx.make_not_current() };
        }
    }
}

struct Notifier(WindowId, EventLoopProxy<AppEvent>);
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Self(self.0, self.1.clone()))
    }

    fn wake_up(&self) {}

    fn new_frame_ready(&self, _: DocumentId, _: bool, _: bool, _: Option<u64>) {
        let _ = self.1.send_event(AppEvent::FrameReady(self.0));
    }
}
