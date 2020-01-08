use super::*;
use fnv::FnvHashMap;
use glutin::event::Event as GEvent;
use glutin::event_loop::EventLoopWindowTarget;
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

                window_id: None,
                widget_id: None,
                context_vars: FnvHashMap::default(),
                visited_vars: FnvHashMap::default(),

                update: UpdateFlags::empty(),
                window_update: UpdateFlags::empty(),
                updates: Vec::default(),
                cleanup: Vec::default(),
            },
        }
    }
}

pub struct EventContext<'a> {
    ctx: &'a mut AppContext,
    event_loop: &'a EventLoopWindowTarget<WebRenderEvent>,
}

impl<'a> EventContext<'a> {
    pub fn app_ctx(&self) -> &AppContext {
        self.ctx
    }

    /// Schedules an update notification.
    pub fn push_notify<T: 'static>(&mut self, sender: EventEmitter<T>, args: T) {
        self.ctx.push_notify(sender, args);
    }

    /// Gets a service reference if the service is registered in the application.
    pub fn try_service<S: Service>(&mut self) -> Option<&mut S> {
        self.ctx.try_service::<S>()
    }

    /// Gets a service reference.
    ///
    /// # Panics
    /// If  the service is not registered in application.
    pub fn service<S: Service>(&mut self) -> &mut S {
        self.ctx.service::<S>()
    }

    pub(crate) fn event_loop(&self) -> &EventLoopWindowTarget<WebRenderEvent> {
        self.event_loop
    }
}

impl AppRegister {
    /// Register a new event for the duration of the application context.
    pub fn register_event<E: Event>(&mut self, listener: EventListener<E::Args>) {
        self.ctx.events.insert(TypeId::of::<E>(), Box::new(listener));
    }

    /// Register a new service for the duration of the application context.
    pub fn register_service<S: Service>(&mut self, service: S) {
        self.ctx.services.insert(TypeId::of::<S>(), Box::new(service));
    }
}

type AnyMap = FnvHashMap<TypeId, Box<dyn Any>>;
enum UntypedRef {}
impl UntypedRef {
    fn pack<T>(r: &T) -> *const UntypedRef {
        (r as *const T) as *const UntypedRef
    }

    unsafe fn unpack<'a, T>(pointer: *const Self) -> &'a T {
        &*(pointer as *const T)
    }
}
enum ContextVarEntry {
    Value(*const UntypedRef, bool, u32),
    ContextVar(TypeId, *const UntypedRef, Option<(bool, u32)>),
}
type UpdateOnce = Box<dyn FnOnce(&mut Vec<Box<dyn FnOnce()>>)>;
type CleanupOnce = Box<dyn FnOnce()>;

uid! {
   /// Unique id of an [AppContext] instance.
   pub struct AppContextId(_);

   /// Unique id of a widget.
   pub struct WidgetId(_);
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

    window_id: Option<WindowId>,
    widget_id: Option<WidgetId>,
    context_vars: FnvHashMap<TypeId, ContextVarEntry>,
    visited_vars: AnyMap,

    update: UpdateFlags,
    window_update: UpdateFlags,
    updates: Vec<UpdateOnce>,
    cleanup: Vec<CleanupOnce>,
}

trait AnyExt {
    #[inline]
    unsafe fn downcast_ref_unchecked<T: 'static>(&self) -> &T {
        &*(self as *const Self as *const T)
    }

    #[inline]
    unsafe fn downcast_mut_unchecked<T: 'static>(&mut self) -> &mut T {
        &mut *(self as *mut Self as *mut T)
    }
}
impl AnyExt for Box<dyn Any> {}

impl AppContext {
    /// Gets this context instance id. There is usually a single context
    /// per application but more then one context can happen in tests.
    pub fn id(&self) -> AppContextId {
        self.id
    }

    /// Creates an event listener if the event is registered in the application.
    pub fn try_listen<E: Event>(&self) -> Option<EventListener<E::Args>> {
        if let Some(any) = self.events.get(&TypeId::of::<E>()) {
            // SAFETY: This is safe because args are always the same type as key in
            // `AppRegister::register_event` witch is the only place where insertion occurs.
            Some(unsafe { any.downcast_ref_unchecked::<EventListener<E::Args>>().clone() })
        } else {
            None
        }
    }

    /// Creates an event listener.
    ///
    /// # Panics
    /// If the event is not registered in the application.
    pub fn listen<E: Event>(&self) -> EventListener<E::Args> {
        self.try_listen::<E>()
            .unwrap_or_else(|| panic!("event `{}` is required", type_name::<E>()))
    }

    /// Gets a service reference if the service is registered in the application.
    pub fn try_service<S: Service>(&mut self) -> Option<&mut S> {
        if let Some(any) = self.events.get_mut(&TypeId::of::<S>()) {
            // SAFETY: This is safe because services are always the same type as key in
            // `AppRegister::register_service` witch is the only place where insertion occurs.
            Some(unsafe { any.downcast_mut_unchecked::<S>() })
        } else {
            None
        }
    }

    /// Gets a service reference.
    ///
    /// # Panics
    /// If  the service is not registered in application.
    pub fn service<S: Service>(&mut self) -> &mut S {
        self.try_service::<S>()
            .unwrap_or_else(|| panic!("service `{}` is required", type_name::<S>()))
    }

    fn get_impl<T>(&self, var: TypeId, default: &'static T) -> (&T, bool, u32) {
        if let Some(ctx_var) = self.context_vars.get(&var) {
            match ctx_var {
                ContextVarEntry::Value(pointer, is_new, version) => {
                    // SAFETY: This is safe because context_vars are only inserted for the duration
                    // of [with_var] that holds the reference.
                    let value = unsafe { UntypedRef::unpack(*pointer) };
                    (value, *is_new, *version)
                }
                ContextVarEntry::ContextVar(var, default, meta_override) => {
                    // SAFETY: This is safe because default is a &'static T.
                    let r = self.get_impl(*var, unsafe { UntypedRef::unpack(*default) });
                    if let Some((is_new, version)) = *meta_override {
                        (r.0, is_new, version)
                    } else {
                        r
                    }
                }
            }
        } else {
            (default, false, 0)
        }
    }

    /// Gets the current window ID. Can be none if we are not in a window.
    pub fn try_window_id(&self) -> Option<WindowId> {
        self.window_id
    }

    /// Gets the current widget ID. Can be none if we are not in a widget.
    pub fn try_widget_id(&self) -> Option<WidgetId> {
        self.widget_id
    }

    /// Gets the current window ID.
    ///
    /// # Panics
    ///
    /// Panics if we are not in a window.
    ///
    /// Code inside an [UiNode] method should assume that this does not panic.
    pub fn window_id(&self) -> WindowId {
        self.window_id.expect("not in window")
    }

    /// Gets the current widget ID.
    ///
    /// # Panics
    ///
    /// Panics if we are not in a widget.
    ///
    /// Code inside an [UiNode] method should assume that this does not panic.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id.expect("not in widget")
    }

    /// Get the context var value or default.
    pub fn get<V: ContextVar>(&self) -> &V::Type {
        self.get_impl(TypeId::of::<V>(), V::default()).0
    }

    /// Gets if the context var value is new.
    pub fn get_is_new<V: ContextVar>(&self) -> bool {
        self.get_impl(TypeId::of::<V>(), V::default()).1
    }

    /// Gets the context var value version.
    pub fn get_version<V: ContextVar>(&self) -> u32 {
        self.get_impl(TypeId::of::<V>(), V::default()).2
    }

    /// Gets the context var value if it is new.
    pub fn get_new<V: ContextVar>(&self) -> Option<&V::Type> {
        let (value, is_new, _) = self.get_impl(TypeId::of::<V>(), V::default());

        if is_new {
            Some(value)
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

    /// Get the visited var value or panics if its not set.
    pub fn get_visited<V: VisitedVar>(&self) -> &V::Type {
        self.try_get_visited::<V>()
            .unwrap_or_else(|| panic!("visited var `{}` is required", type_name::<V>()))
    }

    /// Sets the visited var value for the rest of the update.
    pub fn set_visited<V: VisitedVar>(&mut self, value: V::Type) {
        self.visited_vars.insert(TypeId::of::<V>(), Box::new(value));
    }

    #[inline]
    fn with_var_impl(&mut self, type_id: TypeId, value: ContextVarEntry, f: impl FnOnce(&mut AppContext)) {
        let prev = self.context_vars.insert(type_id, value);

        f(self);

        if let Some(prev) = prev {
            self.context_vars.insert(type_id, prev);
        } else {
            self.context_vars.remove(&type_id);
        }
    }

    /// Runs a function with the context var.
    pub fn with_var<V: ContextVar>(
        &mut self,
        _: V,
        value: &V::Type,
        is_new: bool,
        version: u32,
        f: impl FnOnce(&mut AppContext),
    ) {
        self.with_var_impl(
            TypeId::of::<V>(),
            ContextVarEntry::Value(UntypedRef::pack(value), is_new, version),
            f,
        )
    }

    /// Runs a function with the context var set from another var.
    pub fn with_var_bind<V: ContextVar, O: SizedVar<V::Type>>(
        &mut self,
        context_var: V,
        var: &O,
        f: impl FnOnce(&mut AppContext),
    ) {
        use crate::core2::protected::BindInfo;

        match var.bind_info(self) {
            BindInfo::Var(value, is_new, version) => self.with_var(context_var, value, is_new, version, f),
            BindInfo::ContextVar(var, default, meta) => {
                let type_id = TypeId::of::<V>();
                let mut bind_to = var;
                let circular_binding = loop {
                    if let Some(ContextVarEntry::ContextVar(var, _, _)) = self.context_vars.get(&bind_to) {
                        bind_to = *var;
                        if bind_to == type_id {
                            break true;
                        }
                    } else {
                        break false;
                    }
                };

                if circular_binding {
                    eprintln!(
                        "circular context var binding `{}`=`{}` ignored",
                        type_name::<V>(),
                        type_name::<O>()
                    );
                } else {
                    self.with_var_impl(
                        type_id,
                        ContextVarEntry::ContextVar(var, UntypedRef::pack(default), meta),
                        f,
                    )
                }
            }
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
    pub fn push_notify<T: 'static>(&mut self, sender: EventEmitter<T>, args: T) {
        self.update.insert(if sender.is_high_pressure() {
            UpdateFlags::UPD_HP
        } else {
            UpdateFlags::UPDATE
        });

        let self_id = self.id;
        self.updates
            .push(Box::new(move |cleanup| sender.notify(self_id, args, cleanup)));
    }

    /// Schedules a switch variable index change for the next update.
    pub fn push_switch<T: 'static>(&mut self, var: impl SwitchVar<T>, new_index: usize) {
        self.update.insert(UpdateFlags::UPDATE);
        self.updates
            .push(Box::new(move |cleanup| var.modify(new_index, cleanup)));
    }

    /// Schedules a layout update.
    pub fn push_layout(&mut self) {
        self.window_update |= UpdateFlags::LAYOUT;
        self.update |= UpdateFlags::LAYOUT;
    }

    /// Schedules a new render.
    pub fn push_frame(&mut self) {
        self.window_update |= UpdateFlags::RENDER;
        self.update |= UpdateFlags::RENDER;
    }

    /// Applies a window update collecting the window specific [UpdateFlags]
    pub(crate) fn window_update(
        &mut self,
        window_id: WindowId,
        root_id: WidgetId,
        update: impl FnOnce(&mut AppContext),
    ) -> UpdateFlags {
        self.window_update = UpdateFlags::empty();
        self.window_id = Some(window_id);
        self.widget_id = Some(root_id);

        update(self);

        self.window_id = None;
        self.widget_id = None;
        std::mem::replace(&mut self.window_update, UpdateFlags::empty())
    }

    /// Applies a widget update.
    pub(crate) fn widget_update(&mut self, id: WidgetId, update: impl FnOnce(&mut AppContext)) {
        let parent_id = std::mem::replace(&mut self.widget_id, Some(id));

        update(self);

        self.widget_id = parent_id;
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

        self.visited_vars.clear();

        std::mem::replace(&mut self.update, UpdateFlags::empty())
    }
}

#[derive(Default)]
pub(crate) struct AppContextOwnership {
    id: std::cell::Cell<Option<AppContextId>>,
}

impl AppContextOwnership {
    pub fn new(context: AppContextId) -> Self {
        AppContextOwnership {
            id: std::cell::Cell::new(Some(context)),
        }
    }

    pub fn check(&self, id: AppContextId, already_owned_error: std::fmt::Arguments) {
        if let Some(ctx_id) = self.id.get() {
            if ctx_id != id {
                panic!("{}", already_owned_error)
            }
        } else {
            self.id.set(Some(id));
        }
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
    fn respond(&mut self, _ctx: &mut EventContext) {}
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

    fn respond(&mut self, ctx: &mut EventContext) {
        self.0.respond(ctx);
        self.1.respond(ctx);
    }
}

impl AppExtension for () {
    fn register(&mut self, _: &mut AppRegister) {}
}

/// Identifies a service type.
pub trait Service: 'static {}

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
        let event_loop = EventLoop::with_user_event();

        let mut extensions = (AppWindows::new(event_loop.create_proxy()), self.extensions);

        let mut register = AppRegister::default();
        extensions.register(&mut register);

        let mut in_sequence = false;
        let mut sequence_update = UpdateFlags::empty();
        let mut ctx = register.ctx;

        event_loop.run(move |event, event_loop, control_flow| {
            let mut context = EventContext {
                ctx: &mut ctx,
                event_loop,
            };

            *control_flow = ControlFlow::Wait;
            match event {
                GEvent::NewEvents(_) => {
                    in_sequence = true;
                }
                GEvent::EventsCleared => {
                    in_sequence = false;
                }

                GEvent::WindowEvent { window_id, event } => {
                    extensions.on_window_event(window_id, &event, &mut context);
                }
                GEvent::UserEvent(WebRenderEvent::NewFrameReady(window_id)) => {
                    extensions.0.new_frame_ready(window_id);
                }
                GEvent::DeviceEvent { device_id, event } => {
                    extensions.on_device_event(device_id, &event, &mut context);
                }
                _ => {}
            }

            let mut event_update = context.ctx.apply_updates();

            if event_update.contains(UpdateFlags::UPD_HP) {
                event_update.remove(UpdateFlags::UPD_HP);
                extensions.0.update_hp(context.ctx);
            }
            if event_update.contains(UpdateFlags::UPDATE) {
                event_update.remove(UpdateFlags::UPDATE);
                extensions.0.update(context.ctx);
            }

            let ui_node_update = context.ctx.apply_updates();

            sequence_update |= event_update | ui_node_update;

            extensions.respond(&mut context);

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
