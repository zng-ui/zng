use super::{EventUpdate, KeyboardEvents, MouseEvents, UpdateFlags, UpdateNotice};
use crate::core::WebRenderEvent;
use fnv::FnvHashMap;
use glutin::event::{DeviceEvent, DeviceId, Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowId;
use std::any::{Any, TypeId};

#[derive(Default)]
pub struct AppRegister {
    events: FnvHashMap<TypeId, Box<dyn Any>>,
}

impl AppRegister {
    pub fn register_event<E: EventNotifier>(&mut self, listener: UpdateNotice<E::Args>) {
        self.events.insert(TypeId::of::<E>(), Box::new(listener));
    }

    pub fn listener<E: EventNotifier>(&self) -> Option<UpdateNotice<E::Args>> {
        if let Some(any) = self.events.get(&TypeId::of::<E>()) {
            any.downcast_ref::<UpdateNotice<E::Args>>().cloned()
        } else {
            None
        }
    }
}

/// An [App] extension.
pub trait AppExtension: 'static {
    /// Register this extension.
    fn register(&mut self, r: &mut AppRegister);

    /// Called when the OS sends an event to a device.
    fn on_device_event(&mut self, _device_id: DeviceId, _event: &DeviceEvent, _update: &mut EventUpdate) {}

    /// Called when the OS sends an event to a window.
    fn on_window_event(&mut self, _window_id: WindowId, _event: &WindowEvent, _update: &mut EventUpdate) {}
}

impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    fn register(&mut self, r: &mut AppRegister) {
        self.0.register(r);
        self.1.register(r);
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, update: &mut EventUpdate) {
        self.0.on_device_event(device_id, event, update);
        self.1.on_device_event(device_id, event, update);
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, update: &mut EventUpdate) {
        self.0.on_window_event(window_id, event, update);
        self.1.on_window_event(window_id, event, update);
    }
}

impl AppExtension for () {
    fn register(&mut self, _: &mut AppRegister) {}
}

/// Identifies an event type.
pub trait EventNotifier: 'static {
    /// Event arguments.
    type Args: 'static;
}

pub struct App<Exts: AppExtension> {
    extensions: Exts,
}

impl<E: AppExtension> App<E> {
    /// Application without any extension.
    pub fn empty() -> App<()> {
        App { extensions: () }
    }

    /// Application with default extensions.
    pub fn default() -> App<(MouseEvents, KeyboardEvents)> {
        App {
            extensions: (MouseEvents::default(), KeyboardEvents::default()),
        }
    }

    /// Includes an [AppExtension] in the application.
    pub fn extend<F: AppExtension>(self, extension: F) -> App<(E, F)> {
        App {
            extensions: (self.extensions, extension),
        }
    }

    /// Runs the application.
    pub fn run(self) -> ! {
        let App { mut extensions } = self;

        let mut register = AppRegister::default();
        extensions.register(&mut register);

        let event_loop = EventLoop::with_user_event();
        let mut in_event_sequence = false;
        let mut event_update = EventUpdate::default();

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(_) => {
                    in_event_sequence = true;
                }
                Event::EventsCleared => {
                    in_event_sequence = false;
                }

                Event::WindowEvent { window_id, event } => {
                    extensions.on_window_event(window_id, &event, &mut event_update);
                }
                Event::UserEvent(WebRenderEvent::NewFrameReady(_window_id)) => {}
                Event::DeviceEvent { device_id, event } => {
                    extensions.on_device_event(device_id, &event, &mut event_update);
                }
                _ => {}
            }

            if !in_event_sequence {
                let updates = event_update.apply();

                if updates.contains(UpdateFlags::UPDATE) {
                    todo!();
                }
                if updates.contains(UpdateFlags::UPD_HP) {
                    todo!();
                }
                if updates.contains(UpdateFlags::LAYOUT) {
                    todo!();
                }
                if updates.contains(UpdateFlags::RENDER) {
                    todo!();
                }
            }
        })
    }
}
