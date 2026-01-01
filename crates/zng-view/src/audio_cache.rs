use std::io::Cursor;

use zng_task::channel::{IpcBytes, IpcReceiver};
use zng_txt::ToTxt;
use zng_view_api::{Event, audio::*};

use crate::AppEventSender;

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

        let mss = symphonia::core::io::MediaSourceStream::new(Cursor::new(&data[..]), Default::default());

        let source = match request.format {
            AudioDataFormat::FileExtension(ext) => rodio::decoder::DecoderBuilder::new()
                .with_byte_len(data.len() as _)
                .with_seekable(true)
                .with_data(Cursor::new(data))
                .with_hint(&ext)
                .build(),
            AudioDataFormat::MimeType(t) => rodio::decoder::DecoderBuilder::new()
                .with_byte_len(data.len() as _)
                .with_seekable(true)
                .with_data(Cursor::new(data))
                .with_mime_type(&t)
                .build(),
            AudioDataFormat::Unknown => rodio::decoder::DecoderBuilder::new()
                .with_byte_len(data.len() as _)
                .with_seekable(true)
                .with_data(Cursor::new(data))
                .build(),
            _ => unreachable!(),
        };
        let source = match source {
            Ok(s) => s,
            Err(e) => {
                let _ = app_sender.send(crate::AppEvent::Notify(Event::AudioDecodeError {
                    audio: id,
                    error: e.to_txt(),
                }));
                return;
            }
        };
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
}
