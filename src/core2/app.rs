use super::*;
use context::*;
use glutin::event::Event as GEvent;
pub use glutin::event::{DeviceEvent, DeviceId, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
pub use glutin::window::WindowId;

/// An [App] extension.
pub trait AppExtension: 'static {
    /// Initializes this extension.
    fn init(&mut self, _ctx: &mut AppInitContext) {}

    /// Called when the OS sends an event to a device.
    fn on_device_event(&mut self, _device_id: DeviceId, _event: &DeviceEvent, _ctx: &mut AppContext) {}

    /// Called when the OS sends an event to a window.
    fn on_window_event(&mut self, _window_id: WindowId, _event: &WindowEvent, _ctx: &mut AppContext) {}

    /// Called when a new frame is ready to be presented.
    fn on_new_frame_ready(&mut self, _window_id: WindowId, _ctx: &mut AppContext) {}

    /// Called every update after the Ui update.
    fn update(&mut self, _update: UpdateRequest, _ctx: &mut AppContext) {}

    /// Called after every sequence of updates if display update was requested.
    fn update_display(&mut self, _update: DisplayUpdate) {}
}

impl AppExtension for () {}

impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    fn init(&mut self, ctx: &mut AppInitContext) {
        self.0.init(ctx);
        self.1.init(ctx);
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        self.0.on_device_event(device_id, event, ctx);
        self.1.on_device_event(device_id, event, ctx);
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        self.0.on_window_event(window_id, event, ctx);
        self.1.on_window_event(window_id, event, ctx);
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.0.on_new_frame_ready(window_id, ctx);
        self.1.on_new_frame_ready(window_id, ctx);
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.0.update(update, ctx);
        self.1.update(update, ctx);
    }

    fn update_display(&mut self, update: DisplayUpdate) {
        self.0.update_display(update);
        self.1.update_display(update);
    }
}

/// Identifies a service type.
pub trait Service: 'static {}

/// Defines and runs an application.
pub struct App;

impl App {
    /// Application without any extension.
    pub fn empty() -> ExtendedApp<()> {
        ExtendedApp { extensions: () }
    }

    /// Application with default extensions.
    pub fn default() -> ExtendedApp<impl AppExtension> {
        App::empty()
            .extend(MouseEvents::default())
            .extend(KeyboardEvents::default())
            .extend(FontCache::default())
            .extend(AppWindows::default())
    }
}

pub struct ExtendedApp<E: AppExtension> {
    extensions: E,
}

impl<E: AppExtension> ExtendedApp<E> {
    pub fn extend<F: AppExtension>(self, extension: F) -> ExtendedApp<impl AppExtension> {
        ExtendedApp {
            extensions: (self.extensions, extension),
        }
    }

    /// Runs the application.
    pub fn run(self) -> ! {
        let event_loop = EventLoop::with_user_event();

        let mut extensions = self.extensions;

        let mut owned_ctx = OwnedAppContext::instance();

        extensions.init(&mut owned_ctx.borrow_init(event_loop.create_proxy()));

        let mut in_sequence = false;
        let mut sequence_update = DisplayUpdate::None;

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                GEvent::NewEvents(_) => {
                    in_sequence = true;
                }
                GEvent::EventsCleared => {
                    in_sequence = false;
                }

                GEvent::WindowEvent { window_id, event } => {
                    extensions.on_window_event(window_id, &event, &mut owned_ctx.borrow(event_loop));
                }
                GEvent::UserEvent(WebRenderEvent::NewFrameReady(window_id)) => {
                    extensions.on_new_frame_ready(window_id, &mut owned_ctx.borrow(event_loop));
                }
                GEvent::DeviceEvent { device_id, event } => {
                    extensions.on_device_event(device_id, &event, &mut owned_ctx.borrow(event_loop));
                }
                _ => {}
            }

            loop {
                let (update, display) = owned_ctx.apply_updates();
                sequence_update |= display;

                if update.update || update.update_hp {
                    extensions.update(update, &mut owned_ctx.borrow(event_loop));
                } else {
                    break;
                }
            }

            if !in_sequence && sequence_update.is_some() {
                extensions.update_display(sequence_update);
                sequence_update = DisplayUpdate::None;
            }
        })
    }
}

#[derive(Debug)]
pub enum WebRenderEvent {
    NewFrameReady(WindowId),
}
