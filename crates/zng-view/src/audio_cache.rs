#![cfg_attr(not(feature = "audio_any"), allow(unused))]

#[cfg(feature = "audio_any")]
use std::io::{self, Read as _, Seek as _, SeekFrom};
use std::{fmt, time::Duration};

#[cfg(feature = "audio_any")]
use rodio::Source as _;
use rustc_hash::FxHashMap;
#[cfg(feature = "audio_mp3")]
use symphonia::core::probe::QueryDescriptor;
#[cfg(feature = "audio_any")]
use zng_task::channel::IpcReadBlocking;
use zng_task::channel::{IpcBytes, IpcBytesCast, IpcBytesCastIntoIter, IpcReadHandle, IpcReceiver};
use zng_txt::{ToTxt, formatx};
use zng_view_api::{Event, audio::*};

#[cfg(not(feature = "audio_any"))]
mod rodio {
    pub struct MixerDeviceSink;
    pub struct Player;
}

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

#[cfg(feature = "audio_any")]
fn symphonia_format(buf: &mut IpcReadBlocking) -> io::Result<&'static AudioFormat> {
    let mut magic = vec![];
    buf.by_ref().take(24).read_to_end(&mut magic)?;
    buf.seek(SeekFrom::Start(0))?;

    let sf = std::iter::empty();

    #[cfg(feature = "audio_mp3")]
    let sf = sf.chain(symphonia::default::formats::MpaReader::query());
    #[cfg(feature = "audio_mp4")]
    let sf = sf.chain(symphonia::default::formats::IsoMp4Reader::query());
    #[cfg(feature = "audio_flac")]
    let sf = sf.chain(symphonia::default::formats::FlacReader::query());
    #[cfg(feature = "audio_vorbis")]
    let sf = sf.chain(symphonia::default::formats::OggReader::query());
    #[cfg(feature = "audio_wav")]
    let sf = sf.chain(symphonia::default::formats::WavReader::query());

    let mut found_mime = &[][..];
    'search: for f in sf {
        for m in f.markers {
            if magic.len() >= m.len() && magic[..m.len()] == **m {
                found_mime = f.mime_types;
                break 'search;
            }
        }
    }
    if !found_mime.is_empty() {
        for f in FORMATS {
            if found_mime.iter().any(|m| f.matches(m)) {
                return Ok(f);
            }
        }
    }

    #[cfg(feature = "audio_mp3")]
    if magic.len() > 3 && &magic[..3] == b"ID3" {
        // ID3 tag, MP3 file have an ID3 header and after the MP3
        return Ok(FORMATS.iter().find(|f| f.matches("mp3")).unwrap());
    }

    Err(io::Error::new(io::ErrorKind::InvalidData, "unknown format"))
}

pub(crate) struct AudioCache {
    app_sender: AppEventSender,
    id_gen: AudioId,
    play_id_gen: AudioPlayId,
    tracks: FxHashMap<AudioId, AudioTrack>,
    device_streams: Vec<rodio::MixerDeviceSink>,
    streams: FxHashMap<AudioOutputId, VpOutput>,
}
struct VpOutput {
    sink: rodio::Player,
    channel_count: u16,
    sample_rate: u32,
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

    pub(crate) fn add(&mut self, request: AudioRequest<IpcReadHandle>) -> AudioId {
        let id = self.id_gen.incr();
        let app_sender = self.app_sender.clone();
        rayon::spawn(move || Self::add_impl(app_sender, id, request));
        id
    }

    #[cfg(not(feature = "audio_any"))]
    fn add_impl(app_sender: AppEventSender, id: AudioId, request: AudioRequest<IpcReadHandle>) {
        app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
            audio: id,
            error: r#"not built with "audio_any""#.to_txt(),
        }));
    }

    #[cfg(feature = "audio_any")]
    fn add_impl(app_sender: AppEventSender, id: AudioId, request: AudioRequest<IpcReadHandle>) {
        macro_rules! error {
            ($($msg:tt)+) => {
                {
                    let _ = app_sender.send(AppEvent::Notify(Event::AudioDecodeError {
                        audio: id,
                        error: formatx!($($msg)+),
                    }));
                }
            };
        }

        let mut data = match request.data.read_blocking() {
            Ok(d) => d,
            Err(e) => return error!("cannot read data, {e}"),
        };
        let data_len = match data.remaining_len() {
            Ok(l) => l,
            Err(e) => return error!("cannot read data, {e}"),
        };

        if let AudioDataFormat::InterleavedF32 {
            channel_count,
            sample_rate,
            total_duration,
        } = request.format
        {
            // already decoded
            if !data_len.is_multiple_of(4) {
                return error!("data cannot be cast to f32, not a multiple of 4");
            }
            if !(data_len / 4).is_multiple_of(channel_count as _) {
                return error!(
                    "data not an interleaved sequence {0} channel samples, not not a multiple of {0}",
                    channel_count
                );
            }

            let d = Duration::from_secs_f64(data_len as f64 / channel_count as f64 / sample_rate as f64);
            if let Some(md) = total_duration
                && (d.as_millis() != md.as_millis())
            {
                tracing::error!("incorrect `total_duration` {md:?}, corrected to {d:?}");
            }
            let total_duration = d;

            let mut meta = AudioMetadata::new(id, channel_count, sample_rate);
            meta.total_duration = Some(total_duration);

            let data = match data.read_to_bytes() {
                Ok(d) => d.cast::<f32>(),
                Err(e) => return error!("cannot read data, {e}"),
            };

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

        // symphonia does not provide the identified container type, but all it does it check
        // the magic number, so we do it again here, with the symphonia numbers
        let format = match symphonia_format(&mut data) {
            Ok(f) => f,
            Err(e) => return error!("cannot determinate format, {e}"),
        };

        // will read other metadata here in the future

        let decoder = rodio::decoder::DecoderBuilder::new()
            .with_byte_len(data_len)
            .with_seekable(true)
            .with_data(data)
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
            None => return error!("only audio sources with known duration are currently supported"),
        };

        let mut meta = AudioMetadata::new(id, decoder.channels().get(), decoder.sample_rate().get());
        meta.total_duration = Some(total_duration);

        let mut track = AudioTrack {
            channel_count: meta.channel_count,
            sample_rate: meta.sample_rate,
            total_duration,
            raw: IpcBytesCast::default(),
        };

        // rodio/symphonia iterator does not provide a size_hint, because the API needs to support
        // streaming with shifting sample rate, but we can calculate it for this use case, and this
        // drastically improves `from_iter_blocking` performance.
        let sample_len = (meta.channel_count as f64 * meta.sample_rate as f64 * total_duration.as_secs_f64()).ceil() as usize;

        if app_sender.send(AppEvent::Notify(Event::AudioMetadataDecoded(meta))).is_err() {
            return;
        }

        struct SizeHint<I> {
            iter: I,
            sample_len: usize,
        }
        impl<I: Iterator> Iterator for SizeHint<I> {
            type Item = <I as Iterator>::Item;

            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.sample_len, Some(self.sample_len))
            }
        }
        match IpcBytesCast::<f32>::from_iter_blocking(SizeHint { iter: decoder, sample_len }) {
            Ok(d) => track.raw = d,
            Err(e) => return error!("cannot allocate memory for decode, {e}"),
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

    #[cfg(not(feature = "audio_any"))]
    pub(crate) fn open_output(&mut self, output: AudioOutputRequest) {
        let _ = self.app_sender.send(AppEvent::Notify(Event::AudioOutputOpenError {
            id: output.id,
            error: r#"cannot open audio output device stream, not built with "audio_any""#.to_txt(),
        }));
    }

    #[cfg(feature = "audio_any")]
    pub(crate) fn open_output(&mut self, output: AudioOutputRequest) {
        let id = output.id;

        // only supports the default stream for this release
        if self.device_streams.is_empty() {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(s) => {
                    self.device_streams.push(s);
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
        let device_stream = &self.device_streams[0];

        let data = AudioOutputOpenData::new(
            device_stream.config().channel_count().get(),
            device_stream.config().sample_rate().get(),
        );

        let sink = rodio::Player::connect_new(device_stream.mixer());
        sink.set_volume(output.config.volume.0);
        sink.set_speed(output.config.speed.0);
        match output.config.state {
            AudioOutputState::Paused | AudioOutputState::Stopped => sink.pause(),
            AudioOutputState::Playing => {}
            _ => unreachable!(),
        }

        let c = device_stream.config();
        self.streams.insert(
            id,
            VpOutput {
                sink,
                channel_count: c.channel_count().get(),
                sample_rate: c.sample_rate().get(),
            },
        );

        let _ = self.app_sender.send(AppEvent::Notify(Event::AudioOutputOpened(id, data)));
    }

    #[cfg(not(feature = "audio_any"))]
    pub(crate) fn update_output(&mut self, _: AudioOutputUpdateRequest) {}

    #[cfg(feature = "audio_any")]
    pub(crate) fn update_output(&mut self, request: AudioOutputUpdateRequest) {
        if let Some(s) = self.streams.get(&request.id) {
            match &request.config.state {
                AudioOutputState::Playing => {}
                AudioOutputState::Paused => s.sink.pause(),
                AudioOutputState::Stopped => {
                    s.sink.pause();
                    s.sink.clear();
                }
                _ => unreachable!(),
            }
            s.sink.set_volume(request.config.volume.0);
            s.sink.set_speed(request.config.speed.0);
            if let AudioOutputState::Playing = &request.config.state {
                s.sink.play();
            }
        }
    }

    #[cfg(not(feature = "audio_any"))]
    pub(crate) fn close_output(&mut self, _: AudioOutputId) {}

    #[cfg(feature = "audio_any")]
    pub(crate) fn close_output(&mut self, id: AudioOutputId) {
        if let Some(s) = self.streams.remove(&id) {
            s.sink.stop();
        }
    }

    #[cfg(not(feature = "audio_any"))]
    pub(crate) fn play(&mut self, request: AudioPlayRequest) -> AudioPlayId {
        let id = self.play_id_gen.incr();

        let _ = self.app_sender.send(AppEvent::Notify(Event::AudioPlayError {
            play: id,
            error: formatx!("output stream {:?} not found", request.output),
        }));

        id
    }

    #[cfg(feature = "audio_any")]
    pub(crate) fn play(&mut self, request: AudioPlayRequest) -> AudioPlayId {
        let id = self.play_id_gen.incr();

        if let Some(s) = self.streams.get(&request.output) {
            match self.vp_mix_to_source(request.mix, s.channel_count, s.sample_rate) {
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
#[cfg(feature = "audio_any")]
impl rodio::Source for AudioTrackPlay {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.track.rest().len())
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.channel_count.try_into().unwrap()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.sample_rate.try_into().unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }
}
