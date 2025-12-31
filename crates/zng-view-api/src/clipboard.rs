//! Clipboard types.

use std::{fmt, path::PathBuf};

use zng_task::channel::IpcBytes;
use zng_txt::Txt;

use crate::image::ImageId;

/// Clipboard data.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ClipboardData {
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
    /// Any data format supported only by the specific view-process implementation.
    ///
    /// The view-process implementation may also pass this to the operating system as binary data.
    Extension {
        /// Type key, must be in a format defined by the view-process.
        data_type: Txt,
        /// The raw data.
        data: IpcBytes,
    },
}

/// Clipboard data type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ClipboardType {
    /// A [`ClipboardData::Text`].
    Text,
    /// A [`ClipboardData::Image`].
    Image,
    /// A [`ClipboardData::Paths`].
    Paths,
    /// A [`ClipboardData::Extension`].
    Extension(Txt),
}

/// Clipboard read/write error.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ClipboardError {
    /// Requested format is not set on the clipboard.
    NotFound,
    /// View-process or operating system does not support the data type.
    NotSupported,
    /// Other error.
    ///
    /// The string can be a debug description of the error, only suitable for logging.
    Other(Txt),
}
impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::NotFound => write!(f, "clipboard does not contain the requested format"),
            ClipboardError::NotSupported => write!(f, "clipboard implementation does not support the format"),
            ClipboardError::Other(_) => write!(f, "internal error"),
        }
    }
}
impl std::error::Error for ClipboardError {}

/// Clipboard types and operations implemented by the view-process.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct ClipboardTypes {
    /// Data formats that the implementation can read.
    pub read: Vec<ClipboardType>,

    /// Data formats that the implementation can write.
    pub write: Vec<ClipboardType>,
    /// Implementation can put multiple data on the clipboard at the same time.
    pub write_multi: bool,
}
impl ClipboardTypes {
    /// New.
    pub fn new(read: Vec<ClipboardType>, write: Vec<ClipboardType>, write_multi: bool) -> Self {
        Self { read, write, write_multi }
    }
}
