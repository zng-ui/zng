//! IME types.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// IME composition events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Ime {
    /// Notifies when the IME was enabled.
    ///
    /// After getting this event you could receive [`PreEdit`](Self::PreEdit) and
    /// [`Commit`](Self::Commit) events. You should also start performing IME related requests
    /// like [`Api::set_ime_area`].
    ///
    /// [`Api::set_ime_area`]: crate::Api::set_ime_area
    Enabled,

    /// Notifies when a new composing text should be set at the cursor position.
    ///
    /// The value represents a pair of the pre-edit string and the cursor begin position and end
    /// position. When it's `None`, the cursor should be hidden. When `String` is an empty string
    /// this indicates that pre-edit was cleared.
    ///
    /// The cursor position is byte-wise indexed.
    PreEdit(Arc<str>, Option<(usize, usize)>),

    /// Notifies when text should be inserted into the editor widget.
    ///
    /// Right before this event winit will send empty [`Self::PreEdit`] event.
    Commit(Arc<str>),

    /// Notifies when the IME was disabled.
    ///
    /// After receiving this event you won't get any more [`PreEdit`](Self::PreEdit) or
    /// [`Commit`](Self::Commit) events until the next [`Enabled`](Self::Enabled) event. You should
    /// also stop issuing IME related requests like [`Api::set_ime_area`] and clear pending
    /// pre-edit text.
    ///
    /// [`Api::set_ime_area`]: crate::Api::set_ime_area
    Disabled,
}
