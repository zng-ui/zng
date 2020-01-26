use super::*;
use fnv::FnvHashMap;
use glutin::event_loop::EventLoopProxy;
use glutin::event_loop::EventLoopWindowTarget;
use std::any::{type_name, Any, TypeId};
use std::cell::RefCell;
use std::mem;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use webrender::api::RenderApi;

type AnyMap = FnvHashMap<TypeId, Box<dyn Any>>;

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

type UpdateOnce = Box<dyn FnOnce(&mut Vars, &mut Events, &mut Vec<CleanupOnce>)>;

type CleanupOnce = Box<dyn FnOnce()>;

uid! {
   /// Unique id of a widget.
   pub struct WidgetId(_);
}

/// Required updates for a window layout and frame.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UpdateDisplayRequest {
    /// No update required.
    None,
    /// No re-layout required, just render again.
    Render,
    /// Full update required, re-layout then render again.
    Layout,
}

impl Default for UpdateDisplayRequest {
    #[inline]
    fn default() -> Self {
        UpdateDisplayRequest::None
    }
}

impl std::ops::BitOrAssign for UpdateDisplayRequest {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        use UpdateDisplayRequest::*;
        match rhs {
            Layout => *self = Layout,
            Render => {
                if *self == None {
                    *self = Render;
                }
            }
            _ => {}
        }
    }
}

impl std::ops::BitOr for UpdateDisplayRequest {
    type Output = Self;

    #[inline]
    fn bitor(mut self, rhs: Self) -> Self {
        self |= rhs;
        self
    }
}

/// Updates that where requested during a previous round of
/// updates.
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct UpdateRequest {
    /// If should notify all that variables or events change occurred.
    pub update: bool,
    /// If should notify all that variables or events change occurred using
    /// the hight-pressure band when applicable.
    pub update_hp: bool,
}

impl std::ops::BitOrAssign for UpdateRequest {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.update |= rhs.update;
        self.update_hp |= rhs.update_hp;
    }
}

impl std::ops::BitOr for UpdateRequest {
    type Output = Self;

    #[inline]
    fn bitor(mut self, rhs: Self) -> Self {
        self |= rhs;
        self
    }
}

impl UpdateDisplayRequest {
    /// If contains any update.
    #[inline]
    pub fn is_some(self) -> bool {
        !self.is_none()
    }

    /// If does not contain any update.
    #[inline]
    pub fn is_none(self) -> bool {
        self == UpdateDisplayRequest::None
    }
}

#[derive(Clone)]
pub struct UpdateNotifier {
    event_loop: EventLoopProxy<AppEvent>,
}

impl UpdateNotifier {
    #[inline]
    pub fn new(event_loop: EventLoopProxy<AppEvent>) -> Self {
        UpdateNotifier { event_loop }
    }

    fn update() -> &'static AtomicBool {
        static UPDATE: AtomicBool = AtomicBool::new(false);
        &UPDATE
    }

    fn update_hp() -> &'static AtomicBool {
        static UPDATE_HP: AtomicBool = AtomicBool::new(false);
        &UPDATE_HP
    }

    #[inline]
    pub fn push_update(&self) {
        let update = Self::update().swap(true, atomic::Ordering::Relaxed);
        if !update && !Self::update_hp().load(atomic::Ordering::Relaxed) {
            let _ = self.event_loop.send_event(AppEvent::Update);
        }
    }

    #[inline]
    pub fn push_update_hp(&self) {
        let update_hp = Self::update_hp().swap(true, atomic::Ordering::Relaxed);
        if !Self::update().load(atomic::Ordering::Relaxed) && !update_hp {
            let _ = self.event_loop.send_event(AppEvent::Update);
        }
    }

    #[inline]
    pub fn take_request() -> UpdateRequest {
        UpdateRequest {
            update: Self::update().swap(false, atomic::Ordering::Relaxed),
            update_hp: Self::update_hp().swap(false, atomic::Ordering::Relaxed),
        }
    }
}

/// Access to application variables.
///
/// Only a single instance of this type exists at a time.
pub struct Vars {
    context_vars: RefCell<FnvHashMap<TypeId, ContextVarEntry>>,
}

static VARS_ALIVE: AtomicBool = AtomicBool::new(false);

pub type ContextVarStageId = (Option<WidgetId>, u32);

impl Vars {
    /// Produces the instance of `Vars`. Only a single
    /// instance can exist at a time, panics if called
    /// again before droping the previous instance.
    pub fn instance() -> Self {
        if VARS_ALIVE.load(atomic::Ordering::Acquire) {
            panic!("only a single instance of `Vars` can exist at at time")
        }

        VARS_ALIVE.store(true, atomic::Ordering::Release);

        Vars {
            context_vars: RefCell::default(),
        }
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

impl Drop for Vars {
    fn drop(&mut self) {
        VARS_ALIVE.store(false, atomic::Ordering::Release);
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

    pub fn get_mut<S: StateKey>(&mut self, _key: S) -> Option<&S::Type> {
        if let Some(any) = self.map.get_mut(&TypeId::of::<S>()) {
            Some(any.downcast_mut::<S::Type>().unwrap())
        } else {
            None
        }
    }

    /// Sets a state key without value.
    pub fn flag<S: StateKey<Type = ()>>(&mut self, key: S) -> bool {
        self.set(key, ()).is_some()
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self, _key: S) -> bool {
        self.map.contains_key(&TypeId::of::<S>())
    }
}

/// Access to application events.
///
/// Only a single instance of this type exists at a time.
pub struct Events {
    events: AnyMap,
}

static EVENTS_ALIVE: AtomicBool = AtomicBool::new(false);

impl Events {
    /// Produces the instance of `Events`. Only a single
    /// instance can exist at a time, panics if called
    /// again before droping the previous instance.
    pub fn instance() -> Self {
        if EVENTS_ALIVE.load(atomic::Ordering::Acquire) {
            panic!("only a single instance of `Events` can exist at at time")
        }

        EVENTS_ALIVE.store(true, atomic::Ordering::Release);

        Events {
            events: Default::default(),
        }
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

impl Drop for Events {
    fn drop(&mut self) {
        EVENTS_ALIVE.store(false, atomic::Ordering::Release);
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

        if let Some(any) = self.app.get_mut(&type_id) {
            Some(any.downcast_mut::<S>().unwrap())
        } else if let Some(any) = self.window.get_mut(&type_id) {
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
#[derive(Default)]
pub struct Updates {
    update: UpdateRequest,
    display_update: UpdateDisplayRequest,
    win_display_update: UpdateDisplayRequest,
    updates: Vec<UpdateOnce>,
    cleanup: Vec<CleanupOnce>,
}

impl Updates {
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

    pub(crate) fn push_modify_impl(&mut self, modify: impl FnOnce(&mut Vars, &mut Vec<CleanupOnce>) + 'static) {
        self.update.update = true;
        self.updates
            .push(Box::new(move |assert, _, cleanup| modify(assert, cleanup)));
    }

    /// Schedules an update notification.
    pub fn push_notify<T: 'static>(&mut self, sender: EventEmitter<T>, args: T) {
        if sender.is_high_pressure() {
            self.update.update_hp = true;
        } else {
            self.update.update = true;
        }

        self.updates
            .push(Box::new(move |_, assert, cleanup| sender.notify(args, assert, cleanup)));
    }

    /// Schedules a switch variable index change for the next update.
    pub fn push_switch<T: VarValue>(&mut self, var: impl SwitchVar<T>, new_index: usize) {
        self.update.update = true;
        self.updates
            .push(Box::new(move |_, _, cleanup| var.modify(new_index, cleanup)));
    }

    /// Schedules a layout update.
    pub fn push_layout(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::Layout;
        self.display_update |= UpdateDisplayRequest::Layout;
    }

    /// Schedules a new frame.
    pub fn push_render(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::Render;
        self.display_update |= UpdateDisplayRequest::Render;
    }

    /// Cleanup the previous update and applies the new one.
    ///
    /// Returns what update methods must be pumped.
    ///
    /// # Assert Borrows
    ///
    /// When variable and event values are borrowed the instance of `Vars`/`Events` is
    /// imutable borrowed, so the requirement of borrowing both mutable here is an assert
    /// that all variable and event borrows have been dropped.
    pub fn apply_updates(
        &mut self,
        assert_vars_not_borrowed: &mut Vars,
        assert_events_not_borrowed: &mut Events,
    ) -> (UpdateRequest, UpdateDisplayRequest) {
        for cleanup in self.cleanup.drain(..) {
            cleanup();
        }

        for update in self.updates.drain(..) {
            update(assert_vars_not_borrowed, assert_events_not_borrowed, &mut self.cleanup);
        }

        (
            mem::replace(&mut self.update, UpdateRequest::default()),
            mem::replace(&mut self.display_update, UpdateDisplayRequest::None),
        )
    }
}

/// Owner of [AppContext] objects.
///
/// Because [Vars] and [Events] can only have one instance
/// and this `struct` owns both you can only have one instance
/// of this at a time.
pub struct OwnedAppContext {
    app_state: StageState,
    vars: Vars,
    events: Events,
    services: Services,
    updates: Updates,
}

impl OwnedAppContext {
    /// Produces the single instance of `AppContext`.
    pub fn instance() -> Self {
        OwnedAppContext {
            app_state: StageState::default(),
            vars: Vars::instance(),
            events: Events::instance(),
            services: Services::default(),
            updates: Updates::default(),
        }
    }

    pub fn borrow_init(&mut self, event_loop: EventLoopProxy<AppEvent>) -> AppInitContext {
        AppInitContext {
            app_state: &mut self.app_state,
            event_loop,
            vars: &self.vars,
            events: &mut self.events,
            services: &mut self.services,
            updates: &mut self.updates,
        }
    }

    pub fn borrow<'a>(&'a mut self, event_loop: &'a EventLoopWindowTarget<AppEvent>) -> AppContext<'a> {
        AppContext {
            app_state: &mut self.app_state,
            vars: &self.vars,
            events: &self.events,
            services: &mut self.services,
            updates: &mut self.updates,
            event_loop,
        }
    }

    pub fn apply_updates(&mut self) -> (UpdateRequest, UpdateDisplayRequest) {
        self.updates.apply_updates(&mut self.vars, &mut self.events)
    }
}

/// App extension initialization context.
pub struct AppInitContext<'a> {
    /// State that lives for the duration of the application.
    pub app_state: &'a mut StageState,

    pub event_loop: EventLoopProxy<AppEvent>,

    pub vars: &'a Vars,
    pub events: &'a mut Events,
    pub services: &'a mut Services,
    pub updates: &'a mut Updates,
}

/// Full application context.
pub struct AppContext<'a> {
    /// State that lives for the duration of the application.
    pub app_state: &'a mut StageState,

    pub vars: &'a Vars,
    pub events: &'a Events,
    pub services: &'a mut Services,
    pub updates: &'a mut Updates,

    pub event_loop: &'a EventLoopWindowTarget<AppEvent>,
}

/// Instances of services associated with a window.
pub type WindowServices = AnyMap;

/// Custom state associated with a window.
pub type WindowState = StageState;

impl<'a> AppContext<'a> {
    /// Initializes state and services for a new iwndow.
    pub fn new_window(&mut self, window_id: WindowId, render_api: &Arc<RenderApi>) -> (WindowState, WindowServices) {
        let mut window_state = StageState::default();
        let mut window_services = FnvHashMap::default();

        let window_init = mem::replace(&mut self.services.window_init, Vec::default());
        for (key, new) in window_init.iter() {
            self.window_context(window_id, &mut window_state, &mut window_services, render_api, |ctx| {
                let service = new(ctx);
                ctx.services.window.insert(*key, service);
            });
        }
        self.services.window_init = window_init;

        (window_state, window_services)
    }

    /// Runs a function `f` within the context of a window.
    pub fn window_context(
        &mut self,
        window_id: WindowId,
        window_state: &mut WindowState,
        window_services: &mut AnyMap,
        render_api: &Arc<RenderApi>,
        f: impl FnOnce(&mut WindowContext),
    ) -> UpdateDisplayRequest {
        self.updates.win_display_update = UpdateDisplayRequest::None;
        mem::swap(&mut self.services.window, window_services);

        let mut event_state = StageState::default();
        let mut ctx = WindowContext {
            window_id,
            render_api,
            app_state: self.app_state,
            window_state,
            event_state: &mut event_state,
            vars: self.vars,
            events: self.events,
            services: self.services,
            updates: self.updates,
        };

        f(&mut ctx);

        mem::swap(window_services, &mut self.services.window);
        mem::replace(&mut self.updates.win_display_update, UpdateDisplayRequest::None)
    }
}

/// A window context.
pub struct WindowContext<'a> {
    window_id: WindowId,
    pub render_api: &'a Arc<RenderApi>,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StageState,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StageState,

    /// State that lives for the duration of the event.
    pub event_state: &'a mut StageState,

    pub vars: &'a Vars,
    pub events: &'a Events,
    pub services: &'a mut Services,

    pub updates: &'a mut Updates,
}

impl<'a> WindowContext<'a> {
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Runs a function `f` within the context of a widget.
    pub fn widget_context(&mut self, widget_id: WidgetId, f: impl FnOnce(&mut WidgetContext)) {
        let mut ctx = WidgetContext {
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
pub struct WidgetContext<'a> {
    window_id: WindowId,
    widget_id: WidgetId,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StageState,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StageState,

    /// State that lives for the duration of the event.
    pub event_state: &'a mut StageState,

    pub vars: &'a Vars,
    pub events: &'a Events,
    pub services: &'a mut Services,

    pub updates: &'a mut Updates,
}

impl<'a> WidgetContext<'a> {
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Runs a function `f` within the context of a widget.
    pub fn widget_context(&mut self, widget_id: WidgetId, f: impl FnOnce(&mut WidgetContext<'a>)) {
        let widget_id = mem::replace(&mut self.widget_id, widget_id);

        f(self);

        self.widget_id = widget_id;
    }
}
