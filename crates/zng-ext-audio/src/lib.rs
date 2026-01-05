#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Audio loading and cache and playback.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    hash::Hash, mem, path::{Path, PathBuf}, pin::Pin
};

mod types;
pub use types::*;

mod output;
pub use output::*;

use zng_app::{
    APP, AppExtension,
    event::app_local,
    update::EventUpdate,
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewAudioHandle,
        raw_events::{LOW_MEMORY_EVENT, RAW_AUDIO_DECODE_ERROR_EVENT, RAW_AUDIO_DECODED_EVENT, RAW_AUDIO_METADATA_DECODED_EVENT},
    },
};
use zng_clone_move::async_clmv;
use zng_task::{self as task, channel::IpcBytes};
#[cfg(feature = "http")]
use zng_txt::ToTxt;
use zng_txt::Txt;
use zng_unique_id::IdMap;
use zng_var::{Var, VarValue, WeakVar, const_var, var};
use zng_view_api::audio::AudioMetadata;

/// Application extension that provides an audio cache and output stream creation.
///
/// # Services
///
/// Services this extension provides.
///
/// * [`AUDIOS`]
#[derive(Default)]
#[non_exhaustive]
pub struct AudioManager {}
impl AppExtension for AudioManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_AUDIO_METADATA_DECODED_EVENT.on(update) {
        } else if let Some(args) = RAW_AUDIO_DECODED_EVENT.on(update) {
        } else if let Some(args) = RAW_AUDIO_DECODE_ERROR_EVENT.on(update) {
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
        } else if LOW_MEMORY_EVENT.on(update).is_some() {
            AUDIOS.clean_all();
        }
    }

    fn update_preview(&mut self) {
        // update loading tasks:
    }

    fn update(&mut self) {}
}

/// Audio loading, cache and output service.
///
/// If the app is running without a [`VIEW_PROCESS`] all audios are dummy, see [`load_in_headless`] for
/// details.
///
/// # Provider
///
/// This service is provided by the [`AudioManager`] extension, it will panic if used in an app not extended.
///
/// [`load_in_headless`]: AUDIOS::load_in_headless
/// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
pub struct AUDIOS;
impl AUDIOS {
    /// If should still download/read audio bytes in headless/renderless mode.
    ///
    /// When an app is in headless mode without renderer no [`VIEW_PROCESS`] is available, so
    /// audios cannot be decoded, in this case all audios are the [`dummy`] audio and no attempt
    /// to download/read the audio files is made. You can enable loading in headless tests to detect
    /// IO errors, in this case if there is an error acquiring the audio file the audio will be a
    /// [`dummy`] with error.
    ///
    /// [`dummy`]: AUDIOS::dummy
    /// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
    pub fn load_in_headless(&self) -> Var<bool> {
        AUDIOS_SV.read().load_in_headless.clone()
    }

    /// Default loading and decoding limits for each audio.
    pub fn limits(&self) -> Var<AudioLimits> {
        AUDIOS_SV.read().limits.clone()
    }

    /// Returns a dummy audio that reports it is loading or an error.
    pub fn dummy(&self, error: Option<Txt>) -> AudioVar {
        const_var(AudioTrack::new_empty(error.unwrap_or_default()))
    }

    /// Cache or load an audio file from a file system `path`.
    pub fn read(&self, path: impl Into<PathBuf>) -> AudioVar {
        self.audio_impl(path.into().into(), AudioOptions::cache(), None)
    }

    /// Get a cached `uri` or download it.
    ///
    /// Optionally define the HTTP ACCEPT header, if not set all audio formats supported by the view-process
    /// backend are accepted.
    #[cfg(feature = "http")]
    pub fn download<U>(&self, uri: U, accept: Option<Txt>) -> AudioVar
    where
        U: TryInto<task::http::Uri>,
        <U as TryInto<task::http::Uri>>::Error: ToTxt,
    {
        match uri.try_into() {
            Ok(uri) => self.audio(AudioSource::Download(uri, accept), AudioOptions::cache(), None),
            Err(e) => self.dummy(Some(e.to_txt())),
        }
    }

    /// Get a cached audio from `&'static [u8]` data.
    ///
    /// The data can be any of the formats described in [`AudioDataFormat`].
    ///
    /// The audio key is a [`AudioHash`] of the audio data. The audio is fully decoded.
    ///
    /// # Examples
    ///
    /// Get an audio from a WAV file embedded in the app executable using [`include_bytes!`].
    ///
    /// ```
    /// # use zng_ext_audio::*;
    /// # macro_rules! include_bytes { ($tt:tt) => { &[] } }
    /// # fn demo() {
    /// let audio_var = AUDIOS.from_static(include_bytes!("signal.wav"), "wav");
    /// # }
    pub fn from_static(&self, data: &'static [u8], format: impl Into<AudioDataFormat>) -> AudioVar {
        self.audio_impl((data, format.into()).into(), AudioOptions::cache(), None)
    }

    /// Get a cached audio from shared data.
    ///
    /// The audio key is a [`AudioHash`] of the audio data. The audio is fully decoded.
    /// The data reference is held only until the audio is decoded.
    ///
    /// The data can be any of the formats described in [`AudioDataFormat`].
    pub fn from_data(&self, data: IpcBytes, format: impl Into<AudioDataFormat>) -> AudioVar {
        self.audio_impl((data, format.into()).into(), AudioOptions::cache(), None)
    }

    /// Get or load an audio with full configuration.
    ///
    /// If `limits` is `None` the [`AUDIOS.limits`] is used.
    ///
    /// [`AUDIOS.limits`]: AUDIOS::limits
    pub fn audio(&self, source: impl Into<AudioSource>, options: AudioOptions, limits: Option<AudioLimits>) -> AudioVar {
        self.audio_impl(source.into(), options, limits)
    }
    fn audio_impl(&self, source: AudioSource, options: AudioOptions, limits: Option<AudioLimits>) -> AudioVar {
        todo!()
    }

    /// Await for an audio source, then get or load the audio.
    ///
    /// If `limits` is `None` the [`AUDIOS.limits`] is used.
    ///
    /// This method returns immediately with a loading [`AudioVar`], when `source` is ready it
    /// is used to get the actual [`AudioVar`] and binds it to the returned audio.
    ///
    /// Note that the [`cache_mode`] always applies to the inner audio, and only to the return audio if `cache_key` is set.
    ///
    /// [`AUDIOS.limits`]: AUDIOS::limits
    /// [`cache_mode`]: AudioOptions::cache_mode
    pub fn audio_task<F>(&self, source: impl IntoFuture<IntoFuture = F>, options: AudioOptions, limits: Option<AudioLimits>) -> AudioVar
    where
        F: Future<Output = AudioSource> + Send + 'static,
    {
        self.audio_task_impl(Box::pin(source.into_future()), options, limits)
    }
    fn audio_task_impl(
        &self,
        source: Pin<Box<dyn Future<Output = AudioSource> + Send + 'static>>,
        options: AudioOptions,
        limits: Option<AudioLimits>,
    ) -> AudioVar {
        let audio = var(AudioTrack::new_empty(Txt::from_static("")));
        task::spawn(async_clmv!(audio, {
            let source = source.await;
            let actual_audio = AUDIOS.audio_impl(source, options, limits);
            actual_audio.set_bind(&audio).perm();
            audio.hold(actual_audio).perm();
        }));
        audio.read_only()
    }

    /// Associate the `audio` produced by direct interaction with the view-process with the `key` in the cache.
    ///
    /// If the `key` is not set the audio is not cached, the service only manages it until it is loaded.
    ///
    /// Returns `Ok(AudioVar)` with the new audio var that tracks `audio`, or `Err(audio, AudioVar)`
    /// that returns the `audio` and a clone of the var already associated with the `key`.
    ///
    /// Note that you can register tracks on the returned [`AudioTrack::insert_track`].
    #[allow(clippy::result_large_err)] // boxing here does not really help performance
    pub fn register(
        &self,
        key: Option<AudioHash>,
        audio: (ViewAudioHandle, AudioMetadata),
    ) -> std::result::Result<AudioVar, ((ViewAudioHandle, AudioMetadata), AudioVar)> {
    }

    /// Remove the audio from the cache, if it is only held by the cache.
    ///
    /// You can use [`AudioSource::hash128_read`] and [`AudioSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the audio was removed.
    pub fn clean(&self, key: AudioHash) -> bool {}

    /// Remove the audio from the cache, even if it is still referenced outside of the cache.
    ///
    /// You can use [`AudioSource::hash128_read`] and [`AudioSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the audio was removed, that is, if it was cached.
    pub fn purge(&self, key: AudioHash) -> bool {}

    /// Gets the cache key of an audio.
    pub fn cache_key(&self, audio: &AudioTrack) -> Option<AudioHash> {
        if let Some(key) = &audio.cache_key
            && AUDIOS_SV.read().cache.contains_key(key)
        {
            return Some(*key);
        }
        None
    }

    /// If the audio is cached.
    pub fn is_cached(&self, audio: &AudioTrack) -> bool {
        audio
            .cache_key
            .as_ref()
            .map(|k| AUDIOS_SV.read().cache.contains_key(k))
            .unwrap_or(false)
    }

    /// Returns an audio that is not cached.
    ///
    /// If the `audio` is the only reference returns it and removes it from the cache. If there are other
    /// references a new [`AudioVar`] is generated from a clone of the audio.
    pub fn detach(&self, audio: AudioVar) -> AudioVar {}

    /// Clear cached audios that are not referenced outside of the cache.
    pub fn clean_all(&self) {}

    /// Clear all cached audios, including audios that are still referenced outside of the cache.
    ///
    /// Audio memory only drops when all strong references are removed, so if an audio is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_all(&self) {
        let mut img = AUDIOS_SV.write();
        img.cache.clear();
        img.extensions.iter_mut().for_each(|p| p.clear(true));
    }

    /// Add an audios service extension.
    ///
    /// See [`AudiosExtension`] for extension capabilities.
    pub fn extend(&self, extension: Box<dyn AudiosExtension>) {
        AUDIOS_SV.write().extensions.push(extension);
    }

    /// Audio formats implemented by the current view-process and extensions.
    pub fn available_formats(&self) -> Vec<AudioFormat> {
        let mut formats = VIEW_PROCESS.info().audio.clone();
        for ext in &AUDIOS_SV.read().extensions {
            ext.available_formats(&mut formats);
        }
        formats
    }
}

/// Options for [`AUDIOS.audio`].
///
/// [`AUDIOS.audio`]: AUDIOS::audio
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AudioOptions {
    /// If and how the audio is cached.
    pub cache_mode: AudioCacheMode,
}

impl AudioOptions {
    /// New.
    pub fn new(cache_mode: AudioCacheMode) -> Self {
        Self { cache_mode }
    }

    /// New with only cache enabled.
    pub fn cache() -> Self {
        Self::new(AudioCacheMode::Cache)
    }
}

fn absolute_path(path: &Path, base: impl FnOnce() -> PathBuf, allow_escape: bool) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        let mut dir = base();
        if allow_escape {
            dir.push(path);
            normalize_path(&dir)
        } else {
            dir.push(normalize_path(path));
            dir
        }
    }
}
/// Resolves `..` components, without any system request.
///
/// Source: https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

app_local! {
    static AUDIOS_SV: AudiosService = {
        APP.extensions().require::<crate::AudioManager>();
        AudiosService::new()
    };
}

pub(crate) struct AudiosService {
    load_in_headless: Var<bool>,
    limits: Var<AudioLimits>,

    extensions: Vec<Box<dyn AudiosExtension>>,

    download_accept: Txt,

}
impl AudiosService {
    fn new() -> Self {
        Self {
            load_in_headless: var(false),
            limits: var(AudioLimits::default()),
            extensions: vec![],
            download_accept: Txt::from_static(""),
        }
    }
}

struct VarCache<K: Eq + Hash + 'static,  T: VarValue> {
    entries: IdMap<K, CacheEntry<T>>
}

enum CacheEntry<T: VarValue> {
    Cached(Var<T>),
    NotCached(WeakVar<T>),
}