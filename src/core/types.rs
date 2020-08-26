//! Assorted small types.
pub type Angle = euclid::Angle<f32>;

pub use webrender::api::units::{LayoutPoint, LayoutRect, LayoutSideOffsets, LayoutSize, LayoutTransform};

pub use webrender::api::{BorderRadius, ColorF, FontInstanceKey, GlyphInstance, GlyphOptions, GradientStop, LineOrientation};

pub use font_kit::properties::{Properties as FontProperties, Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};

pub use glutin::event::{
    DeviceEvent, DeviceId, ElementState, KeyboardInput, ModifiersState, MouseButton, ScanCode, VirtualKeyCode, WindowEvent,
};
pub use glutin::window::{CursorIcon, WindowId};

/// Id of a rendered or rendering window frame. Not unique across windows.
pub type FrameId = webrender::api::Epoch;

unique_id! {
    /// Unique id of a widget.
    ///
    /// # Details
    /// Underlying value is a `NonZeroU64` generated using a relaxed global atomic `fetch_add`,
    /// so IDs are unique for the process duration, but order is not guaranteed.
    ///
    /// Panics if you somehow reach `u64::max_value()` calls to `new`.
    pub WidgetId;
}

use crate::core::var::{IntoVar, OwnedVar};
use std::{borrow::Cow, fmt};

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

/// [`rgb`] and [`rgba`] argument conversion helper.
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

pub fn rotate(degrees: f32) -> LayoutTransform {
    LayoutTransform::create_rotation(0.0, 0.0, -1.0, Angle::degrees(degrees))
}

pub trait Units{
    fn deg(self) -> Angle;
    fn rad(self) -> Angle;
    fn grad(self) -> Angle;
    fn turn(self) -> Angle;
}
impl Units for f32{
    fn deg(self) -> Angle {
        self.to_radians();
        Angle::degrees(self)
    }
    fn rad(self) -> Angle {
        Angle::radians(self)
    }

    fn grad(self) -> Angle {
        Angle::radians(self * std::f32::consts::PI / 200.0)
    }

    fn turn(self) -> Angle {
        Angle::radians(self * std::f32::consts::PI * 2.0f32)
    }
}

#[cfg(test)]
mod unit_tests{
    use super::Units;
    //shortcut for testing if two f32 values are equal accounting for the imprecision inherit in float/f32 calculations
    fn about_equal(a: f32, b: f32) -> bool {
        (a - b).abs() < f32::EPSILON
    }

    //1.0 turn is equal to 360 degrees
    #[test]
    fn turn(){
         assert!(about_equal(1.0.turn().to_degrees(), 360.0))
    }

    //400 gradians is equal to 360 degrees
    #[test]
    fn grad(){
        assert!(about_equal(400.0.grad().to_degrees(), 360.0))
    }
}

/// Text string type, can be either a `&'static str` or a `String`.
pub type Text = Cow<'static, str>;

/// A trait for converting a value to a [`Text`].
///
/// This trait is automatically implemented for any type which implements the [`ToString`] trait.
///
/// You can use [`formatx!`](macro.formatx.html) to `format!` a text.
pub trait ToText {
    fn to_text(self) -> Text;
}

impl<T: ToString> ToText for T {
    fn to_text(self) -> Text {
        self.to_string().into()
    }
}

pub use zero_ui_macros::formatx;

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

/// A type that can be a [`property`]((../zero_ui/attr.property.html)) argument for properties that can be used in when expressions.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait ArgWhenCompatible: Clone {}

impl<T: Clone> ArgWhenCompatible for T {}

pub use bezier::*;

mod bezier {
    /* This Source Code Form is subject to the terms of the Mozilla Public
     * License, v. 2.0. If a copy of the MPL was not distributed with this
     * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

    const NEWTON_METHOD_ITERATIONS: u8 = 8;

    /// A unit cubic Bézier curve, used for timing functions in CSS transitions and animations.
    pub struct Bezier {
        ax: f64,
        bx: f64,
        cx: f64,
        ay: f64,
        by: f64,
        cy: f64,
    }

    impl Bezier {
        /// Create a unit cubic Bézier curve from the two middle control points.
        ///
        /// X coordinate is time, Y coordinate is function advancement.
        /// The nominal range for both is 0 to 1.
        ///
        /// The start and end points are always (0, 0) and (1, 1) so that a transition or animation
        /// starts at 0% and ends at 100%.
        #[inline]
        pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Bezier {
            let cx = 3. * x1 as f64;
            let bx = 3. * (x2 as f64 - x1 as f64) - cx;

            let cy = 3. * y1 as f64;
            let by = 3. * (y2 as f64 - y1 as f64) - cy;

            Bezier {
                ax: 1.0 - cx - bx,
                bx,
                cx,
                ay: 1.0 - cy - by,
                by,
                cy,
            }
        }

        #[inline]
        fn sample_curve_x(&self, t: f64) -> f64 {
            // ax * t^3 + bx * t^2 + cx * t
            ((self.ax * t + self.bx) * t + self.cx) * t
        }

        #[inline]
        fn sample_curve_y(&self, t: f64) -> f64 {
            ((self.ay * t + self.by) * t + self.cy) * t
        }

        #[inline]
        fn sample_curve_derivative_x(&self, t: f64) -> f64 {
            (3.0 * self.ax * t + 2.0 * self.bx) * t + self.cx
        }

        #[inline]
        fn solve_curve_x(&self, x: f64, epsilon: f64) -> f64 {
            // Fast path: Use Newton's method.
            let mut t = x;
            for _ in 0..NEWTON_METHOD_ITERATIONS {
                let x2 = self.sample_curve_x(t);
                if x2.approx_eq(x, epsilon) {
                    return t;
                }
                let dx = self.sample_curve_derivative_x(t);
                if dx.approx_eq(0.0, 1e-6) {
                    break;
                }
                t -= (x2 - x) / dx;
            }

            // Slow path: Use bisection.
            let (mut lo, mut hi, mut t) = (0.0, 1.0, x);

            if t < lo {
                return lo;
            }
            if t > hi {
                return hi;
            }

            while lo < hi {
                let x2 = self.sample_curve_x(t);
                if x2.approx_eq(x, epsilon) {
                    return t;
                }
                if x > x2 {
                    lo = t
                } else {
                    hi = t
                }
                t = (hi - lo) / 2.0 + lo
            }

            t
        }

        /// Solve the bezier curve for a given `x` and an `epsilon`, that should be
        /// between zero and one.
        #[inline]
        pub fn solve(&self, x: f64, epsilon: f64) -> f64 {
            self.sample_curve_y(self.solve_curve_x(x, epsilon))
        }
    }

    trait ApproxEq {
        fn approx_eq(self, value: Self, epsilon: Self) -> bool;
    }

    impl ApproxEq for f64 {
        #[inline]
        fn approx_eq(self, value: f64, epsilon: f64) -> bool {
            (self - value).abs() < epsilon
        }
    }
}

use font_kit::family_name::FamilyName;

/// A possible value for the `font_family` property.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontName(Text);

impl FontName {
    #[inline]
    pub fn new(name: impl Into<Text>) -> Self {
        FontName(name.into())
    }

    /// New "serif" font.
    ///
    /// Serif fonts represent the formal text style for a script.
    #[inline]
    pub fn serif() -> Self {
        Self::new("serif")
    }

    /// New "sans-serif" font.
    ///
    /// Glyphs in sans-serif fonts, are generally low contrast (vertical and horizontal stems have the close to the same thickness)
    /// and have stroke endings that are plain — without any flaring, cross stroke, or other ornamentation.
    #[inline]
    pub fn sans_serif() -> Self {
        Self::new("sans-serif")
    }

    /// New "monospace" font.
    ///
    /// The sole criterion of a monospace font is that all glyphs have the same fixed width.
    #[inline]
    pub fn monospace() -> Self {
        Self::new("monospace")
    }

    /// New "cursive" font.
    ///
    /// Glyphs in cursive fonts generally use a more informal script style, and the result looks more
    /// like handwritten pen or brush writing than printed letter-work.
    #[inline]
    pub fn cursive() -> Self {
        Self::new("cursive")
    }

    /// New "fantasy" font.
    ///
    /// Fantasy fonts are primarily decorative or expressive fonts that contain decorative or expressive representations of characters.
    #[inline]
    pub fn fantasy() -> Self {
        Self::new("fantasy")
    }

    /// Reference the font name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.0
    }
}
impl From<FamilyName> for FontName {
    #[inline]
    fn from(family_name: FamilyName) -> Self {
        match family_name {
            FamilyName::Title(title) => FontName::new(title),
            FamilyName::Serif => FontName::serif(),
            FamilyName::SansSerif => FontName::sans_serif(),
            FamilyName::Monospace => FontName::monospace(),
            FamilyName::Cursive => FontName::cursive(),
            FamilyName::Fantasy => FontName::fantasy(),
        }
    }
}
impl From<FontName> for FamilyName {
    fn from(font_name: FontName) -> Self {
        match font_name.name() {
            "serif" => FamilyName::Serif,
            "sans-serif" => FamilyName::SansSerif,
            "monospace" => FamilyName::Monospace,
            "cursive" => FamilyName::Cursive,
            "fantasy" => FamilyName::Fantasy,
            _ => FamilyName::Title(font_name.0.into()),
        }
    }
}
impl fmt::Display for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

pub type FontSize = u32;

impl IntoVar<Box<[FontName]>> for &'static str {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Box::new([FontName::new(self)]))
    }
}
impl IntoVar<Box<[FontName]>> for String {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Box::new([FontName::new(self)]))
    }
}
impl IntoVar<Box<[FontName]>> for Text {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Box::new([FontName(self)]))
    }
}
impl IntoVar<Box<[FontName]>> for Vec<FontName> {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into_boxed_slice())
    }
}
impl IntoVar<Box<[FontName]>> for Vec<&'static str> {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into_iter().map(FontName::new).collect::<Vec<FontName>>().into_boxed_slice())
    }
}
impl IntoVar<Box<[FontName]>> for Vec<String> {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into_iter().map(FontName::new).collect::<Vec<FontName>>().into_boxed_slice())
    }
}

/// A device pixel scale factor used for pixel alignment.
///
/// Types that can be aligned with this grid implement [`PixelGridExt`].
#[derive(Copy, Clone, Debug)]
pub struct PixelGrid {
    pub scale_factor: f32,
}
impl PixelGrid {
    #[inline]
    pub fn new(scale_factor: f32) -> Self {
        PixelGrid { scale_factor }
    }

    /// Aligns the layout value `n` using this algorithm:
    ///
    /// scaled `n` | op
    /// -----------|------------------------
    /// < 0.01     | floor (`0`)
    /// < 1.0      | ceil (`1` pixel)
    /// >= 1.0     | round to nearest pixel
    #[inline]
    pub fn snap(self, layout_value: f32) -> f32 {
        let px = layout_value * self.scale_factor;
        if px < 0.01 {
            0.0
        } else if px < 1.0 {
            1.0 / self.scale_factor
        } else {
            px.round() / self.scale_factor
        }
    }

    /// Checks if the layout value is aligned with this grid.
    #[inline]
    pub fn is_aligned(self, layout_value: f32) -> bool {
        let scaled = layout_value * self.scale_factor;
        (scaled - scaled.round()).abs() < 0.0001
    }
}
impl Default for PixelGrid {
    /// `1.0` scale factor.
    #[inline]
    fn default() -> Self {
        PixelGrid::new(1.0)
    }
}
impl PartialEq for PixelGrid {
    fn eq(&self, other: &Self) -> bool {
        (self.scale_factor - other.scale_factor).abs() < 0.01
    }
}

/// Methods for types that can be aligned to a [`PixelGrid`].
pub trait PixelGridExt {
    /// Gets a copy of self that is aligned with the pixel grid.
    fn snap_to(self, grid: PixelGrid) -> Self;
    /// Checks if self is aligned with the pixel grid.
    fn is_aligned_to(self, grid: PixelGrid) -> bool;
}

impl PixelGridExt for LayoutPoint {
    #[inline]
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutPoint::new(grid.snap(self.x), grid.snap(self.y))
    }
    #[inline]
    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        grid.is_aligned(self.x) && grid.is_aligned(self.y)
    }
}

impl PixelGridExt for LayoutSize {
    #[inline]
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutSize::new(grid.snap(self.width), grid.snap(self.height))
    }
    #[inline]
    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        grid.is_aligned(self.width) && grid.is_aligned(self.height)
    }
}

impl PixelGridExt for LayoutRect {
    #[inline]
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutRect::new(self.origin.snap_to(grid), self.size.snap_to(grid))
    }
    #[inline]
    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        self.origin.is_aligned_to(grid) && self.size.is_aligned_to(grid)
    }
}

impl PixelGridExt for LayoutSideOffsets {
    #[inline]
    fn snap_to(self, grid: PixelGrid) -> Self {
        LayoutSideOffsets::new(
            grid.snap(self.top),
            grid.snap(self.right),
            grid.snap(self.bottom),
            grid.snap(self.left),
        )
    }
    #[inline]
    fn is_aligned_to(self, grid: PixelGrid) -> bool {
        grid.is_aligned(self.top) && grid.is_aligned(self.right) && grid.is_aligned(self.bottom) && grid.is_aligned(self.left)
    }
}
