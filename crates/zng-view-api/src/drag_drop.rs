//! Drag&drop types.

use std::{fmt, path::PathBuf};

use zng_txt::Txt;

use crate::ipc::IpcBytes;

use bitflags::bitflags;

/// Drag&drop data payload.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DragDropData {
    /// Text encoded data.
    ///
    /// This can be HTML or JSON for example.
    Text {
        /// MIME type of the data.
        ///
        /// Plain text is `"text/plain"`.
        format: Txt,
        /// Data.
        data: Txt,
    },
    /// File or directory path.
    Path(PathBuf),
    /// Binary encoded data.
    ///
    /// This can be an image for example.
    Binary {
        /// MIME type of the data.
        format: Txt,
        /// Data.
        data: IpcBytes,
    },
}
impl fmt::Debug for DragDropData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text { format, data } => write!(f, "Text {{ format: {:?}, data: {} bytes }}", format, data.len()),
            Self::Path(data) => write!(f, "Path({})", data.display()),
            Self::Binary { format, data } => write!(f, "Binary {{ format: {:?}, data: {} bytes }}", format, data.len()),
        }
    }
}

#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    fn from(plain: Txt) -> DragDropData {
        DragDropData::Text {
            format: "text/plain".into(),
            data: plain,
        }
    }

    fn from(plain: String) -> DragDropData {
        Txt::from(plain).into()
    }

    fn from(plain: &'static str) -> DragDropData {
        Txt::from(plain).into()
    }

    fn from(path: PathBuf) -> DragDropData {
        DragDropData::Path(path)
    }
}

bitflags! {
    /// Drag&drop drop effect on the data source.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct DragDropEffect: u8 {
        /// Indicates that the dragged data will be copied from its present location to the drop location.
        const COPY = 0b001;
        /// Indicates that the dragged data will be moved from its present location to the drop location.
        const MOVE = 0b010;
        /// Indicates that some form of relationship or connection will be created between the source and drop locations.
        const LINK = 0b100;
    }
}
impl DragDropEffect {
    /// Count effects flagged.
    pub fn len(&self) -> u8 {
        [DragDropEffect::COPY, DragDropEffect::MOVE, DragDropEffect::LINK]
            .into_iter()
            .filter(|&f| self.contains(f))
            .count() as u8
    }
}

/// Error for drag start or cancel error.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum DragDropError {
    /// View-process implementer does not support any of the provided data types.
    NotSupported,
    /// Cannot start dragging.
    CannotStart(Txt),
}
impl fmt::Display for DragDropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DragDropError::NotSupported => write!(f, "not supported"),
            DragDropError::CannotStart(txt) => write!(f, "cannot start, {txt}"),
        }
    }
}
impl std::error::Error for DragDropError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
