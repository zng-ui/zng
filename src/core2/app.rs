use super::{EventUpdate, KeyboardEvents, MouseEvents, UpdateFlags, UpdateNotice, WindowsExt};
use crate::core::WebRenderEvent;
use fnv::FnvHashMap;
use glutin::event::Event;
use glutin::event_loop::{ControlFlow, EventLoop};
use std::any::{Any, TypeId};

pub use glutin::event::{DeviceEvent, DeviceId, WindowEvent};
pub use glutin::window::WindowId;

#[derive(Default)]
pub struct AppRegister {
    events: FnvHashMap<TypeId, Box<dyn Any>>,
    services: FnvHashMap<TypeId, Box<dyn Any>>,
}

impl AppRegister {
    pub fn register_event<E: EventNotifier>(&mut self, listener: UpdateNotice<E::Args>) {
        self.events.insert(TypeId::of::<E>(), Box::new(listener));
    }

    pub fn register_service<S: Service>(&mut self, service: S) {
        self.services.insert(TypeId::of::<S>(), Box::new(service));
    }

    pub fn try_listen<E: EventNotifier>(&self) -> Option<UpdateNotice<E::Args>> {
        if let Some(any) = self.events.get(&TypeId::of::<E>()) {
            any.downcast_ref::<UpdateNotice<E::Args>>().cloned()
        } else {
            None
        }
    }

    pub fn listen<E: EventNotifier>(&self) -> UpdateNotice<E::Args> {
        self.try_listen::<E>()
            .unwrap_or_else(|| panic!("event `{}` is required", std::any::type_name::<E>()))
    }

    pub fn try_service<S: Service>(&self) -> Option<&S> {
        if let Some(any) = self.events.get(&TypeId::of::<S>()) {
            any.downcast_ref::<S>()
        } else {
            None
        }
    }

    pub fn service<S: Service>(&self) -> &S {
        self.try_service::<S>()
            .unwrap_or_else(|| panic!("service `{}` is required", std::any::type_name::<S>()))
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
    type Args: std::fmt::Debug + Clone + 'static;
}

/// Identifies a service type.
pub trait Service: Clone + 'static {}

/// Defines and runs an application.
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
        let mut extensions = (WindowsExt::default(), self.extensions);

        let mut register = AppRegister::default();
        extensions.register(&mut register);

        let event_loop = EventLoop::with_user_event();
        let mut in_event_sequence = false;
        let mut event_squence_update = UpdateFlags::empty();
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

            let mut event_update = event_update.apply();
            if event_update.contains(UpdateFlags::UPDATE) {
                event_update.remove(UpdateFlags::UPDATE);
                todo!();
            }
            if event_update.contains(UpdateFlags::UPD_HP) {
                event_update.remove(UpdateFlags::UPD_HP);
                todo!();
            }

            event_squence_update |= event_update;

            if !in_event_sequence {
                if event_squence_update.contains(UpdateFlags::LAYOUT) {
                    todo!();
                }
                if event_squence_update.contains(UpdateFlags::RENDER) {
                    todo!();
                }

                event_squence_update = UpdateFlags::empty();
            }
        })
    }
}
