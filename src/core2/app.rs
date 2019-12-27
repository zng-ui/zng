use super::{
    ContextVar, Event, EventEmitter, EventListener, KeyboardEvents, MouseEvents, SharedVar, VisitedVar, WindowsExt,
};
use fnv::FnvHashMap;
use glutin::event::Event as GEvent;
use glutin::event_loop::{ControlFlow, EventLoop};
use std::any::{type_name, Any, TypeId};

pub use glutin::event::{DeviceEvent, DeviceId, WindowEvent};
pub use glutin::window::WindowId;

pub struct AppRegister {
    ctx: AppContext,
}

impl Default for AppRegister {
    fn default() -> Self {
        AppRegister {
            ctx: AppContext {
                id: AppContextId::new_unique(),
                events: FnvHashMap::default(),
                services: FnvHashMap::default(),
                context_vars: FnvHashMap::default(),
                visited_vars: FnvHashMap::default(),

                update: UpdateFlags::empty(),
                updates: Vec::default(),
                cleanup: Vec::default(),
            },
        }
    }
}

pub struct EventContext {
    ctx: AppContext,
}

impl AppRegister {
    pub fn register_event<E: Event>(&mut self, listener: EventListener<E::Args>) {
        self.ctx.events.insert(TypeId::of::<E>(), Box::new(listener));
    }

    pub fn register_service<S: Service>(&mut self, service: S) {
        self.ctx.services.insert(TypeId::of::<S>(), Box::new(service));
    }
}

type AnyMap = FnvHashMap<TypeId, Box<dyn Any>>;
enum UntypedRef {}
struct ContextVarEntry {
    pointer: *const UntypedRef,
    is_new: bool,
}
type UpdateOnce = Box<dyn FnOnce(&mut Vec<Box<dyn FnOnce()>>)>;
type CleanupOnce = Box<dyn FnOnce()>;

uid! {
   /// Unique id of an [AppContext] instance.
   pub struct AppContextId(_);
}

bitflags! {
    /// What to pump in a Ui tree after an update is applied.
    #[derive(Default)]
    pub(crate) struct UpdateFlags: u8 {
        const UPDATE = 0b0000_0001;
        const UPD_HP = 0b0000_0010;
        const LAYOUT = 0b0000_0100;
        const RENDER = 0b0000_1000;
    }
}

/// Provides access to app events and services.
pub struct AppContext {
    id: AppContextId,
    events: AnyMap,
    services: AnyMap,
    context_vars: FnvHashMap<TypeId, ContextVarEntry>,
    visited_vars: AnyMap,

    update: UpdateFlags,
    updates: Vec<UpdateOnce>,
    cleanup: Vec<CleanupOnce>,
}

impl AppContext {
    /// Gets this context instance id. There is usually a single context
    /// per application but more then one context can happen in tests.
    pub fn id(&self) -> AppContextId {
        self.id
    }

    pub fn try_listen<E: Event>(&self) -> Option<EventListener<E::Args>> {
        if let Some(any) = self.events.get(&TypeId::of::<E>()) {
            any.downcast_ref::<EventListener<E::Args>>().cloned()
        } else {
            None
        }
    }

    pub fn listen<E: Event>(&self) -> EventListener<E::Args> {
        self.try_listen::<E>()
            .unwrap_or_else(|| panic!("event `{}` is required", type_name::<E>()))
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
            .unwrap_or_else(|| panic!("service `{}` is required", type_name::<S>()))
    }

    /// Get the context var value and if it is new or none if its not set.
    pub fn try_get<V: ContextVar>(&self) -> Option<&V::Type> {
        if let Some(ctx_var) = self.context_vars.get(&TypeId::of::<V>()) {
            // REFERENCE SAFETY: This is safe because context_vars are only inserted for the duration
            // of [with_var] that holds the reference.
            Some(unsafe { &*(ctx_var.pointer as *const V::Type) })
        } else {
            None
        }
    }

    /// Gets if the context var value is new or none if its not set.
    pub fn try_get_is_new<V: ContextVar>(&self) -> Option<bool> {
        self.context_vars.get(&TypeId::of::<V>()).map(|v| v.is_new)
    }

    /// Get the context var value or none if its not set or is not new.
    pub fn try_get_new<V: ContextVar>(&self) -> Option<&V::Type> {
        if let Some(ctx_var) = self.context_vars.get(&TypeId::of::<V>()) {
            if ctx_var.is_new {
                // REFERENCE SAFETY: This is safe because context_vars are only inserted for the duration
                // of [with_var] that holds the reference.
                Some(unsafe { &*(ctx_var.pointer as *const V::Type) })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get the visited var value or none if its not set.
    pub fn try_get_visited<V: VisitedVar>(&self) -> Option<&V::Type> {
        if let Some(any) = self.visited_vars.get(&TypeId::of::<V>()) {
            any.downcast_ref::<V::Type>()
        } else {
            None
        }
    }

    /// Get the context var value and if it is new or panics if its not set.
    pub fn get<V: ContextVar>(&self) -> &V::Type {
        self.try_get::<V>()
            .unwrap_or_else(|| panic!("context var `{}` is required", type_name::<V>()))
    }

    /// Gets if the context var value is new or panics if its not set.
    pub fn get_is_new<V: ContextVar>(&self) -> bool {
        self.try_get_is_new::<V>()
            .unwrap_or_else(|| panic!("context var `{}` is required", type_name::<V>()))
    }

    /// Get the visited var value or panics if its not set.
    pub fn get_visited<V: VisitedVar>(&self) -> &V::Type {
        self.try_get_visited::<V>()
            .unwrap_or_else(|| panic!("visited var `{}` is required", type_name::<V>()))
    }

    /// Runs a function with the context var.
    pub fn with_var<V: ContextVar>(&mut self, value: &V::Type, is_new: bool, f: impl FnOnce(&mut AppContext)) {
        let type_id = TypeId::of::<V>();

        let prev = self.context_vars.insert(
            type_id,
            ContextVarEntry {
                pointer: (value as *const V::Type) as *const UntypedRef,
                is_new,
            },
        );

        f(self);

        if let Some(prev) = prev {
            self.context_vars.insert(type_id, prev);
        } else {
            self.context_vars.remove(&type_id);
        }
    }

    /// Schedules a variable change for the next update.
    pub fn push_set<T>(&mut self, var: SharedVar<T>, new_value: T) {
        self.push_change(var, move |value| *value = new_value);
    }

    /// Schedules a variable modification for the next update.
    pub fn push_change<T>(&mut self, var: SharedVar<T>, modify: impl FnOnce(&mut T) + 'static) {
        self.update.insert(UpdateFlags::UPDATE);

        let self_id = self.id;
        self.updates
            .push(Box::new(move |cleanup| var.modify(self_id, modify, cleanup)));
    }

    /// Schedules an update notification.
    pub fn push_notify<T: 'static>(&mut self, sender: EventEmitter<T>, new_update: T) {
        self.update.insert(if sender.is_high_pressure() {
            UpdateFlags::UPD_HP
        } else {
            UpdateFlags::UPDATE
        });

        let self_id = self.id;
        self.updates
            .push(Box::new(move |cleanup| sender.notify(self_id, new_update, cleanup)));
    }

    /// Cleanup the previous update and applies the new one.
    ///
    /// Returns what update methods must be pumped.
    pub(crate) fn apply_updates(&mut self) -> UpdateFlags {
        for cleanup in self.cleanup.drain(..) {
            cleanup();
        }

        for update in self.updates.drain(..) {
            update(&mut self.cleanup);
        }

        std::mem::replace(&mut self.update, UpdateFlags::empty())
    }
}

/// An [App] extension.
pub trait AppExtension: 'static {
    /// Register this extension.
    fn register(&mut self, r: &mut AppRegister);

    /// Called when the OS sends an event to a device.
    fn on_device_event(&mut self, _device_id: DeviceId, _event: &DeviceEvent, _ctx: &mut EventContext) {}

    /// Called when the OS sends an event to a window.
    fn on_window_event(&mut self, _window_id: WindowId, _event: &WindowEvent, _ctx: &mut EventContext) {}

    /// Called every update after the Ui update.
    fn respond(&mut self) {}
}

impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    fn register(&mut self, r: &mut AppRegister) {
        self.0.register(r);
        self.1.register(r);
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut EventContext) {
        self.0.on_device_event(device_id, event, ctx);
        self.1.on_device_event(device_id, event, ctx);
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut EventContext) {
        self.0.on_window_event(window_id, event, ctx);
        self.1.on_window_event(window_id, event, ctx);
    }

    fn respond(&mut self) {
        self.0.respond();
        self.1.respond();
    }
}

impl AppExtension for () {
    fn register(&mut self, _: &mut AppRegister) {}
}

/// Identifies a service type.
pub trait Service: Clone + 'static {}

/// Defines and runs an application.
pub struct App<Exts: AppExtension> {
    extensions: Exts,
}

#[derive(Debug)]
pub(crate) enum WebRenderEvent {
    NewFrameReady(WindowId),
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
        let mut context = EventContext { ctx: register.ctx };

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                GEvent::NewEvents(_) => {
                    in_event_sequence = true;
                }
                GEvent::EventsCleared => {
                    in_event_sequence = false;
                }

                GEvent::WindowEvent { window_id, event } => {
                    extensions.on_window_event(window_id, &event, &mut context);
                }
                GEvent::UserEvent(WebRenderEvent::NewFrameReady(_window_id)) => {}
                GEvent::DeviceEvent { device_id, event } => {
                    extensions.on_device_event(device_id, &event, &mut context);
                }
                _ => {}
            }

            let mut event_update = context.ctx.apply_updates();
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
