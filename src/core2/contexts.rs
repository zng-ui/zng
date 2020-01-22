use super::*;
use fnv::FnvHashMap;
use glutin::event_loop::EventLoopWindowTarget;
use std::any::{type_name, Any, TypeId};
use std::cell::RefCell;
use std::mem;
use std::sync::Arc;
use webrender::api::RenderApi;

pub(crate) type AnyMap = FnvHashMap<TypeId, Box<dyn Any>>;

type WindowServicesInit = Vec<(TypeId, Box<dyn Fn(&WindowContext) -> Box<dyn Any>>)>;

enum AnyRef {}
impl AnyRef {
    fn pack<T>(r: &T) -> *const AnyRef {
        (r as *const T) as *const AnyRef
    }

    unsafe fn unpack<'a, T>(pointer: *const Self) -> &'a T {
        &*(pointer as *const T)
    }
}

enum ContextVarEntry {
    Value(*const AnyRef, bool, u32),
    ContextVar(TypeId, *const AnyRef, Option<(bool, u32)>),
}

type UpdateOnce = Box<dyn FnOnce(&mut Vec<CleanupOnce>)>;

type CleanupOnce = Box<dyn FnOnce()>;

uid! {
   /// Unique id of an [AppContext] instance.
   pub struct AppId(_);

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

/// [Variables](std::core2::Vars) access and context.
pub struct Vars {
    app_id: AppId,
    context_vars: RefCell<FnvHashMap<TypeId, ContextVarEntry>>,
}

pub type ContextVarStageId = (Option<WidgetId>, u32);

impl Vars {
    pub fn new(app_id: AppId) -> Self {
        Vars {
            app_id,
            context_vars: RefCell::default(),
        }
    }

    pub fn app_id(&self) -> AppId {
        self.app_id
    }

    /// Unique id of the context var stage.
    pub fn context_id(&self) -> ContextVarStageId {
        todo!()
    }

    /// Runs a function with the context var.
    pub fn with_context<V: ContextVar>(&self, _: V, value: &V::Type, is_new: bool, version: u32, f: impl FnOnce()) {
        self.with_context_impl(
            TypeId::of::<V>(),
            ContextVarEntry::Value(AnyRef::pack(value), is_new, version),
            f,
        )
    }

    /// Runs a function with the context var set from another var.
    pub fn with_context_bind<V: ContextVar, O: ObjVar<V::Type>>(&self, context_var: V, var: &O, f: impl FnOnce()) {
        use crate::core2::protected::BindInfo;

        match var.bind_info(self) {
            BindInfo::Var(value, is_new, version) => self.with_context(context_var, value, is_new, version, f),
            BindInfo::ContextVar(var, default, meta) => {
                let type_id = TypeId::of::<V>();
                let mut bind_to = var;

                let context_vars = self.context_vars.borrow();
                let circular_binding = loop {
                    if let Some(ContextVarEntry::ContextVar(var, _, _)) = context_vars.get(&bind_to) {
                        bind_to = *var;
                        if bind_to == type_id {
                            break true;
                        }
                    } else {
                        break false;
                    }
                };
                drop(context_vars);

                if circular_binding {
                    eprintln!(
                        "circular context var binding `{}`=`{}` ignored",
                        type_name::<V>(),
                        type_name::<O>()
                    );
                } else {
                    self.with_context_impl(
                        type_id,
                        ContextVarEntry::ContextVar(var, AnyRef::pack(default), meta),
                        f,
                    )
                }
            }
        }
    }

    /// Get the context var value or default.
    pub fn context<V: ContextVar>(&self) -> &V::Type {
        self.context_impl(TypeId::of::<V>(), V::default()).0
    }

    /// Gets if the context var value is new.
    pub fn context_is_new<V: ContextVar>(&self) -> bool {
        self.context_impl(TypeId::of::<V>(), V::default()).1
    }

    /// Gets the context var value version.
    pub fn context_version<V: ContextVar>(&self) -> u32 {
        self.context_impl(TypeId::of::<V>(), V::default()).2
    }

    /// Gets the context var value if it is new.
    pub fn context_update<V: ContextVar>(&self) -> Option<&V::Type> {
        let (value, is_new, _) = self.context_impl(TypeId::of::<V>(), V::default());

        if is_new {
            Some(value)
        } else {
            None
        }
    }

    #[inline]
    fn with_context_impl(&self, type_id: TypeId, value: ContextVarEntry, f: impl FnOnce()) {
        let prev = self.context_vars.borrow_mut().insert(type_id, value);

        f();

        let mut ctxs = self.context_vars.borrow_mut();
        if let Some(prev) = prev {
            ctxs.insert(type_id, prev);
        } else {
            ctxs.remove(&type_id);
        }
    }

    fn context_impl<T>(&self, var: TypeId, default: &'static T) -> (&T, bool, u32) {
        let ctxs = self.context_vars.borrow();

        if let Some(ctx_var) = ctxs.get(&var) {
            match ctx_var {
                ContextVarEntry::Value(pointer, is_new, version) => {
                    // SAFETY: This is safe because `TypeId` keys are always associated
                    // with the same type of reference.
                    let value = unsafe { AnyRef::unpack(*pointer) };
                    (value, *is_new, *version)
                }
                ContextVarEntry::ContextVar(var, default, meta_override) => {
                    // SAFETY: This is safe because default is a &'static T.
                    let r = self.context_impl(*var, unsafe { AnyRef::unpack(*default) });
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
}

#[derive(Default)]
pub(crate) struct AppOwnership {
    id: std::cell::Cell<Option<AppId>>,
}

impl AppOwnership {
    pub fn new(context: AppId) -> Self {
        AppOwnership {
            id: std::cell::Cell::new(Some(context)),
        }
    }

    pub fn check(&self, id: AppId, already_owned_error: impl FnOnce() -> String) {
        if let Some(ctx_id) = self.id.get() {
            if ctx_id != id {
                panic!("{}", already_owned_error())
            }
        } else {
            self.id.set(Some(id));
        }
    }
}

/// A key to a value in a [StateStore].
///
/// The type that implements this trait is the key. You
/// can use the [state_key!] macro.
pub trait StateKey: 'static {
    /// The value type.
    type Type: 'static;
}

/// A map of [state keys](StateKey) to values of their associated types that exists for
/// a stage of the application.
#[derive(Default)]
pub struct StageState {
    map: FnvHashMap<TypeId, Box<dyn Any>>,
}

impl StageState {
    pub fn set<S: StateKey>(&mut self, _key: S, value: S::Type) -> Option<S::Type> {
        self.map
            .insert(TypeId::of::<S>(), Box::new(value))
            .map(|any| *any.downcast::<S::Type>().unwrap())
    }

    pub fn get<S: StateKey>(&self, _key: S) -> Option<&S::Type> {
        if let Some(any) = self.map.get(&TypeId::of::<S>()) {
            Some(any.downcast_ref::<S::Type>().unwrap())
        } else {
            None
        }
    }

    pub fn get_mut<S: StateKey>(&self, _key: S) -> Option<&S::Type> {
        if let Some(any) = self.map.get_mut(&TypeId::of::<S>()) {
            Some(any.downcast_mut::<S::Type>().unwrap())
        } else {
            None
        }
    }

    /// Sets a state key without value.
    pub fn flag<S: StateKey<Type=()>>(&mut self, key: S) -> bool {
        self.set(key, ()).is_some()
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type=()>>(&self, _key: S) -> bool {
        self.map.contains_key(&TypeId::of::<S>())
    }
}

/// Access to application events.
pub struct Events {
    app_id: AppId,
    events: AnyMap,
}

impl Events {
    pub fn new(app_id: AppId) -> Self {
        Events {
            app_id,
            events: Default::default(),
        }
    }

    pub fn app_id(&self) -> AppId {
        self.app_id
    }

    /// Register a new event for the duration of the application.
    pub fn register<E: Event>(&mut self, listener: EventListener<E::Args>) {
        self.events.insert(TypeId::of::<E>(), Box::new(listener));
    }

    /// Creates an event listener if the event is registered in the application.
    pub fn try_listen<E: Event>(&self) -> Option<EventListener<E::Args>> {
        if let Some(any) = self.events.get(&TypeId::of::<E>()) {
            // SAFETY: This is safe because args are always the same type as key in
            // `AppRegister::register_event` witch is the only place where insertion occurs.
            Some(any.downcast_ref::<EventListener<E::Args>>().unwrap().clone())
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
}

/// Access to application services.
#[derive(Default)]
pub struct Services {
    app: AnyMap,
    window_init: WindowServicesInit,
    window: AnyMap,
}

impl Services {
    /// Register a new service for the duration of the application context.
    pub fn register<S: Service>(&mut self, service: S) {
        self.app.insert(TypeId::of::<S>(), Box::new(service));
    }

    /// Register a new window service initializer.
    ///
    /// Window services have diferent instances for each window and exist for the duration
    /// of that window. The `new` closure is called when a new window is created to
    pub fn register_wnd<S: Service>(&mut self, new: impl Fn(&WindowContext) -> S + 'static) {
        self.window_init
            .push((TypeId::of::<S>(), Box::new(move |ctx| Box::new(new(ctx)))));
    }

    /// Gets a service reference if the service is registered in the application.
    pub fn get<S: Service>(&mut self) -> Option<&mut S> {
        let type_id = TypeId::of::<S>();

        if let Some(any) = self.app.get(&type_id).or_else(|| self.window.get(&type_id)) {
            Some(any.downcast_mut::<S>().unwrap())
        } else {
            None
        }
    }

    /// Gets a service reference.
    ///
    /// # Panics
    /// If  the service is not registered in application.
    pub fn require<S: Service>(&mut self) -> &mut S {
        self.get::<S>()
            .unwrap_or_else(|| panic!("service `{}` is required", type_name::<S>()))
    }
}

/// Schedule of actions to apply after an Ui update.
pub struct Updates {
    app_id: AppId,
    update: UpdateFlags,
    window_update: UpdateFlags,
    updates: Vec<UpdateOnce>,
    cleanup: Vec<CleanupOnce>,
}

impl Updates {
    pub fn new(app_id: AppId) -> Self {
        Updates {
            app_id,
            update: UpdateFlags::empty(),
            window_update: UpdateFlags::empty(),
            updates: Vec::default(),
            cleanup: Vec::default(),
        }
    }

    pub fn app_id(&self) -> AppId {
        self.app_id
    }

    /// Schedules a variable change for the next update.
    pub fn push_set<T: VarValue>(&mut self, var: &impl ObjVar<T>, new_value: T) -> Result<(), VarIsReadOnly> {
        var.push_set(new_value, self)
    }

    /// Schedules a variable modification for the next update.
    pub fn push_modify<T: VarValue>(
        &mut self,
        var: impl Var<T>,
        modify: impl ModifyFnOnce<T>,
    ) -> Result<(), VarIsReadOnly> {
        var.push_modify(modify, self)
    }

    pub(crate) fn push_modify_impl(&mut self, modify: impl FnOnce(&mut Vec<CleanupOnce>) + 'static) {
        self.update.insert(UpdateFlags::UPDATE);
        self.updates.push(Box::new(modify));
    }

    /// Schedules an update notification.
    pub fn push_notify<T: 'static>(&mut self, sender: EventEmitter<T>, args: T) {
        self.update.insert(if sender.is_high_pressure() {
            UpdateFlags::UPD_HP
        } else {
            UpdateFlags::UPDATE
        });

        let self_id = self.app_id;
        self.updates
            .push(Box::new(move |cleanup| sender.notify(self_id, args, cleanup)));
    }

    /// Schedules a switch variable index change for the next update.
    pub fn push_switch<T: VarValue>(&mut self, var: impl SwitchVar<T>, new_index: usize) {
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

        //self.visited_vars.clear(); TODO

        std::mem::replace(&mut self.update, UpdateFlags::empty())
    }
}

/// Object from witch [AppContext] can be borrowed.
pub struct OwnedAppContext {
    app_id: AppId,
    app_state: StageState,
    vars: Vars,
    events: Events,
    services: Services,
}

impl OwnedAppContext {
    pub fn new() -> Self {
        let app_id = AppId::new_unique();
        OwnedAppContext {
            app_id,
            app_state: StageState::default(),
            vars: Vars::new(app_id),
            events: Events::new(app_id),
            services: Services::default(),
        }
    }

    pub fn borrow(&mut self) -> AppContext {
        AppContext {
            app_id: self.app_id,
            app_state: &mut self.app_state,
            vars: &self.vars,
            events: &mut self.events,
            services: &mut self.services,
        }
    }
}

/// Full application context.
pub struct AppContext<'v, 'sa, 'e, 's> {
    app_id: AppId,

    /// State that lives for the duration of the application.
    pub app_state: &'sa mut StageState,

    pub vars: &'v Vars,
    pub events: &'e mut Events,
    pub services: &'s mut Services,
}

impl<'v, 'sa, 'e, 's> AppContext<'v, 'sa, 'e, 's> {
    pub fn app_id(&self) -> AppId {
        self.app_id
    }

    /// Runs a function `f` within the context of an application extension event handler.
    pub fn event_context(
        &mut self,
        event_loop: &EventLoopWindowTarget<WebRenderEvent>,
        f: impl FnOnce(&mut AppEventContext),
    ) -> UpdateFlags {
        let mut updates = Updates::new(self.app_id);

        let mut ctx = AppEventContext {
            app_id: self.app_id,
            app_state: self.app_state,
            vars: self.vars,
            events: self.events,
            services: self.services,
            updates: &mut updates,
            event_loop,
        };

        f(&mut ctx);

        updates.apply_updates()
    }

    /// Runs a function `f` within the context of a window.
    pub fn window_context(
        &mut self,
        window_id: WindowId,
        window_state: &mut StageState,
        window_services: &mut AnyMap,
        render_api: Arc<RenderApi>,
        updates: &mut Updates,
        f: impl FnOnce(&mut WindowContext),
    ) -> UpdateFlags {
        updates.window_update = UpdateFlags::empty();
        std::mem::swap(&mut self.services.window, window_services);

        let mut event_state = StageState::default();
        let mut ctx = WindowContext {
            app_id: self.app_id,
            window_id,
            render_api,
            app_state: self.app_state,
            window_state,
            event_state: &mut event_state,
            vars: self.vars,
            events: self.events,
            services: self.services,
            updates,
        };

        f(&mut ctx);

        std::mem::swap(window_services, &mut self.services.window);
        std::mem::replace(&mut updates.window_update, UpdateFlags::empty())
    }
}

/// An application extension event context.
pub struct AppEventContext<'v, 'sa, 'e, 's, 'u, 'el> {
    app_id: AppId,

    /// State that lives for the duration of the application.
    pub app_state: &'sa mut StageState,

    pub vars: &'v Vars,
    pub events: &'e mut Events,
    pub services: &'s mut Services,

    pub updates: &'u mut Updates,

    pub event_loop: &'el EventLoopWindowTarget<WebRenderEvent>,
}

impl<'v, 'sa, 'e, 's, 'u, 'el> AppEventContext<'v, 'sa, 'e, 's, 'u, 'el> {
    pub fn app_id(&self) -> AppId {
        self.app_id
    }
}

/// A window context.
pub struct WindowContext<'v, 'sa, 'sw, 'sx, 'e, 's, 'u> {
    app_id: AppId,
    window_id: WindowId,
    render_api: Arc<RenderApi>,

    /// State that lives for the duration of the application.
    pub app_state: &'sa mut StageState,

    /// State that lives for the duration of the window.
    pub window_state: &'sw mut StageState,

    /// State that lives for the duration of the event.
    pub event_state: &'sx StageState,

    pub vars: &'v Vars,
    pub events: &'e mut Events,
    pub services: &'s mut Services,

    pub updates: &'u mut Updates,
}

impl<'v, 'sa, 'sw, 'sx, 'e, 's, 'u> WindowContext<'v, 'sa, 'sw, 'sx, 'e, 's, 'u> {
    pub fn app_id(&self) -> AppId {
        self.app_id
    }

    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    pub fn render_api(&self) -> &Arc<RenderApi> {
        &self.render_api
    }

    /// Instantiates window services.
    pub(crate) fn new_window_services(&self) -> AnyMap {
        self.services
            .window_init
            .iter()
            .map(|(key, new)| (*key, new(self)))
            .collect()
    }

    /// Runs a function `f` within the context of a widget.
    pub fn widget_context(&mut self, widget_id: WidgetId, f: impl FnOnce(&mut WidgetContext)) {
        let mut ctx = WidgetContext {
            app_id: self.app_id,
            window_id: self.window_id,
            widget_id,

            app_state: self.app_state,
            window_state: self.window_state,
            event_state: self.event_state,

            vars: self.vars,
            events: self.events,
            services: self.services,

            updates: self.updates,
        };

        f(&mut ctx);
    }
}

/// A widget context.
pub struct WidgetContext<'v, 'sa, 'sw, 'sx, 'e, 's, 'u> {
    app_id: AppId,
    window_id: WindowId,
    widget_id: WidgetId,

    /// State that lives for the duration of the application.
    pub app_state: &'sa mut StageState,

    /// State that lives for the duration of the window.
    pub window_state: &'sw mut StageState,

    /// State that lives for the duration of the event.
    pub event_state: &'sx StageState,

    pub vars: &'v Vars,
    pub events: &'e mut Events,
    pub services: &'s mut Services,

    pub updates: &'u mut Updates,
}

impl<'v, 'sa, 'sw, 'sx, 'e, 's, 'u> WidgetContext<'v, 'sa, 'sw, 'sx, 'e, 's, 'u> {
    pub fn app_id(&self) -> AppId {
        self.app_id
    }

    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Runs a function `f` within the context of a widget.
    pub fn widget_context(
        &mut self,
        widget_id: WidgetId,
        f: impl FnOnce(&mut WidgetContext<'v, 'sa, 'sw, 'sx, 'e, 's, 'u>),
    ) {
        let widget_id = mem::replace(&mut self.widget_id, widget_id);

        f(self);

        self.widget_id = widget_id;
    }
}
