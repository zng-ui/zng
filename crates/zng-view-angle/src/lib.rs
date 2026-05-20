#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Helpers for enabling ANGLE support of the ZNG apps on Windows.
//!
//! The default ZNG view-process implementation will attempt to dynamically load ANGLE DLLs if requested,
//! this crate helps signal the view-process and acquire the DLLs.
//!
//! The following is a minimal setup example:
//!
//! **First** add the crate dependency, ZNG only supports ANGLE on Windows so the dependency can be conditional.
//!
//! ```toml
//! [target.'cfg(windows)'.dependencies]
//! zng-view-angle = { version = "0.1.0", features = ["download"] }
//! ```
//!
//! With the `"download"` feature enabled the crate will download the required ANGLE DLLs from [zng-ui/build-angle]
//! and copy them to the output dir. Its only two files `libEGL.dll` and `libGLESv2.dll`, around 5MB total, you must
//! ensure these files are packaged with the app installer.
//!
//! **Second** signal the view-process to use ANGLE:
//!
//! ```
//! # macro_rules! demo { () => {
//! use zng::prelude::*;
//!
//! fn main() {
//!     zng::env::init!();
//!
//!     APP.defaults().run_window(async {
//!         #[cfg(windows)]
//!         {
//!             zng_view_angle::register_license();
//!             zng_view_angle::register_root_extender();
//!         }
//!
//!         Window! {
//!             // ...
//!         }
//!     });
//! }
//! # } }
//! ```
//!
//! The [`register_root_extender`] will insert a node in all subsequent windows that signals the view-process to
//! find and use the ANGLE DLLs when render mode uses the GPU. If the DLLs are not found the native OpenGL driver is used instead.
//!
//! The [`register_license`] simply adds the ANGLE BSD3-Clause license to the [`LICENSES`] service.
//!
//! [zng-ui/build-angle]: https://github.com/zng-ui/build-angle
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]

use zng_app::{
    third_party::{LICENSES, License, LicenseUsed, User},
    view_process::{VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT},
};
use zng_ext_window::{WINDOW_Ext as _, WINDOWS_EXTENSIONS};
use zng_view_api::api_extension::*;
use zng_wgt::prelude::*;

fn init(win_id: WindowId) {
    const EXT_NAME: &str = "zng-view.prefer_angle";
    let ext_id = VIEW_PROCESS
        .extension_id(EXT_NAME)
        .ok()
        .flatten()
        .unwrap_or(ApiExtensionId::INVALID);
    if ext_id == ApiExtensionId::INVALID {
        return tracing::error!("view-process does not support {EXT_NAME:?}");
    }

    WINDOWS_EXTENSIONS
        .view_extensions_init(win_id, ext_id, ApiExtensionPayload::serialize(&true).unwrap())
        .unwrap();
}

/// Signal the view-process to try and use EGL for this window when the ANGLE DLLs can be found and the render mode is not software.
///
/// Note that the `libEGL.dll` and `libGLESv2.dll` files must be distributed with the executable.
///
/// Note that this will enable ANGLE just for this window, use [`register_root_extender`] to enable angle for all windows.
///
/// This does nothing when app is not running on Windows.
#[property(CONTEXT)]
pub fn prefer_angle_egl(child: impl IntoUiNode, enable: impl IntoValue<bool>) -> UiNode {
    if enable.into() && cfg!(windows) {
        match_node(child, move |_c, op| {
            if let UiNodeOp::Init = op {
                let win_id = WINDOW.id();

                let mut loading_handle = None;
                if VIEW_PROCESS.is_connected() {
                    init(win_id);
                } else {
                    loading_handle = WINDOW.loading_handle(5.secs(), "prefer_angle_egl");
                }
                let handle = VIEW_PROCESS_INITED_EVENT.hook(move |_| {
                    let _ = loading_handle.take();
                    init(win_id);
                    // retain in case of respawn
                    true
                });
                WIDGET.push_var_handle(handle);
            }
        })
    } else {
        child.into_node()
    }
}

/// Enable ANGLE for all subsequent open windows.
///
/// This uses a [`WINDOWS_EXTENSIONS::register_root_extender`] to set [`prefer_angle_egl`] in all windows.
///
/// [`prefer_angle_egl`]: fn@prefer_angle_egl
pub fn register_root_extender() {
    WINDOWS_EXTENSIONS.register_root_extender(|a| prefer_angle_egl(a.root, true));
}

/// Register the ANGLE license with the [`LICENSES`] service.
///
/// The license is a [BSD3-Clause type license](https://github.com/google/angle/blob/fca8ca8a87c387515e0d2901916e4fae6b97e83f/LICENSE).
pub fn register_license() {
    fn l() -> Vec<LicenseUsed> {
        vec![LicenseUsed {
            license: License::new("BSD3-Clause", r#"BSD 3-Clause "Revised" License"#, LICENSE),
            used_by: vec![User::new(
                std::env!("CARGO_PKG_NAME"),
                std::env!("CARGO_PKG_VERSION"),
                "https://github.com/zng-ui/zng",
            )],
        }]
    }
    LICENSES.register(l);
}

const LICENSE: &str = r#"
// Copyright 2018 The ANGLE Project Authors.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
//     Redistributions of source code must retain the above copyright
//     notice, this list of conditions and the following disclaimer.
//
//     Redistributions in binary form must reproduce the above
//     copyright notice, this list of conditions and the following
//     disclaimer in the documentation and/or other materials provided
//     with the distribution.
//
//     Neither the name of TransGaming Inc., Google Inc., 3DLabs Inc.
//     Ltd., nor the names of their contributors may be used to endorse
//     or promote products derived from this software without specific
//     prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
// FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
// COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
// INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
// LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
// ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
"#;
