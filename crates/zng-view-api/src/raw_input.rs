//! Types for "input" devices.
//!
//! This represents the more basic subset of HIDs, keyboard, mouse, game controllers, it does not include media devices.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use zng_txt::Txt;

use crate::{
    AxisId,
    keyboard::{KeyCode, KeyState},
    mouse::{ButtonId, ButtonState, MouseScrollDelta},
};

crate::declare_id! {
    /// Input device ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is mapped to an unique id, but does not survived View crashes.
    ///
    /// The View Process defines the ID.
    pub struct InputDeviceId(_);
}

/// Info about an human input device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InputDeviceInfo {
    /// Display name.
    pub name: Txt,
    /// Device capabilities.
    pub capabilities: InputDeviceCapability,
}

impl InputDeviceInfo {
    /// New info.
    pub fn new(name: impl Into<Txt>, capabilities: InputDeviceCapability) -> Self {
        Self {
            name: name.into(),
            capabilities,
        }
    }
}

bitflags! {
    /// Capabilities of an input device.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct InputDeviceCapability: u8 {
        /// Device can produce keyboard key presses.
        const KEY = 0b0000_0001;
        /// Device can produce button presses.
        const BUTTON = 0b0000_0010;
        /// Device provides scrolling wheel deltas.
        const SCROLL_MOTION = 0b0001_0000;
        /// Device provides axis aligned 1D motion.
        const AXIS_MOTION = 0b0010_0000;
        /// Device provides 2D pointer motion.
        const POINTER_MOTION = 0b0100_0000;
    }
}

/// Raw input device event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputDeviceEvent {
    /// 2D pointer motion.
    ///
    /// The values if the delta of movement (x, y), not position.
    PointerMotion {
        /// Delta of change in the cursor position.
        delta: euclid::Vector2D<f64, ()>,
    },
    /// Scroll wheel motion.
    ScrollMotion {
        /// Delta of change in the mouse scroll wheel state.
        delta: MouseScrollDelta,
    },
    /// Motion on some analog axis.
    ///
    /// This includes the mouse device and any other that fits.
    AxisMotion {
        /// Device dependent axis of the motion.
        axis: AxisId,
        /// Device dependent value.
        value: f64,
    },
    /// Device button press or release.
    Button {
        /// Device dependent button that was used.
        button: ButtonId,
        /// If the button was pressed or released.
        state: ButtonState,
    },
    /// Device key press or release.
    Key {
        /// Physical key.
        key_code: KeyCode,
        /// If the key was pressed or released.
        state: KeyState,
    },
}
impl InputDeviceEvent {
    /// Change `self` to incorporate `other` or returns `other` if both events cannot be coalesced.
    pub fn coalesce(&mut self, other: Self) -> Result<(), Self> {
        use InputDeviceEvent::*;
        match (self, other) {
            (PointerMotion { delta }, PointerMotion { delta: n_delta }) => {
                *delta += n_delta;
            }
            (
                ScrollMotion {
                    delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
                },
                ScrollMotion {
                    delta: MouseScrollDelta::LineDelta(n_delta_x, n_delta_y),
                },
            ) => {
                *delta_x += n_delta_x;
                *delta_y += n_delta_y;
            }

            (
                ScrollMotion {
                    delta: MouseScrollDelta::PixelDelta(delta_x, delta_y),
                },
                ScrollMotion {
                    delta: MouseScrollDelta::PixelDelta(n_delta_x, n_delta_y),
                },
            ) => {
                *delta_x += n_delta_x;
                *delta_y += n_delta_y;
            }
            (_, e) => return Err(e),
        }
        Ok(())
    }
}
