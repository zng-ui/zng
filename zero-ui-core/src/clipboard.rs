//! Clipboard app extension, service and commands.
//!
//! This module is a thin wrapper around the [`VIEW_PROCESS`] provided clipboard service.

use crate::{
    app::{view_process::VIEW_PROCESS, AppExtension},
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
    /// Gets the text string set on the clipboard.
    pub fn text(&self) -> Option<Txt> {
        VIEW_PROCESS.clipboard().ok()?.text().ok()?.map(|t| Txt::from_str(t.as_str()))
    }
    /// Sets the text string on the clipboard, returns `true` if the operation succeeded.
    pub fn set_text(&self, txt: impl AsRef<str>) {
        if let Ok(c) = VIEW_PROCESS.clipboard() {
            let _ = c.set_text(txt.as_ref().to_owned());
        }
    }

    /// Gets the clipboard image.
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

    /// Set the image on the clipboard.
    pub fn set_image(&self, img: &Img) {
        // !!: wait load?
        if let Some(img) = img.view() {
            if let Ok(c) = VIEW_PROCESS.clipboard() {
                let _ = c.set_image(img);
            }
        }
    }

    // !!: TODO, other `ClipboardType` methods.
}
