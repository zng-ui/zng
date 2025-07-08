#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! App process implementation.
//!
//! # Widget Instantiation
//!
//! See [`enable_widget_macros!`] if you want to instantiate widgets without depending on the `zng` crate.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![recursion_limit = "256"]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    any::{TypeId, type_name},
    collections::HashMap,
    fmt, ops,
    path::PathBuf,
    sync::Arc,
};

pub mod access;
pub mod crash_handler;
pub mod event;
pub mod handler;
pub mod render;
pub mod shortcut;
pub mod third_party;
pub mod timer;
pub mod trace_recorder;
pub mod update;
pub mod view_process;
pub mod widget;
pub mod window;

mod tests;

use view_process::VIEW_PROCESS;
use widget::UiTaskWidget;
#[doc(hidden)]
pub use zng_layout as layout;
use zng_txt::Txt;
#[doc(hidden)]
pub use zng_var as var;

pub use zng_time::{DInstant, Deadline, INSTANT, InstantMode};

use update::{EventUpdate, InfoUpdates, LayoutUpdates, RenderUpdates, UPDATES, UpdatesTrace, WidgetUpdates};
use window::WindowMode;
use zng_app_context::{AppId, AppScope, LocalContext};
use zng_task::UiTask;

pub use zng_unique_id::static_id;

/// Enable widget instantiation in crates that can't depend on the `zng` crate.
///
/// This must be called at the top of the crate:
///
/// ```
/// // in lib.rs or main.rs
/// # use zng_app::*;
/// enable_widget_macros!();
/// ```
#[macro_export]
macro_rules! enable_widget_macros {
    () => {
        #[doc(hidden)]
        #[allow(unused_extern_crates)]
        extern crate self as zng;

        #[doc(hidden)]
        pub use $crate::__proc_macro_util;
    };
}

#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zng;

#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zng_app; // for doc-tests

#[doc(hidden)]
pub mod __proc_macro_util {
    // * don't add glob re-exports, the types leak in rust-analyzer even if all is doc(hidden).
    // * don't use macro_rules! macros that use $crate , they will fail with "unresolved import" when used from the re-exports.

    #[doc(hidden)]
    pub use zng_unique_id::static_id;

    #[doc(hidden)]
    pub mod widget {
        #[doc(hidden)]
        pub mod builder {
            #[doc(hidden)]
            pub use crate::widget::builder::{
                AnyArcWidgetHandler, ArcWidgetHandler, Importance, InputKind, PropertyArgs, PropertyId, PropertyInfo, PropertyInput,
                PropertyInputTypes, PropertyNewArgs, SourceLocation, UiNodeInWhenExprError, UiNodeListInWhenExprError, WgtInfo, WhenInput,
                WhenInputMember, WhenInputVar, WidgetHandlerInWhenExprError, WidgetType, getter_var, iter_input_build_actions,
                nest_group_items, new_dyn_other, new_dyn_ui_node, new_dyn_ui_node_list, new_dyn_var, new_dyn_widget_handler, panic_input,
                state_var, ui_node_list_to_args, ui_node_to_args, value_to_args, var_to_args, when_condition_expr_var,
                widget_handler_to_args,
            };
        }

        #[doc(hidden)]
        pub mod base {
            pub use crate::widget::base::{NonWidgetBase, WidgetBase, WidgetExt, WidgetImpl};
        }

        #[doc(hidden)]
        pub mod node {
            pub use crate::widget::node::{
                ArcNode, ArcNodeList, BoxedUiNode, BoxedUiNodeList, NilUiNode, UiNode, UiNodeList, UiVec, ui_node_list_default,
            };
        }

        #[doc(hidden)]
        pub mod info {
            pub use crate::widget::info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure};
        }

        #[doc(hidden)]
        pub use crate::widget::{easing_property, widget_new};

        #[doc(hidden)]
        pub use crate::widget::WIDGET;
    }

    #[doc(hidden)]
    pub mod update {
        pub use crate::update::{EventUpdate, WidgetUpdates};
    }

    #[doc(hidden)]
    pub mod layout {
        #[doc(hidden)]
        pub mod unit {
            #[doc(hidden)]
            pub use crate::layout::unit::{PxSize, TimeUnits};
        }

        #[doc(hidden)]
        pub mod context {
            #[doc(hidden)]
            pub use crate::layout::context::LAYOUT;
        }
    }

    #[doc(hidden)]
    pub mod render {
        pub use crate::render::{FrameBuilder, FrameUpdate};
    }

    #[doc(hidden)]
    pub mod handler {
        #[doc(hidden)]
        pub use crate::handler::hn;
    }

    #[doc(hidden)]
    pub mod var {
        #[doc(hidden)]
        pub use crate::var::{AnyVar, AnyVarValue, BoxedVar, Var, expr_var};

        #[doc(hidden)]
        pub mod animation {
            #[doc(hidden)]
            pub mod easing {
                #[doc(hidden)]
                pub use crate::var::animation::easing::{
                    back, bounce, circ, cubic, cubic_bezier, ease_in, ease_in_out, ease_out, ease_out_in, elastic, expo, linear, none,
                    quad, quart, quint, reverse, reverse_out, sine, step_ceil, step_floor,
                };
            }
        }
    }
}

/// An app extension.
///
/// App extensions setup and update core features such as services and events. App instances
/// are fully composed of app extensions.
///
/// See the `zng::app` module level documentation for more details, including the call order of methods
/// of this trait.
pub trait AppExtension: 'static {
    /// Register info abound this extension on the info list.
    #[inline(always)]
    fn register(&self, info: &mut AppExtensionsInfo)
    where
        Self: Sized,
    {
        info.push::<Self>()
    }

    /// Initializes this extension.
    #[inline(always)]
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
    #[inline(always)]
    fn enable_device_events(&self) -> bool {
        false
    }

    /// Called just before [`event_ui`](Self::event_ui) when an event notifies.
    ///
    /// Extensions can handle this method to intercept event updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `event_ui`.
    #[inline(always)]
    fn event_preview(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called just before [`event`](Self::event).
    ///
    /// Only extensions that generate windows should handle this method. The [`UiNode::event`](crate::widget::node::UiNode::event)
    /// method is called here.
    #[inline(always)]
    fn event_ui(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called after [`event_ui`](Self::event_ui).
    ///
    /// This is the general extensions event handler, it gives the chance for the UI to signal stop propagation.
    #[inline(always)]
    fn event(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called when info rebuild is requested for windows and widgets.
    ///
    /// The [`UiNode::info`] method is called here.
    ///
    /// [`UiNode::info`]: crate::widget::node::UiNode::info
    #[inline(always)]
    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        let _ = info_widgets;
    }

    /// Called just before [`update_ui`](Self::update_ui).
    ///
    /// Extensions can handle this method to react to updates before the UI.
    ///
    /// Note that this is not related to the `on_event_preview` properties, all UI events
    /// happen in `update_ui`.
    #[inline(always)]
    fn update_preview(&mut self) {}

    /// Called just before [`update`](Self::update).
    ///
    /// Only extensions that manage windows should handle this method.
    ///
    /// The [`UiNode::update`] method is called here.
    ///
    /// [`UiNode::update`]: crate::widget::node::UiNode::update
    #[inline(always)]
    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        let _ = update_widgets;
    }

    /// Called after every [`update_ui`](Self::update_ui) and [`info`](Self::info).
    ///
    /// This is the general extensions update, it gives the chance for
    /// the UI to make service requests.
    #[inline(always)]
    fn update(&mut self) {}

    /// Called when layout is requested for windows and widgets.
    ///
    /// The [`UiNode::layout`] method is called here.
    ///
    /// [`UiNode::layout`]: crate::widget::node::UiNode::layout
    #[inline(always)]
    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        let _ = layout_widgets;
    }

    /// Called when render is requested for windows and widgets.
    ///
    /// The [`UiNode::render`] and [`UiNode::render_update`] methods are called here.
    ///
    /// [`UiNode::render`]: crate::widget::node::UiNode::render
    /// [`UiNode::render_update`]: crate::widget::node::UiNode::render_update
    #[inline(always)]
    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        let _ = (render_widgets, render_update_widgets);
    }

    /// Called when the application is exiting.
    ///
    /// Update requests and event notifications generated during this call are ignored,
    /// the extensions will be dropped after every extension received this call.
    #[inline(always)]
    fn deinit(&mut self) {}

    /// Gets the extension boxed.
    ///
    /// Boxed app extensions also implement `AppExtension`, this method does not double box.
    #[inline(always)]
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
/// See [`APP::extensions`] for more details.
#[derive(Clone, Copy)]
#[non_exhaustive]
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
    pub(crate) fn start() -> Self {
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

/// Desired next step of app main loop.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use = "methods that return `AppControlFlow` expect to be inside a controlled loop"]
pub enum AppControlFlow {
    /// Immediately try to receive more app events.
    Poll,
    /// Sleep until an app event is received.
    ///
    /// Note that a deadline might be set in case a timer is running.
    Wait,
    /// Exit the loop and drop the app.
    Exit,
}
impl AppControlFlow {
    /// Assert that the value is [`AppControlFlow::Wait`].
    #[track_caller]
    pub fn assert_wait(self) {
        assert_eq!(AppControlFlow::Wait, self)
    }

    /// Assert that the value is [`AppControlFlow::Exit`].
    #[track_caller]
    pub fn assert_exit(self) {
        assert_eq!(AppControlFlow::Exit, self)
    }
}

/// A headless app controller.
///
/// Headless apps don't cause external side-effects like visible windows and don't listen to system events.
/// They can be used for creating apps like a command line app that renders widgets, or for creating integration tests.
///
/// You can start a headless app using [`AppExtended::run_headless`].
pub struct HeadlessApp {
    app: RunningApp<Box<dyn AppExtensionBoxed>>,
}
impl HeadlessApp {
    /// If headless rendering is enabled.
    ///
    /// When enabled windows are still not visible but frames will be rendered and the frame
    /// image can be requested.
    ///
    /// Note that [`UiNode::render`] is still called when a renderer is disabled and you can still
    /// query the latest frame from `WINDOWS.widget_tree`. The only thing that
    /// is disabled is the actual renderer that converts display lists to pixels.
    ///
    /// [`UiNode::render`]: crate::widget::node::UiNode::render
    pub fn renderer_enabled(&mut self) -> bool {
        VIEW_PROCESS.is_available()
    }

    /// Does updates unobserved.
    ///
    /// See [`update_observed`] for more details.
    ///
    /// [`update_observed`]: HeadlessApp::update
    pub fn update(&mut self, wait_app_event: bool) -> AppControlFlow {
        self.update_observed(&mut (), wait_app_event)
    }

    /// Does updates observing [`update`] only.
    ///
    /// See [`update_observed`] for more details.
    ///
    /// [`update`]: AppEventObserver::update
    /// [`update_observed`]: HeadlessApp::update
    pub fn update_observe(&mut self, on_update: impl FnMut(), wait_app_event: bool) -> AppControlFlow {
        struct Observer<F>(F);
        impl<F: FnMut()> AppEventObserver for Observer<F> {
            fn update(&mut self) {
                (self.0)()
            }
        }
        let mut observer = Observer(on_update);

        self.update_observed(&mut observer, wait_app_event)
    }

    /// Does updates observing [`event`] only.
    ///
    /// See [`update_observed`] for more details.
    ///
    /// [`event`]: AppEventObserver::event
    /// [`update_observed`]: HeadlessApp::update
    pub fn update_observe_event(&mut self, on_event: impl FnMut(&mut EventUpdate), wait_app_event: bool) -> AppControlFlow {
        struct Observer<F>(F);
        impl<F: FnMut(&mut EventUpdate)> AppEventObserver for Observer<F> {
            fn event(&mut self, update: &mut EventUpdate) {
                (self.0)(update);
            }
        }
        let mut observer = Observer(on_event);
        self.update_observed(&mut observer, wait_app_event)
    }

    /// Does updates with an [`AppEventObserver`].
    ///
    /// If `wait_app_event` is `true` the thread sleeps until at least one app event is received or a timer elapses,
    /// if it is `false` only responds to app events already in the buffer.
    pub fn update_observed<O: AppEventObserver>(&mut self, observer: &mut O, mut wait_app_event: bool) -> AppControlFlow {
        if self.app.has_exited() {
            return AppControlFlow::Exit;
        }

        loop {
            match self.app.poll(wait_app_event, observer) {
                AppControlFlow::Poll => {
                    wait_app_event = false;
                    continue;
                }
                flow => return flow,
            }
        }
    }

    /// Execute the async `task` in the UI thread, updating the app until it finishes or the app shuts-down.
    ///
    /// Returns the task result if the app has not shut-down.
    pub fn run_task<R, T>(&mut self, task: impl IntoFuture<IntoFuture = T>) -> Option<R>
    where
        R: 'static,
        T: Future<Output = R> + Send + Sync + 'static,
    {
        let mut task = UiTask::new(None, task);

        let mut flow = self.update_observe(
            || {
                task.update();
            },
            false,
        );

        if task.update().is_some() {
            let r = task.into_result().ok();
            debug_assert!(r.is_some());
            return r;
        }

        let mut n = 0;
        while flow != AppControlFlow::Exit {
            flow = self.update_observe(
                || {
                    task.update();
                },
                true,
            );

            if n == 10_000 {
                tracing::error!("excessive future awaking, run_task ran 10_000 update cycles without finishing");
            } else if n == 100_000 {
                panic!("run_task stuck, ran 100_000 update cycles without finishing");
            }
            n += 1;

            match task.into_result() {
                Ok(r) => return Some(r),
                Err(t) => task = t,
            }
        }
        task.cancel();

        None
    }

    /// Requests and wait for app exit.
    ///
    /// Forces deinit if exit is cancelled.
    pub fn exit(mut self) {
        self.run_task(async move {
            let req = APP.exit();
            req.wait_rsp().await;
        });
    }

    /// If the app has exited.
    ///
    /// Exited apps cannot update anymore. The app should be dropped to unload the app scope.
    pub fn has_exited(&self) -> bool {
        self.app.has_exited()
    }
}

/// Observer for [`HeadlessApp::update_observed`].
///
/// This works like a temporary app extension that runs only for the update call.
pub trait AppEventObserver {
    /// Called for each raw event received.
    fn raw_event(&mut self, ev: &zng_view_api::Event) {
        let _ = ev;
    }

    /// Called just after [`AppExtension::event_preview`].
    fn event_preview(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called just after [`AppExtension::event_ui`].
    fn event_ui(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called just after [`AppExtension::event`].
    fn event(&mut self, update: &mut EventUpdate) {
        let _ = update;
    }

    /// Called just after [`AppExtension::update_preview`].
    fn update_preview(&mut self) {}

    /// Called just after [`AppExtension::update_ui`].
    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        let _ = update_widgets;
    }

    /// Called just after [`AppExtension::update`].
    fn update(&mut self) {}

    /// Called just after [`AppExtension::info`].
    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        let _ = info_widgets;
    }

    /// Called just after [`AppExtension::layout`].
    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        let _ = layout_widgets;
    }

    /// Called just after [`AppExtension::render`].
    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        let _ = (render_widgets, render_update_widgets);
    }

    /// Cast to dynamically dispatched observer, this can help avoid code bloat.
    ///
    /// The app methods that accept observers automatically use this method if the feature `"dyn_app_extension"` is active.
    fn as_dyn(&mut self) -> DynAppEventObserver<'_>
    where
        Self: Sized,
    {
        DynAppEventObserver(self)
    }
}
/// Nil observer, does nothing.
impl AppEventObserver for () {}

#[doc(hidden)]
pub struct DynAppEventObserver<'a>(&'a mut dyn AppEventObserverDyn);

trait AppEventObserverDyn {
    fn raw_event_dyn(&mut self, ev: &zng_view_api::Event);
    fn event_preview_dyn(&mut self, update: &mut EventUpdate);
    fn event_ui_dyn(&mut self, update: &mut EventUpdate);
    fn event_dyn(&mut self, update: &mut EventUpdate);
    fn update_preview_dyn(&mut self);
    fn update_ui_dyn(&mut self, updates: &mut WidgetUpdates);
    fn update_dyn(&mut self);
    fn info_dyn(&mut self, info_widgets: &mut InfoUpdates);
    fn layout_dyn(&mut self, layout_widgets: &mut LayoutUpdates);
    fn render_dyn(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates);
}
impl<O: AppEventObserver> AppEventObserverDyn for O {
    fn raw_event_dyn(&mut self, ev: &zng_view_api::Event) {
        self.raw_event(ev)
    }

    fn event_preview_dyn(&mut self, update: &mut EventUpdate) {
        self.event_preview(update)
    }

    fn event_ui_dyn(&mut self, update: &mut EventUpdate) {
        self.event_ui(update)
    }

    fn event_dyn(&mut self, update: &mut EventUpdate) {
        self.event(update)
    }

    fn update_preview_dyn(&mut self) {
        self.update_preview()
    }

    fn update_ui_dyn(&mut self, update_widgets: &mut WidgetUpdates) {
        self.update_ui(update_widgets)
    }

    fn update_dyn(&mut self) {
        self.update()
    }

    fn info_dyn(&mut self, info_widgets: &mut InfoUpdates) {
        self.info(info_widgets)
    }

    fn layout_dyn(&mut self, layout_widgets: &mut LayoutUpdates) {
        self.layout(layout_widgets)
    }

    fn render_dyn(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        self.render(render_widgets, render_update_widgets)
    }
}
impl AppEventObserver for DynAppEventObserver<'_> {
    fn raw_event(&mut self, ev: &zng_view_api::Event) {
        self.0.raw_event_dyn(ev)
    }

    fn event_preview(&mut self, update: &mut EventUpdate) {
        self.0.event_preview_dyn(update)
    }

    fn event_ui(&mut self, update: &mut EventUpdate) {
        self.0.event_ui_dyn(update)
    }

    fn event(&mut self, update: &mut EventUpdate) {
        self.0.event_dyn(update)
    }

    fn update_preview(&mut self) {
        self.0.update_preview_dyn()
    }

    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        self.0.update_ui_dyn(update_widgets)
    }

    fn update(&mut self) {
        self.0.update_dyn()
    }

    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        self.0.info_dyn(info_widgets)
    }

    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        self.0.layout_dyn(layout_widgets)
    }

    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        self.0.render_dyn(render_widgets, render_update_widgets)
    }

    fn as_dyn(&mut self) -> DynAppEventObserver<'_> {
        DynAppEventObserver(self.0)
    }
}

impl AppExtension for () {
    fn register(&self, _: &mut AppExtensionsInfo) {}
}
impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    fn init(&mut self) {
        self.0.init();
        self.1.init();
    }

    fn register(&self, info: &mut AppExtensionsInfo) {
        self.0.register(info);
        self.1.register(info);
    }

    fn enable_device_events(&self) -> bool {
        self.0.enable_device_events() || self.1.enable_device_events()
    }

    fn update_preview(&mut self) {
        self.0.update_preview();
        self.1.update_preview();
    }

    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        self.0.update_ui(update_widgets);
        self.1.update_ui(update_widgets);
    }

    fn update(&mut self) {
        self.0.update();
        self.1.update();
    }

    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        self.0.info(info_widgets);
        self.1.info(info_widgets);
    }

    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        self.0.layout(layout_widgets);
        self.1.layout(layout_widgets);
    }

    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        self.0.render(render_widgets, render_update_widgets);
        self.1.render(render_widgets, render_update_widgets);
    }

    fn event_preview(&mut self, update: &mut EventUpdate) {
        self.0.event_preview(update);
        self.1.event_preview(update);
    }

    fn event_ui(&mut self, update: &mut EventUpdate) {
        self.0.event_ui(update);
        self.1.event_ui(update);
    }

    fn event(&mut self, update: &mut EventUpdate) {
        self.0.event(update);
        self.1.event(update);
    }

    fn deinit(&mut self) {
        self.1.deinit();
        self.0.deinit();
    }
}

#[cfg(feature = "dyn_app_extension")]
impl AppExtension for Vec<Box<dyn AppExtensionBoxed>> {
    fn init(&mut self) {
        for ext in self {
            ext.init();
        }
    }

    fn register(&self, info: &mut AppExtensionsInfo) {
        for ext in self {
            ext.register(info);
        }
    }

    fn enable_device_events(&self) -> bool {
        self.iter().any(|e| e.enable_device_events())
    }

    fn update_preview(&mut self) {
        for ext in self {
            ext.update_preview();
        }
    }

    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        for ext in self {
            ext.update_ui(update_widgets);
        }
    }

    fn update(&mut self) {
        for ext in self {
            ext.update();
        }
    }

    fn event_preview(&mut self, update: &mut EventUpdate) {
        for ext in self {
            ext.event_preview(update);
        }
    }

    fn event_ui(&mut self, update: &mut EventUpdate) {
        for ext in self {
            ext.event_ui(update);
        }
    }

    fn event(&mut self, update: &mut EventUpdate) {
        for ext in self {
            ext.event(update);
        }
    }

    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        for ext in self {
            ext.info(info_widgets);
        }
    }

    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        for ext in self {
            ext.layout(layout_widgets);
        }
    }

    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        for ext in self {
            ext.render(render_widgets, render_update_widgets);
        }
    }

    fn deinit(&mut self) {
        for ext in self.iter_mut().rev() {
            ext.deinit();
        }
    }
}

/// Start and manage an app process.
pub struct APP;
impl APP {
    /// If the crate was built with `feature="multi_app"`.
    ///
    /// If `true` multiple apps can run in the same process, but only one app per thread at a time.
    pub fn multi_app_enabled(&self) -> bool {
        cfg!(feature = "multi_app")
    }

    /// If an app is already running in the current thread.
    ///
    /// Apps are *running* as soon as they start building, and stop running after
    /// [`AppExtended::run`] returns or the [`HeadlessApp`] is dropped.
    ///
    /// You can use [`app_local!`] to create *static* resources that live for the app lifetime.
    ///
    /// [`app_local!`]: zng_app_context::app_local
    pub fn is_running(&self) -> bool {
        LocalContext::current_app().is_some()
    }

    /// Gets the unique ID of the current app.
    ///
    /// This ID usually does not change as most apps only run once per process, but it can change often during tests.
    /// Resources that interact with [`app_local!`] values can use this ID to ensure that they are still operating in the same
    /// app.
    ///
    /// [`app_local!`]: zng_app_context::app_local
    pub fn id(&self) -> Option<AppId> {
        LocalContext::current_app()
    }

    #[cfg(not(feature = "multi_app"))]
    fn assert_can_run_single() {
        use std::sync::atomic::*;
        static CAN_RUN: AtomicBool = AtomicBool::new(true);

        if !CAN_RUN.swap(false, Ordering::SeqCst) {
            panic!("only one app is allowed per process")
        }
    }

    fn assert_can_run() {
        #[cfg(not(feature = "multi_app"))]
        Self::assert_can_run_single();
        if APP.is_running() {
            panic!("only one app is allowed per thread")
        }
    }

    /// Returns a [`WindowMode`] value that indicates if the app is headless, headless with renderer or headed.
    ///
    /// Note that specific windows can be in headless mode even if the app is headed.
    pub fn window_mode(&self) -> WindowMode {
        if VIEW_PROCESS.is_available() {
            if VIEW_PROCESS.is_headless_with_render() {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headed
            }
        } else {
            WindowMode::Headless
        }
    }
    /// List of app extensions that are part of the current app.
    pub fn extensions(&self) -> Arc<AppExtensionsInfo> {
        APP_PROCESS_SV.read().extensions()
    }

    /// If device events are enabled for the current app.
    ///
    /// See [`AppExtension::enable_device_events`] for more details.
    pub fn device_events(&self) -> bool {
        APP_PROCESS_SV.read().device_events
    }
}

impl APP {
    /// Starts building an application with no extensions.
    #[cfg(feature = "dyn_app_extension")]
    pub fn minimal(&self) -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        zng_env::init_process_name("app-process");

        #[cfg(debug_assertions)]
        print_tracing(tracing::Level::INFO);
        assert_not_view_process();
        Self::assert_can_run();
        check_deadlock();

        let _ = INSTANT.now();
        let scope = LocalContext::start_app(AppId::new_unique());
        AppExtended {
            extensions: vec![],
            view_process_exe: None,
            view_process_env: HashMap::new(),
            _cleanup: scope,
        }
    }

    /// Starts building an application with no extensions.
    #[cfg(not(feature = "dyn_app_extension"))]
    pub fn minimal(&self) -> AppExtended<()> {
        #[cfg(debug_assertions)]
        print_tracing(tracing::Level::INFO);
        assert_not_view_process();
        Self::assert_can_run();
        check_deadlock();
        let scope = LocalContext::start_app(AppId::new_unique());
        AppExtended {
            extensions: (),
            view_process_exe: None,
            view_process_env: HashMap::new(),
            _cleanup: scope,
        }
    }
}

/// Application builder.
///
/// You can use `APP` to start building the app.
pub struct AppExtended<E: AppExtension> {
    extensions: E,
    view_process_exe: Option<PathBuf>,
    view_process_env: HashMap<Txt, Txt>,

    // cleanup on drop.
    _cleanup: AppScope,
}
#[cfg(feature = "dyn_app_extension")]
impl AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
    /// Includes an application extension.
    pub fn extend<F: AppExtension>(mut self, extension: F) -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        self.extensions.push(TraceAppExt(extension).boxed());
        self
    }

    /// If the application should notify raw device events.
    ///
    /// Device events are raw events not targeting any window, like a mouse move on any part of the screen.
    /// They tend to be high-volume events so there is a performance cost to activating this. Note that if
    /// this is `false` you still get the mouse move over windows of the app.
    pub fn enable_device_events(self) -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        struct EnableDeviceEvents;
        impl AppExtension for EnableDeviceEvents {
            fn enable_device_events(&self) -> bool {
                true
            }
        }
        self.extend(EnableDeviceEvents)
    }

    fn run_dyn(self, start: std::pin::Pin<Box<dyn Future<Output = ()> + Send + 'static>>) {
        let app = RunningApp::start(
            self._cleanup,
            self.extensions,
            true,
            true,
            self.view_process_exe,
            self.view_process_env,
        );

        UPDATES.run(start).perm();

        app.run_headed();
    }

    fn run_headless_dyn(self, with_renderer: bool) -> HeadlessApp {
        let app = RunningApp::start(
            self._cleanup,
            self.extensions.boxed(),
            false,
            with_renderer,
            self.view_process_exe,
            self.view_process_env,
        );

        HeadlessApp { app }
    }
}

// Monomorphize dyn app. Without this the entire RunningApp code is generic that must build on the dependent crates.
#[cfg(feature = "dyn_app_extension")]
impl<E: AppExtension> AppExtended<E> {
    fn cast_app(self) -> AppExtended<Vec<Box<dyn AppExtensionBoxed>>> {
        let app: Box<dyn std::any::Any> = Box::new(self);
        match app.downcast::<AppExtended<Vec<Box<dyn AppExtensionBoxed>>>>() {
            Ok(ok) => *ok,
            Err(e) => {
                let app = *e.downcast::<Self>().unwrap();
                AppExtended {
                    extensions: vec![app.extensions.boxed()],
                    view_process_exe: app.view_process_exe,
                    view_process_env: app.view_process_env,
                    _cleanup: app._cleanup,
                }
            }
        }
    }

    fn run_impl(self, start: impl Future<Output = ()> + Send + 'static) {
        self.cast_app().run_dyn(Box::pin(start))
    }

    fn run_headless_impl(self, with_renderer: bool) -> HeadlessApp {
        self.cast_app().run_headless_dyn(with_renderer)
    }
}

#[cfg(not(feature = "dyn_app_extension"))]
impl<E: AppExtension> AppExtended<E> {
    /// Includes an application extension.
    pub fn extend<F: AppExtension>(self, extension: F) -> AppExtended<impl AppExtension> {
        AppExtended {
            _cleanup: self._cleanup,
            extensions: (self.extensions, TraceAppExt(extension)),
            view_process_exe: self.view_process_exe,
            view_process_env: self.view_process_env,
        }
    }

    /// If the application should notify raw device events.
    ///
    /// Device events are raw events not targeting any window, like a mouse move on any part of the screen.
    /// They tend to be high-volume events so there is a performance cost to activating this. Note that if
    /// this is `false` you still get the mouse move over windows of the app.
    pub fn enable_device_events(self) -> AppExtended<impl AppExtension> {
        struct EnableDeviceEvents;
        impl AppExtension for EnableDeviceEvents {
            fn enable_device_events(&self) -> bool {
                true
            }
        }
        self.extend(EnableDeviceEvents)
    }

    fn run_impl(self, start: impl Future<Output = ()> + Send + 'static) {
        let app = RunningApp::start(
            self._cleanup,
            self.extensions,
            true,
            true,
            self.view_process_exe,
            self.view_process_env,
        );

        UPDATES.run(start).perm();

        app.run_headed();
    }

    fn run_headless_impl(self, with_renderer: bool) -> HeadlessApp {
        let app = RunningApp::start(
            self._cleanup,
            self.extensions.boxed(),
            false,
            with_renderer,
            self.view_process_exe,
            self.view_process_env,
        );

        HeadlessApp { app }
    }
}
impl<E: AppExtension> AppExtended<E> {
    /// Set the path to the executable for the *View Process*.
    ///
    /// By the default the current executable is started again as a *View Process*, you can use
    /// two executables instead, by setting this value.
    ///
    /// Note that the `view_process_exe` must start a view server and both
    /// executables must be build using the same exact [`VERSION`].
    ///
    /// [`VERSION`]: zng_view_api::VERSION  
    pub fn view_process_exe(mut self, view_process_exe: impl Into<PathBuf>) -> Self {
        self.view_process_exe = Some(view_process_exe.into());
        self
    }

    /// Set an env variable for the view-process.
    pub fn view_process_env(mut self, name: impl Into<Txt>, value: impl Into<Txt>) -> Self {
        self.view_process_env.insert(name.into(), value.into());
        self
    }

    /// Starts the app, then starts polling `start` to run.
    ///
    /// This method only returns when the app has exited.
    ///
    /// The `start` task runs in a [`UiTask`] in the app context, note that it only needs to start the app, usually
    /// by opening a window, the app will keep running after `start` is finished.
    pub fn run<F: Future<Output = ()> + Send + 'static>(self, start: impl IntoFuture<IntoFuture = F>) {
        let start = start.into_future();
        #[cfg(feature = "dyn_closure")]
        let start = Box::pin(start);
        self.run_impl(start)
    }

    /// Initializes extensions in headless mode and returns an [`HeadlessApp`].
    ///
    /// If `with_renderer` is `true` spawns a renderer process for headless rendering. See [`HeadlessApp::renderer_enabled`]
    /// for more details.
    pub fn run_headless(self, with_renderer: bool) -> HeadlessApp {
        self.run_headless_impl(with_renderer)
    }
}

// this module is declared here on purpose so that advanced `impl APP` blocks show later in the docs.
mod running;
pub use running::*;

mod private {
    // https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
    pub trait Sealed {}
}

/// Enables [`tracing`] events printing if a subscriber is not already set.
///
/// All non-fatal errors in the Zng project are logged using tracing.
///
/// In debug builds this function is called automatically with level INFO on app start.
///
/// In `"wasm32"` builds logs to the browser console.
///
/// In `"android"` builds logs to logcat.
///
/// See also [`test_log`] to enable panicking on error log.
///
/// See also [`print_tracing_filter`] for the filter used by this.
///
/// [`tracing`]: https://docs.rs/tracing
pub fn print_tracing(max: tracing::Level) -> bool {
    use tracing_subscriber::prelude::*;

    let layers = tracing_subscriber::registry().with(FilterLayer(max));

    #[cfg(target_os = "android")]
    let layers = layers.with(tracing_android::layer(&zng_env::about().pkg_name).unwrap());

    #[cfg(not(target_os = "android"))]
    let layers = {
        let fmt_layer = tracing_subscriber::fmt::layer().without_time();

        #[cfg(target_arch = "wasm32")]
        let fmt_layer = fmt_layer.with_ansi(false).with_writer(tracing_web::MakeWebConsoleWriter::new());

        layers.with(fmt_layer)
    };

    layers.try_init().is_ok()
}

struct FilterLayer(tracing::Level);
impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for FilterLayer {
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _: tracing_subscriber::layer::Context<'_, S>) -> bool {
        print_tracing_filter(&self.0, metadata)
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(self.0.into())
    }

    #[cfg(any(test, feature = "test_util"))]
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().level() == &tracing::Level::ERROR && APP.is_running() && TEST_LOG.get() {
            struct MsgCollector<'a>(&'a mut String);
            impl tracing::field::Visit for MsgCollector<'_> {
                fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
                    use std::fmt::Write;
                    write!(self.0, "\n  {} = {:?}", field.name(), value).unwrap();
                }
            }

            let meta = event.metadata();
            let file = meta.file().unwrap_or("");
            let line = meta.line().unwrap_or(0);

            let mut msg = format!("[{file}:{line}]");
            event.record(&mut MsgCollector(&mut msg));

            panic!("[LOG-ERROR]{msg}")
        }
    }
}
/// Filter used by [`print_tracing`], removes some log noise from dependencies.
///
/// Use `tracing_subscriber::filter::FilterFn` plug this filter into a tracing setup.
pub fn print_tracing_filter(level: &tracing::Level, metadata: &tracing::Metadata) -> bool {
    if metadata.level() > level {
        return false;
    }

    if metadata.level() == &tracing::Level::INFO {
        // suppress large info about texture cache
        if metadata.target() == "zng_webrender::device::gl" {
            return false;
        }
        // suppress config dump
        if metadata.target() == "zng_webrender::renderer::init" {
            return false;
        }
    } else if metadata.level() == &tracing::Level::WARN {
        // suppress webrender warnings:
        //
        if metadata.target() == "zng_webrender::device::gl" {
            // Suppress "Cropping texture upload Box2D((0, 0), (0, 1)) to None"
            // This happens when an empty frame is rendered.
            if metadata.line() == Some(4647) {
                return false;
            }
        }

        // suppress font-kit warnings:
        //
        if metadata.target() == "font_kit::loaders::freetype" {
            // Suppress "$fn(): found invalid platform ID $n"
            // This does not look fully implemented and generates a lot of warns
            // with the default Ubuntu font set all with valid platform IDs.
            if metadata.line() == Some(734) {
                return false;
            }
        }
    }

    true
}

/// Modifies the [`print_tracing`] subscriber to panic for error logs in the current app.
#[cfg(any(test, feature = "test_util"))]
pub fn test_log() {
    TEST_LOG.set(true);
}

#[cfg(any(test, feature = "test_util"))]
zng_app_context::app_local! {
    static TEST_LOG: bool = false;
}

#[doc(hidden)]
pub fn name_from_pkg_name(name: &'static str) -> Txt {
    let mut n = String::new();
    let mut sep = "";
    for part in name.split(&['-', '_']) {
        n.push_str(sep);
        let mut chars = part.char_indices();
        let (_, c) = chars.next().unwrap();
        c.to_uppercase().for_each(|c| n.push(c));
        if let Some((i, _)) = chars.next() {
            n.push_str(&part[i..]);
        }
        sep = " ";
    }
    n.into()
}

#[doc(hidden)]
pub fn txt_from_pkg_meta(value: &'static str) -> Txt {
    value.into()
}
