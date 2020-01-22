use super::*;
use context::*;
use glutin::event::Event as GEvent;
pub use glutin::event::{DeviceEvent, DeviceId, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
pub use glutin::window::WindowId;

/// An [App] extension.
pub trait AppExtension: 'static {
    /// Register this extension.
    fn init(&mut self, r: &mut AppInitContext);

    /// Called when the OS sends an event to a device.
    fn on_device_event(&mut self, _device_id: DeviceId, _event: &DeviceEvent, _ctx: &mut AppContext) {}

    /// Called when the OS sends an event to a window.
    fn on_window_event(&mut self, _window_id: WindowId, _event: &WindowEvent, _ctx: &mut AppContext) {}

    /// Called every update after the Ui update.
    fn respond(&mut self, _ctx: &mut AppContext) {}
}

impl AppExtension for Box<dyn AppExtension> {
    fn init(&mut self, r: &mut AppInitContext) {
        self.as_mut().init(r);
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        self.as_mut().on_device_event(device_id, event, ctx);
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        self.as_mut().on_window_event(window_id, event, ctx);
    }

    fn respond(&mut self, ctx: &mut AppContext) {
        self.as_mut().respond(ctx);
    }
}

impl<E: AppExtension> AppExtension for Vec<E> {
    fn init(&mut self, r: &mut AppInitContext) {
        for inner in self.iter_mut() {
            inner.init(r);
        }
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        for inner in self.iter_mut() {
            inner.on_device_event(device_id, event, ctx);
        }
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        for inner in self.iter_mut() {
            inner.on_window_event(window_id, event, ctx);
        }
    }

    fn respond(&mut self, ctx: &mut AppContext) {
        for inner in self.iter_mut() {
            inner.respond(ctx);
        }
    }
}

impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    fn init(&mut self, r: &mut AppInitContext) {
        self.0.init(r);
        self.1.init(r);
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        self.0.on_device_event(device_id, event, ctx);
        self.1.on_device_event(device_id, event, ctx);
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        self.0.on_window_event(window_id, event, ctx);
        self.1.on_window_event(window_id, event, ctx);
    }

    fn respond(&mut self, ctx: &mut AppContext) {
        self.0.respond(ctx);
        self.1.respond(ctx);
    }
}

/// Identifies a service type.
pub trait Service: 'static {}

/// Defines and runs an application.
pub struct App {
    extensions: Vec<Box<dyn AppExtension>>,
}

#[derive(Debug)]
pub enum WebRenderEvent {
    NewFrameReady(WindowId),
}

impl App {
    /// Application without any extension.
    pub fn empty() -> App {
        App {
            extensions: Vec::default(),
        }
    }

    /// Application with default extensions.
    pub fn default() -> App {
        App::empty()
            .extend(MouseEvents::default())
            .extend(KeyboardEvents::default())
            .extend(FontCache::default())
    }

    /// Includes an [AppExtension] in the application.
    pub fn extend<F: AppExtension>(self, extension: F) -> App {
        let mut extensions = self.extensions;
        extensions.push(Box::new(extension));
        App { extensions }
    }

    /// Runs the application.
    pub fn run(self) -> ! {
        let event_loop = EventLoop::with_user_event();

        let mut extensions = (AppWindows::new(event_loop.create_proxy()), self.extensions);

        let mut ctx = OwnedAppContext::new();

        extensions.init(&mut ctx.borrow_init());

        let mut in_sequence = false;
        let mut sequence_update = UpdateFlags::empty();

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            let mut event_update = UpdateFlags::empty();

            let mut ctx = ctx.borrow(event_loop);

            match event {
                GEvent::NewEvents(_) => {
                    in_sequence = true;
                }
                GEvent::EventsCleared => {
                    in_sequence = false;
                }

                GEvent::WindowEvent { window_id, event } => {
                    extensions.on_window_event(window_id, &event, &mut ctx);
                }
                GEvent::UserEvent(WebRenderEvent::NewFrameReady(window_id)) => {
                    extensions.0.new_frame_ready(window_id);
                }
                GEvent::DeviceEvent { device_id, event } => {
                    extensions.on_device_event(device_id, &event, &mut ctx);
                }
                _ => {}
            }

            if event_update.contains(UpdateFlags::UPD_HP) {
                event_update.remove(UpdateFlags::UPD_HP);
                extensions.0.update_hp(&mut ctx);
            }
            if event_update.contains(UpdateFlags::UPDATE) {
                event_update.remove(UpdateFlags::UPDATE);
                extensions.0.update(&mut ctx);
            }

            let ui_update = ctx.updates.apply_updates();

            sequence_update |= event_update | ui_update;

            extensions.respond(&mut ctx);

            if !in_sequence {
                if sequence_update.contains(UpdateFlags::LAYOUT) {
                    extensions.0.layout();
                }
                if sequence_update.contains(UpdateFlags::RENDER) {
                    extensions.0.render();
                }

                sequence_update = UpdateFlags::empty();
            }
        })
    }
}
