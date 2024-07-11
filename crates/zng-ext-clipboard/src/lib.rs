#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Clipboard app extension, service and commands.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use core::fmt;
use std::path::PathBuf;

use zng_app::{
    event::{command, CommandInfoExt as _, CommandNameExt as _},
    shortcut::{shortcut, CommandShortcutExt as _, ShortcutFilter},
    view_process::{ViewClipboard, VIEW_PROCESS},
    AppExtension,
};
use zng_app_context::app_local;
use zng_ext_image::{ImageHasher, ImageVar, Img, IMAGES};
use zng_txt::Txt;
use zng_var::{response_var, ResponderVar, ResponseVar};
use zng_view_api::ViewProcessOffline;
use zng_wgt::{wgt_fn, CommandIconExt as _, ICONS};

use zng_view_api::clipboard as clipboard_api;
use zng_view_api::ipc::IpcBytes;

/// Clipboard app extension.
///
/// # Services
///
/// Services provided by this extension.
///
/// * [`CLIPBOARD`]
#[derive(Default)]
pub struct ClipboardManager {}

impl AppExtension for ClipboardManager {
    fn update(&mut self) {
        let mut clipboard = CLIPBOARD_SV.write();
        clipboard.text.update(|v, txt| v.write_text(txt));
        clipboard.image.map_update(
            |img| {
                if let Some(img) = img.view() {
                    Ok(img.clone())
                } else {
                    Err(ClipboardError::ImageNotLoaded)
                }
            },
            |v, img| v.write_image(&img),
        );
        clipboard.file_list.update(|v, list| v.write_file_list(list));
        clipboard.ext.update(|v, (data_type, data)| v.write_extension(data_type, data))
    }
}

app_local! {
    static CLIPBOARD_SV: ClipboardService = ClipboardService::default();
}

#[derive(Default)]
struct ClipboardService {
    text: ClipboardData<Txt, Txt>,
    image: ClipboardData<ImageVar, Img>,
    file_list: ClipboardData<Vec<PathBuf>, Vec<PathBuf>>,
    ext: ClipboardData<IpcBytes, (Txt, IpcBytes)>,
}
struct ClipboardData<O: 'static, I: 'static> {
    latest: Option<Result<Option<O>, ClipboardError>>,
    request: Option<(I, ResponderVar<Result<bool, ClipboardError>>)>,
}
impl<O: 'static, I: 'static> Default for ClipboardData<O, I> {
    fn default() -> Self {
        Self {
            latest: None,
            request: None,
        }
    }
}
impl<O: Clone + 'static, I: 'static> ClipboardData<O, I> {
    pub fn get(
        &mut self,
        getter: impl FnOnce(&ViewClipboard) -> Result<Result<O, clipboard_api::ClipboardError>, ViewProcessOffline>,
    ) -> Result<Option<O>, ClipboardError> {
        self.latest
            .get_or_insert_with(|| {
                let r = CLIPBOARD.view().and_then(|v| match getter(v) {
                    Ok(r) => match r {
                        Ok(r) => Ok(Some(r)),
                        Err(e) => match e {
                            clipboard_api::ClipboardError::NotFound => Ok(None),
                            clipboard_api::ClipboardError::NotSupported => Err(ClipboardError::NotSupported),
                            clipboard_api::ClipboardError::Other(e) => Err(ClipboardError::Other(e)),
                        },
                    },
                    Err(ViewProcessOffline) => Err(ClipboardError::ViewProcessOffline),
                });
                if let Err(e) = &r {
                    tracing::error!("clipboard get error, {e:?}");
                }
                r
            })
            .clone()
    }

    pub fn request(&mut self, r: I) -> ResponseVar<Result<bool, ClipboardError>> {
        let (responder, response) = response_var();

        if let Some((_, r)) = self.request.replace((r, responder)) {
            r.respond(Ok(false));
        }

        response
    }

    pub fn update(
        &mut self,
        setter: impl FnOnce(&ViewClipboard, I) -> Result<Result<(), clipboard_api::ClipboardError>, ViewProcessOffline>,
    ) {
        self.map_update(Ok, setter)
    }

    pub fn map_update<VI>(
        &mut self,
        to_view: impl FnOnce(I) -> Result<VI, ClipboardError>,
        setter: impl FnOnce(&ViewClipboard, VI) -> Result<Result<(), clipboard_api::ClipboardError>, ViewProcessOffline>,
    ) {
        self.latest = None;
        if let Some((i, rsp)) = self.request.take() {
            let vi = match to_view(i) {
                Ok(vi) => vi,
                Err(e) => {
                    tracing::error!("clipboard set error, {e:?}");
                    rsp.respond(Err(e));
                    return;
                }
            };
            let r = CLIPBOARD.view().and_then(|v| match setter(v, vi) {
                Ok(r) => match r {
                    Ok(()) => Ok(true),
                    Err(e) => match e {
                        clipboard_api::ClipboardError::NotFound => {
                            Err(ClipboardError::Other(Txt::from_static("not found error in set operation")))
                        }
                        clipboard_api::ClipboardError::NotSupported => Err(ClipboardError::NotSupported),
                        clipboard_api::ClipboardError::Other(e) => Err(ClipboardError::Other(e)),
                    },
                },
                Err(ViewProcessOffline) => Err(ClipboardError::ViewProcessOffline),
            });
            if let Err(e) = &r {
                tracing::error!("clipboard set error, {e:?}");
            }
            rsp.respond(r);
        }
    }
}

/// Error getting or setting the clipboard.
///
/// The [`CLIPBOARD`] service already logs the error.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardError {
    /// No view-process available to process the request.
    ViewProcessOffline,
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
            ClipboardError::ViewProcessOffline => write!(f, "no view-process available to process the request"),
            ClipboardError::NotSupported => write!(f, "view-process or operating system does not support the data type"),
            ClipboardError::ImageNotLoaded => write!(
                f,
                "cannot set image in clipboard because it has not finished loading or loaded with error"
            ),
            ClipboardError::Other(e) => write!(f, "{e}"),
        }
    }
}

/// Clipboard service.
///
/// This service synchronizes with the UI update cycle, the getter methods provide the same data for all requests in the
/// same update pass, even if the system clipboard happens to change mid update, the setter methods only set the system clipboard
/// at the end of the update pass.
///
/// This service needs a running view-process to actually interact with the system clipboard, in a headless app
/// without renderer (no view-process) the service will always return [`ClipboardError::ViewProcessOffline`].
pub struct CLIPBOARD;
impl CLIPBOARD {
    fn view(&self) -> Result<&ViewClipboard, ClipboardError> {
        match VIEW_PROCESS.clipboard() {
            Ok(c) => Ok(c),
            Err(ViewProcessOffline) => Err(ClipboardError::ViewProcessOffline),
        }
    }

    /// Gets a text string from the clipboard.
    pub fn text(&self) -> Result<Option<Txt>, ClipboardError> {
        CLIPBOARD_SV
            .write()
            .text
            .get(|v| v.read_text())
            .map(|s| s.map(|s| Txt::from_str(&s)))
    }
    /// Sets the text string on the clipboard after the current update.
    ///
    /// Returns a response var that updates to `Ok(true)` is the text is put on the clipboard,
    /// `Ok(false)` if another request made on the same update pass replaces this one or `Err(ClipboardError)`.
    pub fn set_text(&self, txt: impl Into<Txt>) -> ResponseVar<Result<bool, ClipboardError>> {
        CLIPBOARD_SV.write().text.request(txt.into())
    }

    /// Gets an image from the clipboard.
    ///
    /// The image is loaded in parallel and cached by the [`IMAGES`] service.
    ///
    /// [`IMAGES`]: zng_ext_image::IMAGES
    pub fn image(&self) -> Result<Option<ImageVar>, ClipboardError> {
        CLIPBOARD_SV.write().image.get(|v| {
            let img = v.read_image()?;
            match img {
                Ok(img) => {
                    let mut hash = ImageHasher::new();
                    hash.update("zng_ext_clipboard::CLIPBOARD");
                    hash.update(img.id().unwrap().get().to_be_bytes());
                    match IMAGES.register(hash.finish(), img) {
                        Ok(r) => Ok(Ok(r)),
                        Err((_, r)) => Ok(Ok(r)),
                    }
                }
                Err(e) => Ok(Err(e)),
            }
        })
    }

    /// Set the image on the clipboard after the current update, if it is loaded.
    ///
    /// Returns a response var that updates to `Ok(true)` is the text is put on the clipboard,
    /// `Ok(false)` if another request made on the same update pass replaces this one or `Err(ClipboardError)`.
    pub fn set_image(&self, img: Img) -> ResponseVar<Result<bool, ClipboardError>> {
        CLIPBOARD_SV.write().image.request(img)
    }

    /// Gets a file list from the clipboard.
    pub fn file_list(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        CLIPBOARD_SV.write().file_list.get(|v| v.read_file_list())
    }

    /// Sets the file list on the clipboard after the current update.
    ///
    /// Returns a response var that updates to `Ok(true)` is the text is put on the clipboard,
    /// `Ok(false)` if another request made on the same update pass replaces this one or `Err(ClipboardError)`.
    pub fn set_file_list(&self, list: impl Into<Vec<PathBuf>>) -> ResponseVar<Result<bool, ClipboardError>> {
        CLIPBOARD_SV.write().file_list.request(list.into())
    }

    /// Gets custom data from the clipboard.
    ///
    /// The current view-process must support `data_type`.
    pub fn extension(&self, data_type: impl Into<Txt>) -> Result<Option<IpcBytes>, ClipboardError> {
        CLIPBOARD_SV.write().ext.get(|v| v.read_extension(data_type.into()))
    }

    /// Set a custom data on the clipboard.
    ///
    /// The current view-process must support `data_type` after the current update.
    ///
    /// Returns a response var that updates to `Ok(true)` is the text is put on the clipboard,
    /// `Ok(false)` if another request made on the same update pass replaces this one or `Err(ClipboardError)`.
    pub fn set_extension(&self, data_type: impl Into<Txt>, data: IpcBytes) -> ResponseVar<Result<bool, ClipboardError>> {
        CLIPBOARD_SV.write().ext.request((data_type.into(), data))
    }
}

command! {
    /// Represents the clipboard **cut** action.
    pub static CUT_CMD = {
        l10n!: true,
        name: "Cut",
        info: "Remove the selection and place it in the clipboard.",
        shortcut: [shortcut!(CTRL+'X'), shortcut!(SHIFT+Delete), shortcut!(Cut)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get("cut")),
    };

    /// Represents the clipboard **copy** action.
    pub static COPY_CMD = {
        l10n!: true,
        name: "Copy",
        info: "Place a copy of the selection in the clipboard.",
        shortcut: [shortcut!(CTRL+'C'), shortcut!(CTRL+Insert), shortcut!(Copy)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get("copy")),
    };

    /// Represents the clipboard **paste** action.
    pub static PASTE_CMD = {
        l10n!: true,
        name: "Paste",
        info: "Insert content from the clipboard.",
        shortcut: [shortcut!(CTRL+'V'), shortcut!(SHIFT+Insert), shortcut!(Paste)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
        icon: wgt_fn!(|_| ICONS.get("paste")),
    };
}
