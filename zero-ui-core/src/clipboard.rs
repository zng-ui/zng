//! Clipboard app extension, service and commands.
//!
//! This module is a thin wrapper around the [`arboard`] crate, the crate is also re-exported here for convenience and compatibility.

use std::{borrow::Cow, sync::Arc};

use crate::{
    app::AppExtension,
    app_local,
    image::{ImageDataFormat, ImageHash, ImageSource, ImageVar, Img, IMAGES},
    text::Txt,
    units::*,
};

pub use arboard;
use parking_lot::MappedRwLockWriteGuard;

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
pub struct CLIPBOARD;
impl CLIPBOARD {
    /// Gets the text string set on the clipboard.
    pub fn text(&self) -> Result<Txt, arboard::Error> {
        self.arboard()?.get_text().map(Txt::from)
    }
    /// Sets the text string on the clipboard, returns `true` if the operation succeeded.
    pub fn set_text(&self, txt: impl AsRef<str>) -> Result<(), arboard::Error> {
        self.arboard()?.set_text(txt.as_ref())
    }

    /// Gets the HTML text set on the clipboard.
    pub fn html(&self) -> Result<Txt, arboard::Error> {
        let s = self.arboard()?.get_text()?;

        // arboard does not have `get_html`
        if s.starts_with("<html>") {
            return Ok(Txt::from(s));
        }

        Err(arboard::Error::ContentNotAvailable)
    }
    /// Sets the HTML text on the clipboard.
    pub fn set_html(&self, html: impl AsRef<str>, alt_text: impl AsRef<str>) -> Result<(), arboard::Error> {
        self.set_html_impl(html.as_ref(), alt_text.as_ref())
    }
    fn set_html_impl(&self, html: &str, alt_text: &str) -> Result<(), arboard::Error> {
        self.arboard()?
            .set_html(html, if alt_text.is_empty() { None } else { Some(alt_text) })
    }

    /// Gets the clipboard image.
    ///
    /// The image is loaded in parallel and cached by the [`IMAGES`] service.
    pub fn image(&self) -> Result<ImageVar, arboard::Error> {
        let img = self.arboard()?.get_image()?;
        Ok(IMAGES.cache(img))
    }

    /// Set the image on the clipboard.
    pub fn set_image(&self, img: &Img) -> Result<(), arboard::Error> {
        if let Some(data) = img.bgra8() {
            let mut board = self.arboard()?;
            let size = img.size();

            // bgra -> rgba
            let mut data = data.to_vec();
            for pixel in data.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }

            board.set_image(arboard::ImageData {
                width: size.width.0 as _,
                height: size.height.0 as _,
                bytes: Cow::Owned(data),
            })
        } else {
            Err(arboard::Error::ContentNotAvailable)
        }
    }

    /// Access the clipboard instance for the current app.
    ///
    /// If the clipboard failed to create, retries on every request.
    pub fn arboard(&self) -> Result<MappedRwLockWriteGuard<arboard::Clipboard>, arboard::Error> {
        let mut board = CLIPBOARD_SV.write();
        if board.is_none() {
            match arboard::Clipboard::new() {
                Ok(b) => *board = Some(b),
                Err(e) => return Err(e),
            }
        }

        Ok(MappedRwLockWriteGuard::map(board, |r| r.as_mut().unwrap()))
    }
}

app_local! {
    static CLIPBOARD_SV: Option<arboard::Clipboard> = const { None };
}

impl<'a> From<arboard::ImageData<'a>> for ImageSource {
    fn from(img: arboard::ImageData<'a>) -> Self {
        let mut data = img.bytes.to_vec();
        // rgba -> bgra
        for pixel in data.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        let hash = ImageHash::compute(&data);

        ImageSource::Data(
            hash,
            Arc::new(data),
            ImageDataFormat::Bgra8 {
                size: PxSize::new(Px(img.width as _), Px(img.height as _)),
                ppi: None,
            },
        )
    }
}
