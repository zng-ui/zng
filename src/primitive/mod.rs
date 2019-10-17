#[cfg(test)]
pub mod test;

mod border;
mod color;
mod event;
mod cursor;
mod layout;
mod log;
mod focus;
mod parent_value;
mod stack;
mod text;

pub use self::log::*;
use crate::core::{IntoValue, LayoutSideOffsets, Owned};
pub use border::*;
pub use color::*;
pub use cursor::*;
pub use event::*;
pub use focus::*;
pub use glutin::event::{ElementState, ModifiersState, MouseButton, ScanCode, VirtualKeyCode};
pub use glutin::window::CursorIcon;
pub use layout::*;
pub use parent_value::*;
pub use stack::*;
pub use text::*;

/// for uniform
impl IntoValue<LayoutSideOffsets> for f32 {
    type Value = Owned<LayoutSideOffsets>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutSideOffsets::new_all_same(self))
    }
}

///for (top-bottom, left-right)
impl IntoValue<LayoutSideOffsets> for (f32, f32) {
    type Value = Owned<LayoutSideOffsets>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutSideOffsets::new(self.0, self.1, self.0, self.1))
    }
}

///for (top, right, bottom, left)
impl IntoValue<LayoutSideOffsets> for (f32, f32, f32, f32) {
    type Value = Owned<LayoutSideOffsets>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutSideOffsets::new(self.0, self.1, self.2, self.3))
    }
}
