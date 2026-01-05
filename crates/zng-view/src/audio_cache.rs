use std::{fmt, io::Cursor, sync::Arc, time::Duration};

use rodio::Source;
use rustc_hash::FxHashMap;
use zng_task::channel::{IpcBytes, IpcBytesCast, IpcBytesCastIntoIter, IpcBytesMutCast, IpcReceiver};
use zng_txt::{ToTxt, formatx};
use zng_view_api::{Event, audio::*};

use crate::{AppEvent, AppEventSender};

mod mix;

pub(crate) const FORMATS: &[AudioFormat] = &[
    #[cfg(feature = "audio_mp3")]
    AudioFormat::from_static("MP3", "mpeg", "mp3,mpga", AudioFormatCapability::empty()),
    #[cfg(feature = "audio_mp4")]
    AudioFormat::from_static("MP4", "mp4", "m4a,m4b,m4r", AudioFormatCapability::empty()),
    #[cfg(feature = "audio_flac")]
    AudioFormat::from_static("FLAC", "flac", "flac", AudioFormatCapability::empty()),
    #[cfg(feature = "audio_vorbis")]
    AudioFormat::from_static("Vorbis", "ogg,vorbis", "ogg", AudioFormatCapability::empty()),
    #[cfg(feature = "audio_wav")]
    AudioFormat::from_static("WAV", "wav,vnd.wave", "wav,wave", AudioFormatCapability::empty()),
];

pub(crate) struct AudioCache {
    app_sender: AppEventSender,
    id_gen: AudioId,
    play_id_gen: AudioPlayId,
    tracks: FxHashMap<AudioId, AudioTrack>,
    device_streams: Vec<std::sync::Weak<rodio::OutputStream>>,
    streams: FxHashMap<AudioOutputId, VpOutput>,
}
struct VpOutput {
    device_stream: Arc<rodio::OutputStream>,
    sink: rodio::Sink,
}
impl AudioCache {
    pub(crate) fn new(app_sender: AppEventSender) -> Self {
        Self {
            app_sender,
            id_gen: AudioId::first(),
            play_id_gen: AudioPlayId::first(),
            tracks: FxHashMap::default(),
            device_streams: vec![],
            streams: FxHashMap::default(),
        }
    }

    pub(crate) fn add(&mut self, request: AudioRequest<IpcBytes>) -> AudioId {
        let id = self.id_gen.incr();
        let app_sender = self.app_sender.clone();
        rayon::spawn(move || Self::add_impl(app_sender, id, request));
        id
    }

    fn add_impl(app_sender: AppEventSender, id: AudioId, request: AudioRequest<IpcBytes>) {
        let data = request.data;

        if let AudioDataFormat::InterleavedF32 {
            channel_count,
            sample_rate,
            total_duration,
        } = request.format
        {
            // already decoded

            if !data.len().is_multiple_of(4) {
                let _ = app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
                    audio: id,
                    error: formatx!("data cannot be cast to f32, not a multiple of 4"),
                }));
                return;
            }
            let data = data.cast::<f32>();
            if !data.len().is_multiple_of(channel_count as usize) {
                let _ = app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
                    audio: id,
                    error: formatx!(
                        "data not an interleaved sequence {0} channel samples, not not a multiple of {0}",
                        channel_count
                    ),
                }));
                return;
            }

            let d = Duration::from_secs_f64(data.len() as f64 / channel_count as f64 / sample_rate as f64);
            if let Some(md) = total_duration
                && (d.as_millis() != md.as_millis())
            {
                tracing::error!("incorrect `total_duration` {md:?}, corrected to {d:?}");
            }
            let total_duration = d;

            let mut meta = AudioMetadata::new(id, channel_count, sample_rate);
            meta.total_duration = Some(total_duration);

            let track = AudioTrack {
                channel_count: meta.channel_count,
                sample_rate: meta.sample_rate,
                total_duration,
                raw: data,
            };

            if app_sender.send(AppEvent::Notify(Event::AudioMetadataDecoded(meta))).is_err() {
                return;
            }

            let mut decoded = AudioDecoded::new(id, track.raw.clone());
            decoded.is_full = true;

            if app_sender.send(AppEvent::AudioCanPlay(id, track)).is_err() {
                return;
            }

            let _ = app_sender.send(AppEvent::Notify(Event::AudioDecoded(decoded)));

            return;
        }

        // decode

        let mss = symphonia::core::io::MediaSourceStream::new(Box::new(Cursor::new(data.clone())), Default::default());
        let mut format_hint = symphonia::core::probe::Hint::new();
        match request.format {
            AudioDataFormat::FileExtension(ext) => {
                format_hint.with_extension(&ext);
            }
            AudioDataFormat::MimeType(t) => {
                format_hint.with_extension(&t);
            }
            _ => (),
        }

        let probe = symphonia::default::get_probe().format(
            &format_hint,
            mss,
            &symphonia::core::formats::FormatOptions::default(),
            &symphonia::core::meta::MetadataOptions::default(),
        );

        let probe = match probe {
            Ok(p) => p,
            Err(e) => {
                let _ = app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
                    audio: id,
                    error: e.to_txt(),
                }));
                return;
            }
        };

        // not sure if the container detected is available in the metadata, can't find docs for it
        // so we
        let mut format = None;
        let container_name_hack = std::any::type_name_of_val(&*probe.format).to_ascii_lowercase();
        for f in FORMATS {
            if f.file_extensions_iter().any(|e| container_name_hack.contains(e)) {
                format = Some(f);
                break;
            }
        }
        let format = format.unwrap_or_else(|| panic!("cannot identify format from symphonia type name {container_name_hack:?}"));

        // will read other metadata here in the future

        let decoder = rodio::decoder::DecoderBuilder::new()
            .with_byte_len(data.len() as _)
            .with_seekable(true)
            .with_data(Cursor::new(data))
            .with_mime_type(&format.media_types().next().unwrap())
            .build();
        let decoder = match decoder {
            Ok(d) => d,
            Err(e) => {
                let _ = app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
                    audio: id,
                    error: e.to_txt(),
                }));
                return;
            }
        };
        let total_duration = match decoder.total_duration() {
            Some(d) => d,
            None => {
                let _ = app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
                    audio: id,
                    error: formatx!("only audio sources with known duration are currently supported"),
                }));
                return;
            }
        };

        let mut meta = AudioMetadata::new(id, decoder.channels(), decoder.sample_rate());
        meta.total_duration = Some(total_duration);

        let mut track = AudioTrack {
            channel_count: meta.channel_count,
            sample_rate: meta.sample_rate,
            total_duration,
            raw: IpcBytesCast::default(),
        };

        if app_sender.send(AppEvent::Notify(Event::AudioMetadataDecoded(meta))).is_err() {
            return;
        }

        let decoded = (|| -> std::io::Result<IpcBytesCast<f32>> {
            let mut raw = IpcBytesMutCast::<f32>::new_blocking(decoder.current_span_len().unwrap())?;
            for (f, df) in raw.iter_mut().zip(decoder) {
                *f = df;
            }
            raw.finish_blocking()
        })();
        match decoded {
            Ok(d) => track.raw = d,
            Err(e) => {
                let _ = app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
                    audio: id,
                    error: formatx!("cannot allocate memory for decode, {e}"),
                }));
                return;
            }
        }

        let mut decoded = AudioDecoded::new(id, track.raw.clone());
        decoded.is_full = true;

        if app_sender.send(AppEvent::AudioCanPlay(id, track)).is_err() {
            return;
        }

        let _ = app_sender.send(AppEvent::Notify(Event::AudioDecoded(decoded)));
    }

    pub(crate) fn add_pro(&mut self, _request: AudioRequest<IpcReceiver<IpcBytes>>) -> AudioId {
        let id = self.id_gen.incr();
        let _ = self.app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
            audio: id,
            error: "add_pro not implemented".to_txt(),
        }));
        id
    }

    pub(crate) fn forget(&mut self, id: AudioId) {
        self.tracks.remove(&id);
    }

    pub(crate) fn open_output(&mut self, output: AudioOutputRequest) {
        let id = output.id;

        // only supports the default stream for this update
        let mut device_stream = self.device_streams.first().and_then(|w| w.upgrade());

        if device_stream.is_none() {
            self.device_streams.retain(|w| w.strong_count() > 0);
            match rodio::OutputStreamBuilder::open_default_stream() {
                Ok(s) => {
                    let s = Arc::new(s);
                    self.device_streams.push(Arc::downgrade(&s));
                    device_stream = Some(s);
                }
                Err(e) => {
                    let _ = self.app_sender.send(AppEvent::Notify(Event::AudioOutputOpenError {
                        id,
                        error: formatx!("cannot open audio output device stream, {e}"),
                    }));
                    return;
                }
            }
        }
        let device_stream = device_stream.unwrap();

        let data = AudioOutputOpenData::new(device_stream.config().channel_count(), device_stream.config().sample_rate());

        let sink = rodio::Sink::connect_new(device_stream.mixer());
        sink.set_volume(output.config.volume.0);
        sink.set_speed(output.config.speed.0);
        match output.config.state {
            AudioOutputState::Pause | AudioOutputState::Stop => sink.pause(),
            AudioOutputState::Play => {}
            _ => unreachable!(),
        }

        self.streams.insert(id, VpOutput { device_stream, sink });

        let _ = self.app_sender.send(AppEvent::Notify(Event::AudioOutputOpened(id, data)));
    }

    pub(crate) fn update_output(&mut self, request: AudioOutputUpdateRequest) {
        if let Some(s) = self.streams.get(&request.id) {
            match &request.config.state {
                AudioOutputState::Play => {}
                AudioOutputState::Pause => s.sink.pause(),
                AudioOutputState::Stop => {
                    s.sink.pause();
                    s.sink.clear();
                }
                _ => unreachable!(),
            }
            s.sink.set_volume(request.config.volume.0);
            s.sink.set_speed(request.config.speed.0);
            if let AudioOutputState::Play = &request.config.state {
                s.sink.play();
            }
        }
    }

    pub(crate) fn close_output(&mut self, id: AudioOutputId) {
        if let Some(s) = self.streams.remove(&id) {
            s.sink.stop();
        }
    }

    pub(crate) fn play(&mut self, request: AudioPlayRequest) -> AudioPlayId {
        let id = self.play_id_gen.incr();

        if let Some(s) = self.streams.get(&request.output) {
            match self.vp_mix_to_source(
                request.mix,
                s.device_stream.config().channel_count(),
                s.device_stream.config().sample_rate(),
            ) {
                Ok(source) => s.sink.append(source),
                Err(e) => {
                    let _ = self.app_sender.send(AppEvent::Notify(Event::AudioPlayError { play: id, error: e }));
                }
            }
        } else {
            let _ = self.app_sender.send(AppEvent::Notify(Event::AudioPlayError {
                play: id,
                error: formatx!("output stream {:?} not found", request.output),
            }));
        }

        id
    }

    /// Called after receive and first chunk decode completes correctly.
    pub(crate) fn on_audio_can_play(&mut self, id: AudioId, data: AudioTrack) {
        self.tracks.insert(id, data);
    }

    pub(crate) fn on_low_memory(&mut self) {}
}

pub struct AudioTrack {
    channel_count: u16,
    sample_rate: u32,
    total_duration: Duration,
    raw: IpcBytesCast<f32>,
}
impl fmt::Debug for AudioTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioTrack").finish_non_exhaustive()
    }
}
impl AudioTrack {
    fn play_source(&self) -> AudioTrackPlay {
        AudioTrackPlay {
            channel_count: self.channel_count,
            sample_rate: self.sample_rate,
            total_duration: self.total_duration,
            track: self.raw.clone().into_iter(),
        }
    }
}

struct AudioTrackPlay {
    channel_count: u16,
    sample_rate: u32,
    total_duration: Duration,
    track: IpcBytesCastIntoIter<f32>,
}
impl Iterator for AudioTrackPlay {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.track.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.track.size_hint()
    }
}
impl rodio::Source for AudioTrackPlay {
    fn current_span_len(&self) -> Option<usize> {
        todo!()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.channel_count
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }
}
