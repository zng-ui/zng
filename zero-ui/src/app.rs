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
