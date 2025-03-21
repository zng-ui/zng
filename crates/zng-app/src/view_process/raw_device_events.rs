//! Events directly from view-process not targeting any windows.
//!
//! These events get emitted only if the app [`enable_device_events`]. When enabled they
//! can be used like [`raw_events`].
//!
//! [`enable_device_events`]: crate::AppExtended::enable_device_events
//! [`raw_events`]: crate::view_process::raw_events

use std::fmt;

use crate::event::*;

use zng_layout::unit::euclid;
use zng_view_api::{
    AxisId,
    keyboard::{KeyCode, KeyState},
    mouse::{ButtonId, ButtonState, MouseScrollDelta},
};

use once_cell::sync::Lazy;

zng_unique_id::unique_id_64! {
    /// Unique identifier of a device event source.
    pub struct DeviceId;
}
zng_unique_id::impl_unique_id_bytemuck!(DeviceId);
impl DeviceId {
    /// Virtual keyboard ID used in keyboard events generated by code.
    pub fn virtual_keyboard() -> DeviceId {
        static ID: Lazy<DeviceId> = Lazy::new(DeviceId::new_unique);
        *ID
    }

    /// Virtual mouse ID used in mouse events generated by code.
    pub fn virtual_mouse() -> DeviceId {
        static ID: Lazy<DeviceId> = Lazy::new(DeviceId::new_unique);
        *ID
    }

    /// Virtual generic device ID used in device events generated by code.
    pub fn virtual_generic() -> DeviceId {
        static ID: Lazy<DeviceId> = Lazy::new(DeviceId::new_unique);
        *ID
    }
}
impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("DeviceId")
                .field("id", &self.get())
                .field("sequential", &self.sequential())
                .finish()
        } else {
            write!(f, "DeviceId({})", self.sequential())
        }
    }
}
impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeviceId({})", self.get())
    }
}

event_args! {
    /// Arguments for [`DEVICE_ADDED_EVENT`] and [`DEVICE_REMOVED_EVENT`].
    pub struct DeviceArgs {
        /// Device that was added/removed.
        pub device_id: DeviceId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for [`MOUSE_MOTION_EVENT`].
    pub struct MouseMotionArgs {
        /// Mouse device that generated the event.
        pub device_id: DeviceId,

        /// Motion (x, y) delta.
        pub delta: euclid::Vector2D<f64, ()>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for [`MOUSE_WHEEL_EVENT`].
    pub struct MouseWheelArgs {
        /// Mouse device that generated the event.
        pub device_id: DeviceId,

        /// Wheel motion delta, value is in pixels if the *wheel* is a touchpad.
        pub delta: MouseScrollDelta,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for [`MOTION_EVENT`].
    pub struct MotionArgs {
        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Analog axis.
        pub axis: AxisId,

        /// Motion amount.
        pub value: f64,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`BUTTON_EVENT`].
    pub struct ButtonArgs {
        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Button raw id.
        pub button: ButtonId,

        /// If the button was pressed or released.
        pub state: ButtonState,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`KEY_EVENT`].
    pub struct KeyArgs {
        /// Keyboard device that generated the event.
        pub device_id: DeviceId,

        /// Physical key.
        pub key_code: KeyCode,

        /// If the key was pressed or released.
        pub state: KeyState,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`TEXT_EVENT`].
    pub struct TextArgs {
        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Character received.
        pub code_point: char,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}

event! {
    /// A device event source was added/installed.
    pub static DEVICE_ADDED_EVENT: DeviceArgs;

    /// A device event source was removed/un-installed.
    pub static DEVICE_REMOVED_EVENT: DeviceArgs;

    /// Mouse device unfiltered move delta.
    pub static MOUSE_MOTION_EVENT: MouseMotionArgs;

    /// Mouse device unfiltered wheel motion delta.
    pub static MOUSE_WHEEL_EVENT: MouseWheelArgs;

    /// Motion on some analog axis.
    ///
    /// This event will be reported for all arbitrary input devices that the view-process supports on this platform,
    /// including mouse devices. If the device is a mouse device then this will be reported alongside the [`MOUSE_MOTION_EVENT`].
    pub static MOTION_EVENT: MotionArgs;

    /// Button press/release from a device, probably a mouse.
    pub static BUTTON_EVENT: ButtonArgs;

    /// Keyboard device key press.
    pub static KEY_EVENT: KeyArgs;

    /// Raw text input.
    pub static TEXT_EVENT: TextArgs;
}
