use zng_var::Var;
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
    state: Var<AudioOutputState>,
}
impl AudioOutput {
    /// Unique ID of this output.
    pub fn id(&self) -> AudioOutputId {
        self.id
    }

    /// Enqueue the `audio` for playback in this output.
    pub fn cue(&self, audio: AudioMix) {

    }

    /// Output playback state.
    ///
    /// The default value is [`AudioOutputState::Playing`].
    pub fn state(&self) -> Var<AudioOutputState> {
        self.state.clone()
    }

    /// Change state to [`Playing`].
    /// 
    /// [`Playing`]: AudioOutputState::Playing
    pub fn play(&self) {
        self.state.set(AudioOutputState::Playing);
    }

        /// Change state to [`Paused`].
    /// 
    /// [`Paused`]: AudioOutputState::Paused
    pub fn pause(&self) {
        self.state.set(AudioOutputState::Paused);
    }

        /// Change state to [`Stopped`].
    /// 
    /// [`Stopped`]: AudioOutputState::Stopped
    pub fn stop(&self) {
        self.state.set(AudioOutputState::Stopped);
    }
}
