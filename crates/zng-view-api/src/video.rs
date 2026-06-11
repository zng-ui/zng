//! Video types.
//!
//! # Under Development
//!
//! This API is not ready yet, the types here are only the basics to
//! avoid a breaking change release when video is implemented.

use serde::{Deserialize, Serialize};
use zng_txt::Txt;

crate::declare_id! {
    /// Id of a decoded or streaming video in the cache.
    ///
    /// The View Process defines the ID.
    pub struct VideoId(_);

    /// Id of a video playing in a renderer.
    ///
    /// The View Process defines the ID.
    pub struct VideoTextureId(_);

    /// Id of a video encode task.
    ///
    /// The View Process defines the ID.
    pub struct VideoEncodeId(_);
}

/// Represents a video load/decode request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VideoRequest<D> {
    /// Video data format.
    pub format: VideoDataFormat,

    /// Video data.
    ///
    /// Bytes layout depends on the `format`, data structure is [`IpcReadHandle`] or [`IpcReceiver<IpcBytes>`] in the view API.
    ///
    /// [`IpcReadHandle`]: zng_task::channel::IpcReadHandle
    /// [`IpcReceiver<IpcBytes>`]: zng_task::channel::IpcReceiver
    pub data: D,
}

/// Format of the video bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VideoDataFormat {
    /// The video is encoded.
    ///
    /// This file extension maybe identifies the format. Fallback to `Unknown` handling if the file extension
    /// is unknown or the file header does not match.
    FileExtension(Txt),

    /// The video is encoded.
    ///
    /// This MIME type maybe identifies the format. Fallback to `Unknown` handling if the file extension
    /// is unknown or the file header does not match.
    MimeType(Txt),

    /// The image is encoded.
    ///
    /// A decoder will be selected using the "magic number" at the start of the bytes buffer.
    Unknown,
}
