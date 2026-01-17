#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Audio loading and cache and playback.
//!
//! # Services
//!
//! Services this extension provides.
//!
//! * [`AUDIOS`]
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{mem, path::PathBuf, pin::Pin};

use zng_app::{
    update::UPDATES,
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewAudioHandle,
        raw_events::{RAW_AUDIO_DECODE_ERROR_EVENT, RAW_AUDIO_DECODED_EVENT, RAW_AUDIO_METADATA_DECODED_EVENT},
    },
};
use zng_app_context::app_local;
use zng_clone_move::clmv;
use zng_task::channel::{IpcBytes, IpcBytesCast};
use zng_txt::ToTxt;
use zng_txt::Txt;
use zng_unique_id::{IdEntry, IdMap};
use zng_unit::ByteLength;
use zng_var::{Var, VarHandle, const_var, var};
use zng_view_api::audio::{AudioDecoded, AudioId, AudioMetadata, AudioRequest};

mod types;
pub use types::*;

app_local! {
    static AUDIOS_SV: AudiosService = AudiosService::new();
}

struct AudiosService {
    load_in_headless: Var<bool>,
    limits: Var<AudioLimits>,

    extensions: Vec<Box<dyn AudiosExtension>>,

    cache: IdMap<AudioHash, AudioVar>,
}
impl AudiosService {
    pub fn new() -> Self {
        Self {
            load_in_headless: var(false),
            limits: var(AudioLimits::default()),

            extensions: vec![],

            cache: IdMap::new(),
        }
    }
}

/// Audio loading and cache service.
///
/// If the app is running without a [`VIEW_PROCESS`] all audios are dummy, see [`load_in_headless`] for
/// details.
///
/// [`load_in_headless`]: AUDIOS::load_in_headless
/// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
pub struct AUDIOS;
impl AUDIOS {
    /// If should still download/read audio bytes in headless/renderless mode.
    ///
    /// When an app is in headless mode without renderer no [`VIEW_PROCESS`] is available, so
    /// audio cannot be decoded, in this case all audio are dummy loading and no attempt
    /// to download/read the audio files is made. You can enable loading in headless tests to detect
    /// IO errors, in this case if there is an error acquiring the audio file the audio will be a
    /// [`AudioTrack::error`].
    ///
    /// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
    pub fn load_in_headless(&self) -> Var<bool> {
        AUDIOS_SV.read().load_in_headless.clone()
    }

    /// Default loading and decoding limits for each audio.
    pub fn limits(&self) -> Var<AudioLimits> {
        AUDIOS_SV.read().limits.clone()
    }

    /// Request an audio, reads from a `path` and caches it.
    ///
    /// This is shorthand for calling [`AUDIOS.audio`] with [`AudioSource::Read`] and [`AudioOptions::cache`].
    ///
    /// [`AUDIOS.audio`]: AUDIOS::audio
    pub fn read(&self, path: impl Into<PathBuf>) -> AudioVar {
        self.audio_impl(path.into().into(), AudioOptions::cache(), None)
    }

    /// Request an audio, downloads from an `uri` and caches it.
    ///
    /// Optionally define the HTTP ACCEPT header, if not set all audio formats supported by the view-process
    /// backend are accepted.
    ///
    /// This is shorthand for calling [`AUDIOS.audio`] with [`AudioSource::Download`] and [`AudioOptions::cache`].
    ///
    /// [`AUDIOS.audio`]: AUDIOS::audio
    #[cfg(feature = "http")]
    pub fn download<U>(&self, uri: U, accept: Option<Txt>) -> AudioVar
    where
        U: TryInto<zng_task::http::Uri>,
        <U as TryInto<zng_task::http::Uri>>::Error: ToTxt,
    {
        match uri.try_into() {
            Ok(uri) => self.audio_impl(AudioSource::Download(uri, accept), AudioOptions::cache(), None),
            Err(e) => const_var(AudioTrack::new_error(e.to_txt())),
        }
    }

    /// Request an audio from `&'static [u8]` data.
    ///
    /// The data can be any of the formats described in [`AudioDataFormat`].
    ///
    /// This is shorthand for calling [`AUDIOS.audio`] with [`AudioSource::Data`] and [`AudioOptions::cache`].
    ///
    /// # Examples
    ///
    /// Get an audio from a PNG file embedded in the app executable using [`include_bytes!`].
    ///
    /// ```
    /// # use zng_ext_image::*;
    /// # macro_rules! include_bytes { ($tt:tt) => { &[] } }
    /// # fn demo() {
    /// let audio_var = AUDIOS.from_static(include_bytes!("ping.wav"), "wav");
    /// # }
    /// ```
    ///
    /// [`AUDIOS.audio`]: AUDIOS::audio
    pub fn from_static(&self, data: &'static [u8], format: impl Into<AudioDataFormat>) -> AudioVar {
        self.audio_impl((data, format.into()).into(), AudioOptions::cache(), None)
    }

    /// Get a cached audio from shared data.
    ///
    /// The data can be any of the formats described in [`AudioDataFormat`].
    ///
    /// This is shorthand for calling [`AUDIOS.audio`] with [`AudioSource::Data`] and [`AudioOptions::cache`].
    ///
    /// [`AUDIOS.audio`]: AUDIOS::audio
    pub fn from_data(&self, data: IpcBytes, format: impl Into<AudioDataFormat>) -> AudioVar {
        self.audio_impl((data, format.into()).into(), AudioOptions::cache(), None)
    }

    /// Request an audio, with full load and cache configuration.
    ///
    /// If `limits` is `None` the [`AUDIOS.limits`] is used.
    ///
    /// Always returns a *loading* audio due to the deferred nature of services. If the audio is already in cache
    /// it will be set and bound to it once the current update finishes.
    ///
    /// [`AUDIOS.limits`]: AUDIOS::limits
    pub fn audio(&self, source: impl Into<AudioSource>, options: AudioOptions, limits: Option<AudioLimits>) -> AudioVar {
        self.audio_impl(source.into(), options, limits)
    }
    fn audio_impl(&self, source: AudioSource, options: AudioOptions, limits: Option<AudioLimits>) -> AudioVar {
        let r = var(AudioTrack::new_loading());
        let ri = r.read_only();
        UPDATES.once_update("AUDIOS.audio", move || {
            audio(source, options, limits, r);
        });
        ri
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
        let r = var(AudioTrack::new_loading());
        let ri = r.read_only();
        zng_task::spawn(async move {
            let source = source.await;
            audio(source, options, limits, r);
        });
        ri
    }

    /// Associate the `audio` produced by direct interaction with the view-process with the `key` in the cache.
    ///
    /// Returns an audio var that tracks the audio, note that if the `key` is already known does not use the `audio` data.
    ///
    /// Note that you can register tracks in [`AudioTrack::insert_track`], this method is only for tracking a new entry.
    ///
    /// Note that the audio will not automatically restore on respawn if the view-process fails while decoding.
    pub fn register(&self, key: Option<AudioHash>, audio: (ViewAudioHandle, AudioMetadata, AudioDecoded)) -> AudioVar {
        let r = var(AudioTrack::new_loading());
        let rr = r.read_only();
        UPDATES.once_update("AUDIOS.register", move || {
            audio_view(key, audio.0, audio.1, audio.2, None, r);
        });
        rr
    }

    /// Remove the audio from the cache, if it is only held by the cache.
    ///
    /// You can use [`AudioSource::hash128_read`] and [`AudioSource::hash128_download`] to get the `key`
    /// for files or downloads.
    pub fn clean(&self, key: AudioHash) {
        UPDATES.once_update("AUDIOS.clean", move || {
            if let IdEntry::Occupied(e) = AUDIOS_SV.write().cache.entry(key)
                && e.get().strong_count() == 1
            {
                e.remove();
            }
        });
    }

    /// Remove the audio from the cache, even if it is still referenced outside of the cache.
    ///
    /// You can use [`AudioSource::hash128_read`] and [`AudioSource::hash128_download`] to get the `key`
    /// for files or downloads.
    pub fn purge(&self, key: AudioHash) {
        UPDATES.once_update("AUDIOS.purge", move || {
            AUDIOS_SV.write().cache.remove(&key);
        });
    }

    /// Gets the cache key of an audio.
    pub fn cache_key(&self, audio: &AudioTrack) -> Option<AudioHash> {
        let key = audio.cache_key?;
        if AUDIOS_SV.read().cache.contains_key(&key) {
            Some(key)
        } else {
            None
        }
    }

    /// If the audio is cached.
    pub fn is_cached(&self, audio: &AudioTrack) -> bool {
        match &audio.cache_key {
            Some(k) => AUDIOS_SV.read().cache.contains_key(k),
            None => false,
        }
    }

    /// Clear cached audio that are not referenced outside of the cache.
    pub fn clean_all(&self) {
        UPDATES.once_update("AUDIOS.clean_all", || {
            AUDIOS_SV.write().cache.retain(|_, v| v.strong_count() > 1);
        });
    }

    /// Clear all cached audio, including audio that are still referenced outside of the cache.
    ///
    /// Audio memory only drops when all strong references are removed, so if an audio is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_all(&self) {
        UPDATES.once_update("AUDIOS.purge_all", || {
            AUDIOS_SV.write().cache.clear();
        });
    }

    /// Add an audio service extension.
    ///
    /// See [`AudiosExtension`] for extension capabilities.
    pub fn extend(&self, extension: Box<dyn AudiosExtension>) {
        UPDATES.once_update("AUDIOS.extend", move || {
            AUDIOS_SV.write().extensions.push(extension);
        });
    }

    /// Audio formats implemented by the current view-process and extensions.
    pub fn available_formats(&self) -> Vec<AudioFormat> {
        let mut formats = VIEW_PROCESS.info().audio.clone();

        let mut exts = mem::take(&mut AUDIOS_SV.write().extensions);
        for ext in exts.iter_mut() {
            ext.available_formats(&mut formats);
        }
        let mut s = AUDIOS_SV.write();
        exts.append(&mut s.extensions);
        s.extensions = exts;

        formats
    }

    fn http_accept(&self) -> Txt {
        let mut s = String::new();
        let mut sep = "";
        for f in self.available_formats() {
            for f in f.media_type_suffixes_iter() {
                s.push_str(sep);
                s.push_str("audio/");
                s.push_str(f);
                sep = ",";
            }
        }
        s.into()
    }
}

fn audio(mut source: AudioSource, mut options: AudioOptions, limits: Option<AudioLimits>, r: Var<AudioTrack>) {
    let mut s = AUDIOS_SV.write();

    let limits = limits.unwrap_or_else(|| s.limits.get());

    // apply extensions
    let mut exts = mem::take(&mut s.extensions);
    drop(s); // drop because extensions may use the service
    for ext in &mut exts {
        ext.audio(&limits, &mut source, &mut options);
    }
    let mut s = AUDIOS_SV.write();
    exts.append(&mut s.extensions);

    if let AudioSource::Audio(var) = source {
        // Audio is passthrough, cache config is ignored
        var.set_bind(&r).perm();
        r.hold(var).perm();
        return;
    }

    if !VIEW_PROCESS.is_available() && !s.load_in_headless.get() {
        tracing::debug!("ignoring audio request due headless mode");
        return;
    }

    let key = source.hash128(&options).unwrap();

    // setup cache and drop service lock
    match options.cache_mode {
        AudioCacheMode::Ignore => (),
        AudioCacheMode::Cache => {
            match s.cache.entry(key) {
                IdEntry::Occupied(e) => {
                    // already cached
                    let var = e.get();
                    var.set_bind(&r).perm();
                    r.hold(var.clone()).perm();
                    return;
                }
                IdEntry::Vacant(e) => {
                    // cache
                    e.insert(r.clone());
                }
            }
        }
        AudioCacheMode::Retry => {
            match s.cache.entry(key) {
                IdEntry::Occupied(mut e) => {
                    let var = e.get();
                    if var.with(AudioTrack::is_error) {
                        // already cached with error

                        // bind old entry to new, in case there are listeners to it,
                        // can't use `strong_count` to optimize here because it might have weak refs out there
                        r.set_bind(var).perm();
                        var.hold(r.clone()).perm();

                        // new var `r` becomes the entry
                        e.insert(r.clone());
                    } else {
                        // already cached ok
                        var.set_bind(&r).perm();
                        r.hold(var.clone()).perm();
                        return;
                    }
                }
                IdEntry::Vacant(e) => {
                    // cache
                    e.insert(r.clone());
                }
            }
        }
        AudioCacheMode::Reload => {
            match s.cache.entry(key) {
                IdEntry::Occupied(mut e) => {
                    let var = e.get();
                    r.set_bind(var).perm();
                    var.hold(r.clone()).perm();

                    e.insert(r.clone());
                }
                IdEntry::Vacant(e) => {
                    // cache
                    e.insert(r.clone());
                }
            }
        }
    }
    drop(s);

    match source {
        AudioSource::Read(path) => {
            fn read(path: PathBuf, limit: ByteLength) -> std::io::Result<IpcBytes> {
                let file = std::fs::File::open(path)?;
                if file.metadata()?.len() > limit.bytes() as u64 {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "file length exceeds limit"));
                }
                IpcBytes::from_file_blocking(file)
            }
            let limit = limits.max_encoded_len;
            let data_format = match path.extension() {
                Some(ext) => AudioDataFormat::FileExtension(ext.to_string_lossy().to_txt()),
                None => AudioDataFormat::Unknown,
            };
            zng_task::spawn_wait(move || match read(path, limit) {
                Ok(data) => audio_data(false, Some(key), data_format, data, options, limits, r),
                Err(e) => {
                    r.set(AudioTrack::new_error(e.to_txt()));
                }
            });
        }
        #[cfg(feature = "http")]
        AudioSource::Download(uri, accept) => {
            let accept = accept.unwrap_or_else(|| AUDIOS.http_accept());

            use zng_task::http::*;
            async fn download(uri: Uri, accept: Txt, limit: ByteLength) -> Result<(AudioDataFormat, IpcBytes), Error> {
                let request = Request::get(uri)?.max_length(limit).header(header::ACCEPT, accept.as_str())?;
                let mut response = send(request).await?;
                let data_format = match response.header().get(&header::CONTENT_TYPE).and_then(|m| m.to_str().ok()) {
                    Some(m) => AudioDataFormat::MimeType(m.to_txt()),
                    None => AudioDataFormat::Unknown,
                };
                let data = response.body().await?;

                Ok((data_format, data))
            }

            let limit = limits.max_encoded_len;
            zng_task::spawn(async move {
                match download(uri, accept, limit).await {
                    Ok((fmt, data)) => {
                        audio_data(false, Some(key), fmt, data, options, limits, r);
                    }
                    Err(e) => r.set(AudioTrack::new_error(e.to_txt())),
                }
            });
        }
        AudioSource::Data(_, data, format) => audio_data(false, Some(key), format, data, options, limits, r),
        _ => unreachable!(),
    }
}

// source data acquired, setup view-process handle
fn audio_data(
    is_respawn: bool,
    cache_key: Option<AudioHash>,
    format: AudioDataFormat,
    data: IpcBytes,
    options: AudioOptions,
    limits: AudioLimits,
    r: Var<AudioTrack>,
) {
    if !is_respawn && let Some(key) = cache_key {
        let mut replaced = false;
        let mut exts = mem::take(&mut AUDIOS_SV.write().extensions);
        for ext in &mut exts {
            if let Some(replacement) = ext.audio_data(limits.max_decoded_len, &key, &data, &format, &options) {
                replacement.set_bind(&r).perm();
                r.hold(replacement).perm();

                replaced = true;
                break;
            }
        }

        {
            let mut s = AUDIOS_SV.write();
            exts.append(&mut s.extensions);
            s.extensions = exts;

            if replaced {
                return;
            }
        }
    }

    if !VIEW_PROCESS.is_available() {
        tracing::debug!("ignoring audio view request after test load due to headless mode");
        return;
    }

    let mut request = AudioRequest::new(format.clone(), data.clone(), limits.max_decoded_len.bytes() as u64);
    request.tracks = options.tracks;

    let try_gen = VIEW_PROCESS.generation();

    match VIEW_PROCESS.add_audio(request) {
        Ok(view_audio) => audio_view(
            cache_key,
            view_audio,
            AudioMetadata::new(AudioId::INVALID, 0, 0),
            AudioDecoded::new(AudioId::INVALID, IpcBytesCast::default()),
            Some((format, data, options, limits)),
            r,
        ),
        Err(_) => {
            tracing::debug!("audio view request failed, will retry on respawn");

            zng_task::spawn(async move {
                VIEW_PROCESS_INITED_EVENT.wait_match(move |a| a.generation != try_gen).await;
                audio_data(true, cache_key, format, data, options, limits, r);
            });
        }
    }
}
// monitor view-process handle until it is loaded
fn audio_view(
    cache_key: Option<AudioHash>,
    handle: ViewAudioHandle,
    meta: AudioMetadata,
    decoded: AudioDecoded,
    respawn_data: Option<(AudioDataFormat, IpcBytes, AudioOptions, AudioLimits)>,
    r: Var<AudioTrack>,
) {
    let a = AudioTrack::new(cache_key, handle, meta, decoded);
    let is_loaded = a.is_loaded();
    let is_dummy = a.view_handle().is_dummy();
    r.set(a);

    if is_loaded {
        audio_decoded(r);
        return;
    }

    if is_dummy {
        tracing::error!("tried to register dummy handle");
        return;
    }

    // handle respawn during audio decode
    let decoding_respawn_handle = if respawn_data.is_some() {
        let r_weak = r.downgrade();
        let mut respawn_data = respawn_data;
        VIEW_PROCESS_INITED_EVENT.hook(move |_| {
            if let Some(r) = r_weak.upgrade() {
                let (format, data, options, limits) = respawn_data.take().unwrap();
                audio_data(true, cache_key, format, data, options, limits, r);
            }
            false
        })
    } else {
        // audio registered (without source info), respawn is the responsibility of the caller
        VarHandle::dummy()
    };

    // handle decode error
    let r_weak = r.downgrade();
    let decode_error_handle = RAW_AUDIO_DECODE_ERROR_EVENT.hook(move |args| match r_weak.upgrade() {
        Some(r) => {
            if r.with(|a| a.view_handle() == args.handle) {
                r.set(AudioTrack::new_error(args.error.clone()));
                false
            } else {
                r.with(AudioTrack::is_loading)
            }
        }
        None => false,
    });

    // handle metadata decoded
    let r_weak = r.downgrade();
    let decode_meta_handle = RAW_AUDIO_METADATA_DECODED_EVENT.hook(move |args| match r_weak.upgrade() {
        Some(r) => {
            if r.with(|a| a.view_handle() == args.handle) {
                let meta = args.meta.clone();
                r.modify(move |i| i.meta = meta);
            } else if let Some(p) = &args.meta.parent
                && p.parent == r.with(|a| a.view_handle().audio_id())
            {
                // discovered a track for this audio, start tracking it

                let data = AudioDecoded::new(AudioId::INVALID, IpcBytesCast::default());
                let track = var(AudioTrack::new(None, args.handle.clone(), args.meta.clone(), data.clone()));
                r.modify(clmv!(track, |i| i.insert_track(track)));
                audio_view(None, args.handle.clone(), args.meta.clone(), data, None, track);
            }
            r.with(AudioTrack::is_loading)
        }
        None => false,
    });

    // handle pixels decoded
    let r_weak = r.downgrade();
    RAW_AUDIO_DECODED_EVENT
        .hook(move |args| {
            let _hold = [&decoding_respawn_handle, &decode_error_handle, &decode_meta_handle];
            match r_weak.upgrade() {
                Some(r) => {
                    if r.with(|a| a.view_handle() == args.handle) {
                        let data = args.audio.clone();
                        let is_loading = !data.is_full;
                        r.modify(move |i| i.data = data);
                        if !is_loading {
                            audio_decoded(r);
                        }
                        is_loading
                    } else {
                        r.with(AudioTrack::is_loading)
                    }
                }
                None => false,
            }
        })
        .perm();
}
// audio decoded ok, setup respawn handle
fn audio_decoded(r: Var<AudioTrack>) {
    let r_weak = r.downgrade();
    VIEW_PROCESS_INITED_EVENT
        .hook(move |_| {
            if let Some(r) = r_weak.upgrade() {
                let a = r.get();
                if !a.is_loaded() {
                    // audio rebound, maybe due to cache refresh
                    return false;
                }

                // respawn the audio as decoded data
                let format = AudioDataFormat::InterleavedF32 {
                    channel_count: a.meta.channel_count,
                    sample_rate: a.meta.sample_rate,
                    total_duration: a.meta.total_duration,
                };
                audio_data(
                    true,
                    a.cache_key,
                    format,
                    a.data.chunk.into(),
                    AudioOptions::none(),
                    AudioLimits::none(),
                    r,
                );
            }
            false
        })
        .perm();
}
