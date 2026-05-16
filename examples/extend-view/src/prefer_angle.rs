//! Enable the built in extension "zng-view.prefer_angle".

use zng::prelude_wgt::*;
use zng_app::view_process::{VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT};
use zng_ext_window::{WINDOW_Ext, WINDOWS_EXTENSIONS};
use zng_view_api::api_extension::{ApiExtensionId, ApiExtensionPayload};

pub fn extension_id() -> ApiExtensionId {
    VIEW_PROCESS
        .extension_id("zng-view.prefer_angle")
        .ok()
        .flatten()
        .unwrap_or(ApiExtensionId::INVALID)
}

///Use ANGLE as the OpenGL backend on Windows.
///
/// Note that ANGLE requires some DLLs, see the `build.rs` script for more details.
#[property(CONTEXT)]
pub fn use_angle_egl(child: impl IntoUiNode, enable: impl IntoValue<bool>) -> UiNode {
    let enable = enable.into() && cfg!(windows);
    match_node(child, move |_c, op| {
        if let UiNodeOp::Init = op
            && enable
        {
            if VIEW_PROCESS.is_connected() {
                WINDOWS_EXTENSIONS
                    .view_extensions_init(WINDOW.id(), extension_id(), ApiExtensionPayload::serialize(&true).unwrap())
                    .unwrap();
            }
            let win_id = WINDOW.id();
            let mut loading_handle = WINDOW.loading_handle(5.secs(), "use_angle_egl");
            let handle = VIEW_PROCESS_INITED_EVENT.hook(move |_| {
                let _ = loading_handle.take();
                WINDOWS_EXTENSIONS
                    .view_extensions_init(win_id, extension_id(), ApiExtensionPayload::serialize(&true).unwrap())
                    .unwrap();

                // retain in case of respawn
                true
            });
            WIDGET.push_var_handle(handle);
        }
    })
}
