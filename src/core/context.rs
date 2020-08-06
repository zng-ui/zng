//! Context information for app extensions, windows and widgets.

use crate::core::app::{AppEvent, EventLoopProxy, EventLoopWindowTarget};
use crate::core::event::{Event, EventEmitter, EventListener};
use crate::core::types::{WidgetId, WindowId};
use crate::core::var::*;
use fnv::{FnvHashMap, FnvHashSet};
use std::any::{type_name, Any, TypeId};
use std::cell::RefCell;
use std::mem;
use std::sync::atomic::{self, AtomicBool, AtomicU8};
use std::{marker::PhantomData, sync::Arc};
use webrender::api::RenderApi;

type AnyMap = FnvHashMap<TypeId, Box<dyn Any>>;

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
    /// the high-pressure band when applicable.
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

/// Out-of-band update request sender.
///
/// Use this to cause an update cycle without direct access to a context.
#[derive(Clone)]
pub struct UpdateNotifier {
    r: Arc<UpdateNotifierInner>,
}

struct UpdateNotifierInner {
    event_loop: EventLoopProxy,
    request: AtomicU8,
}

impl UpdateNotifier {
    #[inline]
    fn new(event_loop: EventLoopProxy) -> Self {
        UpdateNotifier {
            r: Arc::new(UpdateNotifierInner {
                event_loop,
                request: AtomicU8::new(0),
            }),
        }
    }

    const UPDATE: u8 = 0b01;
    const UPDATE_HP: u8 = 0b11;

    #[inline]
    fn set(&self, update: u8) {
        let old = self.r.request.fetch_or(update, atomic::Ordering::Relaxed);
        if old == 0 {
            self.r.event_loop.send_event(AppEvent::Update);
        }
    }

    /// Flags an update request and sends an update event
    /// if none was sent since the last one was consumed.
    #[inline]
    pub fn push_update(&self) {
        self.set(Self::UPDATE);
    }

    /// Flags an update request(high-pressure) and sends an update event
    /// if none was sent since the last one was consumed.
    #[inline]
    pub fn push_update_hp(&self) {
        self.set(Self::UPDATE_HP);
    }
}

macro_rules! singleton_assert {
    ($Singleton:ident) => {
        struct $Singleton {}

        impl $Singleton {
            fn flag() -> &'static AtomicBool {
                static ALIVE: AtomicBool = AtomicBool::new(false);
                &ALIVE
            }

            pub fn assert_new() -> Self {
                if Self::flag().load(atomic::Ordering::Acquire) {
                    panic!("only a single instance of `{}` can exist at at time", stringify!($Singleton))
                }

                Self::flag().store(true, atomic::Ordering::Release);

                $Singleton {}
            }
        }

        impl Drop for $Singleton {
            fn drop(&mut self) {
                Self::flag().store(false, atomic::Ordering::Release);
            }
        }
    };
}

singleton_assert!(SingletonEvents);
singleton_assert!(SingletonVars);

/// Access to application variables.
///
/// Only a single instance of this type exists at a time.
pub struct Vars {
    context_vars: RefCell<FnvHashMap<TypeId, ContextVarEntry>>,
    _singleton: SingletonVars,
}

pub type ContextVarStageId = (Option<WidgetId>, u32);

impl Vars {
    /// Produces the instance of `Vars`. Only a single
    /// instance can exist at a time, panics if called
    /// again before dropping the previous instance.
    pub fn instance() -> Self {
        Vars {
            context_vars: RefCell::default(),
            _singleton: SingletonVars::assert_new(),
        }
    }

    /// Unique id of the context var stage.
    pub fn context_id(&self) -> ContextVarStageId {
        todo!()
    }

    /// Runs a function with the context var.
    pub fn with_context<V: ContextVar>(&self, _: V, value: &V::Type, is_new: bool, version: u32, f: impl FnOnce()) {
        self.with_context_impl(TypeId::of::<V>(), ContextVarEntry::Value(AnyRef::pack(value), is_new, version), f)
    }

    /// Runs a function with the context var set from another var.
    pub fn with_context_bind<V: ContextVar, O: ObjVar<V::Type>>(&self, context_var: V, var: &O, f: impl FnOnce()) {
        use crate::core::var::protected::BindInfo;

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
                    error_println!("circular context var binding `{}`=`{}` ignored", type_name::<V>(), type_name::<O>());
                } else {
                    self.with_context_impl(type_id, ContextVarEntry::ContextVar(var, AnyRef::pack(default), meta), f)
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

/// A key to a value in a [`StateMap`].
///
/// The type that implements this trait is the key. You
/// can use the [`state_key!`](macro.state_key.html) macro.
pub trait StateKey: 'static {
    /// The value type.
    type Type: 'static;
}

pub use zero_ui_macros::state_key;

/// A map of [state keys](StateKey) to values of their associated types that exists for
/// a stage of the application.
#[derive(Debug, Default)]
pub struct StateMap {
    map: FnvHashMap<TypeId, Box<dyn Any>>,
}

impl StateMap {
    pub fn set<S: StateKey>(&mut self, key: S, value: S::Type) -> Option<S::Type> {
        let _ = key;
        self.map
            .insert(TypeId::of::<S>(), Box::new(value))
            .map(|any| *any.downcast::<S::Type>().unwrap())
    }

    /// Sets a value that is its own [`StateKey`].
    pub fn set_single<S: StateKey<Type = S>>(&mut self, value: S) -> Option<S> {
        self.map
            .insert(TypeId::of::<S>(), Box::new(value))
            .map(|any| *any.downcast::<S>().unwrap())
    }

    pub fn contains<S: StateKey>(&self, key: S) -> bool {
        let _ = key;
        self.map.contains_key(&TypeId::of::<S>())
    }

    pub fn get<S: StateKey>(&self, key: S) -> Option<&S::Type> {
        let _ = key;
        if let Some(any) = self.map.get(&TypeId::of::<S>()) {
            Some(any.downcast_ref::<S::Type>().unwrap())
        } else {
            None
        }
    }

    pub fn get_mut<S: StateKey>(&mut self, key: S) -> Option<&mut S::Type> {
        let _ = key;
        if let Some(any) = self.map.get_mut(&TypeId::of::<S>()) {
            Some(any.downcast_mut::<S::Type>().unwrap())
        } else {
            None
        }
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry<S: StateKey>(&mut self, key: S) -> StateMapEntry<S> {
        let _ = key;
        StateMapEntry {
            _key: PhantomData,
            entry: self.map.entry(TypeId::of::<S>()),
        }
    }

    /// Sets a state key without value.
    pub fn flag<S: StateKey<Type = ()>>(&mut self, key: S) -> bool {
        self.set(key, ()).is_some()
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self, key: S) -> bool {
        let _ = key;
        self.map.contains_key(&TypeId::of::<S>())
    }
}

/// A view into a single entry in a state map, which may either be vacant or occupied.
pub struct StateMapEntry<'a, S: StateKey> {
    _key: PhantomData<S>,
    entry: std::collections::hash_map::Entry<'a, TypeId, Box<dyn Any>>,
}
impl<'a, S: StateKey> StateMapEntry<'a, S> {
    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns a mutable reference to the value in the entry.
    pub fn or_insert(self, default: S::Type) -> &'a mut S::Type {
        self.entry.or_insert_with(|| Box::new(default)).downcast_mut::<S::Type>().unwrap()
    }

    /// Ensures a value is in the entry by inserting the result of the
    /// default function if empty, and returns a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> S::Type>(self, default: F) -> &'a mut S::Type {
        self.entry.or_insert_with(|| Box::new(default())).downcast_mut::<S::Type>().unwrap()
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts into the map.
    pub fn and_modify<F: FnOnce(&mut S::Type)>(self, f: F) -> Self {
        let entry = self.entry.and_modify(|a| f(a.downcast_mut::<S::Type>().unwrap()));
        StateMapEntry { _key: PhantomData, entry }
    }
}
impl<'a, S: StateKey> StateMapEntry<'a, S>
where
    S::Type: Default,
{
    /// Ensures a value is in the entry by inserting the default value if empty,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_default(self) -> &'a mut S::Type {
        self.entry
            .or_insert_with(|| Box::new(<S::Type as Default>::default()))
            .downcast_mut::<S::Type>()
            .unwrap()
    }
}

/// A [`StateMap`] that only takes one `usize` of memory if not used.
#[derive(Debug, Default)]
pub struct LazyStateMap {
    m: Option<Box<StateMap>>,
}

impl LazyStateMap {
    fn borrow_mut(&mut self) -> &mut StateMap {
        self.m.get_or_insert_with(|| Box::new(StateMap::default()))
    }

    pub fn contains<S: StateKey>(&self, key: S) -> bool {
        if let Some(m) = self.m.as_ref() {
            m.contains(key)
        } else {
            false
        }
    }

    pub fn set<S: StateKey>(&mut self, key: S, value: S::Type) -> Option<S::Type> {
        self.borrow_mut().set(key, value)
    }

    /// Sets a value that is its own [`StateKey`].
    pub fn set_single<S: StateKey<Type = S>>(&mut self, value: S) -> Option<S> {
        self.borrow_mut().set_single(value)
    }

    pub fn get<S: StateKey>(&self, key: S) -> Option<&S::Type> {
        self.m.as_ref().and_then(|m| m.get(key))
    }

    pub fn get_mut<S: StateKey>(&mut self, key: S) -> Option<&mut S::Type> {
        self.m.as_mut().and_then(|m| m.get_mut(key))
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    ///
    /// This causes lazy map initialization to an empty map even if you don't insert a value using the entry.
    pub fn entry<S: StateKey>(&mut self, key: S) -> StateMapEntry<S> {
        self.borrow_mut().entry(key)
    }

    /// Sets a state key without value.
    pub fn flag<S: StateKey<Type = ()>>(&mut self, key: S) -> bool {
        self.borrow_mut().flag(key)
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self, key: S) -> bool {
        self.get(key).is_some()
    }
}

/// Access to application events.
///
/// Only a single instance of this type exists at a time.
pub struct Events {
    events: AnyMap,
    _singleton: SingletonEvents,
}

impl Events {
    /// Produces the instance of `Events`. Only a single
    /// instance can exist at a time, panics if called
    /// again before dropping the previous instance.
    pub fn instance() -> Self {
        Events {
            events: Default::default(),
            _singleton: SingletonEvents::assert_new(),
        }
    }

    /// Register a new event for the duration of the application.
    pub fn register<E: Event>(&mut self, listener: EventListener<E::Args>) {
        assert_eq!(E::IS_HIGH_PRESSURE, listener.is_high_pressure());
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

/// Identifies an application level service type.
pub trait AppService: 'static {}

/// Identifies a window level service type.
pub trait WindowService: 'static {}

mod protected {
    use std::any::*;
    pub trait TypeBundle<'m> {
        type Borrowed;
        fn type_ids() -> Box<[TypeId]>;
        fn type_names() -> Box<[&'static str]>;
        fn downcast_mut(instances: Vec<&'m mut Box<dyn Any>>) -> Self::Borrowed;
    }
}
#[doc(hidden)]
pub trait AppServicesTuple<'m>: protected::TypeBundle<'m> {}
#[doc(hidden)]
pub trait WindowServicesTuple<'m>: protected::TypeBundle<'m> {}

macro_rules! impl_type_bundle {
    ($N:expr, $T:ident) => {
        //DONE
    };
    ($N:expr, $TLast:ident, $($T:ident),+) => {
        impl_type_bundle!($N - 1, $($T),+);
        impl_type_bundle!(impl $N, next -> $TLast, $(next -> $T),+);
    };
    (impl $N: expr, $($next:ident -> $T:ident),+) => {
        impl<'m, $($T: 'static),+> protected::TypeBundle<'m> for ($($T),+) {
            type Borrowed = ($(&'m mut $T),+);

            fn type_ids() -> Box<[TypeId]> {
                Box::new([
                    $(TypeId::of::<$T>(),)+
                ])
            }

            fn type_names() -> Box<[&'static str]> {
                Box::new([
                    $(type_name::<$T>(),)+
                ])
            }

            fn downcast_mut(instances: Vec<&'m mut Box<dyn Any>>) -> Self::Borrowed {
                let mut instances = instances.into_iter();
                #[allow(non_snake_case)]
                match ($(instances.$next()),+) {
                    ($(Some($T)),+) => ($($T.downcast_mut::<$T>().unwrap()),+),
                    _ => panic!("expected {} instances", $N)
                }
            }
        }

        impl<'m, $($T: AppService),+> AppServicesTuple<'m> for ($($T),+) {}

        impl<'m, $($T: WindowService),+> WindowServicesTuple<'m> for ($($T),+) {}
    };
}

mod type_bundle_impls {
    use super::*;
    impl_type_bundle!(16, T15, T14, T13, T12, T11, T10, T9, T8, T7, T6, T5, T4, T3, T2, T1, T0);
}

#[derive(Default)]
struct ServiceMap {
    m: AnyMap,
}

impl ServiceMap {
    pub fn insert<S: 'static>(&mut self, service: S) {
        self.m.insert(TypeId::of::<S>(), Box::new(service));
    }

    pub fn get<S: 'static>(&mut self) -> Option<&mut S> {
        let type_id = TypeId::of::<S>();
        self.m.get_mut(&type_id).map(|any| any.downcast_mut::<S>().unwrap())
    }

    pub fn req<S: 'static>(&mut self) -> &mut S {
        self.get::<S>()
            .unwrap_or_else(|| panic!("service `{}` is required", type_name::<S>()))
    }

    fn borrow_multi<'m, M: protected::TypeBundle<'m>>(&'m mut self) -> Result<M::Borrowed, &'static str> {
        let mut unique = FnvHashSet::default();
        let type_ids = M::type_ids();
        let mut instances = Vec::with_capacity(type_ids.len());

        for (i, tid) in type_ids.iter().enumerate() {
            if unique.insert(tid) {
                if let Some(any) = self.m.get_mut(tid) {
                    let p = any as *mut _;
                    instances.push(unsafe { &mut *p });
                } else {
                    return Err(M::type_names()[i]);
                }
            } else {
                panic!("service `{}` already borrowed", M::type_names()[i]);
            }
        }

        Ok(M::downcast_mut(instances))
    }

    pub fn get_multi<'m, M: protected::TypeBundle<'m>>(&'m mut self) -> Option<M::Borrowed> {
        self.borrow_multi::<M>().ok()
    }

    pub fn ret_multi<'m, M: protected::TypeBundle<'m>>(&'m mut self) -> M::Borrowed {
        self.borrow_multi::<M>().unwrap_or_else(|s| panic!("service `{}` is required", s))
    }
}

/// Application services with registration access.
pub struct AppServicesInit {
    m: AppServices,
}

impl Default for AppServicesInit {
    fn default() -> Self {
        AppServicesInit {
            m: AppServices { m: ServiceMap::default() },
        }
    }
}

impl AppServicesInit {
    /// Register a new service for the duration of the application context.
    pub fn register<S: AppService>(&mut self, service: S) {
        self.m.m.insert(service)
    }

    /// Moves the registered services into a new [`AppServices`].
    pub fn services(&mut self) -> &mut AppServices {
        &mut self.m
    }
}

/// Application services access.
pub struct AppServices {
    m: ServiceMap,
}

impl AppServices {
    /// Gets a service reference if the service is registered in the application.
    pub fn get<S: AppService>(&mut self) -> Option<&mut S> {
        self.m.get::<S>()
    }

    // Requires a service reference.
    ///
    /// # Panics
    /// If  the service is not registered in the application.
    pub fn req<S: AppService>(&mut self) -> &mut S {
        self.m.req::<S>()
    }

    /// Gets multiple service references if all services are registered in the application.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`AppService`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    /// If the same service type is requested more then once.
    pub fn get_multi<'m, M: AppServicesTuple<'m>>(&'m mut self) -> Option<M::Borrowed> {
        self.m.get_multi::<M>()
    }

    /// Requires multiple service references.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`AppService`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    /// If any of the services is not registered in the application.
    ///
    /// If the same service type is required more then once.
    pub fn req_multi<'m, M: AppServicesTuple<'m>>(&'m mut self) -> M::Borrowed {
        self.m.ret_multi::<M>()
    }
}

type WindowServicesBuilder = Vec<(TypeId, Box<dyn Fn(&WindowContext) -> Box<dyn Any>>)>;

/// Window services registration.
#[derive(Default)]
pub struct WindowServicesInit {
    builders: WindowServicesBuilder,
}

impl WindowServicesInit {
    /// Register a new window service initializer.
    ///
    /// Window services have different instances for each window and exist for the duration
    /// of that window. The `new` closure is called when a new window is created to
    pub fn register<S: WindowService>(&mut self, new: impl Fn(&WindowContext) -> S + 'static) {
        self.builders.push((TypeId::of::<S>(), Box::new(move |ctx| Box::new(new(ctx)))));
    }

    /// Initializes services for a window context.
    pub fn init(&self, ctx: &WindowContext) -> WindowServices {
        WindowServices {
            m: ServiceMap {
                m: self.builders.iter().map(|(k, v)| (*k, (v)(ctx))).collect(),
            },
        }
    }
}

/// Window services access.
pub struct WindowServices {
    m: ServiceMap,
}

impl WindowServices {
    /// Gets a service reference if the service is registered in the application.
    pub fn get<S: WindowService>(&mut self) -> Option<&mut S> {
        self.m.get::<S>()
    }

    // Requires a service reference.
    ///
    /// # Panics
    /// If  the service is not registered in application.
    pub fn req<S: WindowService>(&mut self) -> &mut S {
        self.m.req::<S>()
    }

    /// Gets multiple service references if all services are registered in the application.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`AppService`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    ///
    /// If the same service type is requested more then once.
    pub fn get_multi<'m, M: AppServicesTuple<'m>>(&'m mut self) -> Option<M::Borrowed> {
        self.m.get_multi::<M>()
    }

    /// Requires multiple service references.
    ///
    /// # Service Types
    ///
    /// The type argument must be a tuple (2..=16) of [`AppService`] implementers. No type must repeat.
    /// The return type is a tuple with each service type borrowed mutable (`&mut S`).
    ///
    /// # Panics
    ///
    /// If any of the services is not registered in the application.
    ///
    /// If the same service type is required more then once.
    pub fn req_multi<'m, M: AppServicesTuple<'m>>(&'m mut self) -> M::Borrowed {
        self.m.ret_multi::<M>()
    }
}

/// Executor access to [`Updates`].
pub struct OwnedUpdates {
    pub updates: Updates,
}

impl OwnedUpdates {
    pub fn new(event_loop: EventLoopProxy) -> Self {
        OwnedUpdates {
            updates: Updates::new(event_loop),
        }
    }

    /// Takes the update request generated by [`notifier`](Updates::notifier) since
    /// the last time it was taken.
    #[inline]
    pub fn take_request(&self) -> UpdateRequest {
        let request = self.updates.notifier.r.request.swap(0, atomic::Ordering::Relaxed);

        const UPDATE: u8 = UpdateNotifier::UPDATE;
        const UPDATE_HP: u8 = UpdateNotifier::UPDATE_HP;

        UpdateRequest {
            update: request & UPDATE == UPDATE,
            update_hp: request & UPDATE_HP == UPDATE_HP,
        }
    }

    /// Cleanup the previous update and applies the new one.
    ///
    /// Returns what update methods must be pumped.
    ///
    /// # Assert Borrows
    ///
    /// When variable and event values are borrowed the instance of `Vars`/`Events` is
    /// immutable borrowed, so the requirement of borrowing both mutable here is an assert
    /// that all variable and event borrows have been dropped.
    pub fn apply_updates(
        &mut self,
        assert_vars_not_borrowed: &mut Vars,
        assert_events_not_borrowed: &mut Events,
    ) -> (UpdateRequest, UpdateDisplayRequest) {
        for cleanup in self.updates.cleanup.drain(..) {
            cleanup();
        }

        for update in self.updates.updates.drain(..) {
            update(assert_vars_not_borrowed, assert_events_not_borrowed, &mut self.updates.cleanup);
        }

        (mem::take(&mut self.updates.update), mem::take(&mut self.updates.display_update))
    }
}

/// Schedule of actions to apply after an update.
///
/// An instance of this struct can be build by [`OwnedUpdates`].
pub struct Updates {
    notifier: UpdateNotifier,
    update: UpdateRequest,
    display_update: UpdateDisplayRequest,
    win_display_update: UpdateDisplayRequest,
    updates: Vec<UpdateOnce>,
    cleanup: Vec<CleanupOnce>,
}

impl Updates {
    fn new(event_loop: EventLoopProxy) -> Self {
        Updates {
            notifier: UpdateNotifier::new(event_loop),
            update: UpdateRequest::default(),
            display_update: UpdateDisplayRequest::None,
            win_display_update: UpdateDisplayRequest::None,
            updates: Vec::default(),
            cleanup: Vec::default(),
        }
    }

    /// Cloneable out-of-band notification sender.
    pub fn notifier(&self) -> &UpdateNotifier {
        &self.notifier
    }

    /// Schedules a variable change for the next update.
    pub fn push_set<T: VarValue>(&mut self, var: &impl ObjVar<T>, new_value: T, vars: &Vars) -> Result<(), VarIsReadOnly> {
        var.push_set(new_value, vars, self)
    }

    /// Schedules a variable modification for the next update.
    pub fn push_modify<T: VarValue>(
        &mut self,
        var: impl Var<T>,
        modify: impl FnOnce(&mut T) + 'static,
        vars: &Vars,
    ) -> Result<(), VarIsReadOnly> {
        var.push_modify(modify, vars, self)
    }

    pub(crate) fn push_modify_impl(&mut self, modify: impl FnOnce(&mut Vars, &mut Vec<CleanupOnce>) + 'static) {
        self.update.update = true;
        self.updates.push(Box::new(move |assert, _, cleanup| modify(assert, cleanup)));
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

    /// Schedules a low-pressure update.
    pub fn push_update(&mut self) {
        self.update.update = true;
    }

    /// Schedules a high-pressure update.
    pub fn push_update_hp(&mut self) {
        self.update.update_hp = true;
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
}

/// Owner of [`AppContext`] objects.
///
/// Because [`Vars`] and [`Events`] can only have one instance
/// and this `struct` owns both you can only have one instance
/// of this at a time.
pub struct OwnedAppContext {
    event_loop: EventLoopProxy,
    app_state: StateMap,
    headless_state: Option<StateMap>,
    vars: Vars,
    events: Events,
    services: AppServicesInit,
    window_services: WindowServicesInit,
    updates: OwnedUpdates,
}

impl OwnedAppContext {
    /// Produces the single instance of `AppContext` for a normal app run.
    pub fn instance(event_loop: EventLoopProxy) -> Self {
        OwnedAppContext {
            app_state: StateMap::default(),
            headless_state: None,
            vars: Vars::instance(),
            events: Events::instance(),
            services: AppServicesInit::default(),
            window_services: WindowServicesInit::default(),
            updates: OwnedUpdates::new(event_loop.clone()),
            event_loop,
        }
    }

    /// If the context is in headless mode.
    pub fn is_headless(&self) -> bool {
        self.headless_state.is_some()
    }

    /// State that lives for the duration of a headless application.
    pub fn headless_state(&self) -> Option<&StateMap> {
        self.headless_state.as_ref()
    }

    /// State that lives for the duration of a headless application.
    pub fn headless_state_mut(&mut self) -> Option<&mut StateMap> {
        self.headless_state.as_mut()
    }

    pub fn borrow_init(&mut self) -> AppInitContext {
        AppInitContext {
            app_state: &mut self.app_state,
            headless: HeadlessInfo::new(self.headless_state.as_mut()),
            event_loop: &self.event_loop,
            vars: &self.vars,
            events: &mut self.events,
            services: &mut self.services,
            window_services: &mut self.window_services,
            updates: &mut self.updates.updates,
        }
    }

    pub fn borrow<'a>(&'a mut self, event_loop: EventLoopWindowTarget<'a>) -> AppContext<'a> {
        AppContext {
            app_state: &mut self.app_state,
            headless: HeadlessInfo::new(self.headless_state.as_mut()),
            vars: &self.vars,
            events: &self.events,
            services: self.services.services(),
            window_services: &self.window_services,
            updates: &mut self.updates.updates,
            event_loop,
        }
    }

    /// Takes the request that generated an [`AppEvent::Update`](AppEvent::Update).
    pub fn take_request(&mut self) -> UpdateRequest {
        self.updates.take_request()
    }

    pub fn apply_updates(&mut self) -> (UpdateRequest, UpdateDisplayRequest) {
        self.updates.apply_updates(&mut self.vars, &mut self.events)
    }
}

/// Information about a headless app context.
pub struct HeadlessInfo<'a> {
    state: Option<&'a mut StateMap>,
}

impl<'a> HeadlessInfo<'a> {
    fn new(state: Option<&'a mut StateMap>) -> Self {
        HeadlessInfo { state }
    }

    /// If the application is running in headless mode.
    pub fn is_headless(&self) -> bool {
        self.state.is_some()
    }

    /// State that lives for the duration of the headless application.
    pub fn state(&'a mut self) -> Option<&'a mut StateMap> {
        match &mut self.state {
            None => None,
            Some(state) => Some(state),
        }
    }
}

/// App extension initialization context.
pub struct AppInitContext<'a> {
    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,
    /// Information about this context if it is running in headless mode.
    pub headless: HeadlessInfo<'a>,

    /// Reference to the event loop.
    pub event_loop: &'a EventLoopProxy,

    /// Variables access.
    ///
    /// ### Note
    /// In the application initialization context there are no variable updates, so
    /// `[`Var::update`](Var::update)` is always none.
    pub vars: &'a Vars,

    /// Events listener access and registration.
    ///
    /// ### Note
    /// Events are registered in the order the extensions appear in [`App`](crate::core::app::App), if an
    /// extension needs a listener for an event of another extension this dependency
    /// must be mentioned in documentation.
    pub events: &'a mut Events,

    /// Application services access and registration.
    ///
    /// ### Note
    /// Services are registered in the order the extensions appear in [`App`](crate::core::app::App), if an
    /// extension needs a service from another extension this dependency
    /// must be mentioned in documentation.
    pub services: &'a mut AppServicesInit,

    /// Window services registration.
    ///
    /// ### Note
    /// Window services are services that require a window to exist so none
    /// can be accessed during the application initialization, they can only
    /// be registered here.
    pub window_services: &'a mut WindowServicesInit,

    /// Changes to be applied after initialization.
    ///
    /// ### Note
    /// There is no notification of updates for this one, the updates are
    /// applied and then vars and events are reset.
    pub updates: &'a mut Updates,
}

/// Full application context.
pub struct AppContext<'a> {
    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,
    /// Information about this context if it is running in headless mode.
    pub headless: HeadlessInfo<'a>,

    pub vars: &'a Vars,
    pub events: &'a Events,
    pub services: &'a mut AppServices,
    pub window_services: &'a WindowServicesInit,
    pub updates: &'a mut Updates,

    pub event_loop: EventLoopWindowTarget<'a>,
}

/// Custom state associated with a window.
pub type WindowState = StateMap;

impl<'a> AppContext<'a> {
    /// Initializes state and services for a new window.
    pub fn new_window(&mut self, window_id: WindowId, render_api: &Arc<RenderApi>) -> (WindowState, WindowServices) {
        let mut window_state = StateMap::default();
        let mut event_state = StateMap::default();

        let mut window_services = WindowServices { m: Default::default() };
        let ctx = WindowContext {
            window_id,
            render_api,
            app_state: self.app_state,
            window_state: &mut window_state,
            event_state: &mut event_state,
            window_services: &mut window_services,
            vars: self.vars,
            events: self.events,
            services: self.services,
            updates: self.updates,
        };

        window_services = self.window_services.init(&ctx);

        (window_state, window_services)
    }

    /// Runs a function `f` within the context of a window.
    pub fn window_context(
        &mut self,
        window_id: WindowId,
        window_state: &mut WindowState,
        window_services: &mut WindowServices,
        render_api: &Arc<RenderApi>,
        f: impl FnOnce(&mut WindowContext),
    ) -> UpdateDisplayRequest {
        self.updates.win_display_update = UpdateDisplayRequest::None;

        let mut event_state = StateMap::default();

        f(&mut WindowContext {
            window_id,
            render_api,
            app_state: self.app_state,
            window_state,
            window_services,
            event_state: &mut event_state,
            vars: self.vars,
            events: self.events,
            services: self.services,
            updates: self.updates,
        });

        mem::take(&mut self.updates.win_display_update)
    }
}

/// A window context.
pub struct WindowContext<'a> {
    pub window_id: WindowId,
    pub render_api: &'a Arc<RenderApi>,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the event.
    pub event_state: &'a mut StateMap,

    pub vars: &'a Vars,
    pub events: &'a Events,
    pub services: &'a mut AppServices,
    pub window_services: &'a mut WindowServices,

    pub updates: &'a mut Updates,
}

impl<'a> WindowContext<'a> {
    /// Runs a function `f` within the context of a widget.
    pub fn widget_context(&mut self, widget_id: WidgetId, widget_state: &mut LazyStateMap, f: impl FnOnce(&mut WidgetContext)) {
        f(&mut WidgetContext {
            window_id: self.window_id,
            widget_id,

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state,
            event_state: self.event_state,

            vars: self.vars,
            events: self.events,
            services: self.services,
            window_services: self.window_services,

            updates: self.updates,
        });
    }
}

/// A widget context.
pub struct WidgetContext<'a> {
    pub window_id: WindowId,
    pub widget_id: WidgetId,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the widget.
    pub widget_state: &'a mut LazyStateMap,

    /// State that lives for the duration of the event.
    pub event_state: &'a mut StateMap,

    pub vars: &'a Vars,
    pub events: &'a Events,
    pub services: &'a mut AppServices,
    pub window_services: &'a mut WindowServices,

    pub updates: &'a mut Updates,
}

impl<'a> WidgetContext<'a> {
    pub fn widget_is_focused(&self) -> bool {
        todo!()
    }

    /// Runs a function `f` within the context of a widget.
    pub fn widget_context(&mut self, widget_id: WidgetId, widget_state: &mut LazyStateMap, f: impl FnOnce(&mut WidgetContext)) {
        f(&mut WidgetContext {
            window_id: self.window_id,
            widget_id,

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state,
            event_state: self.event_state,

            vars: self.vars,
            events: self.events,
            services: self.services,
            window_services: self.window_services,

            updates: self.updates,
        });
    }
}
