//! Context information for app extensions, windows and widgets.

use super::app::{AppEvent, EventLoopProxy, EventLoopWindowTarget};
use super::event::Events;
use super::service::{AppServices, AppServicesInit, WindowServices, WindowServicesInit};
use super::sync::Sync;
use super::units::{LayoutSize, PixelGrid};
use super::var::Vars;
use super::window::WindowId;
use super::AnyMap;
use super::WidgetId;
use std::sync::atomic::{self, AtomicU8};
use std::{
    any::{Any, TypeId},
    time::Instant,
};
use std::{fmt, mem};
use std::{marker::PhantomData, sync::Arc};
use webrender::api::RenderApi;

/// Required updates for a window layout and frame.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UpdateDisplayRequest {
    /// No update required.
    None = 0,
    /// No new full frame required, just update the current one and render again.
    RenderUpdate = 1,
    /// No re-layout required, just render again.
    Render = 2,
    /// Full update required, re-layout then render again.
    Layout = 3,
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
        let a = (*self) as u8;
        let b = rhs as u8;
        *self = match a.max(b) {
            3 => UpdateDisplayRequest::Layout,
            2 => UpdateDisplayRequest::Render,
            1 => UpdateDisplayRequest::RenderUpdate,
            n => {
                debug_assert_eq!(n, 0);
                UpdateDisplayRequest::None
            }
        };
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
impl std::cmp::PartialOrd for UpdateDisplayRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        std::cmp::PartialOrd::partial_cmp(&(*self as u8), &(*other as u8))
    }
}
impl std::cmp::Ord for UpdateDisplayRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        std::cmp::Ord::cmp(&(*self as u8), &(*other as u8))
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
    event_loop: EventLoopProxy,
    request: Arc<AtomicU8>,
}
impl UpdateNotifier {
    #[inline]
    fn new(event_loop: EventLoopProxy) -> Self {
        UpdateNotifier {
            event_loop,
            request: Arc::new(AtomicU8::new(0)),
        }
    }

    const UPDATE: u8 = 0b01;
    const UPDATE_HP: u8 = 0b11;

    #[inline]
    fn set(&self, update: u8) {
        let old = self.request.fetch_or(update, atomic::Ordering::Relaxed);
        if old == 0 {
            self.event_loop.send_event(AppEvent::Update);
        }
    }

    /// Flags an update request and sends an update event
    /// if none was sent since the last one was consumed.
    #[inline]
    pub fn update(&self) {
        self.set(Self::UPDATE);
    }

    /// Flags an update request(high-pressure) and sends an update event
    /// if none was sent since the last one was consumed.
    #[inline]
    pub fn update_hp(&self) {
        self.set(Self::UPDATE_HP);
    }
}

/// A key to a value in a [`StateMap`].
///
/// The type that implements this trait is the key. You
/// can use the [`state_key!`](crate::context::state_key) macro.
pub trait StateKey: Clone + Copy + 'static {
    /// The value type.
    type Type: 'static;
}

/// Declares new [`StateKey`](crate::context::StateKey) types.
///
/// # Example
///
/// ```
/// # use zero_ui_core::context::state_key;
/// state_key! {
///     /// Key docs.
///     pub struct FooKey: u32;
/// }
/// ```
/// # Naming Convention
///
/// It is recommended that the type name ends with the `Key` suffix.
#[macro_export]
macro_rules! state_key {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty;)+) => {$(
        $(#[$outer])*
        /// # StateKey
        /// This `struct` is a [`StateKey`](crate::context::StateKey).
        #[derive(Clone, Copy)]
        $vis struct $ident;

        impl $crate::context::StateKey for $ident {
            type Type = $type;
        }
    )+};
}

#[doc(inline)]
pub use crate::state_key;

/// A map of [state keys](StateKey) to values of their associated types that exists for
/// a stage of the application.
#[derive(Debug, Default)]
pub struct StateMap {
    map: AnyMap,
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
    ///
    /// Returns if the state key was already flagged.
    pub fn flag<S: StateKey<Type = ()>>(&mut self, key: S) -> bool {
        self.set(key, ()).is_some()
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self, key: S) -> bool {
        let _ = key;
        self.map.contains_key(&TypeId::of::<S>())
    }

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
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

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.m.as_ref().map(|m| m.is_empty()).unwrap_or(true)
    }
}

/// Executor access to [`Updates`].
pub struct OwnedUpdates(Updates);

impl OwnedUpdates {
    pub fn new(event_loop: EventLoopProxy) -> Self {
        Self(Updates::new(event_loop))
    }

    /// Takes the update request generated by [`notifier`](Updates::notifier) since
    /// the last time it was taken.
    #[inline]
    pub fn take_request(&self) -> UpdateRequest {
        let request = self.0.notifier.request.swap(0, atomic::Ordering::Relaxed);

        const UPDATE: u8 = UpdateNotifier::UPDATE;
        const UPDATE_HP: u8 = UpdateNotifier::UPDATE_HP;

        UpdateRequest {
            update: request & UPDATE == UPDATE,
            update_hp: request & UPDATE_HP == UPDATE_HP,
        }
    }

    /// Take what update methods must be pumped.
    pub fn take_updates(&mut self) -> (UpdateRequest, UpdateDisplayRequest) {
        (mem::take(&mut self.0.update), mem::take(&mut self.0.display_update))
    }

    /// Reference the [`Updates`].
    pub fn updates(&mut self) -> &mut Updates {
        &mut self.0
    }
}

/// Schedule of actions to apply after an update.
///
/// An instance of this struct can be built by [`OwnedUpdates`].
pub struct Updates {
    notifier: UpdateNotifier,
    update: UpdateRequest,
    display_update: UpdateDisplayRequest,
    win_display_update: UpdateDisplayRequest,
}

impl Updates {
    fn new(event_loop: EventLoopProxy) -> Self {
        Updates {
            notifier: UpdateNotifier::new(event_loop),
            update: UpdateRequest::default(),
            display_update: UpdateDisplayRequest::None,
            win_display_update: UpdateDisplayRequest::None,
        }
    }

    /// Cloneable out-of-band notification sender.
    #[inline]
    pub fn notifier(&self) -> &UpdateNotifier {
        &self.notifier
    }

    /// Schedules a low-pressure update.
    #[inline]
    pub fn update(&mut self) {
        self.update.update = true;
    }

    /// Gets `true` if a low-pressure update was requested.
    #[inline]
    pub fn update_requested(&self) -> bool {
        self.update.update
    }

    /// Schedules a high-pressure update.
    #[inline]
    pub fn update_hp(&mut self) {
        self.update.update_hp = true;
    }

    /// Gets `true` if a high-pressure update was requested.
    #[inline]
    pub fn update_hp_requested(&self) -> bool {
        self.update.update_hp
    }

    /// Schedules the `updates`.
    #[inline]
    pub fn schedule_updates(&mut self, updates: UpdateRequest) {
        self.update |= updates;
    }

    /// Schedules a layout update.
    #[inline]
    pub fn layout(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::Layout;
        self.display_update |= UpdateDisplayRequest::Layout;
    }

    /// Gets `true` if a layout update is scheduled.
    #[inline]
    pub fn layout_requested(&self) -> bool {
        self.win_display_update == UpdateDisplayRequest::Layout
    }

    /// Schedules a new frame.
    #[inline]
    pub fn render(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::Render;
        self.display_update |= UpdateDisplayRequest::Render;
    }

    /// Gets `true` if a new frame is scheduled.
    #[inline]
    pub fn render_requested(&self) -> bool {
        self.win_display_update >= UpdateDisplayRequest::Render
    }

    /// Schedule a frame update.
    #[inline]
    pub fn render_update(&mut self) {
        self.win_display_update |= UpdateDisplayRequest::RenderUpdate;
        self.display_update |= UpdateDisplayRequest::RenderUpdate;
    }

    /// Gets `true` if a frame update is scheduled.
    #[inline]
    pub fn render_update_requested(&self) -> bool {
        self.win_display_update >= UpdateDisplayRequest::RenderUpdate
    }

    /// Schedule the `updates`.
    #[inline]
    pub fn schedule_display_updates(&mut self, updates: UpdateDisplayRequest) {
        self.win_display_update |= updates;
        self.display_update |= updates;
    }
}

/// Owner of [`AppContext`] objects.
///
/// You can only have one instance of this at a time.
pub struct OwnedAppContext {
    event_loop: EventLoopProxy,
    app_state: StateMap,
    headless_state: Option<StateMap>,
    vars: Vars,
    events: Events,
    services: AppServicesInit,
    window_services: WindowServicesInit,
    sync: Sync,
    updates: OwnedUpdates,
}

impl OwnedAppContext {
    /// Produces the single instance of `AppContext` for a normal app run.
    pub fn instance(event_loop: EventLoopProxy) -> Self {
        let updates = OwnedUpdates::new(event_loop.clone());
        OwnedAppContext {
            app_state: StateMap::default(),
            headless_state: None,
            vars: Vars::instance(),
            events: Events::instance(),
            services: AppServicesInit::default(),
            window_services: WindowServicesInit::default(),
            sync: Sync::new(updates.0.notifier.clone()),
            updates,
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
            sync: &mut self.sync,
            updates: &mut self.updates.0,
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
            sync: &mut self.sync,
            updates: &mut self.updates.0,
            event_loop,
        }
    }

    /// Takes the request that generated an [`AppEvent::Update`](AppEvent::Update).
    pub fn take_request(&mut self) -> UpdateRequest {
        self.updates.take_request()
    }

    /// Takes the window service visitor requests, of there is any.
    pub fn take_window_service_visitors(&mut self) -> Option<super::service::WindowServicesVisitors> {
        self.window_services.take_visitors()
    }

    /// Applies pending, `sync`, `vars`, `events` and takes all the update requests.
    ///
    /// Returns the update requests and a time for the loop wake back and call
    /// [`Sync::update_timers`].
    pub fn apply_updates(&mut self) -> ((UpdateRequest, UpdateDisplayRequest), Option<Instant>) {
        let wake = self.sync.update(&mut AppSyncContext {
            vars: &mut self.vars,
            events: &mut self.events,
            updates: &mut self.updates.0,
        });
        self.vars.apply(&mut self.updates.0);
        self.events.apply(&mut self.updates.0);
        (self.updates.take_updates(), wake)
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

    /// Async tasks.
    ///
    /// ### Note
    /// Tasks will not be completed during this initialization.
    pub sync: &'a mut Sync,

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

    /// Access to application variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a Events,
    /// Access to application services.
    pub services: &'a mut AppServices,

    /// Window services registration.
    pub window_services: &'a WindowServicesInit,

    /// Async tasks.
    pub sync: &'a mut Sync,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,

    /// Reference to raw event loop.
    pub event_loop: EventLoopWindowTarget<'a>,
}

/// App context view for tasks synchronization.
pub(super) struct AppSyncContext<'a> {
    /// Access to application variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a Events,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}

/// Custom state associated with a window.
pub type WindowState = StateMap;

impl<'a> AppContext<'a> {
    /// Initializes state and services for a new window.
    pub fn new_window(&mut self, window_id: WindowId, render_api: &Arc<RenderApi>) -> (WindowState, WindowServices) {
        let mut window_state = StateMap::default();
        let mut event_state = StateMap::default();

        let mut window_services = WindowServices::new();
        let ctx = WindowContext {
            window_id: ReadOnly(window_id),
            render_api,
            app_state: self.app_state,
            window_state: &mut window_state,
            event_state: &mut event_state,
            window_services: &mut window_services,
            vars: self.vars,
            events: self.events,
            services: self.services,
            sync: self.sync,
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
        let unloader = window_services.load();

        f(&mut WindowContext {
            window_id: ReadOnly(window_id),
            render_api,
            app_state: self.app_state,
            window_state,
            window_services: unloader.window_services,
            event_state: &mut event_state,
            vars: self.vars,
            events: self.events,
            services: self.services,
            sync: self.sync,
            updates: self.updates,
        });

        mem::take(&mut self.updates.win_display_update)
    }
}

/// A window context.
pub struct WindowContext<'a> {
    pub window_id: ReadOnly<WindowId>,
    pub render_api: &'a Arc<RenderApi>,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the event.
    pub event_state: &'a mut StateMap,

    /// Access to application variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a Events,
    /// Access to application services.
    pub services: &'a mut AppServices,
    /// Access to window services.
    pub window_services: &'a mut WindowServices,

    /// Async tasks.
    pub sync: &'a mut Sync,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}

/// Read-only value in a public context struct field.
#[derive(Clone, Copy)]
pub struct ReadOnly<T>(T);
impl<T: Copy> ReadOnly<T> {
    #[inline]
    pub fn get(self) -> T {
        self.0
    }
}
impl<T> std::ops::Deref for ReadOnly<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> WindowContext<'a> {
    /// Runs a function `f` within the context of a widget.
    pub fn widget_context(&mut self, widget_id: WidgetId, widget_state: &mut LazyStateMap, f: impl FnOnce(&mut WidgetContext)) {
        let mut path = WidgetContextPath::new(self.window_id.0, widget_id);
        f(&mut WidgetContext {
            path: &mut path,

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state,
            event_state: self.event_state,

            vars: self.vars,
            events: self.events,
            services: self.services,
            window_services: self.window_services,

            sync: self.sync,

            updates: self.updates,
        });
    }
}

/// A mock [`WidgetContext`] for testing.
///
/// Only a single instance of this type exists at a time, see [`Self::wait_new`] for details.
#[cfg(test)]
pub struct TestWidgetContext {
    /// WARNING: Default value is [`WindowId::dummy()`] which is unsafe.
    pub window_id: WindowId,
    pub root_id: WidgetId,
    pub app_state: StateMap,
    pub window_state: StateMap,
    pub event_state: StateMap,
    pub services: AppServicesInit,
    pub event_loop: crate::app::EventLoop,
    pub updates: OwnedUpdates,
    pub vars: Vars,
    pub events: Events,
    pub window_services: WindowServices,
    pub sync: Sync,
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
static TEST_CONTEXT_LOCK: once_cell::sync::Lazy<std::sync::Mutex<()>> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(()));

#[cfg(test)]
impl TestWidgetContext {
    /// Gets a new [`TestWidgetContext`] instance. If another instance is alive in another thread
    /// **blocks until the other instance is dropped**.
    pub fn wait_new() -> Self {
        let lock = TEST_CONTEXT_LOCK.lock().unwrap_or_else(|e|e.into_inner());
        let event_loop = crate::app::EventLoop::new(true);
        let updates = OwnedUpdates::new(event_loop.create_proxy());
        let update_notifier = updates.0.notifier().clone();
        Self {
            // SAFETY: this is test only code and we have documentation warning users.
            window_id: unsafe { WindowId::dummy() },
            root_id: WidgetId::new_unique(),
            app_state: StateMap::default(),
            window_state: StateMap::default(),
            event_state: StateMap::default(),
            services: AppServicesInit::default(),
            event_loop,
            updates,
            vars: Vars::instance(),
            events: Events::instance(),
            window_services: WindowServices::new(),
            sync: Sync::new(update_notifier),
            _lock: lock,
        }
    }

    pub fn widget_context<R>(&mut self, widget_state: &mut LazyStateMap, action: impl FnOnce(&mut WidgetContext) -> R) -> R {
        action(&mut WidgetContext {
            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            app_state: &mut self.app_state,
            window_state: &mut self.window_state,
            widget_state,
            event_state: &mut self.event_state,
            vars: &mut self.vars,
            events: &mut self.events,
            services: self.services.services(),
            window_services: &mut self.window_services,
            sync: &mut self.sync,
            updates: self.updates.updates(),
        })
    }

    /// Applies pending, `sync`, `vars`, `events` and takes all the update requests.
    ///
    /// Returns the update requests and a time for the loop wake back and call
    /// [`Sync::update_timers`].
    pub fn apply_updates(&mut self) -> ((UpdateRequest, UpdateDisplayRequest), Option<Instant>) {
        let wake = self.sync.update(&mut AppSyncContext {
            vars: &mut self.vars,
            events: &mut self.events,
            updates: &mut self.updates.0,
        });
        self.vars.apply(&mut self.updates.0);
        self.events.apply(&mut self.updates.0);
        (self.updates.take_updates(), wake)
    }
}

/// A widget context.
pub struct WidgetContext<'a> {
    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the widget.
    pub widget_state: &'a mut LazyStateMap,

    /// State that lives for the duration of the event.
    pub event_state: &'a mut StateMap,

    /// Access to application variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a Events,
    /// Access to application services.
    pub services: &'a mut AppServices,
    /// Access to window services.
    pub window_services: &'a mut WindowServices,

    /// Async tasks.
    pub sync: &'a mut Sync,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}
impl<'a> WidgetContext<'a> {
    /// Runs a function `f` within the context of a widget.
    pub fn widget_context(&mut self, widget_id: WidgetId, widget_state: &mut LazyStateMap, f: impl FnOnce(&mut WidgetContext)) {
        self.path.push(widget_id);
        f(&mut WidgetContext {
            path: self.path,

            app_state: self.app_state,
            window_state: self.window_state,
            widget_state,
            event_state: self.event_state,

            vars: self.vars,
            events: self.events,
            services: self.services,
            window_services: self.window_services,

            sync: self.sync,

            updates: self.updates,
        });
        self.path.pop();
    }
}

/// Current widget context path.
#[derive(Debug)]
pub struct WidgetContextPath {
    window_id: WindowId,
    widget_ids: Vec<WidgetId>,
}

impl WidgetContextPath {
    fn new(window_id: WindowId, root_id: WidgetId) -> Self {
        let mut widget_ids = Vec::with_capacity(50);
        widget_ids.push(root_id);
        WidgetContextPath { window_id, widget_ids }
    }

    fn push(&mut self, widget_id: WidgetId) {
        self.widget_ids.push(widget_id);
    }

    fn pop(&mut self) {
        debug_assert!(self.widget_ids.len() > 1, "cannot pop root");
        self.widget_ids.pop();
    }

    /// Parent window id.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Window root widget id.
    #[inline]
    pub fn root_id(&self) -> WidgetId {
        self.widget_ids[0]
    }

    /// Current widget id.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.widget_ids[self.widget_ids.len() - 1]
    }

    /// Ancestor widgets, parent first.
    #[inline]
    #[allow(clippy::needless_lifetimes)] // clippy bug
    pub fn ancestors<'s>(&'s self) -> impl Iterator<Item = WidgetId> + 's {
        let max = self.widget_ids.len() - 1;
        self.widget_ids[0..max].iter().copied().rev()
    }

    /// Parent widget id.
    #[inline]
    pub fn parent(&self) -> Option<WidgetId> {
        self.ancestors().next()
    }

    /// If the `widget_id` is part of the path.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.widget_ids.iter().any(move |&w| w == widget_id)
    }
}

/// A widget layout context.
#[derive(Debug)]
pub struct LayoutContext {
    font_size: f32,
    root_font_size: f32,
    pixel_grid: PixelGrid,
    viewport_size: LayoutSize,
}

impl LayoutContext {
    #[inline]
    pub fn new(root_font_size: f32, viewport_size: LayoutSize, pixel_grid: PixelGrid) -> Self {
        LayoutContext {
            font_size: root_font_size,
            root_font_size,
            viewport_size,
            pixel_grid,
        }
    }

    /// Current computed font size.
    #[inline]
    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    #[inline]
    pub fn root_font_size(&self) -> f32 {
        self.root_font_size
    }

    #[inline]
    pub fn pixel_grid(&self) -> PixelGrid {
        self.pixel_grid
    }

    #[inline]
    pub fn viewport_size(&self) -> LayoutSize {
        self.viewport_size
    }

    #[inline]
    pub fn viewport_min(&self) -> f32 {
        self.viewport_size.width.min(self.viewport_size.height)
    }

    #[inline]
    pub fn viewport_max(&self) -> f32 {
        self.viewport_size.width.max(self.viewport_size.height)
    }

    /// Runs a function `f` within a context that has the new computed font size.
    pub fn with_font_size<R>(&mut self, new_font_size: f32, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        let old_font_size = mem::replace(&mut self.font_size, new_font_size);
        let r = f(self);
        self.font_size = old_font_size;
        r
    }
}

/// Error when an service or event of the same type is registered twice.
#[derive(Debug, Clone, Copy)]
pub struct AlreadyRegistered {
    /// Type name of the service.
    pub type_name: &'static str,
}
impl fmt::Display for AlreadyRegistered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is already registered", self.type_name)
    }
}
impl std::error::Error for AlreadyRegistered {}
