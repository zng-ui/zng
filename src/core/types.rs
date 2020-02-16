//! Assorted small types.

pub use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize};

pub use webrender::api::{BorderRadius, ColorF, FontInstanceKey, GlyphInstance, GlyphOptions, GradientStop};

pub use glutin::event::{
    DeviceEvent, DeviceId, ElementState, KeyboardInput, ModifiersState, MouseButton, ScanCode, VirtualKeyCode, WindowEvent,
};
pub use glutin::window::{CursorIcon, WindowId};

/// Id of a rendered or rendering window frame. Not unique across windows.
pub type FrameId = webrender::api::Epoch;

uid! {
   /// Unique id of a widget.
   pub struct WidgetId(_);
}

impl WidgetId {
    /// Creates an id from a raw value.
    ///
    /// # Safety
    ///
    /// This is only safe if called with a value provided by [WidgetId::get].
    pub unsafe fn from_raw(raw: u64) -> WidgetId {
        WidgetId(std::num::NonZeroU64::new_unchecked(raw))
    }
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

/// Opaque RGB color.
///
/// # Arguments
///
/// The arguments can either be `f32` in the `0.0..=1.0` range or
/// `u8` in the `0..=255` range.
///
/// # Example
/// ```
/// use zero_ui::core::types::rgb;
///
/// let red = rgb(1.0, 0.0, 0.0);
/// let green = rgb(0, 255, 0);
/// ```
pub fn rgb<C: Into<ColorFComponent>>(r: C, g: C, b: C) -> ColorF {
    rgba(r, g, b, 1.0)
}

/// RGBA color.
///
/// # Arguments
///
/// The arguments can either be floating pointer in the `0.0..=1.0` range or
/// integers in the `0..=255` range.
///
/// The rgb arguments must be of the same type, the alpha argument can be of a different type.
///
/// # Example
/// ```
/// use zero_ui::core::types::rgba;
///
/// let half_red = rgba(255, 0, 0, 0.5);
/// let green = rgba(0.0, 1.0, 0.0, 1.0);
/// let transparent = rgba(0, 0, 0, 0);
/// ```
pub fn rgba<C: Into<ColorFComponent>, A: Into<ColorFComponent>>(r: C, g: C, b: C, a: A) -> ColorF {
    ColorF::new(r.into().0, g.into().0, b.into().0, a.into().0)
}

/// [rgb] and [rgba] argument conversion helper.
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

/// Text string type, can be either a `&'static str` or a `String`.
pub type Text = Cow<'static, str>;

/// A trait for converting a value to a [`Text`](Text).
///
/// This trait is automatically implemented for any type which implements the [`ToString`](ToString) trait.
pub trait ToText {
    fn to_text(self) -> Text;
}

impl<T: ToString> ToText for T {
    fn to_text(self) -> Text {
        self.to_string().into()
    }
}

impl IntoVar<Text> for &'static str {
    type Var = OwnedVar<Text>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Cow::from(self))
    }
}

impl IntoVar<Text> for String {
    type Var = OwnedVar<Text>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Cow::from(self))
    }
}

impl IntoVar<LayoutPoint> for (f32, f32) {
    type Var = OwnedVar<LayoutPoint>;

    fn into_var(self) -> Self::Var {
        let (x, y) = self;
        OwnedVar(LayoutPoint::new(x, y))
    }
}

impl IntoVar<LayoutSize> for (f32, f32) {
    type Var = OwnedVar<LayoutSize>;

    fn into_var(self) -> Self::Var {
        let (w, h) = self;
        OwnedVar(LayoutSize::new(w, h))
    }
}

impl IntoVar<LayoutRect> for (f32, f32, f32, f32) {
    type Var = OwnedVar<LayoutRect>;

    fn into_var(self) -> Self::Var {
        let (x, y, w, h) = self;
        OwnedVar(LayoutRect::new(LayoutPoint::new(x, y), LayoutSize::new(w, h)))
    }
}

use std::any::type_name;
use std::marker::PhantomData;
use std::sync::atomic::{self, AtomicBool};

pub(crate) struct Singleton<S> {
    _self: PhantomData<S>,
}
impl<S> Singleton<S> {
    fn flag() -> &'static AtomicBool {
        static ALIVE: AtomicBool = AtomicBool::new(false);
        &ALIVE
    }

    pub fn assert_new() -> Self {
        if Self::flag().load(atomic::Ordering::Acquire) {
            panic!("only a single instance of `{}` can exist at at time", type_name::<S>())
        }

        Self::flag().store(true, atomic::Ordering::Release);

        Singleton { _self: PhantomData }
    }
}

impl<S> Drop for Singleton<S> {
    fn drop(&mut self) {
        Self::flag().store(false, atomic::Ordering::Release);
    }
}
