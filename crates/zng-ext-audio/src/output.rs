use zng_app::{command, view_process::ViewAudioOutput};
use zng_unit::{Factor, FactorUnits as _};
use zng_var::{Var, var};
use zng_view_api::audio::{AudioMix, AudioOutputState};

zng_unique_id::unique_id_32! {
    /// Unique identifier of an open audio output.
    ///
    /// # Name
    ///
    /// IDs are only unique for the same process.
    /// You can associate a [`name`] with an ID to give it a persistent identifier.
    ///
    /// [`name`]: AudioOutputId::name
    pub struct AudioOutputId;
}
zng_unique_id::impl_unique_id_name!(AudioOutputId);
zng_unique_id::impl_unique_id_fmt!(AudioOutputId);
zng_unique_id::impl_unique_id_bytemuck!(AudioOutputId);

/// Represents an open audio output.
pub struct AudioOutput {
    id: AudioOutputId,
    volume: Var<Factor>,
    speed: Var<Factor>,
    state: Var<AudioOutputState>,
    view: Option<ViewAudioOutput>,
}
impl AudioOutput {
    pub(crate) fn new(id: AudioOutputId) -> Self {
        Self {
            id,
            volume: var(1.fct()),
            speed: var(1.fct()),
            state: var(AudioOutputState::Playing),
            view: None,
        }
    }

    /// Unique ID of this output.
    pub fn id(&self) -> AudioOutputId {
        self.id
    }

    /// Enqueue the `audio` for playback in this output.
    pub fn cue(&self, audio: AudioMix) {

    }

    /// Volume of the sound.
    ///
    /// The value multiplies the samples, `1.fct()` is the *natural* volume from the source.
    pub fn volume(&self) -> Var<Factor> {
        self.volume.clone()
    }

    /// Speed of the sound.
    ///
    /// This is a multiplier of the playback speed and pitch.
    ///
    /// * `0.5.fct()` doubles the total duration and halves (lowers) the pitch.
    /// * `2.fct()` halves the total duration and doubles (raises) the pitch.
    pub fn speed(&self) ->Var<Factor> {
        self.speed.clone()
    }

    /// Output playback state.
    /// 
    /// This variable can be set to change the state. 
    /// 
    /// Note that because variable modifications apply at once you cannot stop and play in the same update cycle using this. Use the 
    ///
    /// The default value is [`AudioOutputState::Playing`].
    pub fn state(&self) -> Var<AudioOutputState> {
        self.state.clone()
    }

    /// Change state to [`Playing`].
    /// 
    /// Audio is sent to the device for playback as audio is [cued].
    /// 
    /// [`Playing`]: AudioOutputState::Playing
    /// [cued]: Self::cue
    pub fn play(&self) {
        self.state.set(AudioOutputState::Playing);
    }

    /// Change state to [`Paused`].
    ///
    /// Audio playback is paused, cue requests are buffered.
    /// 
    /// [`Paused`]: AudioOutputState::Paused
    pub fn pause(&self) {
        self.state.set(AudioOutputState::Paused);
    }

    /// Change state to [`Stopped`].
    /// 
    /// Audio playback is paused, all current cue requests are dropped.
    ///
    /// [`Stopped`]: AudioOutputState::Stopped
    pub fn stop(&self) {
        self.state.set(AudioOutputState::Stopped);
    }

    /// Change state to [`Stopped`] and then [`Playing`] in the same update cycle.
    /// 
    /// [`Stopped`]: AudioOutputState::Stopped
    /// [`Playing`]: AudioOutputState::Playing
    pub fn stop_play(&self) {

    }
}

/*
 * stop_play ()
 * Commands? Not here, at the widget.
 * 
 */