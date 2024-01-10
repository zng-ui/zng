//! Window service, widget, events, commands and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_window`], [`zero_ui_app::window`] and [`zero_ui_wgt_window`] for the full window API.

pub use zero_ui_app::window::{MonitorId, StaticMonitorId, StaticWindowId, WindowId, WindowMode, WINDOW};

pub use zero_ui_ext_window::{
    AppRunWindowExt, AutoSize, CloseWindowResult, FocusIndicator, FrameCaptureMode, FrameImageReadyArgs, HeadlessAppWindowExt,
    HeadlessMonitor, ImeArgs, MonitorInfo, MonitorQuery, MonitorsChangedArgs, ParallelWin, RenderMode, RendererDebug, StartPosition,
    VideoMode, WINDOW_Ext, WidgetInfoBuilderImeArea, WidgetInfoImeArea, WindowChangedArgs, WindowChrome, WindowCloseArgs,
    WindowCloseRequestedArgs, WindowIcon, WindowLoadingHandle, WindowOpenArgs, WindowRoot, WindowRootExtenderArgs, WindowState,
    WindowStateAllowed, WindowVars, FRAME_IMAGE_READY_EVENT, IME_EVENT, MONITORS, MONITORS_CHANGED_EVENT, WINDOWS, WINDOW_CHANGED_EVENT,
    WINDOW_CLOSE_EVENT, WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_LOAD_EVENT, WINDOW_OPEN_EVENT,
};

pub use zero_ui_view_api::webrender_api::DebugFlags;

/// Window commands.
pub mod cmd {
    pub use zero_ui_ext_window::cmd::*;

    #[cfg(inspector)]
    pub use zero_ui_wgt_inspector::INSPECT_CMD;
}

pub use zero_ui_wgt_window::{SaveState, Window};

/// Native dialog types.
pub mod native_dialog {
    pub use zero_ui_view_api::dialog::{
        FileDialog, FileDialogKind, FileDialogResponse, MsgDialog, MsgDialogButtons, MsgDialogIcon, MsgDialogResponse,
    };
}
