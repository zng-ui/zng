use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use zng_app::view_process::ViewAudioOutput;
use zng_unit::{Factor, FactorUnits as _};
use zng_var::{Var, impl_from_and_into_var, var};
use zng_view_api::audio::{AudioMix as ViewAudioMix, AudioMixLayer, AudioOutputConfig, AudioOutputState};

use crate::{AudioOutputId, AudioTrack, service};

pub(crate) struct AudioOutputData {
    id: AudioOutputId,
    view: Option<ViewAudioOutput>,

    volume: Var<Factor>,
    speed: Var<Factor>,
    state: Var<AudioOutputState>,
    state_stop_if_start: AtomicBool,
}

/// Represents an open audio output stream.
///
/// You can use [`AUDIOS.open_output`] to open a new output stream.
///
/// [`AUDIOS.open_output`]: crate::AUDIOS::open_output
#[derive(Clone)]
pub struct AudioOutput(pub(crate) Arc<AudioOutputData>);
impl fmt::Debug for AudioOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioOutput")
            .field("id", &self.0.id)
            .field("state", &self.0.state.get())
            .field("volume", &self.0.volume.get())
            .field("speed", &self.0.speed.get())
            .finish_non_exhaustive()
    }
}
impl PartialEq for AudioOutput {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}
impl Eq for AudioOutput {}
impl AudioOutput {
    pub(crate) fn new(id: AudioOutputId, view: Option<ViewAudioOutput>) -> Self {
        Self(Arc::new(AudioOutputData {
            id,
            view,
            volume: var(1.fct()),
            speed: var(1.fct()),
            state: var(AudioOutputState::Playing),
            state_stop_if_start: AtomicBool::new(false),
        }))
    }

    /// Unique ID of this output.
    pub fn id(&self) -> AudioOutputId {
        self.0.id
    }

    /// Enqueue the `audio` for playback in this output.
    ///
    /// The audio will play when the output is playing and the previous cued audio finishes.
    pub fn cue(&self, audio: impl Into<AudioMix>) {
        self.cue_impl(audio.into());
    }
    fn cue_impl(&self, audio: AudioMix) {
        if let Some(view) = self.0.view.clone() {
            service::cue(view.clone(), audio.into());
        }
    }

    /// Volume of the sound.
    ///
    /// The value multiplies the samples, `1.fct()` is the *natural* volume from the source.
    pub fn volume(&self) -> Var<Factor> {
        self.0.volume.clone()
    }

    /// Speed of the sound.
    ///
    /// This is a multiplier of the playback speed and pitch.
    ///
    /// * `0.5.fct()` doubles the total duration and halves (lowers) the pitch.
    /// * `2.fct()` halves the total duration and doubles (raises) the pitch.
    pub fn speed(&self) -> Var<Factor> {
        self.0.speed.clone()
    }

    /// Output playback state.
    ///
    /// This variable can be set to change the state.
    ///
    /// Note that because variable modifications apply at once you cannot stop and play in the same update cycle using this. Use the
    ///
    /// The default value is [`AudioOutputState::Playing`].
    pub fn state(&self) -> Var<AudioOutputState> {
        self.0.state.clone()
    }

    /// Change state to [`Playing`].
    ///
    /// Audio is sent to the device for playback as audio is [cued].
    ///
    /// [`Playing`]: AudioOutputState::Playing
    /// [cued]: Self::cue
    pub fn play(&self) {
        self.0.state.set(AudioOutputState::Playing);
    }

    /// Change state to [`Paused`].
    ///
    /// Audio playback is paused, cue requests are buffered.
    ///
    /// [`Paused`]: AudioOutputState::Paused
    pub fn pause(&self) {
        self.0.state.set(AudioOutputState::Paused);
    }

    /// Change state to [`Stopped`].
    ///
    /// Audio playback is paused, all current cue requests are dropped.
    ///
    /// [`Stopped`]: AudioOutputState::Stopped
    pub fn stop(&self) {
        self.0.state.set(AudioOutputState::Stopped);
    }

    /// Change state to [`Stopped`] and then [`Playing`] in the same update cycle.
    ///
    /// [`Stopped`]: AudioOutputState::Stopped
    /// [`Playing`]: AudioOutputState::Playing
    pub fn stop_play(&self) {
        self.0.state.set(AudioOutputState::Playing);
        self.0.state_stop_if_start.store(true, Ordering::Relaxed);
    }

    pub(crate) fn update(&self) {
        if let Some(view) = &self.0.view {
            let state = self.0.state.get();
            let stop_play = self.0.state_stop_if_start.swap(false, Ordering::Relaxed) && matches!(state, AudioOutputState::Playing);

            if stop_play || self.0.state.is_new() {
                let cfg = AudioOutputConfig::new(state, self.0.volume.get(), self.0.speed.get());
                if stop_play {
                    let mut stop_cfg = cfg.clone();
                    stop_cfg.state = AudioOutputState::Stopped;
                    let _ = view.update(stop_cfg);
                }
                let _ = view.update(cfg);
            }
        }
    }
}

/// Represents an audio source.
///
/// Audio is defined by layers, each subsequent layer applies to the computed result of the previous layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioMix {
    view: ViewAudioMix,
}
impl AudioMix {
    /// New empty.
    pub fn new() -> Self {
        Self { view: ViewAudioMix::new() }
    }

    /// Plays silence for the `duration` before starting layers mix.
    pub fn with_delay(mut self, duration: Duration) -> Self {
        self.view.delay = duration;
        self
    }

    /// Set the total duration.
    ///
    /// If not set audio plays until all audio and mix layers end. If set audio plays for the duration, if layers end before the duration
    /// plays silent at the end, if layers exceed the duration they are clipped.
    ///
    /// Note that the `duration` must account for the [initial delay], the initial silence is included in the the total duration.
    ///
    /// [initial delay]: Self::with_delay
    pub fn with_total_duration(mut self, duration: Duration) -> Self {
        self.view.total_duration = Some(duration);
        self
    }

    /// Add layer that plays the cached audio.
    ///
    /// The audio samples are adapted to the output format and each sample added to the under layers result.
    pub fn with_audio(mut self, audio: &AudioTrack) -> Self {
        if !audio.can_cue() {
            let e = audio.error();
            tracing::error!(
                "cannot cue audio, {}",
                e.as_deref().unwrap_or("metadata not decoded in view-process")
            );
            return self;
        }

        self.view.layers.push(AudioMixLayer::Audio {
            audio: audio.view_handle().audio_id(),
            skip: Duration::ZERO,
            take: Duration::MAX,
        });
        self
    }

    /// Add layer that clips and plays the cached audio.
    ///
    /// This is similar to [`with_audio`], but only the range `skip..skip + take` is played.
    ///
    /// [`with_audio`]: Self::with_audio
    pub fn with_audio_clip(mut self, audio: &AudioTrack, skip: Duration, take: Duration) -> Self {
        if !audio.can_cue() {
            let e = audio.error();
            tracing::error!(
                "cannot cue audio, {}",
                e.as_deref().unwrap_or("metadata not decoded in view-process")
            );
            return self;
        }

        self.view.layers.push(AudioMixLayer::Audio {
            audio: audio.view_handle().audio_id(),
            skip,
            take,
        });
        self
    }

    /// Add layer that plays another mix.
    ///
    /// The inner `mix` is sampled as computed audio, that is, samples are computed first and added to the under layers result.
    pub fn with_mix(self, mix: impl Into<AudioMix>) -> Self {
        self.with_mix_clip(mix.into(), Duration::ZERO, Duration::MAX)
    }
    /// Add layer that clips and plays another mix.
    ///
    /// This is similar to [`with_mix`], but only the range `skip..skip + take` is played.
    ///
    /// [`with_mix`]: Self::with_mix
    pub fn with_mix_clip(mut self, mix: impl Into<AudioMix>, skip: Duration, take: Duration) -> Self {
        self.view.layers.push(AudioMixLayer::AudioMix {
            mix: mix.into().view,
            skip,
            take,
        });
        self
    }
    /// Add layer that generates a sine wave sound.
    ///
    /// The generated sound samples are added to the under layers result.
    pub fn with_sine_wave(mut self, frequency: f32, duration: Duration) -> Self {
        self.view.layers.push(AudioMixLayer::SineWave { frequency, duration });
        self
    }

    /// Add effect layer that applies a linear volume transition.
    ///
    /// When the playback is in range the computed sample of under layers is multiplied by the linear interpolation
    /// between `start_volume` and `end_volume`.
    ///
    /// Note that outside the volume range is not affected, before and after.
    pub fn with_volume_linear(mut self, start: Duration, duration: Duration, start_volume: Factor, end_volume: Factor) -> Self {
        self.view.layers.push(AudioMixLayer::VolumeLinear {
            start,
            duration,
            start_volume,
            end_volume,
        });
        self
    }

    /// Add an effect layer that fades in from the start over a transition duration.
    ///
    /// A linear volume transition at the start raises the volume of under layers from zero to normal over the `transition_duration`.
    pub fn with_fade_in(self, transition_duration: Duration) -> Self {
        self.with_volume_linear(Duration::ZERO, transition_duration, 0.fct(), 1.fct())
    }

    /// Add an effect layer that fades out the audio after `start`.
    ///
    /// A linear volume transition lowers the volume of under layers after `start` to zero over `transition_duration`,
    /// the volume remains zeroed until audio end.
    ///
    /// Note that this does not affect the total duration, you must also call [`with_total_duration`] to *fade out and stop*.
    ///
    /// [`with_total_duration`]: Self::with_total_duration
    pub fn with_fade_out(self, start: Duration, transition_duration: Duration) -> Self {
        self.with_volume_linear(start, transition_duration, 1.fct(), 0.fct())
            .with_volume_linear(start + transition_duration, Duration::MAX, 0.fct(), 0.fct())
    }
}
impl Default for AudioMix {
    fn default() -> Self {
        Self::new()
    }
}
impl_from_and_into_var! {
    fn from(mix: AudioMix) -> ViewAudioMix {
        mix.view
    }
    fn from(audio: AudioTrack) -> AudioMix {
        AudioMix::new().with_audio(&audio)
    }
}
impl From<&AudioTrack> for AudioMix {
    fn from(audio: &AudioTrack) -> Self {
        AudioMix::new().with_audio(audio)
    }
}
