use std::{fmt, io::Cursor};

use rodio::Source;
use zng_task::channel::{IpcBytes, IpcBytesCast, IpcReceiver};
use zng_txt::ToTxt;
use zng_view_api::{Event, audio::*};

use crate::{AppEvent, AppEventSender};

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
}
impl AudioCache {
    pub(crate) fn new(app_sender: AppEventSender) -> Self {
        Self {
            app_sender,
            id_gen: AudioId::first(),
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
            },
        };

        let meta = AudioMetadata::new(id, vec![TrackMetadata::new(decoder.channels(), decoder.sample_rate())]);
        let mut track = AudioTrack { decoder };
        if app_sender.send(AppEvent::AudioLoaded(meta, track)).is_err() {
            return;
        }


        // will decode only a chunk by default in the future



        todo!()
    }

    pub(crate) fn add_pro(&mut self, request: AudioRequest<IpcReceiver<IpcBytes>>) -> AudioId {
        todo!()
    }

    pub(crate) fn forget(&self, id: AudioId) {
        todo!()
    }

    pub(crate) fn playback(&self, request: PlaybackRequest) -> zng_view_api::audio::PlaybackId {
        todo!()
    }

    pub(crate) fn playback_update(&self, id: PlaybackId, request: PlaybackUpdateRequest) {
        todo!()
    }

    /// Called after receive and first chunk decode completes correctly.
    pub(crate) fn loaded(&mut self, meta: AudioMetadata, data: AudioTrack) {
    }

    pub(crate) fn on_low_memory(&mut self) {
    }
}

pub struct AudioTrack {
    raw: IpcBytesCast<f32>,
}
impl fmt::Debug for AudioTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioTrack").finish_non_exhaustive()
    }
}
impl rodio::Source for AudioTrack {
    fn current_span_len(&self) -> Option<usize> {
        self.decoder.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.decoder.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.decoder.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.decoder.total_duration()
    }
}
impl Iterator for AudioTrack {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.decoder.size_hint()
    }
}