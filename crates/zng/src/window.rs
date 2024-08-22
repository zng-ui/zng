//! Window service, widget, events, commands and other types.
//!
//! The [`Window!`](struct@Window) widget instantiates a window root, the windows service uses the window root as the
//! root widget of new window.
//!
//! The example below declares a window that toggles if it can close.
//!
//! ```
//! # fn main() {}
//! use zng::prelude::*;
//!
//! fn app() {
//!     APP.defaults().run_window(async { window() });
//! }
//!
//! fn window() -> window::WindowRoot {
//!     let allow_close = var(true);
//!     Window! {
//!         on_close_requested = hn!(allow_close, |args: &window::WindowCloseRequestedArgs| {
//!             if !allow_close.get() {
//!                 args.propagation().stop();
//!             }
//!         });
//!
//!         title = "Can I Close?";
//!         child_align = layout::Align::CENTER;
//!         child = Toggle! {
//!             child = Text!(allow_close.map(|a| formatx!("allow close = {a:?}")));
//!             checked = allow_close;
//!         }
//!     }
//! }
//! ```
//!
//! The [`WINDOWS`] service can be used to open, manage and close windows. The example below
//! opens a parent and child window.
//!
//! ```
//! use zng::prelude::*;
//!
//! fn app() {
//!     APP.defaults().run(async {
//!         let r = WINDOWS.open(async { main_window() });
//!         println!("opened {}", r.wait_rsp().await);
//!     });
//! }
//!
//! fn main_window() -> window::WindowRoot {
//!     Window! {
//!         title = "Main Window";
//!         child_align = layout::Align::CENTER;
//!         child = {
//!             let enabled = var(true);
//!             Button! {
//!                 child = Text!("Open/Close Child");
//!                 on_click = async_hn!(enabled, |_| {
//!                     enabled.set(false);
//!
//!                     if WINDOWS.is_open("child-id") {
//!                         if let Ok(r) = WINDOWS.close("child-id") {
//!                             r.wait_done().await;
//!                         }
//!                     } else {
//!                         let parent = WINDOW.id();
//!                         WINDOWS.open_id(
//!                             "child-id",
//!                             async move { child_window(parent) }
//!                         )
//!                         .wait_done()
//!                         .await;
//!                     }
//!
//!                     enabled.set(true);
//!                 });
//!                 widget::enabled;
//!             }
//!         }
//!     }
//! }
//!
//! fn child_window(parent: WindowId) -> window::WindowRoot {
//!     Window! {
//!         parent;
//!         title = "Child Window";
//!         size = (200, 100);
//!         child = Button! {
//!             child = Text!("Close");
//!             on_click = hn!(|_| {
//!                 let _ = WINDOW.close();
//!             });
//!         };
//!     }
//! }
//! ```
//!
//! # Full API
//!
//! See [`zng_ext_window`], [`zng_app::window`] and [`zng_wgt_window`] for the full window API.

pub use zng_app::window::{MonitorId, WindowId, WindowMode, WINDOW};

pub use zng_ext_window::{
    AppRunWindowExt, AutoSize, CloseWindowResult, FocusIndicator, FrameCaptureMode, FrameImageReadyArgs, HeadlessAppWindowExt,
    HeadlessMonitor, ImeArgs, MonitorInfo, MonitorQuery, MonitorsChangedArgs, ParallelWin, RenderMode, StartPosition, VideoMode,
    WINDOW_Ext, WidgetInfoBuilderImeArea, WidgetInfoImeArea, WindowButton, WindowChangedArgs, WindowCloseArgs, WindowCloseRequestedArgs,
    WindowIcon, WindowLoadingHandle, WindowOpenArgs, WindowRoot, WindowRootExtenderArgs, WindowState, WindowStateAllowed, WindowVars,
    FRAME_IMAGE_READY_EVENT, IME_EVENT, MONITORS, MONITORS_CHANGED_EVENT, WINDOWS, WINDOW_CHANGED_EVENT, WINDOW_CLOSE_EVENT,
    WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_LOAD_EVENT, WINDOW_OPEN_EVENT,
};

/// Window commands.
pub mod cmd {
    pub use zng_ext_window::cmd::*;

    #[cfg(feature = "inspector")]
    pub use zng_wgt_inspector::INSPECT_CMD;
}

pub use zng_wgt_window::{BlockWindowLoad, Window, IS_MOBILE_VAR, is_mobile};

pub use zng_wgt_window::events::{
    on_frame_image_ready, on_ime, on_pre_frame_image_ready, on_pre_ime, on_pre_window_changed, on_pre_window_close_requested,
    on_pre_window_exited_fullscreen, on_pre_window_fullscreen, on_pre_window_load, on_pre_window_maximized, on_pre_window_minimized,
    on_pre_window_moved, on_pre_window_open, on_pre_window_resized, on_pre_window_restored, on_pre_window_state_changed,
    on_pre_window_unmaximized, on_pre_window_unminimized, on_window_changed, on_window_close_requested, on_window_exited_fullscreen,
    on_window_fullscreen, on_window_load, on_window_maximized, on_window_minimized, on_window_moved, on_window_open, on_window_resized,
    on_window_restored, on_window_state_changed, on_window_unmaximized, on_window_unminimized,
};

/// Debug inspection helpers.
///
/// The properties in this module can be set on a window or widget to visualize layout and render internals.
///
/// Note that you can also use the [`cmd::INSPECT_CMD`] command to open the Inspector that shows the widget tree and properties.
///
/// # Full API
///
/// See [`zng_wgt_inspector`] for the full API.
///
/// [`cmd::INSPECT_CMD`]: crate::window::cmd::INSPECT_CMD
pub mod inspector {
    pub use zng_wgt_inspector::debug::{show_bounds, show_center_points, show_directional_query, show_hit_test, show_rows, InspectMode};
}
