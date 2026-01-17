#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Clipboard app extension, service and commands.
//!
//! # Services
//!
//! Services provided by this extension.
//!
//! * [`CLIPBOARD`]
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{collections::HashMap, fmt, path::PathBuf, sync::Arc};

use zng_app::{command, event::*, shortcut::*, update::UPDATES, view_process::VIEW_PROCESS};
use zng_ext_image::{IMAGES, ImageEntry, ImageVar};
use zng_task::channel::{ChannelError, IpcBytes};
use zng_txt::{ToTxt, Txt};
use zng_var::{ResponseVar, response_var};
use zng_view_api::{
    clipboard::{ClipboardError as ViewError, ClipboardTypes},
    image::ImageDecoded,
};
use zng_wgt::{CommandIconExt as _, ICONS, wgt_fn};

/// Error getting or setting the clipboard.
///
/// The [`CLIPBOARD`] service already logs the error.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ClipboardError {
    /// No view-process available to process the request.
    ///
    /// Note that this error only happens if the view-process is respawning. For headless apps (without renderer)
    /// a in memory "clipboard" is used and this error does not return.
    Disconnected,
    /// View-process or operating system does not support the data type.
    NotSupported,
    /// Cannot set image in clipboard because it has not finished loading or loaded with error.
    ImageNotLoaded,
    /// Other error.
    ///
    /// The string can be a debug description of the error, only suitable for logging.
    Other(Txt),
}
impl std::error::Error for ClipboardError {}
impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::Disconnected => write!(f, "no view-process available to process the request"),
            ClipboardError::NotSupported => write!(f, "view-process or operating system does not support the data type"),
            ClipboardError::ImageNotLoaded => write!(
                f,
                "cannot set image in clipboard because it has not finished loading or loaded with error"
            ),
            ClipboardError::Other(e) => write!(f, "{e}"),
        }
    }
}
impl From<ChannelError> for ClipboardError {
    fn from(e: ChannelError) -> Self {
        match e {
            ChannelError::Disconnected { .. } => ClipboardError::Disconnected,
            e => ClipboardError::Other(e.to_txt()),
        }
    }
}
impl From<ViewError> for ClipboardError {
    fn from(e: ViewError) -> Self {
        match e {
            ViewError::NotSupported => ClipboardError::NotSupported,
            e => ClipboardError::Other(e.to_txt()),
        }
    }
}

#[derive(Default)]
struct ClipboardService {
    update_text: std::sync::Weak<Result<Option<Txt>, ClipboardError>>,
    update_image: std::sync::Weak<Result<Option<ImageVar>, ClipboardError>>,
    update_paths: std::sync::Weak<Result<Option<Vec<PathBuf>>, ClipboardError>>,
    update_exts: HashMap<Txt, std::sync::Weak<Result<Option<IpcBytes>, ClipboardError>>>,
}

app_local! {
    static CLIPBOARD_SV: ClipboardService = ClipboardService::default();
}

/// Clipboard service.
///
/// This service synchronizes with the UI update cycle, the getter methods provide the same data for all requests in the
/// same update pass, even if the system clipboard happens to change mid update, the setter methods only set the system clipboard
/// at the end of the update pass.
///
/// This service needs a running view-process to actually interact with the system clipboard, in a headless app
/// without renderer (no view-process) the service will always return [`ClipboardError::Disconnected`].
pub struct CLIPBOARD;
impl CLIPBOARD {
    /// Gets a text string from the clipboard.
    pub fn text(&self) -> Result<Option<Txt>, ClipboardError> {
        let mut s = CLIPBOARD_SV.write();

        match s.update_text.upgrade() {
            // already requested this update, use same value
            Some(r) => (*r).clone(),
            None => {
                // read
                let r = match VIEW_PROCESS.clipboard()?.read_text()? {
                    Ok(r) => Ok(Some(r)),
                    Err(e) => match e {
                        ViewError::NotFound => Ok(None),
                        ViewError::NotSupported => Err(ClipboardError::NotSupported),
                        e => Err(ClipboardError::Other(e.to_txt())),
                    },
                };

                // hold same value until current update ends
                let arc = Arc::new(r.clone());
                s.update_text = Arc::downgrade(&arc);
                UPDATES.once_update("", || {
                    let _hold = arc;
                });

                r
            }
        }
    }
    /// Sets the text string on the clipboard after the current update.
    ///
    /// Returns a response var that updates once the text is set.
    pub fn set_text(&self, txt: impl Into<Txt>) -> ResponseVar<Result<(), ClipboardError>> {
        self.set_text_impl(txt.into())
    }
    fn set_text_impl(&self, txt: Txt) -> ResponseVar<Result<(), ClipboardError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("CLIPBOARD.set_text", move || match VIEW_PROCESS.clipboard() {
            Ok(c) => match c.write_text(txt) {
                Ok(vr) => r.respond(vr.map_err(ClipboardError::from)),
                Err(e) => r.respond(Err(e.into())),
            },
            Err(e) => r.respond(Err(e.into())),
        });
        rsp
    }

    /// Gets an image from the clipboard.
    ///
    /// The image is loaded in parallel by the [`IMAGES`] service, it is not cached.
    ///
    /// [`IMAGES`]: zng_ext_image::IMAGES
    pub fn image(&self) -> Result<Option<ImageVar>, ClipboardError> {
        let mut s = CLIPBOARD_SV.write();

        match s.update_image.upgrade() {
            Some(r) => (*r).clone(),
            None => {
                let r = match VIEW_PROCESS.clipboard()?.read_image()? {
                    Ok(r) => {
                        let r = IMAGES.register(None, (r, ImageDecoded::default()));
                        Ok(Some(r))
                    }
                    Err(e) => match e {
                        ViewError::NotFound => Ok(None),
                        ViewError::NotSupported => Err(ClipboardError::NotSupported),
                        e => Err(ClipboardError::Other(e.to_txt())),
                    },
                };

                let arc = Arc::new(r.clone());
                s.update_image = Arc::downgrade(&arc);
                UPDATES.once_update("", || {
                    let _hold = arc;
                });

                r
            }
        }
    }

    /// Set the image on the clipboard after the current update, if it is loaded.
    ///
    /// Returns a response var that updates once the image is set.
    pub fn set_image(&self, img: ImageEntry) -> ResponseVar<Result<(), ClipboardError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("CLIPBOARD.set_image", move || match VIEW_PROCESS.clipboard() {
            Ok(c) => {
                if img.is_loaded() {
                    match c.write_image(&img.view_handle()) {
                        Ok(vr) => r.respond(vr.map_err(ClipboardError::from)),
                        Err(e) => r.respond(Err(e.into())),
                    }
                } else {
                    r.respond(Err(ClipboardError::ImageNotLoaded));
                }
            }
            Err(e) => r.respond(Err(e.into())),
        });
        rsp
    }

    /// Gets a path list from the clipboard.
    pub fn paths(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        let mut s = CLIPBOARD_SV.write();

        match s.update_paths.upgrade() {
            Some(r) => (*r).clone(),
            None => {
                let r = match VIEW_PROCESS.clipboard()?.read_paths()? {
                    Ok(r) => Ok(Some(r)),
                    Err(e) => match e {
                        ViewError::NotFound => Ok(None),
                        ViewError::NotSupported => Err(ClipboardError::NotSupported),
                        e => Err(ClipboardError::Other(e.to_txt())),
                    },
                };

                let arc = Arc::new(r.clone());
                s.update_paths = Arc::downgrade(&arc);
                UPDATES.once_update("", || {
                    let _hold = arc;
                });

                r
            }
        }
    }

    /// Sets the file list on the clipboard after the current update.
    ///
    /// Returns a response var that updates once the paths are set.
    pub fn set_paths(&self, list: impl Into<Vec<PathBuf>>) -> ResponseVar<Result<(), ClipboardError>> {
        self.set_paths_impl(list.into())
    }
    fn set_paths_impl(&self, list: Vec<PathBuf>) -> ResponseVar<Result<(), ClipboardError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("CLIPBOARD.set_paths", move || match VIEW_PROCESS.clipboard() {
            Ok(c) => match c.write_paths(list) {
                Ok(vr) => r.respond(vr.map_err(ClipboardError::from)),
                Err(e) => r.respond(Err(e.into())),
            },
            Err(e) => r.respond(Err(e.into())),
        });
        rsp
    }

    /// Gets custom data from the clipboard.
    ///
    /// The current view-process must support `data_type`.
    pub fn extension(&self, data_type: impl Into<Txt>) -> Result<Option<IpcBytes>, ClipboardError> {
        self.extension_impl(data_type.into())
    }
    fn extension_impl(&self, data_type: Txt) -> Result<Option<IpcBytes>, ClipboardError> {
        let mut s = CLIPBOARD_SV.write();
        if s.update_exts.len() > 20 {
            s.update_exts.retain(|_, v| v.strong_count() > 0);
        }
        match s.update_exts.get(&data_type).and_then(|r| r.upgrade()) {
            Some(r) => (*r).clone(),
            None => {
                let r = match VIEW_PROCESS.clipboard()?.read_extension(data_type.clone())? {
                    Ok(r) => Ok(Some(r)),
                    Err(e) => match e {
                        ViewError::NotFound => Ok(None),
                        ViewError::NotSupported => Err(ClipboardError::NotSupported),
                        e => Err(ClipboardError::Other(e.to_txt())),
                    },
                };

                let arc = Arc::new(r.clone());
                s.update_exts.insert(data_type, Arc::downgrade(&arc));
                UPDATES.once_update("", || {
                    let _hold = arc;
                });

                r
            }
        }
    }

    /// Set a custom data on the clipboard.
    ///
    /// The current view-process must support `data_type` after the current update.
    pub fn set_extension(&self, data_type: impl Into<Txt>, data: IpcBytes) -> ResponseVar<Result<(), ClipboardError>> {
        self.set_extension_impl(data_type.into(), data)
    }
    fn set_extension_impl(&self, data_type: Txt, data: IpcBytes) -> ResponseVar<Result<(), ClipboardError>> {
        let (r, rsp) = response_var();
        UPDATES.once_update("CLIPBOARD.set_extension", move || match VIEW_PROCESS.clipboard() {
            Ok(c) => match c.write_extension(data_type, data) {
                Ok(vr) => r.respond(vr.map_err(ClipboardError::from)),
                Err(e) => r.respond(Err(e.into())),
            },
            Err(e) => r.respond(Err(e.into())),
        });
        rsp
    }

    /// Get what clipboard types and operations the current view-process implements.
    pub fn available_types(&self) -> ClipboardTypes {
        VIEW_PROCESS.info().clipboard.clone()
    }
}

command! {
    /// Represents the clipboard **cut** action.
    pub static CUT_CMD = {
        l10n!: true,
        name: "Cut",
        info: "Remove the selection and place it in the clipboard",
        shortcut: [shortcut!(CTRL + 'X'), shortcut!(SHIFT + Delete), shortcut!(Cut)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get("cut")),
    };

    /// Represents the clipboard **copy** action.
    pub static COPY_CMD = {
        l10n!: true,
        name: "Copy",
        info: "Place a copy of the selection in the clipboard",
        shortcut: [shortcut!(CTRL + 'C'), shortcut!(CTRL + Insert), shortcut!(Copy)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get("copy")),
    };

    /// Represents the clipboard **paste** action.
    pub static PASTE_CMD = {
        l10n!: true,
        name: "Paste",
        info: "Insert content from the clipboard",
        shortcut: [shortcut!(CTRL + 'V'), shortcut!(SHIFT + Insert), shortcut!(Paste)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get("paste")),
    };
}
