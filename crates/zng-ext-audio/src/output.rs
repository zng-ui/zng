use std::{fmt, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use zng_app::{
    update::UPDATES,
    view_process::{
        VIEW_PROCESS, ViewAudioOutput,
        raw_events::{RAW_AUDIO_OUTPUT_OPEN_ERROR_EVENT, RAW_AUDIO_OUTPUT_OPEN_EVENT},
    },
};
use zng_txt::{ToTxt as _, Txt};
use zng_unit::{Factor, FactorUnits as _};
use zng_var::{AnyVarHookArgs, Var, impl_from_and_into_var, var};
use zng_view_api::audio::{AudioMix as ViewAudioMix, AudioMixLayer, AudioOutputConfig, AudioPlayId};

use crate::{AUDIOS, AUDIOS_SV, AudioOutputId, AudioTrack};
pub use zng_view_api::audio::AudioOutputState;

pub(crate) struct AudioOutputData {
    id: AudioOutputId,
    view: Var<Result<ViewAudioOutput, Txt>>,

    volume: Var<Factor>,
    speed: Var<Factor>,
    state: Var<AudioOutputState>,
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
    pub(crate) fn open(id: AudioOutputId, opt: AudioOutputOptions) -> Self {
        let r = Self(Arc::new(AudioOutputData {
            id,
            view: var(Err(Txt::from("not connected"))),
            volume: var(opt.config.volume),
            speed: var(opt.config.speed),
            state: var(opt.config.state),
        }));
        r.0.state.as_any().hook(r.update_view_handler()).perm();
        r.0.speed.as_any().hook(r.update_view_handler()).perm();
        r.0.speed.as_any().hook(r.update_view_handler()).perm();

        let handle = RAW_AUDIO_OUTPUT_OPEN_ERROR_EVENT.hook(move |args| {
            if args.output_id == id {
                if let Some(o) = AUDIOS_SV.read().outputs.get(&id)
                    && let Some(o) = o.upgrade()
                {
                    o.0.view.set(Err(args.error.clone()));
                }
                return false;
            }
            true
        });
        let handle = RAW_AUDIO_OUTPUT_OPEN_EVENT.hook(move |args| {
            let _hold = &handle;
            if args.output_id == id {
                if let Some(vo) = args.output.upgrade()
                    && let Some(o) = AUDIOS_SV.read().outputs.get(&id)
                    && let Some(o) = o.upgrade()
                {
                    o.0.view.set(Ok(vo));
                }
                return false;
            }
            true
        });
        r.0.view
            .hook(move |_| {
                // hold until view is set by any of the events
                let _hold = &handle;
                false
            })
            .perm();

        let _ = VIEW_PROCESS.open_audio_output(zng_view_api::audio::AudioOutputRequest::new(
            zng_view_api::audio::AudioOutputId::from_raw(id.get()),
            opt.config,
        ));

        r
    }
    fn update_view_handler(&self) -> impl FnMut(&AnyVarHookArgs) -> bool + Send + 'static {
        let wk = Arc::downgrade(&self.0);
        move |_| {
            if let Some(a) = wk.upgrade() {
                let r = a.view.with(|v| {
                    if let Ok(v) = v {
                        let cfg = AudioOutputConfig::new(a.state.get(), a.volume.get(), a.speed.get());
                        v.update(cfg)
                    } else {
                        Ok(())
                    }
                });
                if let Err(e) = r {
                    // will reconnect on respawn
                    a.view.set(Err(e.to_txt()));
                }
                true
            } else {
                false
            }
        }
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
        let s = self.clone();
        UPDATES.once_update("AudioOutput.cue", move || {
            let r = s.0.view.with(|v| match v {
                Ok(v) => v.cue(audio.view),
                Err(e) => {
                    tracing::error!("failed to cue audio, {e}");
                    Ok(AudioPlayId::INVALID)
                }
            });
            if let Err(e) = r {
                // will reconnect on respawn
                s.0.view.set(Err(e.to_txt()));
            }
        });
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
        let s = self.clone();
        self.0.state.modify(move |a| {
            a.set(AudioOutputState::Playing);
            s.0.view.with(|v| {
                if let Ok(v) = v {
                    let _ = v.update(AudioOutputConfig::new(AudioOutputState::Stopped, s.0.volume.get(), s.0.speed.get()));
                    a.update();
                }
            });
        });
    }

    /// Keep the audio output open until the end of the app.
    pub fn perm(&self) {
        AUDIOS.perm_output(self);
    }

    /// Read-only variable that tracks if this output is connected with the view-process.
    ///
    /// When this is `false` the [`error`] might be set. Outputs automatically try to reconnect in case
    /// of view-process respawn, but note that it will respawn in the [`Stopped`] state.
    ///
    /// [`error`]: Self::error
    /// [`Stopped`]: AudioOutputState::Stopped
    pub fn is_connected(&self) -> Var<bool> {
        self.0.view.map(|v| v.is_ok())
    }

    /// Gets an error message if view-process connection has failed.
    ///
    /// Reconnection is attempted on view-process respawn, the [`is_connected`] tracks the ok status. Note that
    /// the first connection attempt is `true` in [`is_connected`] and `None` here.
    ///
    /// [`is_connected`]: Self::is_connected
    pub fn error(&self) -> Var<Option<Txt>> {
        self.0.view.map(|v| match v {
            Ok(_) => None,
            Err(e) => {
                if e.is_empty() {
                    None
                } else {
                    Some(e.clone())
                }
            }
        })
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

/// Options for a new audio output stream.
#[derive(Debug)]
#[non_exhaustive]
pub struct AudioOutputOptions {
    /// Initial config.
    pub config: AudioOutputConfig,
}

impl Default for AudioOutputOptions {
    fn default() -> Self {
        Self {
            config: AudioOutputConfig::new(AudioOutputState::Playing, 1.fct(), 1.fct()),
        }
    }
}

/// Weak reference to an [`AudioOutput`].
#[derive(Clone)]
pub struct WeakAudioOutput(std::sync::Weak<AudioOutputData>);
impl fmt::Debug for WeakAudioOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakAudioOutput").finish_non_exhaustive()
    }
}
impl PartialEq for WeakAudioOutput {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for WeakAudioOutput {}
impl WeakAudioOutput {
    /// New weak reference that does not allocate and does not upgrade.
    pub const fn new() -> Self {
        Self(std::sync::Weak::new())
    }

    /// Attempt to upgrade to a strong reference to the audio output.
    pub fn upgrade(&self) -> Option<AudioOutput> {
        self.0.upgrade().map(AudioOutput)
    }
}
impl Default for WeakAudioOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioOutput {
    /// Create a weak reference to this audio output.
    pub fn downgrade(&self) -> WeakAudioOutput {
        WeakAudioOutput(std::sync::Arc::downgrade(&self.0))
    }
}
