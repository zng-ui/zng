//! App extensions, context, events and commands API.
//!
//! # Runtime
//!
//! A typical app instance has two processes, the initial process called the *app-process*, and a second process called the
//! *view-process*. The app-process implements the event loop and updates, the view-process implements the platform integration and
//! renderer, the app-process controls the view-process, most of the time app implementers don't interact directly with it, except
//! at the start where the view-process is spawned.
//!
//! The reason for this dual process architecture is mostly for resilience, the unsafe interactions with the operating system and
//! graphics driver are isolated in a different process, in case of crashes the view-process is respawned automatically and
//! all windows are recreated. It is possible to run the app in a single process, in this case the view runs in the main thread
//! and the app main loop in another.
//!
//! ## View-Process
//!
//! To simplify distribution the view-process is an instance of the same app executable, the view-process crate injects
//! their own "main" in the [`zng::env::init!`] call, automatically taking over the process if the executable spawns as a view-process.
//!
//! On the first instance of the app executable the `init` only inits the env and returns, the app init spawns a second process
//! marked as the view-process, on this second instance the init call never returns, for this reason the init
//! must be called early in main, all code before the `init` call runs in both the app and view processes.
//!
//! ```toml
//! [dependencies]
//! zng = { version = "0.17.4", features = ["view_prebuilt"] }
//! ```
//!
//! ```no_run
//! use zng::prelude::*;
//!
//! fn main() {
//!     app_and_view();
//!     zng::env::init!(); // init only returns if it is not called in the view-process.
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
//! use zng::prelude::*;
//!
//! fn main() {
//!     zng::env::init!();
//!     zng::view_process::prebuilt::run_same_process(app);
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
//! Note that you must still call `init!` as it also initializes the app metadata and directories.
//!
//! # Headless
//!
//! The app can also run *headless*, where no window is actually created, optionally with real rendering.
//! This mode is useful for running integration tests, or for rendering images.
//!
//! ```
//! use zng::prelude::*;
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
//!         on_frame_image_ready = async_hn!(|args| {
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
//! use zng::{
//!     APP,
//!     app::{AppExtended, AppExtension},
//! };
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
//! # use zng::var::*;
//! #[expect(non_camel_case_types)]
//! pub struct SCREAMING_CASE;
//! impl SCREAMING_CASE {
//!     pub fn state(&self) -> Var<bool> {
//!         # var(true)
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
//! A common pattern in the zng services is to name the app locals with a `_SV` suffix.
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
//! This is even true for the [`INSTANT`] service, although this can be configured for this service using [`APP.pause_time_for_update`].
//!
//! [`APP.pause_time_for_update`]: zng_app::APP::pause_time_for_update
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
//! use zng::{app::AppExtension, prelude_wgt::*};
//!
//! /// Foo service.
//! pub struct FOO;
//!
//! impl FOO {
//!     /// Foo read-write var.
//!     pub fn config(&self) -> Var<bool> {
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
//!     config: Var<bool>,
//!     requests: Vec<(char, ResponderVar<char>)>,
//! }
//!
//! app_local! {
//!     static FOO_SV: FooService = FooService {
//!         config: var(false),
//!         requests: vec![],
//!     };
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
//! #[non_exhaustive]
//! pub struct FooManager {}
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
//! # Init & Main Loop
//!
//! A headed app initializes in this sequence:
//!
//! 1. [`AppExtension::register`] is called.
//! 2. Spawn view-process.
//! 3. [`AppExtension::init`] is called.
//! 4. Schedule the app run future to run in the first preview update.
//! 5. Does [updates loop](#updates-loop).
//! 6. Does [update events loop](#update-events-loop).
//! 7. Does [main loop](#main-loop).
//!
//! #### Main Loop
//!
//! The main loop coordinates view-process events, timers, app events and updates. There is no scheduler, update and event requests
//! are captured and coalesced to various buffers that are drained in known sequential order. App extensions update one at a time
//! in the order they are registered. Windows and widgets update in parallel by default, this is controlled by [`WINDOWS.parallel`] and [`parallel`].
//!
//! 1. Sleep if there are not pending events or updates.
//!    * If the view-process is busy blocks until it sends a message, this is a mechanism to stop the app-process
//!      from overwhelming the view-process.
//!    * Block until a message is received, from the view-process or from other app threads.
//!    * If there are [`TIMERS`] or [`VARS`] animations the message block has a deadline to the nearest timer or animation frame.
//!        * Animations have a fixed frame-rate defined in [`VARS.frame_duration`], it is 60 frames-per-second by default.
//! 2. Calls elapsed timer handlers.
//! 3. Calls elapsed animation handlers.
//!     * These handlers mostly just request var updates that are applied in the updates loop.
//! 4. Does a [view events loop](#view-events-loop).
//! 4. Does an [updates loop](#updates-loop).
//! 5. Does an [update events loop](#update-events-loop).
//! 6. If the view-process is not busy does a [layout loop and render](#layout-loop-and-render).
//! 7. If exit was requested and not cancelled breaks the loop.
//!     * Exit is requested automatically when the last open window closes, this is controlled by [`WINDOWS.exit_on_last_close`].
//!     * Exit can also be requested using [`APP.exit`].
//!
//! #### View Events Loop
//!
//! All pending events received from the view-process are coalesced and notify sequentially.
//!
//! 1. For each event in the received order (FIFO) that converts to a `RAW_*_EVENT`.
//!     1. Calls [`AppExtension::event_preview`].
//!     2. Calls [`Event::on_pre_event`] handlers.
//!     3. Calls [`AppExtension::event_ui`].
//!         * Raw events don't target any widget, but widgets can subscribe, subscribers receive the event in parallel by default.
//!     4. Calls [`AppExtension::event`].
//!     5. Calls [`Event::on_event`] handlers.
//!     6. Does an [updates loop](#updates-loop).
//! 2. Frame rendered raw event.
//!     * Same notification sequence as other view-events, just delayed.
//!
//! #### Updates Loop
//!
//! The updates loop rebuilds info trees if needed , applies pending variable updates and hooks and collects event updates
//! requested by the app.
//!
//! 1. Takes info rebuild request flag.
//!     * Calls [`AppExtension::info`] if needed.
//!     * Windows and widgets that requested info (re)build are called.
//!     * Info rebuild happens in parallel by default.
//! 2. Takes events and updates requests.
//!     1. Event hooks are called for new event requests.
//!         * Full event notification is delayed to after the updates loop.
//!     2. [var updates loop](#var-updates-loop)
//!     3. Calls [`AppExtension::update_preview`] if any update was requested.
//!     4. Calls [`UPDATES.on_pre_update`] handlers if needed.
//!     5. Calls [`AppExtension::update_ui`] if any update was requested.
//!         * Windows and widgets that requested update receive it here.
//!         * All the pending updates are processed in one pass, all targeted widgets are visited once, in parallel by default.
//!     6. Calls [`AppExtension::update`] if any update was requested.
//!     7. Calls [`UPDATES.on_update`] handlers if needed.
//! 3. The loop repeats immediately if any info rebuild or update was requested by update callbacks.
//!     * The loops breaks if it repeats over 1000 times.
//!     * An error is logged with a trace of the most frequent sources of update requests.
//!
//! #### Var Updates Loop
//!
//! The variable updates loop applies pending modifications, calls hooks to update variable and bindings.
//!
//! 1. Pending variable modifications are applied.
//! 2. Var hooks are called.
//!     * The mapping and binding mechanism is implemented using hooks.
//! 3. The loop repeats until hooks have stopped modifying variables.
//!     * The loop breaks if it repeats over 1000 times.
//!     * An error is logged if this happens.
//!
//! #### Update Events Loop
//!
//! The update events loop notifies each event raised by the app code during previous updates.
//!
//! 1. For each event in the request order (FIFO).
//!     1. Calls [`AppExtension::event_preview`].
//!     2. Calls [`Event::on_pre_event`] handlers.
//!     3. Calls [`AppExtension::event_ui`].
//!         * Windows and widgets targeted by the event update receive it here.
//!         * If the event targets multiple widgets they receive it in parallel by default.
//!     4. Calls [`AppExtension::event`].
//!     5. Calls [`Event::on_event`] handlers.
//!     6. Does an [updates loop](#updates-loop).
//!
//! #### Layout Loop and Render
//!
//! Layout and render requests are coalesced, multiple layout requests for the same widget update it once, multiple
//! render requests become one frame, and if both `render` and `render_update` are requested for a window it will just fully `render`.
//!
//! 1. Take layout and render requests.
//! 2. Layout loop.
//!     1. Calls [`AppExtension::layout`].
//!         * Windows and widgets that requested layout update in parallel by default.
//!     2. Does an [updates loop](#updates-loop).
//!     3. Does [update events loop](#update-events-loop).
//!     3. Take layout and render requests, the loop repeats immediately if layout was requested again.
//!         * The loop breaks if it repeats over 1000 times.
//!         * An error is logged with a trace the most frequent sources of update requests.
//! 3. If render was requested, calls [`AppExtension::render`].
//!     * Windows and widgets that requested render (or render_update) are rendered in parallel by default.
//!     * The render pass updates widget transforms and hit-test, generates a display list and sends it to the view-process.
//!
//! [`APP.defaults()`]: crate::APP::defaults
//! [`UPDATES.update`]: crate::update::UPDATES::update
//! [`task`]: crate::task
//! [`ResponseVar<R>`]: crate::var::ResponseVar
//! [`TIMERS`]: crate::timer::TIMERS
//! [`VARS`]: crate::var::VARS
//! [`VARS.frame_duration`]: crate::var::VARS::frame_duration
//! [`WINDOWS.parallel`]: crate::window::WINDOWS::parallel
//! [`parallel`]: fn@crate::widget::parallel
//! [`UPDATES.on_pre_update`]: crate::update::UPDATES::on_pre_update
//! [`UPDATES.on_update`]: crate::update::UPDATES::on_update
//! [`Event::on_pre_event`]: crate::event::Event::on_pre_event
//! [`Event::on_event`]: crate::event::Event::on_event
//! [`WINDOWS.exit_on_last_close`]: crate::window::WINDOWS::exit_on_last_close
//! [`APP.exit`]: crate::APP#method.exit
//!
//! # Full API
//!
//! This module provides most of the app API needed to make and extend apps, some more advanced or experimental API
//! may be available at the [`zng_app`], [`zng_app_context`] and [`zng_ext_single_instance`] base crates.

pub use zng_app::{
    AppControlFlow, AppEventObserver, AppExtended, AppExtension, AppExtensionBoxed, AppExtensionInfo, AppStartArgs, DInstant, Deadline,
    EXIT_CMD, EXIT_REQUESTED_EVENT, ExitRequestedArgs, HeadlessApp, INSTANT, InstantMode, on_app_start, print_tracing,
    print_tracing_filter,
};

#[cfg(feature = "test_util")]
pub use zng_app::test_log;

pub use zng_app_context::{
    AppId, AppLocal, AppScope, CaptureFilter, ContextLocal, ContextValueSet, LocalContext, MappedRwLockReadGuardOwned,
    MappedRwLockWriteGuardOwned, ReadOnlyRwLock, RunOnDrop, RwLockReadGuardOwned, RwLockWriteGuardOwned, app_local, context_local,
};
pub use zng_wgt_input::cmd::{
    NEW_CMD, OPEN_CMD, SAVE_AS_CMD, SAVE_CMD, can_new, can_open, can_save, can_save_as, on_new, on_open, on_pre_new, on_pre_open,
    on_pre_save, on_pre_save_as, on_save, on_save_as,
};

pub use zng_app::view_process::raw_events::{LOW_MEMORY_EVENT, LowMemoryArgs};

/// Input device hardware ID and events.
///
/// # Full API
///
/// See [`zng_app::view_process::raw_device_events`] for the full API.
pub mod raw_device_events {
    pub use zng_app::view_process::raw_device_events::{
        AXIS_MOTION_EVENT, AxisId, AxisMotionArgs, INPUT_DEVICES, INPUT_DEVICES_CHANGED_EVENT, InputDeviceCapability, InputDeviceId,
        InputDeviceInfo, InputDevicesChangedArgs,
    };
}

#[cfg(single_instance)]
pub use zng_ext_single_instance::{APP_INSTANCE_EVENT, AppInstanceArgs};

/// App-process crash handler.
///
/// In builds with `"crash_handler"` feature the crash handler takes over the first "app-process" turning it into
/// the monitor-process, it spawns another process that is the monitored app-process. If the app-process crashes
/// the monitor-process spawns a dialog-process that calls the dialog handler to show an error message, upload crash reports, etc.
///
/// The dialog handler can be set using [`crash_handler_config!`].
///
/// [`crash_handler_config!`]: crate::app::crash_handler::crash_handler_config
///
/// # Examples
///
/// The example below demonstrates an app setup to show a custom crash dialog.
///
/// ```no_run
/// use zng::prelude::*;
///
/// fn main() {
///     // tracing applied to all processes.
///     zng::app::print_tracing(tracing::Level::INFO);
///
///     // monitor-process spawns app-process and if needed dialog-process here.
///     zng::env::init!();
///
///     // app-process:
///     app_main();
/// }
///
/// fn app_main() {
///     APP.defaults().run_window(async {
///         Window! {
///             child_align = Align::CENTER;
///             child = Stack! {
///                 direction = StackDirection::top_to_bottom();
///                 spacing = 5;
///                 children = ui_vec![
///                     Button! {
///                         child = Text!("Crash (panic)");
///                         on_click = hn_once!(|_| {
///                             panic!("Test panic!");
///                         });
///                     },
///                     Button! {
///                         child = Text!("Crash (access violation)");
///                         on_click = hn_once!(|_| {
///                             // SAFETY: deliberate access violation
///                             #[expect(deref_nullptr)]
///                             unsafe {
///                                 *std::ptr::null_mut() = true;
///                             }
///                         });
///                     }
///                 ];
///             };
///         }
///     });
/// }
///
/// zng::app::crash_handler::crash_handler_config!(|cfg| {
///     // monitor-process and dialog-process
///
///     cfg.dialog(|args| {
///         // dialog-process
///         APP.defaults().run_window(async move {
///             Window! {
///                 title = "App Crashed!";
///                 auto_size = true;
///                 min_size = (300, 100);
///                 start_position = window::StartPosition::CenterMonitor;
///                 on_load = hn_once!(|_| WINDOW.bring_to_top());
///                 padding = 10;
///                 child = Text!(args.latest().message());
///                 child_bottom =
///                     Stack! {
///                         direction = StackDirection::start_to_end();
///                         layout::align = Align::BOTTOM_END;
///                         spacing = 5;
///                         children = ui_vec![
///                             Button! {
///                                 child = Text!("Restart App");
///                                 on_click = hn_once!(args, |_| {
///                                     args.restart();
///                                 });
///                             },
///                             Button! {
///                                 child = Text!("Exit App");
///                                 on_click = hn_once!(|_| {
///                                     args.exit(0);
///                                 });
///                             },
///                         ];
///                     },
///                     10,
///                 ;
///             }
///         });
///     });
/// });
/// ```
///
/// # Debugger
///
/// Note that because the crash handler spawns a different process for the app debuggers will not
/// stop at break points in the app code. You can configure your debugger to set the `NO_ZNG_CRASH_HANDLER` environment
/// variable to not use a crash handler in debug runs.
///
/// On VS Code with the CodeLLDB extension you can add this workspace configuration:
///
/// ```json
/// "lldb.launch.env": {
///    "ZNG_NO_CRASH_HANDLER": ""
/// }
/// ```
///
/// # Full API
///
/// See [`zng_app::crash_handler`] and [`zng_wgt_inspector::crash_handler`] for the full API.
#[cfg(crash_handler)]
pub mod crash_handler {
    pub use zng_app::crash_handler::{BacktraceFrame, CrashArgs, CrashConfig, CrashError, CrashPanic, crash_handler_config};

    #[cfg(feature = "crash_handler_debug")]
    pub use zng_wgt_inspector::crash_handler::debug_dialog;

    crash_handler_config!(|cfg| {
        cfg.default_dialog(|args| {
            if let Some(c) = &args.dialog_crash {
                eprintln!("DEBUG CRASH DIALOG ALSO CRASHED");
                eprintln!("   {}", c.message());
                eprintln!("ORIGINAL APP CRASH");
                eprintln!("   {}", args.latest().message());
                args.exit(0xBADC0DE)
            } else {
                #[cfg(feature = "crash_handler_debug")]
                {
                    use crate::prelude::*;
                    APP.defaults()
                        .run_window(async_clmv!(args, { zng_wgt_inspector::crash_handler::debug_dialog(args) }));
                }

                #[cfg(not(feature = "crash_handler_debug"))]
                {
                    eprintln!(
                        "app crashed {}\n\nbuild with feature = \"crash_handler_debug\" to se the debug crash dialog",
                        args.latest().message()
                    );
                }
            }
            args.exit(0)
        });
    });
}

/// Trace recording and data model.
///
/// All tracing instrumentation in Zng projects is done using the `tracing` crate, trace recording is done using the `tracing-chrome` crate.
/// The recorded traces can be viewed in `chrome://tracing` or `ui.perfetto.dev` and can be parsed by the [`Trace`] data model.
///
/// Run the app with the `"ZNG_RECORD_TRACE"` env var set to record the app-process and all other processes spawned by the app.
///
/// ```no_run
/// use zng::prelude::*;
///
/// fn main() {
///     unsafe {
///         std::env::set_var("ZNG_RECORD_TRACE", "");
///     }
///     unsafe {
///         std::env::set_var("ZNG_RECORD_TRACE_FILTER", "debug");
///     }
///
///     // recording start here for all app processes when ZNG_RECORD_TRACE is set.
///     zng::env::init!();
///
///     // .. app
/// }
/// ```
///
/// The example above hardcodes trace recording for all app processes by setting the `"ZNG_RECORD_TRACE"` environment
/// variable before the `init!()` call. It also sets `"ZNG_RECORD_TRACE_FILTER"` to a slightly less verbose level.
///
/// # Config
///
/// The `"ZNG_RECORD_TRACE_DIR"` variable can be set to define a custom `output-dir` directory path, relative to the current dir.
/// The default dir is `"./zng-trace/"`.
///
/// The `"ZNG_RECORD_TRACE_FILTER"` or `"RUST_LOG"` variables can be used to set custom tracing filters, see the [filter syntax] for details.
/// The default filter is `"trace"` that records all spans and events.
///
/// # Output
///
/// Raw trace files are saved to `"{--output-dir}/{timestamp}/{pid}.json"`. If the `--output-dir` is not provided
/// it is `"{current_dir}/zng-trace/"`.
///
/// The dir timestamp is in microseconds from Unix epoch and is defined by the first process that runs.
///
/// The process name is defined by an event INFO message that reads `"pid: {pid}, name: {name}"`. See [`zng::env::process_name`] for more details.
///
/// The process record start timestamp is defined by an event INFO message that reads `"zng-record-start: {timestamp}"`. This timestamp is also
/// in microseconds from Unix epoch.
///
/// # Cargo Zng
///
/// You can also use the `cargo zng trace` subcommand to record traces, it handles setting the env variables, merges the multi
/// process traces into a single file and properly names the processes for better compatibility with trace viewers.
///
/// ```console
/// cargo zng trace --filter debug "path/my-exe"
/// ```
///
/// You can also run using custom commands after `--`:
///
/// ```console
/// cargo zng trace -- cargo run my-exe
/// ```
///
/// Call `cargo zng trace --help` for more details.
///
/// # Full API
///
/// See [`zng_app::trace_recorder`] for the full API.
///
/// [`Trace`]: zng::app::trace_recorder::Trace
/// [filter syntax]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html#filtering-events-with-environment-variables
#[cfg(trace_recorder)]
pub mod trace_recorder {
    pub use zng_app::trace_recorder::{EventTrace, ProcessTrace, ThreadTrace, Trace, stop_recording};
}
