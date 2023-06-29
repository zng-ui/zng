//! Clipboard app extension, service and commands.
//!
//! This module is a thin wrapper around the [`VIEW_PROCESS`] provided clipboard service.

use std::path::PathBuf;

use crate::{
    app::{
        view_process::{IpcBytes, VIEW_PROCESS},
        AppExtension,
    },
    image::{ImageHasher, ImageVar, Img, IMAGES},
    text::Txt,
};

/// Clipboard app extension.
///
/// # Services
///
/// Services provided by this extension.
///
/// * [`CLIPBOARD`]
///
/// # Default
///
/// This extension is included in the [default app].
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct ClipboardManager {}

impl AppExtension for ClipboardManager {}

/// Clipboard service.
///
/// This service is a thin wrapper around the [`VIEW_PROCESS`] provided clipboard service. This means
/// the clipboard will not work in headless app without renderer mode.
pub struct CLIPBOARD;
impl CLIPBOARD {
    /// Gets a text string from the clipboard.
    pub fn text(&self) -> Option<Txt> {
        VIEW_PROCESS.clipboard().ok()?.text().ok()?.map(|t| Txt::from_str(t.as_str()))
    }
    /// Sets the text string on the clipboard, returns `true` if the operation succeeded.
    pub fn set_text(&self, txt: impl Into<Txt>) {
        if let Ok(c) = VIEW_PROCESS.clipboard() {
            let _ = c.set_text(txt.into().into());
        }
    }

    /// Gets an image from the clipboard.
    ///
    /// The image is loaded in parallel and cached by the [`IMAGES`] service.
    pub fn image(&self) -> Option<ImageVar> {
        let img = VIEW_PROCESS.clipboard().ok()?.image().ok()??;
        let id = img.id()?;
        let mut hash = ImageHasher::new();
        hash.update("zero_ui_core::CLIPBOARD");
        hash.update(id.get().to_be_bytes());
        Some(match IMAGES.register(hash.finish(), img) {
            Ok(r) => r,
            Err((_, r)) => r,
        })
    }

    /// Set the image on the clipboard if it is loaded.
    pub fn set_image(&self, img: &Img) {
        if let Some(img) = img.view() {
            if let Ok(c) = VIEW_PROCESS.clipboard() {
                let _ = c.set_image(img);
            }
        }
    }

    /// Gets a file list from the clipboard.
    pub fn file_list(&self) -> Option<Vec<PathBuf>> {
        VIEW_PROCESS.clipboard().ok()?.file_list().ok()?
    }

    /// Sets the file list on the clipboard.
    pub fn set_file_list(&self, list: impl Into<Vec<PathBuf>>) {
        if let Ok(c) = VIEW_PROCESS.clipboard() {
            let _ = c.set_file_list(list.into());
        }
    }

    /// Gets custom data from the clipboard.
    ///
    /// The current view-process must support `data_type`.
    pub fn extension(&self, data_type: impl Into<String>) -> Option<IpcBytes> {
        VIEW_PROCESS.clipboard().ok()?.extension(data_type.into()).ok()?
    }

    /// Set a custom data on the clipboard.
    ///
    /// The current view-process must support `data_type`.
    pub fn set_extension(&self, data_type: impl Into<String>, data: IpcBytes) {
        if let Ok(c) = VIEW_PROCESS.clipboard() {
            let _ = c.set_extension(data_type.into(), data);
        }
    }
}
