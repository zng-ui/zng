#![cfg(feature = "audio")]

//! Audio service, widgets and other types.
//!
//! !!: TODO docs
//!
//! # Full API
//!
//! See [`zng_ext_audio`] for the full audio API.

pub use zng_ext_audio::{
    AUDIOS, AudioCacheMode, AudioDataFormat, AudioFormat, AudioHash, AudioLimits, AudioMix, AudioOptions, AudioOutput, AudioOutputId,
    AudioOutputState, AudioSource, AudioSourceFilter, AudioTrack, AudioVar,
};
