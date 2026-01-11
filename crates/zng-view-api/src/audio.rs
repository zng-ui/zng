//! Audio device types.

use std::{num::NonZeroU16, time::Duration};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use zng_task::channel::IpcBytesCast;
use zng_txt::Txt;
use zng_unit::Factor;

crate::declare_id! {
    /// Audio device ID.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is mapped to an unique id, but does not survived View crashes.
    ///
    /// The View Process defines the ID.
    pub struct AudioDeviceId(_);

    /// Id of a decoded or on demand decoding audio track in the cache.
    ///
    /// The View Process defines the ID.
    pub struct AudioId(_);

    /// Audio playback stream ID.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is an unique id that survives View crashes.
    ///
    /// The App Process defines the ID.
    pub struct AudioOutputId(_);

    /// Audio playback request ID.
    ///
    /// The View Process defines the ID.
    pub struct AudioPlayId(_);

    /// Id of an audio encode task.
    ///
    /// The View Process defines the ID.
    pub struct AudioEncodeId(_);
}

/// Info about an input or output device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioDeviceInfo {
    /// Device display name.
    pub name: Txt,
    /// Device input/output capabilities.
    pub capabilities: AudioDeviceCapability,
    /// Input stream modes this device can produce.
    pub input_modes: Vec<AudioStreamMode>,
    /// Output stream modes this device can consume.
    pub output_modes: Vec<AudioStreamMode>,
}

bitflags! {
    /// Represents audio device input/output capabilities.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct AudioDeviceCapability: u8 {
        /// Device can generate audio streams.
        const INPUT = 0b01;
        /// Device can consume audio streams.
        const OUTPUT = 0b11;
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
    Unknown,
}

/// Represent an audio load/decode request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioRequest<D> {
    /// Audio data format.
    pub format: AudioDataFormat,

    /// Audio data.
    pub data: D,

    /// Maximum allowed decoded size in bytes.
    ///
    /// View-process will avoid decoding and return an error if the track would exceed this limit.
    pub max_decoded_len: u64,

    /// Defines what tracks are decoded from multi image containers.
    pub tracks: AudioTracksMode,

    /// Audio is a track (or subtree) of this other audio.
    ///
    /// This value is now used by the view-process, it is just returned with the metadata. This is useful when
    /// an already decoded image is requested after a respawn to maintain the original container structure.
    pub parent: Option<AudioTrackMetadata>,
}
impl<D> AudioRequest<D> {
    /// New.
    pub fn new(format: AudioDataFormat, data: D, max_decoded_len: u64) -> Self {
        Self {
            format,
            data,
            max_decoded_len,
            tracks: AudioTracksMode::PRIMARY,
            parent: None,
        }
    }
}

/// Format of the audio bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AudioDataFormat {
    /// Data is already decoded stream of interleaved `f32` samples.
    InterleavedF32 {
        /// Number of channels interleaved in the track.
        channel_count: u16,
        /// Samples per second.
        ///
        /// A sample is a single sequence of `channel_count`.
        sample_rate: u32,
        /// Total duration of the track, if it is known.
        total_duration: Option<Duration>,
    },

    /// The audio is encoded.
    ///
    /// This file extension maybe identifies the format. Fallback to `Unknown` handling if the file extension
    /// is unknown or the file header does not match.
    FileExtension(Txt),

    /// The audio is encoded.
    ///
    /// This MIME type maybe identifies the format. Fallback to `Unknown` handling if the file extension
    /// is unknown or the file header does not match.
    MimeType(Txt),

    /// The image is encoded.
    ///
    /// A decoder will be selected using the "magic number" at the start of the bytes buffer.
    Unknown,
}
impl From<Txt> for AudioDataFormat {
    fn from(ext_or_mime: Txt) -> Self {
        if ext_or_mime.contains('/') {
            AudioDataFormat::MimeType(ext_or_mime)
        } else {
            AudioDataFormat::FileExtension(ext_or_mime)
        }
    }
}

/// Represents an audio codec capability.
///
/// This type will be used in the next breaking release of the view API.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioFormat {
    /// Display name of the format.
    pub display_name: Txt,

    /// Media types (MIME) associated with the format.
    ///
    /// Lowercase, without `"audio/"` prefix, comma separated if there is more than one.
    pub media_type_suffixes: Txt,

    /// Common file extensions associated with the format.
    ///
    /// Lowercase, without dot, comma separated if there is more than one.
    pub file_extensions: Txt,

    /// Capabilities of this format.
    pub capabilities: AudioFormatCapability,
}
impl AudioFormat {
    /// From static str.
    ///
    /// # Panics
    ///
    /// Panics if `media_type_suffixes` not ASCII.
    pub const fn from_static(
        display_name: &'static str,
        media_type_suffixes: &'static str,
        file_extensions: &'static str,
        capabilities: AudioFormatCapability,
    ) -> Self {
        assert!(media_type_suffixes.is_ascii());
        Self {
            display_name: Txt::from_static(display_name),
            media_type_suffixes: Txt::from_static(media_type_suffixes),
            file_extensions: Txt::from_static(file_extensions),
            capabilities,
        }
    }

    /// Iterate over media type suffixes.
    pub fn media_type_suffixes_iter(&self) -> impl Iterator<Item = &str> {
        self.media_type_suffixes.split(',').map(|e| e.trim())
    }

    /// Iterate over full media types, with `"image/"` prefix.
    pub fn media_types(&self) -> impl Iterator<Item = Txt> {
        self.media_type_suffixes_iter().map(Txt::from_str)
    }

    /// Iterate over extensions.
    pub fn file_extensions_iter(&self) -> impl Iterator<Item = &str> {
        self.file_extensions.split(',').map(|e| e.trim())
    }

    /// Checks if `f` matches any of the mime types or any of the file extensions.
    ///
    /// File extensions comparison ignores dot and ASCII case.
    pub fn matches(&self, f: &str) -> bool {
        let f = f.strip_prefix('.').unwrap_or(f);
        let f = f.strip_prefix("image/").unwrap_or(f);
        self.media_type_suffixes_iter().any(|e| e.eq_ignore_ascii_case(f)) || self.file_extensions_iter().any(|e| e.eq_ignore_ascii_case(f))
    }
}

bitflags! {
    /// Capabilities of an [`AudioFormat`] implementation.
    ///
    /// Note that `DECODE` capability is omitted because the view-process can always decode formats.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct AudioFormatCapability: u8 {
        /// View-process can encode audio in this format.
        const ENCODE = 0b_0000_0001;
    }
}

/// Represent a image encode request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioEncodeRequest {
    /// Audio to encode.
    pub id: AudioId,

    /// Format query, view-process uses [`AudioFormat::matches`] to find the format.
    pub format: Txt,

    /// The audio to encode.
    pub mix: AudioMix,
}
impl AudioEncodeRequest {
    /// New.
    pub fn new(id: AudioId, format: Txt, mix: AudioMix) -> Self {
        Self { id, format, mix }
    }
}

/// Represents decoded header metadata about an audio track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioMetadata {
    /// Audio ID.
    pub id: AudioId,

    /// Number of channels interleaved in the track.
    pub channel_count: u16,
    /// Samples per second.
    ///
    /// A sample is a single sequence of `channel_count`.
    pub sample_rate: u32,
    /// Total duration of the tack, if it is known.
    pub total_duration: Option<Duration>,

    /// Track is an entry (or subtree) of this other track.
    pub parent: Option<AudioTrackMetadata>,
}
impl AudioMetadata {
    /// New.
    pub fn new(id: AudioId, channel_count: u16, sample_rate: u32) -> Self {
        Self {
            id,
            channel_count,
            sample_rate,
            total_duration: None,
            parent: None,
        }
    }
}

/// Represents decoded header metadata about a track position in the container represented by another audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioTrackMetadata {
    /// Image this one belongs too.
    ///
    /// The view-process always sends the parent image metadata first, so this id should be known by the app-process.
    pub parent: AudioId,
    /// Sort index of the track in the list of tracks.
    pub index: usize,
}
impl AudioTrackMetadata {
    /// New.
    pub fn new(parent: AudioId, index: usize) -> Self {
        Self { parent, index }
    }
}

/// Represents a partial or fully decoded audio.
///
/// See [`Event::AudioDecoded`] for more details.
///
/// [`Event::AudioDecoded`]: crate::Event::AudioDecoded
/// [`AudioPartiallyDecoded`]: crate::Event::AudioPartiallyDecoded
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioDecoded {
    /// The audio track ID.
    ///
    /// An [`AudioMetadata`] for this ID was already notified before this event.
    pub id: AudioId,

    /// Offset of the `chunk` on the track.
    ///
    /// This is a count in samples before the first in this chunk, a sample is a sequence of [`channel_count`].
    ///
    /// To convert offset to bytes `offset * channel_count * size_of::<f32>()`.
    ///
    /// [`channel_count`]: TrackMetadata::channel_count
    pub offset: usize,

    /// Interleaved `f32` samples.
    pub chunk: IpcBytesCast<f32>,

    /// If the `chunk` is actually the full decoded audio.
    ///
    /// When this is `true` no more decode events for the `id` are send, (re)playing the audio
    /// will read directly from the cache.
    ///
    /// When this is `false` the `chunk` represent the last decoded chunk on demand because the audio is playing.
    /// Depending on the request the audio may never be fully cached, always decoding again on replay.
    pub is_full: bool,
}
impl AudioDecoded {
    /// New.
    pub fn new(id: AudioId, chunk: IpcBytesCast<f32>) -> Self {
        Self {
            id,
            offset: 0,
            chunk,
            is_full: false,
        }
    }
}

/// Represents a connection request to an audio output device.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioOutputRequest {
    /// ID that will identify the new output.
    pub id: AudioOutputId,

    /// Initial config.
    pub config: AudioOutputConfig,
}
impl AudioOutputRequest {
    /// New.
    pub fn new(id: AudioOutputId, config: AudioOutputConfig) -> Self {
        Self { id, config }
    }
}

/// Represents an audio playback update request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioOutputUpdateRequest {
    /// The output stream.
    pub id: AudioOutputId,
    /// New config.
    pub config: AudioOutputConfig,
}
impl AudioOutputUpdateRequest {
    /// New.
    pub fn new(id: AudioOutputId, config: AudioOutputConfig) -> Self {
        Self { id, config }
    }
}

/// Represents an audio output stream capabilities.
///
/// Any audio played on this output is automatically converted to the channel count and sample rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioOutputOpenData {
    /// Number of channels interleaved supported by this output.
    pub channel_count: u16,
    /// Samples per second.
    ///
    /// A sample is a single sequence of `channel_count`.
    pub sample_rate: u32,
}
impl AudioOutputOpenData {
    /// New.
    pub fn new(channel_count: u16, sample_rate: u32) -> Self {
        Self {
            channel_count,
            sample_rate,
        }
    }
}

/// Audio playback config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioOutputConfig {
    /// Playback state.
    pub state: AudioOutputState,

    /// Volume of the sound.
    ///
    /// The value multiplies the samples, `1.fct()` is the *natural* volume from the source.
    pub volume: Factor,

    /// Speed of the sound.
    ///
    /// This is a multiplier of the playback speed and pitch.
    ///
    /// * `0.5.fct()` doubles the total duration and halves (lowers) the pitch.
    /// * `2.fct()` halves the total duration and doubles (raises) the pitch.
    pub speed: Factor,
}
impl AudioOutputConfig {
    /// New.
    pub fn new(state: AudioOutputState, volume: Factor, speed: Factor) -> Self {
        Self { state, volume, speed }
    }
}

/// Represents the playback state if an audio output stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AudioOutputState {
    /// Audio is sent to the device for playback as in the sequence they are cued.
    Playing,
    /// Audio playback is paused, cue requests are buffered.
    Paused,
    /// Audio playback is paused, all current cue requests are dropped.
    Stopped,
}
impl AudioOutputState {
    /// If is [`Playing`].
    ///
    /// [`Playing`]: Self::Playing
    pub fn is_playing(&self) -> bool {
        matches!(self, Self::Playing)
    }

    /// If is [`Paused`].
    ///
    /// [`Paused`]: Self::Paused
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    /// If is [`Stopped`].
    ///
    /// [`Stopped`]: Self::Stopped
    pub fn is_stopped(&self) -> bool {
        matches!(self, Self::Stopped)
    }
}

/// Represents an audio playback request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioPlayRequest {
    /// The audio output stream.
    ///
    /// If another audio is already playing this request is appended to the end.
    pub output: AudioOutputId,

    /// The audio.
    pub mix: AudioMix,
}
impl AudioPlayRequest {
    /// New.
    pub fn new(output: AudioOutputId, mix: AudioMix) -> Self {
        Self { output, mix }
    }
}

/// Represents an audio source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioMix {
    /// Silent start padding.
    pub delay: Duration,
    /// Total duration.
    ///
    /// If not set audio plays until the last layer. If set audio plays for the duration, if layers end before the duration
    /// plays silent, if layers exceed the duration the end is clipped.
    pub total_duration: Option<Duration>,

    /// Components of this mix.
    ///
    /// Each layer applies to the previous.
    pub layers: Vec<AudioMixLayer>,
}
impl AudioMix {
    /// New empty.
    pub fn new() -> Self {
        Self {
            delay: Duration::ZERO,
            total_duration: None,
            layers: vec![],
        }
    }
}
impl Default for AudioMix {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents an audio source component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AudioMixLayer {
    /// Play the cached audio.
    ///
    /// The audio samples are adapted to the output format and added to the under layers result.
    Audio {
        /// The audio.
        audio: AudioId,
        /// Clip the start of the audio.
        ///
        /// Set to [`Duration::ZERO`] to play from the start.
        skip: Duration,
        /// Clip the end of the audio. Does not add padding, if `skip + take` is greater than the audio length stops early.
        ///
        /// Set to [`Duration::MAX`] to play to the end.
        take: Duration,
    },
    /// Play the mix.
    ///
    /// This mix is sampled as an audio (computed), its effect layers do not affect the parent mix.
    AudioMix {
        /// The inner mix.
        mix: AudioMix,
        /// Clip the start of the audio.
        ///
        /// Set to [`Duration::ZERO`] to play from the start.
        skip: Duration,
        /// Clip the end of the audio. Does not add padding, if `skip + take` is greater than the audio length stops early.
        ///
        /// Set to [`Duration::MAX`] to play to the end.
        take: Duration,
    },

    /// Linear volume transition.
    ///
    /// When the playback is in range the volume is multiplied by the linear interpolation between `start_volume` and `end_volume`. The volume snaps
    /// back to the output stream volume after the end, unless another volume control layer is in effect.
    VolumeLinear {
        /// Start time.
        start: Duration,
        /// Transition duration.
        ///
        /// The effect ends at `start + duration` time.
        duration: Duration,

        /// Volume at the start.
        start_volume: Factor,
        /// Volume at the end,
        end_volume: Factor,
    },

    /// Generate a sine wave sound.
    SineWave {
        /// Sine frequency.
        frequency: f32,
        /// Duration of the sample.
        duration: Duration,
    },
}

bitflags! {
    /// Defines what tracks are decoded from multi track containers.
    #[derive(Copy, Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
    pub struct AudioTracksMode: u8 {
        /// Decodes all tracks.
        const TRACKS = 0b0001;
        /// Decodes only the first track, or the track explicitly marked as primary/default by the container format.
        ///
        /// Note that this is 0, empty.
        const PRIMARY = 0;
    }
}
