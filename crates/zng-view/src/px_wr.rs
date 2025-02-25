use webrender::{
    api::{self as wr},
    euclid,
};
use zng_unit::{Px, PxBox, PxCornerRadius, PxPoint, PxRect, PxSideOffsets, PxSize, PxTransform, PxVector, Rgba};
use zng_view_api::{
    AlphaType, BorderSide, BorderStyle, ExtendMode, ImageRendering, LineOrientation, LineStyle, MixBlendMode, ReferenceFrameId, RepeatMode,
    TransformStyle,
    config::FontAntiAliasing,
    display_list::{FilterOp, FrameValue, FrameValueId, FrameValueUpdate},
    font::FontOptions,
};

/// Conversion from [`Px`] to `webrender` units.
///
/// All conversions are 1 to 1.
pub trait PxToWr {
    /// `Self` equivalent in `webrender::units::LayoutPixel` units.
    type AsLayout;
    /// `Self` equivalent in `webrender::units::DevicePixel` units.
    type AsDevice;
    /// `Self` equivalent in `webrender::units::WorldPixel units.
    type AsWorld;

    /// Returns `self` in `webrender::units::DevicePixel` units.
    fn to_wr_device(self) -> Self::AsDevice;

    /// Returns `self` in `webrender::units::WorldPixel` units.
    fn to_wr_world(self) -> Self::AsWorld;

    /// Returns `self` in `webrender::units::LayoutPixel` units.
    fn to_wr(self) -> Self::AsLayout;
}

/// Conversion from `webrender` to [`Px`] units.
pub trait WrToPx {
    /// `Self` equivalent in [`Px`] units.
    type AsPx;

    /// Returns `self` in [`Px`] units.
    fn to_px(self) -> Self::AsPx;
}

impl PxToWr for Px {
    type AsDevice = wr::units::DeviceIntLength;

    type AsWorld = euclid::Length<f32, wr::units::WorldPixel>;
    type AsLayout = euclid::Length<f32, wr::units::LayoutPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::units::DeviceIntLength::new(self.0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        euclid::Length::new(self.0 as f32)
    }

    fn to_wr(self) -> Self::AsLayout {
        euclid::Length::new(self.0 as f32)
    }
}

impl PxToWr for PxPoint {
    type AsDevice = wr::units::DeviceIntPoint;
    type AsWorld = wr::units::WorldPoint;
    type AsLayout = wr::units::LayoutPoint;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::units::DeviceIntPoint::new(self.x.to_wr_device().0, self.y.to_wr_device().0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::units::WorldPoint::new(self.x.to_wr_world().0, self.y.to_wr_world().0)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::units::LayoutPoint::new(self.x.to_wr().0, self.y.to_wr().0)
    }
}
impl WrToPx for wr::units::LayoutPoint {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x.round() as i32), Px(self.y.round() as i32))
    }
}

impl PxToWr for PxSize {
    type AsDevice = wr::units::DeviceIntSize;
    type AsWorld = wr::units::WorldSize;
    type AsLayout = wr::units::LayoutSize;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::units::DeviceIntSize::new(self.width.to_wr_device().0, self.height.to_wr_device().0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::units::WorldSize::new(self.width.to_wr_world().0, self.height.to_wr_world().0)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::units::LayoutSize::new(self.width.to_wr().0, self.height.to_wr().0)
    }
}
impl WrToPx for wr::units::LayoutSize {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width.round() as i32), Px(self.height.round() as i32))
    }
}

impl WrToPx for wr::units::DeviceIntSize {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width), Px(self.height))
    }
}
impl PxToWr for PxVector {
    type AsDevice = wr::units::DeviceVector2D;

    type AsLayout = wr::units::LayoutVector2D;

    type AsWorld = wr::units::WorldVector2D;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::units::DeviceVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::units::WorldVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::units::LayoutVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }
}
impl WrToPx for wr::units::LayoutVector2D {
    type AsPx = PxVector;

    fn to_px(self) -> Self::AsPx {
        PxVector::new(Px(self.x.round() as i32), Px(self.y.round() as i32))
    }
}

impl PxToWr for PxRect {
    type AsDevice = wr::units::DeviceIntRect;

    type AsWorld = wr::units::WorldRect;

    type AsLayout = wr::units::LayoutRect;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::units::DeviceIntRect::from_origin_and_size(self.origin.to_wr_device(), self.size.to_wr_device())
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::units::WorldRect::from_origin_and_size(self.origin.to_wr_world(), self.size.to_wr_world())
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::units::LayoutRect::from_origin_and_size(self.origin.to_wr(), self.size.to_wr())
    }
}
impl WrToPx for wr::units::LayoutRect {
    type AsPx = PxRect;

    fn to_px(self) -> Self::AsPx {
        self.to_rect().to_px()
    }
}
impl WrToPx for euclid::Rect<f32, wr::units::LayoutPixel> {
    type AsPx = PxRect;

    fn to_px(self) -> Self::AsPx {
        PxRect::new(self.origin.to_px(), self.size.to_px())
    }
}

impl PxToWr for PxBox {
    type AsDevice = wr::units::DeviceBox2D;

    type AsLayout = wr::units::LayoutRect;

    type AsWorld = euclid::Box2D<f32, wr::units::WorldPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::units::DeviceBox2D::new(self.min.to_wr_device().cast(), self.max.to_wr_device().cast())
    }

    fn to_wr_world(self) -> Self::AsWorld {
        euclid::Box2D::new(self.min.to_wr_world(), self.max.to_wr_world())
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::units::LayoutRect::new(self.min.to_wr(), self.max.to_wr())
    }
}

impl PxToWr for PxSideOffsets {
    type AsDevice = wr::units::DeviceIntSideOffsets;

    type AsLayout = wr::units::LayoutSideOffsets;

    type AsWorld = euclid::SideOffsets2D<f32, wr::units::WorldPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::units::DeviceIntSideOffsets::new(
            self.top.to_wr_device().0,
            self.right.to_wr_device().0,
            self.bottom.to_wr_device().0,
            self.left.to_wr_device().0,
        )
    }

    fn to_wr_world(self) -> Self::AsWorld {
        euclid::SideOffsets2D::from_lengths(
            self.top.to_wr_world(),
            self.right.to_wr_world(),
            self.bottom.to_wr_world(),
            self.left.to_wr_world(),
        )
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::units::LayoutSideOffsets::from_lengths(self.top.to_wr(), self.right.to_wr(), self.bottom.to_wr(), self.left.to_wr())
    }
}

impl PxToWr for PxCornerRadius {
    type AsLayout = wr::BorderRadius;
    type AsDevice = ();
    type AsWorld = ();

    /// Convert to `webrender` border radius.
    fn to_wr(self) -> wr::BorderRadius {
        wr::BorderRadius {
            top_left: self.top_left.to_wr(),
            top_right: self.top_right.to_wr(),
            bottom_left: self.bottom_left.to_wr(),
            bottom_right: self.bottom_right.to_wr(),
        }
    }

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }
}

impl PxToWr for PxTransform {
    type AsDevice = euclid::Transform3D<f32, wr::units::DevicePixel, wr::units::DevicePixel>;

    type AsLayout = wr::units::LayoutTransform;

    type AsWorld = euclid::Transform3D<f32, wr::units::WorldPixel, wr::units::WorldPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        self.to_transform().with_source().with_destination()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        self.to_transform().with_source().with_destination()
    }

    fn to_wr(self) -> Self::AsLayout {
        self.to_transform().with_source().with_destination()
    }
}

// to work with to_wr
impl PxToWr for f32 {
    type AsDevice = f32;

    type AsLayout = f32;

    type AsWorld = f32;

    fn to_wr_device(self) -> Self::AsDevice {
        self
    }

    fn to_wr_world(self) -> Self::AsWorld {
        self
    }

    fn to_wr(self) -> Self::AsLayout {
        self
    }
}
// to work with to_wr
impl PxToWr for Rgba {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::ColorF;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::ColorF::new(self.red, self.green, self.blue, self.alpha)
    }
}

impl PxToWr for FilterOp {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::FilterOp;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        match self {
            FilterOp::Blur(w, h) => wr::FilterOp::Blur(w, h),
            FilterOp::Brightness(b) => wr::FilterOp::Brightness(b),
            FilterOp::Contrast(c) => wr::FilterOp::Contrast(c),
            FilterOp::Grayscale(g) => wr::FilterOp::Grayscale(g),
            FilterOp::HueRotate(h) => wr::FilterOp::HueRotate(h),
            FilterOp::Invert(i) => wr::FilterOp::Invert(i),
            FilterOp::Opacity(o) => wr::FilterOp::Opacity(o.to_wr(), *o.value()),
            FilterOp::Saturate(s) => wr::FilterOp::Saturate(s),
            FilterOp::Sepia(s) => wr::FilterOp::Sepia(s),
            FilterOp::DropShadow {
                offset,
                color,
                blur_radius,
            } => wr::FilterOp::DropShadow(wr::Shadow {
                offset: offset.cast_unit(),
                color: color.to_wr(),
                blur_radius,
            }),
            FilterOp::ColorMatrix(m) => wr::FilterOp::ColorMatrix([
                m[0], m[5], m[10], m[15], m[1], m[6], m[11], m[16], m[2], m[7], m[12], m[17], m[3], m[8], m[13], m[18], m[4], m[9], m[14],
                m[19],
            ]),
            FilterOp::Flood(c) => wr::FilterOp::Flood(c.to_wr()),
        }
    }
}

impl PxToWr for BorderSide {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::BorderSide;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::BorderSide {
            color: self.color.to_wr(),
            style: self.style.to_wr(),
        }
    }
}

impl PxToWr for BorderStyle {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::BorderStyle;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        match self {
            BorderStyle::None => wr::BorderStyle::None,
            BorderStyle::Solid => wr::BorderStyle::Solid,
            BorderStyle::Double => wr::BorderStyle::Double,
            BorderStyle::Dotted => wr::BorderStyle::Dotted,
            BorderStyle::Dashed => wr::BorderStyle::Dashed,
            BorderStyle::Hidden => wr::BorderStyle::Hidden,
            BorderStyle::Groove => wr::BorderStyle::Groove,
            BorderStyle::Ridge => wr::BorderStyle::Ridge,
            BorderStyle::Inset => wr::BorderStyle::Inset,
            BorderStyle::Outset => wr::BorderStyle::Outset,
        }
    }
}

impl<T: PxToWr> PxToWr for FrameValue<T> {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::PropertyBinding<T::AsLayout>;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        match self {
            FrameValue::Bind {
                id,
                value,
                animating: true,
            } => wr::PropertyBinding::Binding(
                wr::PropertyBindingKey {
                    id: id.to_wr(),
                    _phantom: std::marker::PhantomData,
                },
                value.to_wr(),
            ),
            FrameValue::Bind {
                value, animating: false, ..
            } => wr::PropertyBinding::Value(value.to_wr()),
            FrameValue::Value(value) => wr::PropertyBinding::Value(value.to_wr()),
        }
    }
}

impl<T: PxToWr> PxToWr for FrameValueUpdate<T> {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = Option<wr::PropertyValue<T::AsLayout>>;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout
    where
        T: PxToWr,
    {
        if self.animating {
            Some(wr::PropertyValue {
                key: wr::PropertyBindingKey {
                    id: self.id.to_wr(),
                    _phantom: std::marker::PhantomData,
                },
                value: self.value.to_wr(),
            })
        } else {
            None
        }
    }
}

impl PxToWr for FontOptions {
    type AsDevice = ();
    type AsWorld = Option<wr::GlyphOptions>;
    type AsLayout = Option<wr::FontInstanceOptions>;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        self.to_wr().map(|o| wr::GlyphOptions {
            render_mode: o.render_mode,
            flags: o.flags,
        })
    }

    fn to_wr(self) -> Self::AsLayout {
        if self == FontOptions::default() {
            None
        } else {
            Some(wr::FontInstanceOptions {
                render_mode: match self.aa {
                    FontAntiAliasing::Default => wr::FontRenderMode::Subpixel,
                    FontAntiAliasing::Subpixel => wr::FontRenderMode::Subpixel,
                    FontAntiAliasing::Alpha => wr::FontRenderMode::Alpha,
                    FontAntiAliasing::Mono => wr::FontRenderMode::Mono,
                },
                flags: if self.synthetic_bold {
                    wr::FontInstanceFlags::SYNTHETIC_BOLD
                } else {
                    wr::FontInstanceFlags::empty()
                },
                synthetic_italics: wr::SyntheticItalics::from_degrees(if self.synthetic_oblique { 14.0 } else { 0.0 }),
                _padding: 0,
            })
        }
    }
}

impl PxToWr for TransformStyle {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::TransformStyle;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        match self {
            TransformStyle::Flat => wr::TransformStyle::Flat,
            TransformStyle::Preserve3D => wr::TransformStyle::Preserve3D,
        }
    }
}

impl PxToWr for ReferenceFrameId {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::SpatialTreeItemKey;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::SpatialTreeItemKey::new(self.0, self.1)
    }
}

impl PxToWr for RepeatMode {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::RepeatMode;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        use wr::RepeatMode::*;
        match self {
            RepeatMode::Stretch => Stretch,
            RepeatMode::Repeat => Repeat,
            RepeatMode::Round => Round,
            RepeatMode::Space => Space,
        }
    }
}

impl PxToWr for MixBlendMode {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::MixBlendMode;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        use wr::MixBlendMode::*;
        match self {
            MixBlendMode::Normal => Normal,
            MixBlendMode::Multiply => Multiply,
            MixBlendMode::Screen => Screen,
            MixBlendMode::Overlay => Overlay,
            MixBlendMode::Darken => Darken,
            MixBlendMode::Lighten => Lighten,
            MixBlendMode::ColorDodge => ColorDodge,
            MixBlendMode::ColorBurn => ColorBurn,
            MixBlendMode::HardLight => HardLight,
            MixBlendMode::SoftLight => SoftLight,
            MixBlendMode::Difference => Difference,
            MixBlendMode::Exclusion => Exclusion,
            MixBlendMode::Hue => Hue,
            MixBlendMode::Saturation => Saturation,
            MixBlendMode::Color => Color,
            MixBlendMode::Luminosity => Luminosity,
            MixBlendMode::PlusLighter => PlusLighter,
        }
    }
}

impl PxToWr for ImageRendering {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::ImageRendering;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        use wr::ImageRendering::*;
        match self {
            ImageRendering::Auto => Auto,
            ImageRendering::CrispEdges => CrispEdges,
            ImageRendering::Pixelated => Pixelated,
        }
    }
}

impl PxToWr for AlphaType {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::AlphaType;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        match self {
            AlphaType::Alpha => wr::AlphaType::Alpha,
            AlphaType::PremultipliedAlpha => wr::AlphaType::PremultipliedAlpha,
        }
    }
}

impl PxToWr for ExtendMode {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::ExtendMode;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        match self {
            ExtendMode::Clamp => wr::ExtendMode::Clamp,
            ExtendMode::Repeat => wr::ExtendMode::Repeat,
        }
    }
}

impl PxToWr for LineOrientation {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::LineOrientation;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        match self {
            LineOrientation::Vertical => wr::LineOrientation::Vertical,
            LineOrientation::Horizontal => wr::LineOrientation::Horizontal,
        }
    }
}

impl PxToWr for LineStyle {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = (wr::LineStyle, f32);

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        match self {
            LineStyle::Solid => (wr::LineStyle::Solid, 0.0),
            LineStyle::Dotted => (wr::LineStyle::Dotted, 0.0),
            LineStyle::Dashed => (wr::LineStyle::Dashed, 0.0),
            LineStyle::Wavy(t) => (wr::LineStyle::Wavy, t),
        }
    }
}

impl PxToWr for FrameValueId {
    type AsDevice = ();
    type AsWorld = ();
    type AsLayout = wr::PropertyBindingId;

    fn to_wr_device(self) -> Self::AsDevice {
        unimplemented!()
    }

    fn to_wr_world(self) -> Self::AsWorld {
        unimplemented!()
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::PropertyBindingId::new(self.get())
    }
}
