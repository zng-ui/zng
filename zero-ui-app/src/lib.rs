use std::{fmt, ops, any::{TypeId, type_name}, borrow::Cow};

pub mod update;
pub mod event;
pub mod handler;
pub mod widget;

use update::{EventUpdate, InfoUpdates, WidgetUpdates, LayoutUpdates, RenderUpdates, UpdatesTrace};
use zero_ui_txt::Txt;

/// An [`App`] extension.
///
/// # App Loop
///
/// Methods in app extension are called in this synchronous order:
///
/// ## 1 - Init
///
/// The [`init`] method is called once at the start of the app. Extensions are initialized in the order then where *inserted* in the app.
///
/// ## 2 - Events
///
/// The [`event_preview`], [`event_ui`] and [`event`] methods are called in this order for each event message received. Events
/// received from other threads are buffered until the app is free and then are processed using these methods.
///
/// ## 3 - Updates
///
/// The [`update_preview`], [`update_ui`] and [`update`] methods are called in this order every time an [update is requested],
/// a sequence of events have processed, variables where assigned or timers elapsed. The app loops between [events] and [updates] until
/// no more updates or events are pending, if [layout] or [render] are requested they are deferred until a event-update cycle is complete.
///
/// # 4 - Layout
///
/// The [`layout`] method is called if during [init], [events] or [updates] a layout was requested, extensions should also remember which
/// unit requested layout, to avoid unnecessary work, for example the [`WindowManager`] remembers witch window requested layout.
///
/// If the [`layout`] call requests updates the app goes back to [updates], requests for render are again deferred.
///
/// # 5 - Render
///
/// The [`render`] method is called if during [init], [events], [updates] or [layout] a render was requested and no other
/// event, update or layout is pending. Extensions should identify which unit is pending a render or render update and generate
/// and send a display list or frame update.
///
/// This method does not block until the frame pixels are rendered, it covers only the creation of a frame request sent to the view-process.
/// A [`RAW_FRAME_RENDERED_EVENT`] is send when a frame finished rendering in the view-process.
///
/// ## 6 - Deinit
///
/// The [`deinit`] method is called once after an exit was requested and not cancelled. Exit is
/// requested using the [`APP_PROCESS`] service, it causes an [`EXIT_REQUESTED_EVENT`] that can be cancelled, if it
/// is not cancelled the extensions are deinited and then dropped.
///
/// Deinit happens from the last inited extension first, so in reverse of init order, the [drop] happens in undefined order. Deinit is not called
/// if the app thread is unwinding from a panic, the extensions will just be dropped in this case.
///
/// # Resize Loop
///
/// The app enters a special loop when a window is resizing,
///
/// [`init`]: AppExtension::init
/// [`event_preview`]: AppExtension::event_preview
/// [`event_ui`]: AppExtension::event_ui
/// [`event`]: AppExtension::event
/// [`update_preview`]: AppExtension::update_preview
/// [`update_ui`]: AppExtension::update_ui
/// [`update`]: AppExtension::update
/// [`layout`]: AppExtension::layout
/// [`render`]: AppExtension::event
/// [`deinit`]: AppExtension::deinit
/// [drop]: Drop
/// [update is requested]: UPDATES::update
/// [init]: #1-init
/// [events]: #2-events
/// [updates]: #3-updates
/// [layout]: #3-layout
/// [render]: #5-render
/// [`RAW_FRAME_RENDERED_EVENT`]: raw_events::RAW_FRAME_RENDERED_EVENT
pub trait AppExtension: 'static {
    /// Register info abound this extension on the info list.
    fn register(&self, info: &mut AppExtensionsInfo)
    where
        Self: Sized,
    {
        info.push::<Self>()
    }

    /// Initializes this extension.
    fn init(&mut self) {}

    /// If the application should notify raw device events.
    ///
    /// Device events are raw events not targeting any window, like a mouse move on any part of the screen.
    /// They tend to be high-volume events so there is a performance cost to activating this. Note that if
    /// this is `false` you still get the mouse move over windows of the app.
    ///
    /// This is called zero or one times before [`init`](Self::init).
    ///
    /// Returns `false` by default.
    fn enable_device_events(&self) -> bool {
        false
    }

    /// Called just before [`event_ui`](Self::event_ui).
    ///
    /// Extensions can handle this method to to intersect event updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `on_event_ui`.
    fn event_preview(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called just before [`event`](Self::event).
    ///
    /// Only extensions that generate windows must handle this method. The [`UiNode::event`](crate::widget_instance::UiNode::event)
    /// method is called here.
    fn event_ui(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called after every [`event_ui`](Self::event_ui).
    ///
    /// This is the general extensions event handler, it gives the chance for the UI to signal stop propagation.
    fn event(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called before and after an update cycle. The [`UiNode::info`] method is called here.
    ///
    /// [`UiNode::info`]: crate::widget_instance::UiNode::info
    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        let _ = info_widgets;
    }

    /// Called just before [`update_ui`](Self::update_ui).
    ///
    /// Extensions can handle this method to interact with updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `update_ui`.
    fn update_preview(&mut self) {}

    /// Called just before [`update`](Self::update).
    ///
    /// Only extensions that manage windows must handle this method. The [`UiNode::update`]
    /// method is called here.
    ///
    /// [`UiNode::update`]: crate::widget_instance::UiNode::update
    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        let _ = update_widgets;
    }

    /// Called after every [`update_ui`](Self::update_ui) and [`info`](Self::info).
    ///
    /// This is the general extensions update, it gives the chance for
    /// the UI to signal stop propagation.
    fn update(&mut self) {}

    /// Called after every sequence of updates if layout was requested.
    ///
    /// The [`UiNode::layout`] method is called here by extensions that manage windows.
    ///
    /// [`UiNode::layout`]: crate::widget_instance::UiNode::layout
    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        let _ = layout_widgets;
    }

    /// Called after every sequence of updates and layout if render was requested.
    ///
    /// The [`UiNode::render`] and [`UiNode::render_update`] methods are called here by extensions that manage windows.
    ///
    /// [`UiNode::render`]: crate::widget_instance::UiNode::render
    /// [`UiNode::render_update`]: crate::widget_instance::UiNode::render_update
    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        let _ = (render_widgets, render_update_widgets);
    }

    /// Called when the application is exiting.
    ///
    /// Update requests and event notifications generated during this call are ignored,
    /// the extensions will be dropped after every extension received this call.
    fn deinit(&mut self) {}

    /// The extension in a box.
    fn boxed(self) -> Box<dyn AppExtensionBoxed>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

/// Boxed version of [`AppExtension`].
#[doc(hidden)]
pub trait AppExtensionBoxed: 'static {
    fn register_boxed(&self, info: &mut AppExtensionsInfo);
    fn init_boxed(&mut self);
    fn enable_device_events_boxed(&self) -> bool;
    fn update_preview_boxed(&mut self);
    fn update_ui_boxed(&mut self, updates: &mut WidgetUpdates);
    fn update_boxed(&mut self);
    fn event_preview_boxed(&mut self, update: &mut EventUpdate);
    fn event_ui_boxed(&mut self, update: &mut EventUpdate);
    fn event_boxed(&mut self, update: &mut EventUpdate);
    fn info_boxed(&mut self, info_widgets: &mut InfoUpdates);
    fn layout_boxed(&mut self, layout_widgets: &mut LayoutUpdates);
    fn render_boxed(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates);
    fn deinit_boxed(&mut self);
}
impl<T: AppExtension> AppExtensionBoxed for T {
    fn register_boxed(&self, info: &mut AppExtensionsInfo) {
        self.register(info);
    }

    fn init_boxed(&mut self) {
        self.init();
    }

    fn enable_device_events_boxed(&self) -> bool {
        self.enable_device_events()
    }

    fn update_preview_boxed(&mut self) {
        self.update_preview();
    }

    fn update_ui_boxed(&mut self, updates: &mut WidgetUpdates) {
        self.update_ui(updates);
    }

    fn info_boxed(&mut self, info_widgets: &mut InfoUpdates) {
        self.info(info_widgets);
    }

    fn update_boxed(&mut self) {
        self.update();
    }

    fn event_preview_boxed(&mut self, update: &mut EventUpdate) {
        self.event_preview(update);
    }

    fn event_ui_boxed(&mut self, update: &mut EventUpdate) {
        self.event_ui(update);
    }

    fn event_boxed(&mut self, update: &mut EventUpdate) {
        self.event(update);
    }

    fn layout_boxed(&mut self, layout_widgets: &mut LayoutUpdates) {
        self.layout(layout_widgets);
    }

    fn render_boxed(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        self.render(render_widgets, render_update_widgets);
    }

    fn deinit_boxed(&mut self) {
        self.deinit();
    }
}
impl AppExtension for Box<dyn AppExtensionBoxed> {
    fn register(&self, info: &mut AppExtensionsInfo) {
        self.as_ref().register_boxed(info);
    }

    fn init(&mut self) {
        self.as_mut().init_boxed();
    }

    fn enable_device_events(&self) -> bool {
        self.as_ref().enable_device_events_boxed()
    }

    fn update_preview(&mut self) {
        self.as_mut().update_preview_boxed();
    }

    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        self.as_mut().update_ui_boxed(update_widgets);
    }

    fn update(&mut self) {
        self.as_mut().update_boxed();
    }

    fn event_preview(&mut self, update: &mut EventUpdate) {
        self.as_mut().event_preview_boxed(update);
    }

    fn event_ui(&mut self, update: &mut EventUpdate) {
        self.as_mut().event_ui_boxed(update);
    }

    fn event(&mut self, update: &mut EventUpdate) {
        self.as_mut().event_boxed(update);
    }

    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        self.as_mut().info_boxed(info_widgets);
    }

    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        self.as_mut().layout_boxed(layout_widgets);
    }

    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        self.as_mut().render_boxed(render_widgets, render_update_widgets);
    }

    fn deinit(&mut self) {
        self.as_mut().deinit_boxed();
    }

    fn boxed(self) -> Box<dyn AppExtensionBoxed>
    where
        Self: Sized,
    {
        self
    }
}

struct TraceAppExt<E: AppExtension>(E);
impl<E: AppExtension> AppExtension for TraceAppExt<E> {
    fn register(&self, info: &mut AppExtensionsInfo) {
        self.0.register(info)
    }

    fn init(&mut self) {
        let _span = UpdatesTrace::extension_span::<E>("init");
        self.0.init();
    }

    fn enable_device_events(&self) -> bool {
        self.0.enable_device_events()
    }

    fn event_preview(&mut self, update: &mut EventUpdate) {
        let _span = UpdatesTrace::extension_span::<E>("event_preview");
        self.0.event_preview(update);
    }

    fn event_ui(&mut self, update: &mut EventUpdate) {
        let _span = UpdatesTrace::extension_span::<E>("event_ui");
        self.0.event_ui(update);
    }

    fn event(&mut self, update: &mut EventUpdate) {
        let _span = UpdatesTrace::extension_span::<E>("event");
        self.0.event(update);
    }

    fn update_preview(&mut self) {
        let _span = UpdatesTrace::extension_span::<E>("update_preview");
        self.0.update_preview();
    }

    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        let _span = UpdatesTrace::extension_span::<E>("update_ui");
        self.0.update_ui(update_widgets);
    }

    fn update(&mut self) {
        let _span = UpdatesTrace::extension_span::<E>("update");
        self.0.update();
    }

    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        let _span = UpdatesTrace::extension_span::<E>("info");
        self.0.info(info_widgets);
    }

    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        let _span = UpdatesTrace::extension_span::<E>("layout");
        self.0.layout(layout_widgets);
    }

    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        let _span = UpdatesTrace::extension_span::<E>("render");
        self.0.render(render_widgets, render_update_widgets);
    }

    fn deinit(&mut self) {
        let _span = UpdatesTrace::extension_span::<E>("deinit");
        self.0.deinit();
    }

    fn boxed(self) -> Box<dyn AppExtensionBoxed>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

/// Info about an app-extension.
///
/// See [`App::extensions`] for more details.
///
/// [`App::extensions`]: crate::app::App::extensions
#[derive(Clone, Copy)]
pub struct AppExtensionInfo {
    /// Extension type ID.
    pub type_id: TypeId,
    /// Extension type name.
    pub type_name: &'static str,
}
impl PartialEq for AppExtensionInfo {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}
impl fmt::Debug for AppExtensionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name)
    }
}
impl Eq for AppExtensionInfo {}
impl AppExtensionInfo {
    /// New info for `E`.
    pub fn new<E: AppExtension>() -> Self {
        Self {
            type_id: TypeId::of::<E>(),
            type_name: type_name::<E>(),
        }
    }
}

/// List of app-extensions that are part of an app.
#[derive(Clone, PartialEq)]
pub struct AppExtensionsInfo {
    infos: Vec<AppExtensionInfo>,
}
impl fmt::Debug for AppExtensionsInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.infos).finish()
    }
}
impl AppExtensionsInfo {
    pub fn start() -> Self {
        Self { infos: vec![] }
    }

    /// Push the extension info.
    pub fn push<E: AppExtension>(&mut self) {
        let info = AppExtensionInfo::new::<E>();
        assert!(!self.contains::<E>(), "app-extension `{info:?}` is already in the list");
        self.infos.push(info);
    }

    /// Gets if the extension `E` is in the list.
    pub fn contains<E: AppExtension>(&self) -> bool {
        self.contains_info(AppExtensionInfo::new::<E>())
    }

    /// Gets i the extension is in the list.
    pub fn contains_info(&self, info: AppExtensionInfo) -> bool {
        self.infos.iter().any(|e| e.type_id == info.type_id)
    }

    /// Panics if the extension `E` is not present.
    #[track_caller]
    pub fn require<E: AppExtension>(&self) {
        let info = AppExtensionInfo::new::<E>();
        assert!(self.contains_info(info), "app-extension `{info:?}` is required");
    }
}
impl ops::Deref for AppExtensionsInfo {
    type Target = [AppExtensionInfo];

    fn deref(&self) -> &Self::Target {
        &self.infos
    }
}

zero_ui_unique_id::unique_id_64! {
    /// Unique id of a widget.
    ///
    /// # Name
    ///
    /// Widget ids are very fast but are just a number that is only unique for the same process that generated then.
    /// You can associate a [`name`] with an id to give it a persistent identifier.
    ///
    /// [`name`]: WidgetId::name
    pub struct WidgetId;
}
zero_ui_unique_id::impl_unique_id_name!(WidgetId);
zero_ui_unique_id::impl_unique_id_fmt!(WidgetId);

zero_ui_var::impl_from_and_into_var! {
    /// Calls [`WidgetId::named`].
    fn from(name: &'static str) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: String) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: Cow<'static, str>) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: char) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: Txt) -> WidgetId {
        WidgetId::named(name)
    }
    fn from(id: WidgetId) -> zero_ui_view_api::access::AccessNodeId {
        zero_ui_view_api::access::AccessNodeId(id.get())
    }

    fn from(some: WidgetId) -> Option<WidgetId>;
}

impl fmt::Debug for StaticWidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.get(), f)
    }
}
impl zero_ui_var::IntoValue<WidgetId> for &'static StaticWidgetId {}
impl serde::Serialize for WidgetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = self.name();
        if name.is_empty() {
            use serde::ser::Error;
            return Err(S::Error::custom("cannot serialize unammed `WidgetId`"));
        }
        name.serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for WidgetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = Txt::deserialize(deserializer)?;
        Ok(WidgetId::named(name))
    }
}

zero_ui_unique_id::unique_id_32! {
    /// Unique identifier of an open window.
    ///
    /// Can be obtained from [`WINDOW.id`] inside a window.
    ///
    /// [`WINDOW.id`]: crate::context::WINDOW::id
    pub struct WindowId;
}
zero_ui_unique_id::impl_unique_id_name!(WindowId);
zero_ui_unique_id::impl_unique_id_fmt!(WindowId);

zero_ui_var::impl_from_and_into_var! {
    /// Calls [`WindowId::named`].
    fn from(name: &'static str) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: String) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: Cow<'static, str>) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: char) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: Txt) -> WindowId {
        WindowId::named(name)
    }
}
impl fmt::Debug for StaticWindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.get(), f)
    }
}
impl serde::Serialize for WindowId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = self.name();
        if name.is_empty() {
            use serde::ser::Error;
            return Err(S::Error::custom("cannot serialize unammed `WindowId`"));
        }
        name.serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for WindowId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = Txt::deserialize(deserializer)?;
        Ok(WindowId::named(name))
    }
}
impl zero_ui_var::IntoValue<WindowId> for &'static StaticWindowId {}