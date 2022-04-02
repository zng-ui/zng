//! Events directly from `winit` not targeting any windows.
//!
//! These events get emitted only if the app [`enable_device_events`]. When enabled they
//! can be used like [`raw_events`].
//!
//! [`enable_device_events`]: crate::app::AppExtended::enable_device_events
//! [`raw_events`]: crate::app::raw_events

use super::DeviceId;
use crate::{
    event::*,
    keyboard::{Key, KeyState, ScanCode},
    mouse::{ButtonState, MouseScrollDelta},
};

use zero_ui_view_api::webrender_api::euclid;
pub use zero_ui_view_api::{AxisId, ButtonId};

event_args! {
    /// Arguments for [`DeviceAddedEvent`] and [`DeviceRemovedEvent`].
    pub struct DeviceArgs {
        /// Device that was added/removed.
        pub device_id: DeviceId,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for [`MouseMotionEvent`].
    pub struct MouseMotionArgs {
        /// Mouse device that generated the event.
        pub device_id: DeviceId,

        /// Motion (x, y) delta.
        pub delta: euclid::Vector2D<f64, ()>,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for [`MouseWheelEvent`].
    pub struct MouseWheelArgs {
        /// Mouse device that generated the event.
        pub device_id: DeviceId,

        /// Wheel motion delta, value is in pixels if the *wheel* is a touchpad.
        pub delta: MouseScrollDelta,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for [`MotionEvent`].
    pub struct MotionArgs {
        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Analog axis.
        pub axis: AxisId,

        /// Motion amount.
        pub value: f64,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`ButtonEvent`].
    pub struct ButtonArgs {
        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Button raw id.
        pub button: ButtonId,

        /// If the button was pressed or released.
        pub state: ButtonState,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`KeyEvent`].
    pub struct KeyArgs {
        /// Keyboard device that generated the event.
        pub device_id: DeviceId,

        /// Raw code of key.
        pub scan_code: ScanCode,

        /// If the key was pressed or released.
        pub state: KeyState,

        /// Symbolic name of [`scan_code`](Self::scan_code).
        pub key: Option<Key>,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`TextEvent`].
    pub struct TextArgs {
        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Character received.
        pub code_point: char,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }
}

event! {
    /// A device event source was added/installed.
    pub DeviceAddedEvent: DeviceArgs;

    /// A device event source was removed/un-installed.
    pub DeviceRemovedEvent: DeviceArgs;

    /// Mouse device unfiltered move delta.
    pub MouseMotionEvent: MouseMotionArgs;

    /// Mouse device unfiltered wheel motion delta.
    pub MouseWheelEvent: MouseWheelArgs;

    /// Motion on some analog axis.
    ///
    /// This event will be reported for all arbitrary input devices that `winit` supports on this platform,
    /// including mouse devices. If the device is a mouse device then this will be reported alongside the [`MouseMotionEvent`].
    pub MotionEvent: MotionArgs;

    /// Button press/release from a device, probably a mouse.
    pub ButtonEvent: ButtonArgs;

    /// Keyboard device key press.
    pub KeyEvent: KeyArgs;

    /// Raw text input.
    pub TextEvent: TextArgs;
}
