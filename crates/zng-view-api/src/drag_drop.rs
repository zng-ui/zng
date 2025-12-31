//! Drag&drop types.

use std::{fmt, path::PathBuf};

use zng_task::channel::IpcBytes;
use zng_txt::Txt;

use bitflags::bitflags;

use crate::image::ImageId;

/// Drag&drop data payload.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum DragDropData {
    /// Text string.
    ///
    /// View-process can convert between [`String`] and the text formats of the platform.
    Text(Txt),
    /// Image data.
    ///
    /// View-process reads from clipboard in any format supported and starts an image decode task
    /// for the data, the [`ImageId`] may still be decoding when received. For writing the
    /// view-process will expect the image to already be loaded, the image will be encoded in
    /// a format compatible with the platform clipboard.
    Image(ImageId),
    /// List of paths.
    Paths(Vec<PathBuf>),
    /// Any data format specific for the view-process implementation.
    ///
    /// The view-process implementation may also pass this to the operating system as binary data.
    Extension {
        /// Type key, must be in a format defined by the view-process.
        data_type: Txt,
        /// The raw data.
        data: IpcBytes,
    },
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
