//! Touch types.

use serde::{Deserialize, Serialize};

use zng_unit::DipPoint;

/// Identifier for a continuous touch contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TouchId(pub u64);

/// Describes touch-screen input state.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum TouchPhase {
    /// A finger touched the screen.
    Start,
    /// A finger moved on the screen.
    Move,
    /// A finger was lifted from the screen.
    End,
    /// The system cancelled tracking for the touch.
    Cancel,
}

/// Identify a new touch contact or a contact update.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct TouchUpdate {
    /// Identify a continuous touch contact or *finger*.
    ///
    /// Multiple points of contact can happen in the same device at the same time,
    /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
    /// on the same device, after a touch is ended an ID may be reused.
    pub touch: TouchId,
    /// Touch phase for the `id`.
    pub phase: TouchPhase,
    /// Touch center, relative to the window top-left in device independent pixels.
    pub position: DipPoint,
    /// Touch pressure force and angle.
    pub force: Option<TouchForce>,
}
impl TouchUpdate {
    /// New update.
    pub fn new(touch: TouchId, phase: TouchPhase, position: DipPoint, force: Option<TouchForce>) -> Self {
        Self {
            touch,
            phase,
            position,
            force,
        }
    }
}

/// Describes the force of a touch event.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TouchForce {
    /// On iOS, the force is calibrated so that the same number corresponds to
    /// roughly the same amount of pressure on the screen regardless of the
    /// device.
    Calibrated {
        /// The force of the touch, where a value of 1.0 represents the force of
        /// an average touch (predetermined by the system, not user-specific).
        ///
        /// The force reported by Apple Pencil is measured along the axis of the
        /// pencil. If you want a force perpendicular to the device, you need to
        /// calculate this value using the `altitude_angle` value.
        force: f64,
        /// The maximum possible force for a touch.
        ///
        /// The value of this field is sufficiently high to provide a wide
        /// dynamic range for values of the `force` field.
        max_possible_force: f64,
        /// The altitude (in radians) of the stylus.
        ///
        /// A value of 0 radians indicates that the stylus is parallel to the
        /// surface. The value of this property is Pi/2 when the stylus is
        /// perpendicular to the surface.
        altitude_angle: Option<f64>,
    },
    /// If the platform reports the force as normalized, we have no way of
    /// knowing how much pressure 1.0 corresponds to – we know it's the maximum
    /// amount of force, but as to how much force, you might either have to
    /// press really hard, or not hard at all, depending on the device.
    Normalized(f64),
}
