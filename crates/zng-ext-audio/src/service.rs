use std::{
    env, mem,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{AudioOutput, AudioOutputData, AudioOutputId, types::*};
use parking_lot::Mutex;
use task::io::AsyncReadExt;
use zng_app::{
    APP,
    update::UPDATES,
    view_process::{
        VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT, ViewAudioHandle, ViewAudioOutput,
        raw_events::{
            LOW_MEMORY_EVENT, RAW_AUDIO_DECODE_ERROR_EVENT, RAW_AUDIO_DECODED_EVENT, RAW_AUDIO_METADATA_DECODED_EVENT,
            RAW_AUDIO_OUTPUT_OPEN_ERROR_EVENT, RAW_AUDIO_OUTPUT_OPEN_EVENT,
        },
    },
    widget::UiTaskWidget,
};
use zng_app_context::app_local;
use zng_task::{self as task, channel::IpcBytesCast};
use zng_task::{UiTask, channel::IpcBytes};
use zng_txt::{ToTxt, Txt, formatx};
use zng_unique_id::{IdEntry, IdMap};
use zng_unit::{ByteLength, ByteUnits as _, FactorUnits as _};
use zng_var::{ResponderVar, ResponseVar, Var, WeakVar, const_var, response_done_var, response_var, var};
use zng_view_api::audio::{
    AudioDecoded, AudioId, AudioMetadata, AudioMix, AudioOutputConfig, AudioOutputId as ViewAudioOutputId, AudioOutputRequest,
    AudioOutputState, AudioRequest, AudioTrackMetadata,
};

app_local! {
    static AUDIOS_SV: AudiosService = {
        APP.extensions().require::<crate::AudioManager>();
        AudiosService::new()
    };
}

struct AudioData {
    format: AudioDataFormat,
    r: std::result::Result<IpcBytes, Txt>,
}
struct AudioLoadingTask {
    task: Mutex<UiTask<AudioData>>,
    audio: Var<AudioTrack>,
    max_decoded_len: ByteLength,
    key: AudioHash,
    options: AudioOptions,
}
struct AudioDecodingTask {
    format: AudioDataFormat,
    data: IpcBytes,
    audio: Var<AudioTrack>,
}
struct CacheEntry {
    audio: Var<AudioTrack>,
    error: AtomicBool,
    max_decoded_len: ByteLength,
    tracks: AudioTracksMode,
}
struct NotCachedEntry {
    audio: WeakVar<AudioTrack>,
    max_decoded_len: ByteLength,
    tracks: AudioTracksMode,
}
struct OutputRequest {
    r: ResponderVar<AudioOutput>,
    cache: bool,
}
struct CachedOutputEntry {
    output: AudioOutput,
}
struct NotCachedOutputEntry {
    output: std::sync::Weak<AudioOutputData>,
}

struct AudiosService {
    load_in_headless: Var<bool>,
    limits: Var<AudioLimits>,

    download_accept: Txt,
    extensions: Vec<Box<dyn AudiosExtension>>,

    loading: Vec<AudioLoadingTask>,
    decoding: Vec<AudioDecodingTask>,
    cache: IdMap<AudioHash, CacheEntry>,
    not_cached: Vec<NotCachedEntry>,

    output_requests: Vec<(AudioOutputId, OutputRequest)>,
    output_opening: Vec<(AudioOutputId, OutputRequest)>,
    outputs: IdMap<AudioOutputId, CachedOutputEntry>,
    not_cached_outputs: IdMap<AudioOutputId, NotCachedOutputEntry>,
    cue: Vec<(ViewAudioOutput, AudioMix)>,
}

pub(crate) fn load_in_headless() -> Var<bool> {
    AUDIOS_SV.read().load_in_headless.clone()
}

pub(crate) fn limits() -> Var<AudioLimits> {
    AUDIOS_SV.read().limits.clone()
}

pub(crate) fn on_app_event_preview(update: &mut EventUpdate) {
    let mut audio = None;
    // handle any ok decode
    if let Some(args) = RAW_AUDIO_METADATA_DECODED_EVENT.on(update) {
        audio = Some((args.handle.clone(), Some(args.meta.clone()), None));
    } else if let Some(args) = RAW_AUDIO_DECODED_EVENT.on(update) {
        audio = Some((args.handle.clone(), None, Some(args.audio.clone())));
    }

    if let Some((handle, meta, data)) = audio {
        let audios = AUDIOS_SV.read();

        if let Some(var) = audios.find_decoding(handle.audio_id()) {
            // audio is registered already, or is track of registered
            if let Some(data) = data {
                var.modify(move |i| i.set_data(data));
            } else if let Some(meta) = meta {
                var.modify(move |i| i.set_meta(meta));
            }
        } else if let Some(meta) = meta
            && let Some(p) = &meta.parent
            && let Some(var) = audios.find_decoding(p.parent)
        {
            // audio is not registered, but is track of audio that is, insert it
            let data = data.unwrap_or_else(|| AudioDecoded::new(meta.id, IpcBytesCast::default()));
            let track = AudioTrack::new(handle, meta, data);
            var.modify(move |i| {
                i.insert_track(track);
            });
        }
    } else if let Some(args) = RAW_AUDIO_DECODE_ERROR_EVENT.on(update) {
        let audios = AUDIOS_SV.read();

        if let Some(var) = audios.find_decoding(args.handle.audio_id()) {
            // audio registered, update to error.

            let error = args.error.clone();
            var.modify(move |i| i.set_error(error));

            if let Some(key) = var.with(|i| i.cache_key)
                && let Some(track) = audios.cache.get(&key)
            {
                track.error.store(true, Ordering::Relaxed);
            }
            // else is loading will flag on the pos update pass
        }
    } else if let Some(args) = RAW_AUDIO_OUTPUT_OPEN_EVENT.on(update) {
        let mut audios = AUDIOS_SV.write();
        if let Some(i) = audios.output_opening.iter().position(|(id, _)| *id == args.output_id) {
            let (id, r) = audios.output_opening.swap_remove(i);
            let output = AudioOutput::new(id, Some(args.output.clone()));
            if r.cache {
                audios.outputs.insert(id, CachedOutputEntry { output: output.clone() });
            } else {
                audios.not_cached_outputs.insert(
                    id,
                    NotCachedOutputEntry {
                        output: Arc::downgrade(&output.0),
                    },
                );
            }
            r.r.respond(output);
        }
    } else if let Some(args) = RAW_AUDIO_OUTPUT_OPEN_ERROR_EVENT.on(update) {
        let audios = AUDIOS_SV.write();
        if let Some(_i) = audios.output_opening.iter().position(|(id, _)| *id == args.output_id) {
            tracing::error!("error opening audio output, {}", args.error);
            // it will respawn? what does windows service
        }
    } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
        let mut audios = AUDIOS_SV.write();
        let audios = &mut *audios;
        audios.cleanup_not_cached(true);
        audios.download_accept.clear();

        let mut decoding_interrupted = mem::take(&mut audios.decoding);
        for (audio_var, max_decoded_len, tracks) in audios.cache.values().map(|e| (e.audio.clone(), e.max_decoded_len, e.tracks)).chain(
            audios
                .not_cached
                .iter()
                .filter_map(|e| e.audio.upgrade().map(|v| (v, e.max_decoded_len, e.tracks))),
        ) {
            let audio = audio_var.get();
            let old_handle = audio.view_handle();

            if !old_handle.is_dummy() {
                if old_handle.view_process_gen() == args.generation {
                    continue; // already recovered, can this happen?
                }
                if let Some(e) = audio.error() {
                    // respawned, but audio was an error.
                    audio_var.set(AudioTrack::new_empty(e));
                } else if let Some(task_i) = decoding_interrupted
                    .iter()
                    .position(|e| e.audio.with(|audio| audio.view_handle() == old_handle))
                {
                    let task = decoding_interrupted.swap_remove(task_i);
                    // respawned, but audio was decoding, need to restart decode.
                    let mut request = AudioRequest::new(task.format.clone(), task.data.clone(), max_decoded_len.0 as u64);
                    request.tracks = tracks;
                    match VIEW_PROCESS.add_audio(request) {
                        Ok(audio) => {
                            audio_var.set(AudioTrack::new_loading(audio));
                        }
                        Err(_) => { /*will receive another event.*/ }
                    }
                    audios.decoding.push(AudioDecodingTask {
                        format: task.format.clone(),
                        data: task.data.clone(),
                        audio: audio_var,
                    });
                } else {
                    // respawned and audio was loaded.

                    let audio_format = AudioDataFormat::InterleavedF32 {
                        channel_count: audio.channel_count(),
                        sample_rate: audio.sample_rate(),
                        total_duration: audio.total_duration(),
                    };

                    let tracks = audio.tracks();

                    let data = audio.chunk().unwrap().into_inner();
                    let request = AudioRequest::new(audio_format.clone(), data.clone(), max_decoded_len.0 as u64);
                    let audio = match VIEW_PROCESS.add_audio(request) {
                        Ok(audio) => audio,
                        Err(_) => return, // we will receive another event.
                    };
                    let mut audio = AudioTrack::new_loading(audio);

                    fn add_tracks(max_decoded_len: ByteLength, tracks: Vec<AudioVar>, audio: &mut AudioTrack) {
                        for (i, track) in tracks.into_iter().enumerate() {
                            let track = track.get();
                            let track_handle = track.view_handle();
                            if !track_handle.is_dummy() {
                                if track.is_loaded() {
                                    let mut request = AudioRequest::new(
                                        AudioDataFormat::InterleavedF32 {
                                            channel_count: track.channel_count(),
                                            sample_rate: track.sample_rate(),
                                            total_duration: track.total_duration(),
                                        },
                                        track.chunk().unwrap().into_inner(),
                                        max_decoded_len.0 as u64,
                                    );
                                    request.parent = Some(AudioTrackMetadata::new(audio.view_handle().audio_id(), i));
                                    let track_audio = match VIEW_PROCESS.add_audio(request) {
                                        Ok(audio) => audio,
                                        Err(_) => return, // we will receive another event.
                                    };
                                    let track_audio = audio.insert_track(AudioTrack::new_loading(track_audio));

                                    add_tracks(max_decoded_len, track.tracks(), &mut track_audio.get());
                                    continue;
                                } else if track.is_error() {
                                    audio.insert_track(track);
                                    continue;
                                }
                            }
                            tracing::warn!("respawn not implemented for multi track audio partially decoded on crash");
                        }
                    }
                    add_tracks(max_decoded_len, tracks, &mut audio);

                    audio_var.set(audio);

                    audios.decoding.push(AudioDecodingTask {
                        format: audio_format,
                        data,
                        audio: audio_var,
                    });
                }
            } else if let Some(task_i) = decoding_interrupted.iter().position(|e| e.audio.var_eq(&audio_var)) {
                // respawned, but audio had not started decoding, start it now.
                let task = decoding_interrupted.swap_remove(task_i);
                let mut request = AudioRequest::new(task.format.clone(), task.data.clone(), max_decoded_len.0 as u64);
                request.tracks = tracks;
                match VIEW_PROCESS.add_audio(request) {
                    Ok(audio) => {
                        audio_var.set(AudioTrack::new_loading(audio));
                    }
                    Err(_) => { /*will receive another event.*/ }
                }
                audios.decoding.push(AudioDecodingTask {
                    format: task.format.clone(),
                    data: task.data.clone(),
                    audio: audio_var,
                });
            }
            // else { *is loading, will continue normally in self.update_preview()* }
        }
    } else if LOW_MEMORY_EVENT.on(update).is_some() {
        clean_all();
    }
}

pub(crate) fn on_app_update_preview() {
    // update loading tasks:

    let mut audios = AUDIOS_SV.write();
    let mut loading = Vec::with_capacity(audios.loading.len());
    let loading_tasks = mem::take(&mut audios.loading);
    let mut extensions = mem::take(&mut audios.extensions);
    drop(audios); // extensions can use AUDIOS

    'loading_tasks: for mut t in loading_tasks {
        t.task.get_mut().update();
        match t.task.into_inner().into_result() {
            Ok(d) => {
                match d.r {
                    Ok(data) => {
                        for ext in &mut extensions {
                            if let Some(audio) = ext.audio_data(t.max_decoded_len, &t.key, &data, &d.format, &t.options) {
                                audio.set_bind(&t.audio).perm();
                                t.audio.hold(audio).perm();
                                continue 'loading_tasks;
                            }
                        }

                        if VIEW_PROCESS.is_available() {
                            // success and we have a view-process.
                            let mut request = AudioRequest::new(d.format.clone(), data.clone(), t.max_decoded_len.0 as u64);
                            request.tracks = t.options.tracks;
                            match VIEW_PROCESS.add_audio(request) {
                                Ok(audio) => {
                                    // request sent, add to `decoding` will receive
                                    // audio decoded events
                                    t.audio.modify(move |v| {
                                        v.set_handle(audio);
                                    });
                                }
                                Err(_) => {
                                    // will recover in VIEW_PROCESS_INITED_EVENT
                                }
                            }
                            AUDIOS_SV.write().decoding.push(AudioDecodingTask {
                                format: d.format,
                                data,
                                audio: t.audio,
                            });
                        } else {
                            // success, but we are only doing `load_in_headless` validation.
                            t.audio.modify(move |v| {
                                v.set_meta(AudioMetadata::new(AudioId::INVALID, 0, 0));
                                v.set_data(AudioDecoded::new(AudioId::INVALID, IpcBytesCast::default()));
                            });
                        }
                    }
                    Err(e) => {
                        tracing::error!("load error: {e:?}");
                        // load error.
                        t.audio.modify(move |v| {
                            v.set_error(e);
                        });

                        // flag error for user retry
                        if let Some(k) = &t.audio.with(|audio| audio.cache_key)
                            && let Some(e) = AUDIOS_SV.read().cache.get(k)
                        {
                            e.error.store(true, Ordering::Relaxed);
                        }
                    }
                }
            }
            Err(task) => {
                loading.push(AudioLoadingTask {
                    task: Mutex::new(task),
                    audio: t.audio,
                    max_decoded_len: t.max_decoded_len,
                    key: t.key,
                    options: t.options,
                });
            }
        }
    }
    let mut audios = AUDIOS_SV.write();
    audios.loading = loading;
    audios.extensions = extensions;
}

pub(crate) fn on_app_update() {
    let mut audios = AUDIOS_SV.write();
    let audios = &mut *audios;

    audios.decoding.retain(|t| {
        t.audio.with(|i| {
            let retain = i.is_loading() || i.has_loading_tracks();

            if !retain
                && i.is_error()
                && let Some(key) = i.cache_key
                && let Some(track) = audios.cache.get_mut(&key)
            {
                *track.error.get_mut() = true;
            }

            retain
        })
    });

    let mut respawn_requests = vec![];
    for (id, r) in audios.output_requests.drain(..) {
        let cfg = AudioOutputConfig::new(AudioOutputState::Playing, 1.fct(), 1.fct());
        if VIEW_PROCESS
            .open_audio_output(AudioOutputRequest::new(ViewAudioOutputId::from_raw(id.get()), cfg))
            .is_ok()
        {
            audios.output_opening.push((id, r));
        } else {
            // will reopen on respawn
            respawn_requests.push((id, r));
        }
    }
    audios.output_requests.extend(respawn_requests);

    for o in audios.outputs.values() {
        o.output.update();
    }
    audios.not_cached_outputs.retain(|_, o| match o.output.upgrade() {
        Some(o) => {
            AudioOutput(o).update();
            true
        }
        None => false,
    });

    for (view, mix) in audios.cue.drain(..) {
        let _ = view.cue(mix);
    }
}

/// Associate the `handle` with the `key` or return it with the already existing audio if the `key` already has an track.
#[allow(clippy::result_large_err)]
pub(crate) fn register(
    key: Option<AudioHash>,
    audio: (ViewAudioHandle, AudioMetadata, AudioDecoded),
    error: Txt,
) -> std::result::Result<AudioVar, ((ViewAudioHandle, AudioMetadata, AudioDecoded), AudioVar)> {
    let mut s = AUDIOS_SV.write();
    let s = &mut *s;

    let limits = s.limits.get();
    let limits = AudioLimits {
        max_encoded_len: limits.max_encoded_len,
        max_decoded_len: limits.max_decoded_len.max(audio.2.chunk.as_bytes().len().bytes()),
        allow_path: PathFilter::BlockAll,
        #[cfg(feature = "http")]
        allow_uri: UriFilter::BlockAll,
    };

    let (handle, audio_meta, audio) = audio;
    let is_error = !error.is_empty();
    let is_loading = !is_error && !audio.is_full;

    if let Some(key) = key {
        match s.cache.entry(key) {
            IdEntry::Occupied(e) => Err(((handle, audio_meta, audio), e.get().audio.read_only())),
            IdEntry::Vacant(e) => {
                let format = AudioDataFormat::InterleavedF32 {
                    channel_count: audio_meta.channel_count,
                    sample_rate: audio_meta.sample_rate,
                    total_duration: audio_meta.total_duration,
                };
                let audio_var = var(AudioTrack::new(handle, audio_meta, audio));
                if is_loading {
                    s.decoding.push(AudioDecodingTask {
                        format,
                        data: IpcBytes::default(),
                        audio: audio_var.clone(),
                    });
                }

                Ok(e.insert(CacheEntry {
                    error: AtomicBool::new(is_error),
                    audio: audio_var,
                    max_decoded_len: limits.max_decoded_len,
                    tracks: AudioTracksMode::PRIMARY,
                })
                .audio
                .read_only())
            }
        }
    } else if is_loading {
        let audio = var(AudioTrack::new(handle, audio_meta, audio));
        s.not_cached.push(NotCachedEntry {
            audio: audio.downgrade(),
            max_decoded_len: limits.max_decoded_len,
            tracks: AudioTracksMode::PRIMARY,
        });
        Ok(audio.read_only())
    } else {
        // not cached and already loaded
        Ok(const_var(AudioTrack::new(handle, audio_meta, audio)))
    }
}

pub(crate) fn detach(audio: AudioVar) -> AudioVar {
    if let Some(key) = &audio.with(|i| i.cache_key) {
        let mut s = AUDIOS_SV.write();

        let decoded_size = audio.with(|audio| audio.chunk().map(|b| b.as_bytes().len()).unwrap_or(0).bytes());
        let mut max_decoded_len = s.limits.with(|l| l.max_decoded_len.max(decoded_size));
        let mut tracks = AudioTracksMode::PRIMARY;

        if let Some(e) = s.cache.get(key) {
            max_decoded_len = e.max_decoded_len;
            tracks = e.tracks;

            // is cached, `clean` if is only external reference.
            if audio.strong_count() == 2 {
                s.cache.remove(key);
            }
        }

        // remove `cache_key` from audio, this clones the `Img` only-if is still in cache.
        let mut audio = audio.get();
        audio.cache_key = None;
        let audio = var(audio);
        s.not_cached.push(NotCachedEntry {
            audio: audio.downgrade(),
            max_decoded_len,
            tracks,
        });
        audio.read_only()
    } else {
        // already not cached
        audio
    }
}

pub(crate) fn remove(mut key: AudioHash, mut purge: bool) -> bool {
    let mut sv = AUDIOS_SV.write();
    let mut extensions = mem::take(&mut sv.extensions);
    if !extensions.is_empty() {
        drop(sv);
        for ext in &mut extensions {
            if !ext.remove(&mut key, &mut purge) {
                AUDIOS_SV.write().restore_extensions(extensions);
                return false;
            }
        }
        sv = AUDIOS_SV.write();
        sv.restore_extensions(extensions);
    }

    if purge || sv.cache.get(&key).map(|v| v.audio.strong_count() > 1).unwrap_or(false) {
        sv.cache.remove(&key).is_some()
    } else {
        false
    }
}

pub(crate) fn audio(mut source: AudioSource, mut options: AudioOptions, limits: Option<AudioLimits>) -> AudioVar {
    let mut sv = AUDIOS_SV.write();
    let limits = limits.unwrap_or_else(|| sv.limits.get());
    let mut extensions = mem::take(&mut sv.extensions);
    if !extensions.is_empty() {
        drop(sv);
        for ext in &mut extensions {
            ext.audio(&limits, &mut source, &mut options);
        }
        sv = AUDIOS_SV.write();
        sv.restore_extensions(extensions);
    }

    let source = match source {
        AudioSource::Read(path) => {
            // check limits
            let path = crate::absolute_path(&path, || env::current_dir().expect("could not access current dir"), true);
            if !limits.allow_path.allows(&path) {
                let error = formatx!("limits filter blocked `{}`", path.display());
                tracing::error!("{error}");
                return var(AudioTrack::new_empty(error)).read_only();
            }
            AudioSource::Read(path)
        }
        #[cfg(feature = "http")]
        // check limits
        AudioSource::Download(uri, accepts) => {
            if !limits.allow_uri.allows(&uri) {
                let error = formatx!("limits filter blocked `{uri}`");
                tracing::error!("{error}");
                return var(AudioTrack::new_empty(error)).read_only();
            }
            AudioSource::Download(uri, accepts)
        }
        // Audio is supposed to return directly
        AudioSource::Audio(r) => {
            return r;
        }
        source => source,
    };

    // continue to loading, ext.audio_data gain, decoding

    let key = source.hash128(&options).unwrap();

    match options.cache_mode {
        AudioCacheMode::Cache => {
            if let Some(v) = sv.cache.get(&key) {
                return v.audio.read_only();
            }
        }
        AudioCacheMode::Retry => {
            if let Some(e) = sv.cache.get(&key)
                && !e.error.load(Ordering::Relaxed)
            {
                return e.audio.read_only();
            }
        }
        AudioCacheMode::Ignore | AudioCacheMode::Reload => {}
    }

    if !VIEW_PROCESS.is_available() && !sv.load_in_headless.get() {
        tracing::warn!("loading dummy audio, set `load_in_headless=true` to actually load without renderer");

        let dummy = var(AudioTrack::new_empty(Txt::from_static("")));
        sv.cache.insert(
            key,
            CacheEntry {
                audio: dummy.clone(),
                error: AtomicBool::new(false),
                max_decoded_len: limits.max_decoded_len,
                tracks: options.tracks,
            },
        );
        return dummy.read_only();
    }

    let max_encoded_size = limits.max_encoded_len;

    match source {
        AudioSource::Read(path) => sv.load_task(
            key,
            limits.max_decoded_len,
            options,
            task::run(async move {
                let mut r = AudioData {
                    format: path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| AudioDataFormat::FileExtension(Txt::from_str(s)))
                        .unwrap_or(AudioDataFormat::Unknown),
                    r: Err(Txt::from_static("")),
                };

                let mut file = match task::fs::File::open(path).await {
                    Ok(f) => f,
                    Err(e) => {
                        r.r = Err(e.to_txt());
                        return r;
                    }
                };

                let len = match file.metadata().await {
                    Ok(m) => m.len() as usize,
                    Err(e) => {
                        r.r = Err(e.to_txt());
                        return r;
                    }
                };

                if len > max_encoded_size.0 {
                    r.r = Err(formatx!("file size `{}` exceeds the limit of `{max_encoded_size}`", len.bytes()));
                    return r;
                }

                let mut data = Vec::with_capacity(len);
                r.r = match file.read_to_end(&mut data).await {
                    Ok(_) => match IpcBytes::from_vec_blocking(data) {
                        Ok(r) => Ok(r),
                        Err(e) => Err(e.to_txt()),
                    },
                    Err(e) => Err(e.to_txt()),
                };

                r
            }),
        ),
        #[cfg(feature = "http")]
        AudioSource::Download(uri, accept) => {
            let accept = accept.unwrap_or_else(|| sv.download_accept());

            sv.load_task(
                key,
                limits.max_decoded_len,
                options,
                task::run(async move {
                    let mut r = AudioData {
                        format: AudioDataFormat::Unknown,
                        r: Err(Txt::from_static("")),
                    };

                    let request = task::http::Request::get(uri)
                        .unwrap()
                        .header(task::http::header::ACCEPT, accept.as_str())
                        .unwrap()
                        .max_length(max_encoded_size);

                    match task::http::send(request).await {
                        Ok(mut rsp) => {
                            if let Some(m) = rsp.header().get(&task::http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
                                let m = m.to_lowercase();
                                if m.starts_with("audio/") {
                                    r.format = AudioDataFormat::MimeType(Txt::from_str(&m));
                                }
                            }

                            r.r = rsp.body().await.map_err(|e| formatx!("download error: {e}"));
                        }
                        Err(e) => {
                            r.r = Err(formatx!("request error: {e}"));
                        }
                    }

                    r
                }),
            )
        }
        AudioSource::Data(_, bytes, fmt) => {
            let r = AudioData { format: fmt, r: Ok(bytes) };
            sv.load_task(key, limits.max_decoded_len, options, async { r })
        }
        AudioSource::Audio(_) => unreachable!(),
    }
}

pub(crate) fn available_formats() -> Vec<AudioFormat> {
    AUDIOS_SV.read().available_formats()
}

impl AudiosService {
    fn new() -> Self {
        Self {
            load_in_headless: var(false),
            limits: var(AudioLimits::default()),
            extensions: vec![],
            loading: vec![],
            decoding: vec![],
            download_accept: Txt::from_static(""),
            cache: IdMap::new(),
            not_cached: vec![],
            output_requests: vec![],
            output_opening: vec![],
            outputs: IdMap::new(),
            not_cached_outputs: IdMap::new(),
            cue: vec![],
        }
    }

    fn restore_extensions(&mut self, mut extensions: Vec<Box<dyn AudiosExtension>>) {
        extensions.append(&mut self.extensions);
        self.extensions = extensions;
    }

    #[cfg(feature = "http")]
    fn download_accept(&mut self) -> Txt {
        if self.download_accept.is_empty() {
            if VIEW_PROCESS.is_available() {
                let mut r = String::new();
                let mut sep = "";
                for fmt in self.available_formats() {
                    for t in fmt.media_type_suffixes_iter() {
                        r.push_str(sep);
                        r.push_str("audio/");
                        r.push_str(t);
                        sep = ",";
                    }
                }
            }
            if self.download_accept.is_empty() {
                self.download_accept = "audio/*".into();
            }
        }
        self.download_accept.clone()
    }

    fn available_formats(&self) -> Vec<AudioFormat> {
        let mut formats = VIEW_PROCESS.info().audio.clone();
        for ext in &self.extensions {
            ext.available_formats(&mut formats);
        }
        formats
    }

    fn cleanup_not_cached(&mut self, force: bool) {
        if force || self.not_cached.len() > 1000 {
            self.not_cached.retain(|c| c.audio.strong_count() > 0);
        }
    }

    fn new_cache_audio(&mut self, key: AudioHash, max_decoded_len: ByteLength, options: AudioOptions) -> Var<AudioTrack> {
        self.cleanup_not_cached(false);

        if let AudioCacheMode::Reload = options.cache_mode {
            self.cache
                .entry(key)
                .or_insert_with(|| CacheEntry {
                    audio: var(AudioTrack::new_cached(key)),
                    error: AtomicBool::new(false),
                    max_decoded_len,
                    tracks: options.tracks,
                })
                .audio
                .clone()
        } else if let AudioCacheMode::Ignore = options.cache_mode {
            let audio = var(AudioTrack::new_loading(ViewAudioHandle::dummy()));
            self.not_cached.push(NotCachedEntry {
                audio: audio.downgrade(),
                max_decoded_len,
                tracks: options.tracks,
            });
            audio
        } else {
            let audio = var(AudioTrack::new_cached(key));
            self.cache.insert(
                key,
                CacheEntry {
                    audio: audio.clone(),
                    error: AtomicBool::new(false),
                    max_decoded_len,
                    tracks: options.tracks,
                },
            );
            audio
        }
    }

    /// The `fetch_bytes` future is polled in the UI thread, use `task::run` for futures that poll a lot.
    #[allow(clippy::too_many_arguments)]
    fn load_task(
        &mut self,
        key: AudioHash,
        max_decoded_len: ByteLength,
        options: AudioOptions,
        fetch_bytes: impl Future<Output = AudioData> + Send + 'static,
    ) -> AudioVar {
        let audio = self.new_cache_audio(key, max_decoded_len, options.clone());
        let r = audio.read_only();

        self.loading.push(AudioLoadingTask {
            task: Mutex::new(UiTask::new(None, fetch_bytes)),
            audio,
            max_decoded_len,
            key,
            options,
        });
        zng_app::update::UPDATES.update(None);

        r
    }

    fn find_decoding(&self, id: zng_view_api::audio::AudioId) -> Option<AudioVar> {
        self.decoding.iter().find_map(|i| {
            if i.audio.with(|i| i.handle.audio_id() == id) {
                Some(i.audio.clone())
            } else {
                i.audio.with(|i| i.find_track(id))
            }
        })
    }
}

pub(crate) fn clean_all() {
    let mut s = AUDIOS_SV.write();
    s.extensions.iter_mut().for_each(|p| p.clear(false));
    s.cache.retain(|_, v| v.audio.strong_count() > 1);
}

pub(crate) fn contains_key(key: &AudioHash) -> bool {
    AUDIOS_SV.read().cache.contains_key(key)
}

pub(crate) fn purge_all() {
    let mut s = AUDIOS_SV.write();
    s.cache.clear();
    s.extensions.iter_mut().for_each(|p| p.clear(true));
}

pub(crate) fn extend(extension: Box<dyn AudiosExtension + 'static>) {
    AUDIOS_SV.write().extensions.push(extension);
}

pub(crate) fn open_output(id: AudioOutputId, try_existing: bool, cache: bool) -> ResponseVar<AudioOutput> {
    let mut s = AUDIOS_SV.write();
    if try_existing {
        if let Some(r) = s.outputs.get(&id) {
            return response_done_var(r.output.clone());
        }
        if let Some(r) = s.not_cached_outputs.get(&id)
            && let Some(r) = r.output.upgrade()
        {
            return response_done_var(AudioOutput(r));
        }
        for (i, r) in &s.output_requests {
            if *i == id {
                return r.r.response_var();
            }
        }
    }

    let (responder, r) = response_var();
    s.output_requests.push((id, OutputRequest { r: responder, cache }));
    UPDATES.update(None);
    r
}

pub(crate) fn cue(view: ViewAudioOutput, audio: AudioMix) {
    AUDIOS_SV.write().cue.push((view, audio));
    UPDATES.update(None);
}
