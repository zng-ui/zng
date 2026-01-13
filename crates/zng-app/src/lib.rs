#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
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

use std::{collections::HashMap, fmt, path::PathBuf};

pub mod access;
pub mod crash_handler;
pub mod event;
pub mod handler;
pub mod memory_profiler;
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
use zng_var::Var;

pub use zng_time::{DInstant, Deadline, INSTANT, InstantMode};

use update::UPDATES;
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
                AnyArcHandler, HandlerInWhenExprError, Importance, InputKind, PropertyArgs, PropertyId, PropertyInfo, PropertyInput,
                PropertyInputTypes, PropertyNewArgs, SourceLocation, UiNodeInWhenExprError, WgtInfo, WhenInput, WhenInputMember,
                WhenInputVar, WidgetBuilding, WidgetType, handler_to_args, iter_input_attributes, nest_group_items, new_dyn_handler,
                new_dyn_other, new_dyn_ui_node, new_dyn_var, panic_input, ui_node_to_args, value_to_args, var_getter, var_state,
                var_to_args,
            };
        }

        #[doc(hidden)]
        pub mod base {
            pub use crate::widget::base::{NonWidgetBase, WidgetBase, WidgetExt, WidgetImpl};
        }

        #[doc(hidden)]
        pub mod node {
            pub use crate::widget::node::{ArcNode, IntoUiNode, UiNode};
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
        pub use crate::update::WidgetUpdates;
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
        pub use crate::handler::{ArcHandler, hn};
    }

    #[doc(hidden)]
    pub mod var {
        #[doc(hidden)]
        pub use crate::var::{AnyVar, AnyVarValue, Var, expr_var};

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
    app: RunningApp,
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
        self.app.
        self.update_observed(&mut (), wait_app_event)
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

/// Start and manage an app process.
pub struct APP;
impl APP {
    /// If the crate was built with `feature="multi_app"`.
    ///
    /// If `true` multiple apps can run in the same process, but only one app per thread at a time.
    pub fn multi_app_enabled(&self) -> bool {
        cfg!(feature = "multi_app")
    }

    /// If an app started building or is running in the current thread.
    ///
    /// This is `true` as soon as `APP.minimal()` or `APP.defaults()` is called.
    ///
    /// You can use [`app_local!`] to create *static* resources that live for the app lifetime, these statics can be used
    /// as soon as this is `true`.
    ///
    /// [`app_local!`]: zng_app_context::app_local
    pub fn is_started(&self) -> bool {
        LocalContext::current_app().is_some()
    }

    /// If an app is running in the current thread.
    ///
    /// Apps are *running* as soon as [`run`], [`run_headless`] or `run_window` are called.
    /// This will remain `true` until run returns or the [`HeadlessApp`] is dropped.
    ///
    /// [`run`]: AppExtended::run
    /// [`run_headless`]: AppExtended::run_headless
    pub fn is_running(&self) -> bool {
        self.is_started() && APP_PROCESS_SV.read().is_running()
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

    /// Defines what raw device events the view-process instance should monitor and notify.
    ///
    /// Raw device events are global and can be received even when the app has no visible window.
    ///
    /// These events are disabled by default as they can impact performance or may require special security clearance,
    /// depending on the view-process implementation and operating system.
    pub fn device_events_filter(&self) -> Var<DeviceEventsFilter> {
        APP_PROCESS_SV.read().device_events_filter.clone()
    }
}

impl APP {
    /// Starts building an application with no extra config.
    #[cfg(feature = "dyn_app_extension")]
    pub fn minimal(&self) -> AppBuilder {
        zng_env::init_process_name("app-process");

        #[cfg(debug_assertions)]
        print_tracing(tracing::Level::INFO);
        assert_not_view_process();
        Self::assert_can_run();
        spawn_deadlock_detection();

        let _ = INSTANT.now();
        let scope = LocalContext::start_app(AppId::new_unique());
        AppBuilder {
            view_process_exe: None,
            view_process_env: HashMap::new(),
            _cleanup: scope,
        }
    }

    /// Starts building an application with no extensions.
    #[cfg(not(feature = "dyn_app_extension"))]
    pub fn minimal(&self) -> AppBuilder<()> {
        #[cfg(debug_assertions)]
        print_tracing(tracing::Level::INFO);
        assert_not_view_process();
        Self::assert_can_run();
        spawn_deadlock_detection();
        let scope = LocalContext::start_app(AppId::new_unique());
        AppBuilder {
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
pub struct AppBuilder {
    view_process_exe: Option<PathBuf>,
    view_process_env: HashMap<Txt, Txt>,

    // cleanup on drop.
    _cleanup: AppScope,
}
impl AppBuilder {
    fn run_dyn(self, start: std::pin::Pin<Box<dyn Future<Output = ()> + Send + 'static>>) {
        let app = RunningApp::start(self._cleanup, true, true, self.view_process_exe, self.view_process_env);

        UPDATES.run(start).perm();

        app.run_headed();
    }

    fn run_headless_dyn(self, with_renderer: bool) -> HeadlessApp {
        let app = RunningApp::start(self._cleanup, false, with_renderer, self.view_process_exe, self.view_process_env);

        HeadlessApp { app }
    }

    fn run_impl(self, start: impl Future<Output = ()> + Send + 'static) {
        self.run_dyn(Box::pin(start))
    }

    fn run_headless_impl(self, with_renderer: bool) -> HeadlessApp {
        self.run_headless_dyn(with_renderer)
    }
}
impl AppBuilder {
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
use zng_view_api::DeviceEventsFilter;

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
