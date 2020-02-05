//! Assorted small types.

pub use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize};

pub use webrender::api::{BorderRadius, ColorF, FontInstanceKey, GlyphInstance, GlyphOptions, GradientStop};

pub use glutin::event::{
    DeviceEvent, DeviceId, ElementState, KeyboardInput, ModifiersState, MouseButton, ScanCode, VirtualKeyCode, WindowEvent,
};
pub use glutin::window::{CursorIcon, WindowId};

uid! {
   /// Unique id of a widget.
   pub struct WidgetId(_);
}

use crate::core::var::{IntoVar, OwnedVar};
use std::borrow::Cow;

/// for uniform
impl IntoVar<LayoutSideOffsets> for f32 {
    type Var = OwnedVar<LayoutSideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(LayoutSideOffsets::new_all_same(self))
    }
}

///for (top-bottom, left-right)
impl IntoVar<LayoutSideOffsets> for (f32, f32) {
    type Var = OwnedVar<LayoutSideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(LayoutSideOffsets::new(self.0, self.1, self.0, self.1))
    }
}

///for (top, right, bottom, left)
impl IntoVar<LayoutSideOffsets> for (f32, f32, f32, f32) {
    type Var = OwnedVar<LayoutSideOffsets>;

    fn into_var(self) -> Self::Var {
        OwnedVar(LayoutSideOffsets::new(self.0, self.1, self.2, self.3))
    }
}

pub fn rgb<C: Into<ColorFComponent>>(r: C, g: C, b: C) -> ColorF {
    rgba(r, g, b, 1.0)
}

pub fn rgba<C: Into<ColorFComponent>, A: Into<ColorFComponent>>(r: C, g: C, b: C, a: A) -> ColorF {
    ColorF::new(r.into().0, g.into().0, b.into().0, a.into().0)
}

pub fn default<D: Default>() -> D {
    D::default()
}

/// `ColorF` component value.
pub struct ColorFComponent(pub f32);

impl From<f32> for ColorFComponent {
    fn from(f: f32) -> Self {
        ColorFComponent(f)
    }
}

impl From<u8> for ColorFComponent {
    fn from(u: u8) -> Self {
        ColorFComponent(f32::from(u) / 255.)
    }
}

impl IntoVar<Vec<GradientStop>> for Vec<(f32, ColorF)> {
    type Var = OwnedVar<Vec<GradientStop>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into_iter().map(|(offset, color)| GradientStop { offset, color }).collect())
    }
}

impl IntoVar<Vec<GradientStop>> for Vec<ColorF> {
    type Var = OwnedVar<Vec<GradientStop>>;

    fn into_var(self) -> Self::Var {
        let point = 1. / (self.len() as f32 - 1.);
        OwnedVar(
            self.into_iter()
                .enumerate()
                .map(|(i, color)| GradientStop {
                    offset: (i as f32) * point,
                    color,
                })
                .collect(),
        )
    }
}

impl IntoVar<Cow<'static, str>> for &'static str {
    type Var = OwnedVar<Cow<'static, str>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Cow::from(self))
    }
}
