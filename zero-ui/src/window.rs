//! Window service, widget, events, commands and types.
//!
//! The [`Window!`](struct@Window) widget declares a window root.
//!
//! The example below declares a window that can toggle if it can close.
//!
//! ```
//! # fn main() {}
//! use zero_ui::prelude::*;
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
//!         title = "Title";
//!         child_align = layout::Align::CENTER;
//!         child = Toggle! {
//!             child = Text!(allow_close.map(|a| formatx!("allow_close = {a:?}")));
//!             checked = allow_close;
//!         }
//!     }
//! }
//! ```
//!
//! The [`WINDOWS`] service can be used to open, manage and close windows. The example below uses the service
//! to open a parent and child window.
//!
//! ```
//! use zero_ui::prelude::*;
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
//!                         WINDOWS.open_id("child-id", async move { child_window(parent) }).wait_done().await;
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
//! See [`zero_ui_ext_window`], [`zero_ui_app::window`] and [`zero_ui_wgt_window`] for the full window API.

pub use zero_ui_app::window::{MonitorId, StaticMonitorId, StaticWindowId, WindowId, WindowMode, WINDOW};

pub use zero_ui_ext_window::{
    AppRunWindowExt, AutoSize, CloseWindowResult, FocusIndicator, FrameCaptureMode, FrameImageReadyArgs, HeadlessAppWindowExt,
    HeadlessMonitor, ImeArgs, MonitorInfo, MonitorQuery, MonitorsChangedArgs, ParallelWin, RenderMode, RendererDebug, StartPosition,
    VideoMode, WINDOW_Ext, WidgetInfoBuilderImeArea, WidgetInfoImeArea, WindowChangedArgs, WindowCloseArgs, WindowCloseRequestedArgs,
    WindowIcon, WindowLoadingHandle, WindowOpenArgs, WindowRoot, WindowRootExtenderArgs, WindowState, WindowStateAllowed, WindowVars,
    FRAME_IMAGE_READY_EVENT, IME_EVENT, MONITORS, MONITORS_CHANGED_EVENT, WINDOWS, WINDOW_CHANGED_EVENT, WINDOW_CLOSE_EVENT,
    WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_LOAD_EVENT, WINDOW_OPEN_EVENT,
};

pub use zero_ui_view_api::webrender_api::DebugFlags;

/// Window commands.
pub mod cmd {
    pub use zero_ui_ext_window::cmd::*;

    #[cfg(inspector)]
    pub use zero_ui_wgt_inspector::INSPECT_CMD;
}

pub use zero_ui_wgt_window::{BlockWindowLoad, SaveState, Window};

pub use zero_ui_wgt_window::events::{
    on_frame_image_ready, on_ime, on_pre_frame_image_ready, on_pre_ime, on_pre_window_changed, on_pre_window_close_requested,
    on_pre_window_exited_fullscreen, on_pre_window_fullscreen, on_pre_window_load, on_pre_window_maximized, on_pre_window_minimized,
    on_pre_window_moved, on_pre_window_open, on_pre_window_resized, on_pre_window_restored, on_pre_window_state_changed,
    on_pre_window_unmaximized, on_pre_window_unminimized, on_window_changed, on_window_close_requested, on_window_exited_fullscreen,
    on_window_fullscreen, on_window_load, on_window_maximized, on_window_minimized, on_window_moved, on_window_open, on_window_resized,
    on_window_restored, on_window_state_changed, on_window_unmaximized, on_window_unminimized,
};

/// Native dialog types.
pub mod native_dialog {
    pub use zero_ui_view_api::dialog::{
        FileDialog, FileDialogKind, FileDialogResponse, MsgDialog, MsgDialogButtons, MsgDialogIcon, MsgDialogResponse,
    };
}

/// Debug inspection helpers.
///
/// # Full API
///
/// See [`zero_ui_wgt_inspector`] for the full API.
pub mod inspector {
    pub use zero_ui_wgt_inspector::debug::{
        show_bounds, show_center_points, show_directional_query, show_hit_test, show_rows, InspectMode,
    };
}
