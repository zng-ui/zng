//! Audio device types.

use std::num::NonZeroU16;

use zng_txt::Txt;
use serde::{Serialize, Deserialize};

crate::declare_id! {
    /// Audio device ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is mapped to an unique id, but does not survived View crashes.
    ///
    /// The View Process defines the ID.
    pub struct AudioDeviceId(_);
}

/// Represents an input or output device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioDeviceInfo {
    /// Device display name.
    pub name: Txt,
    /// Device input/output capabilities.
    pub kind: AudioDeviceKind,
    /// Input stream modes this device can produce.
    pub input_modes: Vec<AudioStreamMode>,
    /// Output stream modes this device can consume.
    pub output_modes: Vec<AudioStreamMode>,
}

/// Represents audio device input/output capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioDeviceKind {
    /// Device only supports audio input.
    Input = 0b01,
    /// Device only supports audio output.
    Output = 0b10,
    /// Device supports both audio input and output.
    Both = 0b11,
}
impl AudioDeviceKind {
    /// Device supports audio input.
    pub fn supports_input(self) -> bool {
        matches!(self, Self::Input | Self::Both)
    }

    /// Device supports audio input.
    pub fn supports_output(self) -> bool {
        matches!(self, Self::Output | Self::Both)
    }
}

/// Represents steam capability of an audio device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioStreamMode {
    /// Number of audio channels.
    pub channels: NonZeroU16,
    /// Minimum and maximum sample rate.
    pub sample_rate: SampleRate,
    /// Minimum and maximum supported buffer size. 
    pub buffer_size: BufferSize,
}

/// Represents the minimum and maximum sample rate per audio channel.
/// 
/// Values are in samples processed per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SampleRate {
    /// Minimum, inclusive.
    pub min: u32,
    /// Maximum, inclusive.
    pub max: u32,
}

/// Represents the minimum and maximum supported buffer size for the device. 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BufferSize {
    /// Range in frames per second.
    Range {
        /// Minimum, inclusive.
        min: u32,
        /// Maximum, inclusive.
        max: u32,
    },
    /// Platform cannot describe buffer size for this device.
    Unknown
}