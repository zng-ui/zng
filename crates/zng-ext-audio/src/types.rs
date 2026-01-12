use std::{
    any::Any,
    env, fmt, mem, ops,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use zng_app::view_process::ViewAudioHandle;
use zng_task::channel::{IpcBytes, IpcBytesCast};
use zng_txt::Txt;
use zng_unit::{ByteLength, ByteUnits as _};
#[cfg(feature = "http")]
use zng_var::impl_from_and_into_var;
#[cfg(not(feature = "http"))]
use zng_var::impl_from_and_into_var;
use zng_var::{Var, VarEq};
use zng_view_api::audio::{AudioDecoded, AudioId, AudioMetadata};

pub use zng_app::view_process::AudioOutputId;
pub use zng_view_api::audio::{AudioDataFormat, AudioFormat, AudioOutputState, AudioTracksMode};

/// A custom extension for the [`AUDIOS`] service.
///
/// Extensions can intercept and modify requests.
///
/// [`AUDIOS`]: crate::AUDIOS
pub trait AudiosExtension: Send + Sync + Any {
    /// Modify a [`AUDIOS.audio`] request.
    ///
    /// Note that all other request methods are shorthand helpers so this will be called for every request.
    ///
    /// Note that the [`AUDIOS`] service can be used in extensions and [`AudioSource::Audio`] is returned directly by the service.
    /// This can be used to fully replace a request here.
    ///
    /// [`AUDIOS.audio`]: crate::AUDIOS::audio
    /// [`AUDIOS`]: crate::AUDIOS
    fn audio(&mut self, limits: &AudioLimits, source: &mut AudioSource, options: &mut AudioOptions) {
        let _ = (limits, source, options);
    }

    /// Audio data loaded.
    ///
    /// This is called for [`AudioSource::Read`], [`AudioSource::Download`] and [`AudioSource::Data`] after the data is loaded and before
    /// decoding starts.
    ///
    /// Return a replacement variable to skip decoding or redirect to a different audio. Note that by the time this is called the service
    /// has already returned a variable in loading state, that variable will be cached according to `mode`. The replacement variable
    /// is bound to the return variable and lives as long as it does.
    ///
    /// Note that the [`AUDIOS`] service can be used in extensions.
    ///
    /// [`AUDIOS`]: crate::AUDIOS
    #[allow(clippy::too_many_arguments)]
    fn audio_data(
        &mut self,
        max_decoded_len: ByteLength,
        key: &AudioHash,
        data: &IpcBytes,
        format: &AudioDataFormat,
        options: &AudioOptions,
    ) -> Option<AudioVar> {
        let _ = (max_decoded_len, key, data, format, options);
        None
    }

    /// Modify a [`AUDIOS.clean`] or [`AUDIOS.purge`] request.
    ///
    /// Return `false` to cancel the removal.
    ///
    /// [`AUDIOS.clean`]: crate::AUDIOS::clean
    /// [`AUDIOS.purge`]: crate::AUDIOS::purge
    fn remove(&mut self, key: &mut AudioHash, purge: &mut bool) -> bool {
        let _ = (key, purge);
        true
    }

    /// Called on [`AUDIOS.clean_all`] and [`AUDIOS.purge_all`].
    ///
    /// These operations cannot be intercepted, the service cache will be cleaned after this call.
    ///
    /// [`AUDIOS.clean_all`]: crate::AUDIOS::clean_all
    /// [`AUDIOS.purge_all`]: crate::AUDIOS::purge_all
    fn clear(&mut self, purge: bool) {
        let _ = purge;
    }

    /// Add or remove formats this extension affects.
    ///
    /// The `formats` value starts with all formats implemented by the current view-process and will be returned
    /// by [`AUDIOS.available_formats`] after all proxies edit it.
    ///
    /// [`AUDIOS.available_formats`]: crate::AUDIOS::available_formats
    fn available_formats(&self, formats: &mut Vec<AudioFormat>) {
        let _ = formats;
    }
}

/// Represents an [`AudioTrack`] tracked by the [`AUDIOS`] cache.
///
/// The variable updates when the audio updates.
///
/// [`AUDIOS`]: super::AUDIOS
pub type AudioVar = Var<AudioTrack>;
/// State of an [`AudioVar`].
#[derive(Debug, Clone)]
pub struct AudioTrack {
    pub(crate) cache_key: Option<AudioHash>,
    pub(crate) handle: ViewAudioHandle,
    meta: AudioMetadata,
    data: AudioDecoded,
    tracks: Vec<VarEq<AudioTrack>>,
    error: Txt,
}
impl PartialEq for AudioTrack {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
            && self.cache_key == other.cache_key
            && self.error == other.error
            && self.meta == other.meta
            && self.data == other.data
            && self.tracks == other.tracks
    }
}
impl AudioTrack {
    pub(super) fn new_cached(cache_key: AudioHash) -> Self {
        let mut s = Self::new_loading(ViewAudioHandle::dummy());
        s.cache_key = Some(cache_key);
        s
    }

    pub(super) fn new_loading(handle: ViewAudioHandle) -> Self {
        Self {
            cache_key: None,
            meta: AudioMetadata::new(handle.audio_id(), 0, 0),
            data: AudioDecoded::new(handle.audio_id(), IpcBytesCast::default()),
            handle,
            tracks: vec![],
            error: Txt::from_static(""),
        }
    }

    /// New from existing view audio.
    pub(super) fn new(handle: ViewAudioHandle, meta: AudioMetadata, data: AudioDecoded) -> Self {
        Self {
            cache_key: None,
            handle,
            meta,
            data,
            tracks: vec![],
            error: Txt::from_static(""),
        }
    }

    /// Create a dummy audio in the loading or error state.
    ///
    /// Note that you can use the [`AUDIOS.register`] method to integrate with audios from other sources. The intention
    /// of this function is creating an initial loading audio or an error message audio.
    ///
    /// [`AUDIOS.register`]: crate::AUDIOS::register
    pub fn new_empty(error: Txt) -> Self {
        let mut s = Self::new_loading(ViewAudioHandle::dummy());
        s.error = error;
        s
    }

    /// Returns `true` if the is still acquiring or decoding the audio bytes.
    pub fn is_loading(&self) -> bool {
        self.error.is_empty() && !self.data.is_full
    }

    /// If the audio has finished loading ok or due to error.
    ///
    /// The audio variable may still update after
    pub fn is_loaded(&self) -> bool {
        !self.is_loading()
    }

    /// If this audio can be cued for playback already.
    ///
    /// This is `true` when the audio has decoded enough that it can begin streaming.
    pub fn can_cue(&self) -> bool {
        !self.view_handle().is_dummy() && !self.is_error()
    }

    /// If the audio failed to load.
    pub fn is_error(&self) -> bool {
        !self.error.is_empty()
    }

    /// Returns an error message if the audio failed to load.
    pub fn error(&self) -> Option<Txt> {
        if self.error.is_empty() { None } else { Some(self.error.clone()) }
    }

    /// If [`tracks`] is not empty.
    ///
    /// [`tracks`]: Self::tracks
    pub fn has_tracks(&self) -> bool {
        !self.tracks.is_empty()
    }

    /// Other audio tracks from the same container that are a *child* of this audio track.
    pub fn tracks(&self) -> Vec<AudioVar> {
        self.tracks.iter().map(|e| e.read_only()).collect()
    }

    /// All other audio tracks from the same container that are a *descendant* of this audio track.
    ///
    /// The values are a tuple of each entry and the length of descendants tracks that follow it.
    ///
    /// The returned variable will update every time any entry descendant var updates.
    pub fn flat_tracks(&self) -> Var<Vec<(VarEq<AudioTrack>, usize)>> {
        // idea here is to just rebuild the flat list on any update,
        // assuming the audio variables don't update much and tha there are not many entries
        // this is more simple than some sort of recursive Var::flat_map_vec setup

        // each entry updates this var on update
        let update_signal = zng_var::var(());

        // init value and update bindings
        let mut out = vec![];
        let mut update_handles = vec![];
        self.flat_entries_init(&mut out, update_signal.clone(), &mut update_handles);
        let out = zng_var::var(out);

        // bind signal to rebuild list on update and rebind update signal
        let self_ = self.clone();
        let signal_weak = update_signal.downgrade();
        update_signal
            .bind_modify(&out, move |_, out| {
                out.clear();
                update_handles.clear();
                self_.flat_entries_init(&mut *out, signal_weak.upgrade().unwrap(), &mut update_handles);
            })
            .perm();
        out.hold(update_signal).perm();
        out.read_only()
    }
    fn flat_entries_init(&self, out: &mut Vec<(VarEq<AudioTrack>, usize)>, update_signal: Var<()>, handles: &mut Vec<zng_var::VarHandle>) {
        for entry in self.tracks.iter() {
            Self::flat_entries_recursive_init(entry.clone(), out, update_signal.clone(), handles);
        }
    }
    fn flat_entries_recursive_init(
        audio: VarEq<AudioTrack>,
        out: &mut Vec<(VarEq<AudioTrack>, usize)>,
        signal: Var<()>,
        handles: &mut Vec<zng_var::VarHandle>,
    ) {
        handles.push(audio.hook(zng_clone_move::clmv!(signal, |_| {
            signal.update();
            true
        })));
        let i = out.len();
        out.push((audio.clone(), 0));
        audio.with(move |audio| {
            for entry in audio.tracks.iter() {
                Self::flat_entries_recursive_init(entry.clone(), out, signal.clone(), handles);
            }
            let len = out.len() - i;
            out[i].1 = len;
        });
    }

    /// Sort index of the audio track in the list of entries of the source container.
    pub fn track_index(&self) -> usize {
        match &self.meta.parent {
            Some(p) => p.index,
            None => 0,
        }
    }

    /// Connection to the audio resource in the view-process.
    pub fn view_handle(&self) -> ViewAudioHandle {
        self.handle.clone()
    }

    /// Number of channels interleaved in the track.
    pub fn channel_count(&self) -> u16 {
        self.meta.channel_count
    }

    /// Samples per second.
    ///
    /// A sample is a single sequence of `channel_count`.
    pub fn sample_rate(&self) -> u32 {
        self.meta.sample_rate
    }

    /// Total duration of the tack, if it is known.
    pub fn total_duration(&self) -> Option<Duration> {
        self.meta.total_duration
    }

    /// Reference the decoded interleaved audio chunk.
    pub fn chunk(&self) -> Option<IpcBytesCast<f32>> {
        if self.is_loaded() { Some(self.data.chunk.clone()) } else { None }
    }

    /// Insert `track` in [`tracks`].
    ///
    /// [`tracks`]: Self::tracks
    pub fn insert_track(&mut self, track: AudioTrack) -> AudioVar {
        let i = track.track_index();
        let i = self
            .tracks
            .iter()
            .position(|v| {
                let entry_i = v.with(|i| i.track_index());
                entry_i > i
            })
            .unwrap_or(self.tracks.len());
        let entry = zng_var::var(track);
        self.tracks.insert(i, VarEq(entry.clone()));
        entry
    }

    pub(crate) fn set_meta(&mut self, meta: AudioMetadata) {
        self.meta = meta;
    }

    pub(crate) fn set_data(&mut self, data: AudioDecoded) {
        self.data = data;
    }

    pub(crate) fn set_error(&mut self, error: Txt) {
        self.error = error;
    }

    pub(crate) fn set_handle(&mut self, handle: ViewAudioHandle) {
        self.handle = handle;
    }

    pub(crate) fn has_loading_tracks(&self) -> bool {
        self.tracks.iter().any(|t| t.with(|t| t.is_loading() || t.has_loading_tracks()))
    }

    pub(crate) fn find_track(&self, id: AudioId) -> Option<AudioVar> {
        self.tracks.iter().find_map(|v| {
            if v.with(|i| i.handle.audio_id() == id) {
                Some(v.0.clone())
            } else {
                v.with(|i| i.find_track(id))
            }
        })
    }
}

/// A 256-bit hash for audio tracks.
///
/// This hash is used to identify audio files in the [`AUDIOS`] cache.
///
/// Use [`AudioHasher`] to compute.
///
/// [`AUDIOS`]: super::AUDIOS
#[derive(Clone, Copy)]
pub struct AudioHash([u8; 32]);
impl AudioHash {
    /// Compute the hash for `data`.
    pub fn compute(data: &[u8]) -> Self {
        let mut h = Self::hasher();
        h.update(data);
        h.finish()
    }

    /// Start a new [`AudioHasher`].
    pub fn hasher() -> AudioHasher {
        AudioHasher::default()
    }
}
impl fmt::Debug for AudioHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("AudioHash").field(&self.0).finish()
        } else {
            use base64::*;
            write!(f, "{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0))
        }
    }
}
impl fmt::Display for AudioHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::hash::Hash for AudioHash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let h64 = [
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7],
        ];
        state.write_u64(u64::from_ne_bytes(h64))
    }
}
impl PartialEq for AudioHash {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for AudioHash {}

/// Hasher that computes a [`AudioHash`].
pub struct AudioHasher(sha2::Sha512_256);
impl Default for AudioHasher {
    fn default() -> Self {
        use sha2::Digest;
        Self(sha2::Sha512_256::new())
    }
}
impl AudioHasher {
    /// New default hasher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process data, updating the internal state.
    pub fn update(&mut self, data: &[u8]) {
        use sha2::Digest;

        // some gigantic audios can take to long to hash, we just
        // need the hash for identification so we sample the data
        const NUM_SAMPLES: usize = 1000;
        const SAMPLE_CHUNK_SIZE: usize = 1024;

        let total_size = data.len();
        if total_size == 0 {
            return;
        }
        if total_size < 1000 * 1000 * 4 {
            return self.0.update(data);
        }

        let step_size = total_size.checked_div(NUM_SAMPLES).unwrap_or(total_size);
        for n in 0..NUM_SAMPLES {
            let start_index = n * step_size;
            if start_index >= total_size {
                break;
            }
            let end_index = (start_index + SAMPLE_CHUNK_SIZE).min(total_size);
            let s = &data[start_index..end_index];
            self.0.update(s);
        }
    }

    /// Finish computing the hash.
    pub fn finish(self) -> AudioHash {
        use sha2::Digest;
        // dependencies `sha2 -> digest` need to upgrade
        // https://github.com/RustCrypto/traits/issues/2036
        // https://github.com/fizyk20/generic-array/issues/158
        AudioHash(self.0.finalize().as_slice().try_into().unwrap())
    }
}
impl std::hash::Hasher for AudioHasher {
    fn finish(&self) -> u64 {
        tracing::warn!("Hasher::finish called for AudioHasher");

        use sha2::Digest;
        let hash = self.0.clone().finalize();
        u64::from_le_bytes(hash[..8].try_into().unwrap())
    }

    fn write(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }
}

/// The different sources of an audio input.
#[derive(Clone)]
#[non_exhaustive]
pub enum AudioSource {
    /// A path to an audio file in the file system.
    ///
    /// Audio equality is defined by the path, a copy of the audio in another path is a different audio.
    Read(PathBuf),
    /// A uri to an audio resource downloaded using HTTP GET with an optional HTTP ACCEPT string.
    ///
    /// If the ACCEPT line is not given, all audio formats supported by the view-process backend are accepted.
    ///
    /// Audio equality is defined by the URI and ACCEPT string.
    #[cfg(feature = "http")]
    Download(zng_task::http::Uri, Option<Txt>),
    /// Shared reference to bytes for an encoded or decoded audio.
    ///
    /// Audio equality is defined by the hash, it is usually the hash of the bytes but it does not need to be.
    ///
    /// Inside [`AUDIOS`] the reference to the bytes is held only until the audio finishes decoding.
    ///
    /// [`AUDIOS`]: super::AUDIOS
    Data(AudioHash, IpcBytes, AudioDataFormat),

    /// Already resolved (decoding or decoded) audio.
    ///
    /// The audio is passed-through, not cached.
    Audio(AudioVar),
}
impl AudioSource {
    /// New source from data.
    pub fn from_data(data: IpcBytes, format: AudioDataFormat) -> Self {
        let mut hasher = AudioHasher::default();
        hasher.update(&data[..]);
        let hash = hasher.finish();
        Self::Data(hash, data, format)
    }

    /// Returns the audio hash, unless the source is [`AudioTrack`].
    pub fn hash128(&self, options: &AudioOptions) -> Option<AudioHash> {
        match self {
            AudioSource::Read(p) => Some(Self::hash128_read(p, options)),
            #[cfg(feature = "http")]
            AudioSource::Download(u, a) => Some(Self::hash128_download(u, a, options)),
            AudioSource::Data(h, _, _) => Some(Self::hash128_data(*h, options)),
            AudioSource::Audio(_) => None,
        }
    }

    /// Compute hash for a borrowed [`Data`] audio.
    ///
    /// [`Data`]: Self::Data
    pub fn hash128_data(data_hash: AudioHash, options: &AudioOptions) -> AudioHash {
        let _ = options; // hash options that affect data if any is added in the future.
        data_hash
    }

    /// Compute hash for a borrowed [`Read`] path.
    ///
    /// [`Read`]: Self::Read
    pub fn hash128_read(path: &Path, options: &AudioOptions) -> AudioHash {
        use std::hash::Hash;
        let mut h = AudioHash::hasher();
        0u8.hash(&mut h);
        path.hash(&mut h);
        let _ = options;
        h.finish()
    }

    /// Compute hash for a borrowed [`Download`] URI and HTTP-ACCEPT.
    ///
    /// [`Download`]: Self::Download
    #[cfg(feature = "http")]
    pub fn hash128_download(uri: &zng_task::http::Uri, accept: &Option<Txt>, options: &AudioOptions) -> AudioHash {
        use std::hash::Hash;
        let mut h = AudioHash::hasher();
        1u8.hash(&mut h);
        uri.hash(&mut h);
        accept.hash(&mut h);
        let _ = options;
        h.finish()
    }
}

impl PartialEq for AudioSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Read(l), Self::Read(r)) => l == r,
            #[cfg(feature = "http")]
            (Self::Download(lu, la), Self::Download(ru, ra)) => lu == ru && la == ra,
            (Self::Audio(l), Self::Audio(r)) => l.var_eq(r),
            (l, r) => {
                let l_hash = match l {
                    AudioSource::Data(h, _, _) => h,
                    _ => return false,
                };
                let r_hash = match r {
                    AudioSource::Data(h, _, _) => h,
                    _ => return false,
                };

                l_hash == r_hash
            }
        }
    }
}
impl fmt::Debug for AudioSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "AudioSource::")?;
        }
        match self {
            AudioSource::Read(p) => f.debug_tuple("Read").field(p).finish(),
            #[cfg(feature = "http")]
            AudioSource::Download(u, a) => f.debug_tuple("Download").field(u).field(a).finish(),
            AudioSource::Data(key, bytes, fmt) => f.debug_tuple("Data").field(key).field(bytes).field(fmt).finish(),
            AudioSource::Audio(_) => write!(f, "Audio(_)"),
        }
    }
}

#[cfg(feature = "http")]
impl_from_and_into_var! {
    fn from(uri: zng_task::http::Uri) -> AudioSource {
        AudioSource::Download(uri, None)
    }
    /// From (URI, HTTP-ACCEPT).
    fn from((uri, accept): (zng_task::http::Uri, &'static str)) -> AudioSource {
        AudioSource::Download(uri, Some(accept.into()))
    }

    /// Converts `http://` and `https://` to [`Download`], `file://` to
    /// [`Read`] the path component, and the rest to [`Read`] the string as a path.
    ///
    /// [`Download`]: AudioSource::Download
    /// [`Read`]: AudioSource::Read
    fn from(s: &str) -> AudioSource {
        use zng_task::http::*;
        if let Ok(uri) = Uri::try_from(s)
            && let Some(scheme) = uri.scheme()
        {
            if scheme == &uri::Scheme::HTTPS || scheme == &uri::Scheme::HTTP {
                return AudioSource::Download(uri, None);
            } else if scheme.as_str() == "file" {
                return PathBuf::from(uri.path()).into();
            }
        }
        PathBuf::from(s).into()
    }
}

#[cfg(not(feature = "http"))]
impl_from_and_into_var! {
    /// Converts to [`Read`].
    ///
    /// [`Read`]: AudioSource::Read
    fn from(s: &str) -> AudioSource {
        PathBuf::from(s).into()
    }
}

impl_from_and_into_var! {
    fn from(audio: AudioVar) -> AudioSource {
        AudioSource::Audio(audio)
    }
    fn from(path: PathBuf) -> AudioSource {
        AudioSource::Read(path)
    }
    fn from(path: &Path) -> AudioSource {
        path.to_owned().into()
    }

    /// Same as conversion from `&str`.
    fn from(s: String) -> AudioSource {
        s.as_str().into()
    }
    /// Same as conversion from `&str`.
    fn from(s: Txt) -> AudioSource {
        s.as_str().into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: AudioDataFormat::Unknown
    fn from(data: &[u8]) -> AudioSource {
        AudioSource::Data(
            AudioHash::compute(data),
            IpcBytes::from_slice_blocking(data).expect("cannot allocate IpcBytes"),
            AudioDataFormat::Unknown,
        )
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: AudioDataFormat::Unknown
    fn from<const N: usize>(data: &[u8; N]) -> AudioSource {
        (&data[..]).into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: AudioDataFormat::Unknown
    fn from(data: IpcBytes) -> AudioSource {
        AudioSource::Data(AudioHash::compute(&data[..]), data, AudioDataFormat::Unknown)
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: AudioDataFormat::Unknown
    fn from(data: Vec<u8>) -> AudioSource {
        IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes").into()
    }
    /// From encoded data of known format.
    fn from<F: Into<AudioDataFormat>>((data, format): (&[u8], F)) -> AudioSource {
        AudioSource::Data(
            AudioHash::compute(data),
            IpcBytes::from_slice_blocking(data).expect("cannot allocate IpcBytes"),
            format.into(),
        )
    }
    /// From encoded data of known format.
    fn from<F: Into<AudioDataFormat>, const N: usize>((data, format): (&[u8; N], F)) -> AudioSource {
        (&data[..], format).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<AudioDataFormat>>((data, format): (Vec<u8>, F)) -> AudioSource {
        (IpcBytes::from_vec_blocking(data).expect("cannot allocate IpcBytes"), format).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<AudioDataFormat>>((data, format): (IpcBytes, F)) -> AudioSource {
        AudioSource::Data(AudioHash::compute(&data[..]), data, format.into())
    }
}

/// Cache mode of [`AUDIOS`].
///
/// [`AUDIOS`]: super::AUDIOS
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AudioCacheMode {
    /// Don't hit the cache, just loads the audio.
    Ignore,
    /// Gets a cached audio or loads the audio and caches it.
    Cache,
    /// Cache or reload if the cached audio is an error.
    Retry,
    /// Reloads the cache audio or loads the audio and caches it.
    ///
    /// The [`AudioVar`] is not replaced, other references to the audio also receive the update.
    Reload,
}
impl fmt::Debug for AudioCacheMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "CacheMode::")?;
        }
        match self {
            Self::Ignore => write!(f, "Ignore"),
            Self::Cache => write!(f, "Cache"),
            Self::Retry => write!(f, "Retry"),
            Self::Reload => write!(f, "Reload"),
        }
    }
}
impl_from_and_into_var! {
    fn from(cache: bool) -> AudioCacheMode {
        if cache { AudioCacheMode::Cache } else { AudioCacheMode::Ignore }
    }
}

/// Represents a [`PathFilter`] and [`UriFilter`].
#[derive(Clone)]
pub enum AudioSourceFilter<U> {
    /// Block all requests of this type.
    BlockAll,
    /// Allow all requests of this type.
    AllowAll,
    /// Custom filter, returns `true` to allow a request, `false` to block.
    Custom(Arc<dyn Fn(&U) -> bool + Send + Sync>),
}
impl<U> AudioSourceFilter<U> {
    /// New [`Custom`] filter.
    ///
    /// [`Custom`]: Self::Custom
    pub fn custom(allow: impl Fn(&U) -> bool + Send + Sync + 'static) -> Self {
        Self::Custom(Arc::new(allow))
    }

    /// Combine `self` with `other`, if they both are [`Custom`], otherwise is [`BlockAll`] if any is [`BlockAll`], else
    /// is [`AllowAll`] if any is [`AllowAll`].
    ///
    /// If both are [`Custom`] both filters must allow a request to pass the new filter.
    ///
    /// [`Custom`]: Self::Custom
    /// [`BlockAll`]: Self::BlockAll
    /// [`AllowAll`]: Self::AllowAll
    pub fn and(self, other: Self) -> Self
    where
        U: 'static,
    {
        use AudioSourceFilter::*;
        match (self, other) {
            (BlockAll, _) | (_, BlockAll) => BlockAll,
            (AllowAll, _) | (_, AllowAll) => AllowAll,
            (Custom(c0), Custom(c1)) => Custom(Arc::new(move |u| c0(u) && c1(u))),
        }
    }

    /// Combine `self` with `other`, if they both are [`Custom`], otherwise is [`AllowAll`] if any is [`AllowAll`], else
    /// is [`BlockAll`] if any is [`BlockAll`].
    ///
    /// If both are [`Custom`] at least one of the filters must allow a request to pass the new filter.
    ///
    /// [`Custom`]: Self::Custom
    /// [`BlockAll`]: Self::BlockAll
    /// [`AllowAll`]: Self::AllowAll
    pub fn or(self, other: Self) -> Self
    where
        U: 'static,
    {
        use AudioSourceFilter::*;
        match (self, other) {
            (AllowAll, _) | (_, AllowAll) => AllowAll,
            (BlockAll, _) | (_, BlockAll) => BlockAll,
            (Custom(c0), Custom(c1)) => Custom(Arc::new(move |u| c0(u) || c1(u))),
        }
    }

    /// Returns `true` if the filter allows the request.
    pub fn allows(&self, item: &U) -> bool {
        match self {
            AudioSourceFilter::BlockAll => false,
            AudioSourceFilter::AllowAll => true,
            AudioSourceFilter::Custom(f) => f(item),
        }
    }
}
impl<U> fmt::Debug for AudioSourceFilter<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlockAll => write!(f, "BlockAll"),
            Self::AllowAll => write!(f, "AllowAll"),
            Self::Custom(_) => write!(f, "Custom(_)"),
        }
    }
}
impl<U: 'static> ops::BitAnd for AudioSourceFilter<U> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}
impl<U: 'static> ops::BitOr for AudioSourceFilter<U> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}
impl<U: 'static> ops::BitAndAssign for AudioSourceFilter<U> {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = mem::replace(self, Self::BlockAll).and(rhs);
    }
}
impl<U: 'static> ops::BitOrAssign for AudioSourceFilter<U> {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = mem::replace(self, Self::BlockAll).or(rhs);
    }
}
impl<U> PartialEq for AudioSourceFilter<U> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(l0), Self::Custom(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// Represents a [`AudioSource::Read`] path request filter.
///
/// Only absolute, normalized paths are shared with the [`Custom`] filter, there is no relative paths or `..` components.
///
/// The paths are **not** canonicalized and existence is not verified, no system requests are made with unfiltered paths.
///
/// See [`AudioLimits::allow_path`] for more information.
///
/// [`Custom`]: AudioSourceFilter::Custom
pub type PathFilter = AudioSourceFilter<PathBuf>;
impl PathFilter {
    /// Allow any file inside `dir` or sub-directories of `dir`.
    pub fn allow_dir(dir: impl AsRef<Path>) -> Self {
        let dir = crate::absolute_path(dir.as_ref(), || env::current_dir().expect("could not access current dir"), true);
        PathFilter::custom(move |r| r.starts_with(&dir))
    }

    /// Allow any path with the `ext` extension.
    pub fn allow_ext(ext: impl Into<std::ffi::OsString>) -> Self {
        let ext = ext.into();
        PathFilter::custom(move |r| r.extension().map(|e| e == ext).unwrap_or(false))
    }

    /// Allow any file inside the [`env::current_dir`] or sub-directories.
    ///
    /// Note that the current directory can be changed and the filter always uses the
    /// *fresh* current directory, use [`allow_dir`] to create a filter the always points
    /// to the current directory at the filter creation time.
    ///
    /// [`allow_dir`]: Self::allow_dir
    pub fn allow_current_dir() -> Self {
        PathFilter::custom(|r| env::current_dir().map(|d| r.starts_with(d)).unwrap_or(false))
    }

    /// Allow any file inside the current executable directory or sub-directories.
    pub fn allow_exe_dir() -> Self {
        if let Ok(mut p) = env::current_exe().and_then(dunce::canonicalize)
            && p.pop()
        {
            return Self::allow_dir(p);
        }

        // not `BlockAll` so this can still be composed using `or`.
        Self::custom(|_| false)
    }

    /// Allow any file inside the [`zng::env::res`] directory or sub-directories.
    ///
    /// [`zng::env::res`]: zng_env::res
    pub fn allow_res() -> Self {
        Self::allow_dir(zng_env::res(""))
    }
}

/// Represents an [`AudioSource::Download`] path request filter.
///
/// See [`AudioLimits::allow_uri`] for more information.
#[cfg(feature = "http")]
pub type UriFilter = AudioSourceFilter<zng_task::http::Uri>;
#[cfg(feature = "http")]
impl UriFilter {
    /// Allow any file from the `host` site.
    pub fn allow_host(host: impl Into<Txt>) -> Self {
        let host = host.into();
        UriFilter::custom(move |u| u.authority().map(|a| a.host() == host).unwrap_or(false))
    }
}

impl<F: Fn(&PathBuf) -> bool + Send + Sync + 'static> From<F> for PathFilter {
    fn from(custom: F) -> Self {
        PathFilter::custom(custom)
    }
}

#[cfg(feature = "http")]
impl<F: Fn(&zng_task::http::Uri) -> bool + Send + Sync + 'static> From<F> for UriFilter {
    fn from(custom: F) -> Self {
        UriFilter::custom(custom)
    }
}

/// Limits for audio loading and decoding.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AudioLimits {
    /// Maximum encoded file size allowed.
    ///
    /// An error is returned if the file size surpasses this value. If the size can read before
    /// read/download the validation happens before download starts, otherwise the error happens when this limit
    /// is reached and all already downloaded bytes are dropped.
    ///
    /// The default is `100mb`.
    pub max_encoded_len: ByteLength,
    /// Maximum decoded file size allowed.
    ///
    /// An error is returned if the decoded audio memory (channels * sample_rate * total_duration * 4) would surpass this.
    ///
    /// Note that for chunked streaming this limits only the chunk_len * 2.
    pub max_decoded_len: ByteLength,

    /// Filter for [`AudioSource::Read`] paths.
    pub allow_path: PathFilter,

    /// Filter for [`AudioSource::Download`] URIs.
    #[cfg(feature = "http")]
    pub allow_uri: UriFilter,
}
impl AudioLimits {
    /// No size limits, allow all paths and URIs.
    pub fn none() -> Self {
        AudioLimits {
            max_encoded_len: ByteLength::MAX,
            max_decoded_len: ByteLength::MAX,
            allow_path: PathFilter::AllowAll,
            #[cfg(feature = "http")]
            allow_uri: UriFilter::AllowAll,
        }
    }

    /// Set the [`max_encoded_len`].
    ///
    /// [`max_encoded_len`]: Self::max_encoded_len
    pub fn with_max_encoded_len(mut self, max_encoded_len: impl Into<ByteLength>) -> Self {
        self.max_encoded_len = max_encoded_len.into();
        self
    }

    /// Set the [`max_decoded_len`].
    ///
    /// [`max_decoded_len`]: Self::max_encoded_len
    pub fn with_max_decoded_len(mut self, max_decoded_len: impl Into<ByteLength>) -> Self {
        self.max_decoded_len = max_decoded_len.into();
        self
    }

    /// Set the [`allow_path`].
    ///
    /// [`allow_path`]: Self::allow_path
    pub fn with_allow_path(mut self, allow_path: impl Into<PathFilter>) -> Self {
        self.allow_path = allow_path.into();
        self
    }

    /// Set the [`allow_uri`].
    ///
    /// [`allow_uri`]: Self::allow_uri
    #[cfg(feature = "http")]
    pub fn with_allow_uri(mut self, allow_url: impl Into<UriFilter>) -> Self {
        self.allow_uri = allow_url.into();
        self
    }
}
impl Default for AudioLimits {
    /// 100 megabytes encoded and 4096 megabytes decoded.
    ///
    /// Allows only paths in `zng::env::res`, blocks all downloads.
    fn default() -> Self {
        Self {
            max_encoded_len: 100.megabytes(),
            max_decoded_len: 4096.megabytes(),
            allow_path: PathFilter::allow_res(),
            #[cfg(feature = "http")]
            allow_uri: UriFilter::BlockAll,
        }
    }
}
impl_from_and_into_var! {
    fn from(some: AudioLimits) -> Option<AudioLimits>;
}

/// Options for [`AUDIOS.audio`].
///
/// [`AUDIOS.audio`]: crate::AUDIOS::audio
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AudioOptions {
    /// If and how the audio is cached.
    pub cache_mode: AudioCacheMode,

    /// How to decode containers with multiple tracks.
    pub tracks: AudioTracksMode,
}

impl AudioOptions {
    /// New.
    pub fn new(cache_mode: AudioCacheMode) -> Self {
        Self {
            cache_mode,
            tracks: AudioTracksMode::PRIMARY,
        }
    }

    /// New with only cache enabled.
    pub fn cache() -> Self {
        Self::new(AudioCacheMode::Cache)
    }
}
