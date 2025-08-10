//! Enable the built in extension "zng-view.prefer_angle".

use zng::{prelude_wgt::*, window::WINDOWS};
use zng_app::view_process::VIEW_PROCESS;
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
        if let UiNodeOp::Init = op {
            if enable {
                WINDOWS
                    .view_extensions_init(WINDOW.id(), extension_id(), ApiExtensionPayload::serialize(&true).unwrap())
                    .unwrap();
            }
        }
    })
}
