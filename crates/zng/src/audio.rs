#![cfg(feature = "audio")]

//! Audio service, widgets and other types.
//!
//! # Service
//!
//! The [`AUDIOS`] service manages audio loading and caching. Audio decoding is
//! implemented by the view-process, for this reason the service must be
//! used in a headed app or headless app with renderer, in a headless app without renderer no view-process
//! is spawned so no audio format will be available.
//!
//! The audio service also define security limits, the [`AUDIOS.limits`](fn@AUDIOS::limits)
//! variable to configure these limits. See [`AudioLimits::default`] for the defaults.
//!
//! ```
//! use zng::{audio, prelude::*};
//! # fn example() {
//!
//! audio::AUDIOS.limits().modify(|l| {
//!     l.allow_uri = audio::UriFilter::allow_host("httpbin.org");
//!     l.max_encoded_len = 1.megabytes();
//!     l.max_decoded_len = 10.megabytes();
//! }); }
//! ```
//!
//! The example above changes the global limits to allow audio downloads only from an specific host and
//! only allow audio with sizes less or equal to 1 megabyte and that only expands to up to 10 megabytes
//! after decoding.
//!
//! # Full API
//!
//! See [`zng_ext_audio`] for the full audio API.

pub use zng_ext_audio::{
    AUDIOS, AudioCacheMode, AudioDataFormat, AudioFormat, AudioHash, AudioLimits, AudioMix, AudioOptions, AudioOutput, AudioOutputId,
    AudioOutputState, AudioSource, AudioSourceFilter, AudioTrack, AudioVar,
};

#[cfg(feature = "http")]
pub use zng_ext_audio::UriFilter;
