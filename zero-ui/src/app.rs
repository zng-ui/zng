//! App extensions, context, events and commands API.
//!
//! # Runtime
//!
//! A typical app instance has two processes, the initial process called the *app-process*, and a second process called the
//! *view-process*. The app-process implements the event loop and updates, the view-process is a platform agnostic GUI and
//! renderer, the app-process controls the view-process, most of the time app implementers don't need to worry about, except
//! at the start where the view-process is spawned.
//!
//! This dual process architecture is done mostly for resilience, the unsafe interactions with the operating system and
//! graphics driver are isolated in a different process, in case of crashes the view-process is respawned automatically and
//! all windows are recreated.
//!
//! ## Spawn View
//!
//! To simplify distribution the view-process is an instance of the same app executable, the view-process crate provides
//! and `init` function that either spawns the view-process or becomes the view-process never returning.
//!
//! On the first instance of the app executable the `zero_ui_view::init` function spawns another instance marked to
//! become the view-process, on this second instance the init function never returns, for this reason the function
//! must be called early in main.
//!
//! ```toml
//! [dependencies]
//! zero-ui = "0.1"
//! zero-ui-view = "0.1"
//! ```
//!
//! ```no_run
//! # mod zero_ui_view { pub fn init() { } }
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     app_and_view();
//!     zero_ui_view::init(); // init only returns if it is not called in the view-process.
//!     app();
//! }
//!
//! fn app_and_view() {
//!     // code here runs in the app-process and view-process.
//! }
//!
//! fn app() {
//!     // code here only runs in the app-process.
//!
//!     APP.defaults().run(async {
//!         // ..
//!     })
//! }
//! ```
//!
//! ## Same Process
//!
//! You can also run the view in the same process, this mode of execution is slightly more efficient, but
//! your app will not be resilient to crashes caused by the operating system or graphics driver, the app code
//! will also run in a different thread, not the main.
//!
//! ```no_run
//! # mod zero_ui_view { pub fn run_same_process(_: impl FnOnce()) { } }
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     zero_ui_view::run_same_process(app);
//! }
//!
//! fn app() {
//!     // code here runs in a different thread, the main thread becomes the view.
//!     APP.defaults().run(async {
//!         // ..
//!     })
//! }
//! ```
//!
//!
//!
//! ## Prebuild
//!
//! You can also use a prebuild view, using a prebuild view will give you much better performance in debug builds,
//! and also that you don't need to build `zero-ui-view`, cutting initial build time by half.
//!
//! ```toml
//! [dependencies]
//! zero-ui = "0.1"
//! zero-ui-view-prebuilt = "0.1"
//! ```
//!
//! ```no_run
//! # mod zero_ui_view_prebuilt { pub fn init() { } pub fn same_process(_: impl FnOnce()) { } }
//! use zero_ui::prelude::*;
//!
//! use zero_ui_view_prebuilt as zero_ui_view;
//!
//! fn main() {
//!     if std::env::var("MY_APP_EXEC_MODE").unwrap_or_default() == "same_process" {
//!         zero_ui_view::same_process(app);
//!     } else {
//!         zero_ui_view::init();
//!         app();
//!     }
//! }
//!
//! fn app() {
//!     APP.defaults().run(async {
//!         // ..
//!     })
//! }
//! ```
//!
//! # Headless
//!
//! The app can also run *headless*, where no window is actually created, optionally with real rendering.
//! This mode is useful for running integration tests, or for rendering images.
//!
//! ```
//! use zero_ui::prelude::*;
//!
//! let mut app = APP.defaults().run_headless(/* with_renderer: */ false);
//! app.run_window(async {
//!     Window! {
//!         child = Text!("Some text");
//!         auto_size = true;
//!
//!         render_mode = window::RenderMode::Software;
//!         frame_capture_mode = window::FrameCaptureMode::Next;
//!
//!         on_frame_image_ready = async_hn!(|args: window::FrameImageReadyArgs| {
//!             if let Some(img) = args.frame_image {
//!                 // if the app runs with `run_headless(/* with_renderer: */ true)` an image is captured
//!                 // and saved here.
//!                 img.save("screenshot.png").await.unwrap();
//!             }
//!
//!             // close the window, causing the app to exit.
//!             WINDOW.close();
//!         });
//!     }
//! });
//! ```
//!
//! You can also run multiple headless apps in the same process, one per thread, if the crate is build using the `"multi_app"` feature.
//!
//! # App Extension
//!
//! Apps can be extended to provide new services and events, in fact all default services and events are implemented as extensions
//! loaded by [`APP.defaults()`]. The app extension API is [`AppExtension`]. Usually extensions are named with suffix `Manager`, but
//! that is not a requirement.
//!
//! ```
//! use zero_ui::{app::{AppExtended, AppExtension}, APP};
//!
//! #[derive(Default)]
//! pub struct HelloManager {}
//! impl AppExtension for HelloManager {
//!     fn init(&mut self) {
//!         println!("Hello init!");
//!     }
//!
//!     fn update_preview(&mut self) {
//!         println!("Hello before UI!");
//!     }
//!
//!     fn update(&mut self) {
//!         println!("Hello after UI!");
//!     }
//! }
//!
//! pub fn app() -> AppExtended<impl AppExtension> {
//!     APP.defaults().extend(HelloManager::default())
//! }
//! ```
//!
//! ## Services
//!
//! App services are defined by convention, there is no service trait or struct. Proper service implementations follow
//! these rules:
//!
//! #### App services are an unit struct named like a static
//!
//! This is because services are a kind of *singleton*. The service API is implemented as methods on the service struct.
//!
//! ```
//! # use zero_ui::var::*;
//! #[allow(non_camel_case_types)]
//! pub struct SCREAMING_CASE;
//! impl SCREAMING_CASE {
//!     pub fn state(&self) -> impl Var<bool> {
//! #       var(true)
//!     }
//! }
//! ```
//!
//! Note that you need to suppress a lint if the service name has more then one word.
//!
//! Service state and config methods should prefer variables over direct values. The use of variables allows the service state
//! to be plugged directly into the UI. Async operations should prefer using [`ResponseVar<R>`] over `async` methods for
//! the same reason.
//!
//! #### App services lifetime is the current app lifetime
//!
//! Unlike a simple singleton app services must only live for the duration of the app and must support
//! multiple parallel instances if built with the `"multi_app"` feature. You can use private
//! [`app_local!`] static variables as backing storage to fulfill this requirement.
//!
//! A common pattern in the zero-ui services is to name the app locals with a `_SV` suffix.
//!
//! Services do not expose the app local locking, all state output is cloned the state is only locked
//! for the duration of the service method call.
//!
//! #### App services don't change public state mid update
//!
//! All widgets using the service during the same update see the same state. State change requests are scheduled
//! for the next update, just like variable updates or event notifications. Services also request
//! an [`UPDATES.update`] after scheduling to wake-up the app in case the service request was made from a [`task`] thread.
//!
//! ### Examples
//!
//! Fulfilling service requests is where the [`AppExtension`] comes in, it is possible to declare a simple standalone
//! service using only variables, `Event::on_event` and `UPDATES.run_hn_once`, but an app extension is more efficient
//! and more easy to implement.
//!
//! If the service request can fail or be delayed it is common for the request method to return a [`ResponseVar<R>`]
//! that is updated once the request is finished. You can also make the method `async`, but a response var is superior
//! because it can be plugged directly into any UI property, and it can still be awaited using the variable async methods.
//!
//! If the service request cannot fail and it is guaranteed to affect an observable change in the service state in the
//! next update a response var is not needed.
//!
//! The example below demonstrates an app extension implementation that provides a service.
//!
//! ```
//! use zero_ui::{prelude_wgt::*, app::AppExtension};
//!
//! /// Foo service.
//! pub struct FOO;
//!
//! impl FOO {
//!     /// Foo read-write var.
//!     pub fn config(&self) -> impl Var<bool> {
//!         FOO_SV.read().config.clone()
//!     }
//!
//!     /// Foo request.
//!     pub fn request(&self, request: char) -> ResponseVar<char> {
//!         UPDATES.update(None);
//!
//!         let mut foo = FOO_SV.write();
//!         let (responder, response) = response_var();
//!         foo.requests.push((request, responder));
//!         response
//!     }
//! }
//!
//! struct FooService {
//!     config: ArcVar<bool>,
//!     requests: Vec<(char, ResponderVar<char>)>,
//! }
//!
//! app_local! {
//!     static FOO_SV: FooService = FooService { config: var(false), requests: vec![] };
//! }
//!
//! /// Foo app extension.
//! ///
//! /// # Services
//! ///
//! /// Services provided by this extension.
//! ///
//! /// * [`FOO`]
//! #[derive(Default)]
//! pub struct FooManager { }
//!
//! impl AppExtension for FooManager {
//!     fn update(&mut self) {
//!         let mut foo = FOO_SV.write();
//!
//!         if let Some(cfg) = foo.config.get_new() {
//!             println!("foo cfg={cfg}");
//!         }
//!
//!         for (request, responder) in foo.requests.drain(..) {
//!             println!("foo request {request:?}");
//!             responder.respond(request);
//!         }
//!     }
//! }
//! ```
//!
//! Note that in the example requests are processed in the [`AppExtension::update`] update that is called
//! after all widgets have had a chance to make requests. Requests can also be made from parallel [`task`] threads so
//! the service also requests an [`UPDATES.update`] just in case there is no update running. If you expect to receive many
//! requests from parallel tasks you can also process requests in the [`AppExtension::update`] instead, but there is probably
//! little practical difference.
//!
//! [`APP.defaults()`]: crate::APP::defaults
//! [`UPDATES.update`]: crate::update::UPDATES::update
//! [`task`]: crate::task
//! [`ResponseVar<R>`]: crate::var::ResponseVar
//!
//! # Full API
//!
//! This module provides most of the app API needed to make and extend apps, some more advanced or experimental API
//! may be available at the [`zero_ui_app`] and [`zero_ui_app_context`] base crates.

pub use zero_ui_app::{
    AppEventObserver, AppExtended, AppExtension, AppExtensionBoxed, AppExtensionInfo, ControlFlow, ExitRequestedArgs, HeadlessApp,
    EXIT_CMD, EXIT_REQUESTED_EVENT,
};
pub use zero_ui_app_context::{
    app_local, context_local, AppId, AppLocal, AppScope, CaptureFilter, ContextLocal, ContextValueSet, FullLocalContext, LocalContext,
    MappedRwLockReadGuardOwned, MappedRwLockWriteGuardOwned, ReadOnlyRwLock, RunOnDrop, RwLockReadGuardOwned, RwLockWriteGuardOwned,
    StaticAppId,
};
pub use zero_ui_wgt_input::cmd::{
    on_new, on_open, on_pre_new, on_pre_open, on_pre_save, on_pre_save_as, on_save, on_save_as, NEW_CMD, OPEN_CMD, SAVE_AS_CMD, SAVE_CMD,
};
