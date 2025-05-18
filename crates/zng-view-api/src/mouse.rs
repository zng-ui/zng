//! Mouse types.

use serde::{Deserialize, Serialize};

use zng_unit::Px;

/// Identifier for a specific button on some device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ButtonId(pub u32);

/// State a [`MouseButton`] has entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ButtonState {
    /// The button was pressed.
    Pressed,
    /// The button was released.
    Released,
}

/// Describes a button of a mouse controller.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MouseButton {
    /// Left button.
    Left,
    /// Right button.
    Right,
    /// Middle button.
    Middle,
    /// Back button.
    Back,
    /// Forward button.
    Forward,
    /// Any other button.
    Other(u16),
}

/// Describes a difference in the mouse scroll wheel state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MouseScrollDelta {
    /// Amount in lines or rows to scroll in the horizontal
    /// and vertical directions.
    ///
    /// Positive values indicate rightwards movement in the X axis and downwards movement in the Y axis.
    LineDelta(f32, f32),
    /// Amount in pixels to scroll in the horizontal and
    /// vertical direction.
    ///
    /// Scroll events are expressed as a pixel delta if
    /// supported by the device (eg. a touchpad) and
    /// platform.
    PixelDelta(f32, f32),
}
impl MouseScrollDelta {
    /// Gets the sign status of x and y.
    ///
    /// Positive values indicate rightwards movement in the X axis and downwards movement in the Y axis.
    pub fn is_sign_positive(self) -> euclid::BoolVector2D {
        match self {
            MouseScrollDelta::LineDelta(x, y) | MouseScrollDelta::PixelDelta(x, y) => euclid::BoolVector2D {
                x: x.is_sign_positive(),
                y: y.is_sign_positive(),
            },
        }
    }

    /// Gets the sign status of x and y.
    ///
    /// Negative values indicate leftwards movement in the X axis and upwards movement in the Y axis.
    pub fn is_sign_negative(self) -> euclid::BoolVector2D {
        self.is_sign_positive().not()
    }

    /// Gets the pixel delta, line delta is converted using the `line_size`.
    pub fn delta(self, line_size: euclid::Size2D<f32, Px>) -> euclid::Vector2D<f32, Px> {
        match self {
            MouseScrollDelta::LineDelta(x, y) => euclid::vec2(line_size.width * x, line_size.height * y),
            MouseScrollDelta::PixelDelta(x, y) => euclid::vec2(x, y),
        }
    }
}
