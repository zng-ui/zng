//! Pixel units.

use webrender_api::units as wr;

use zero_ui_units::*;

/// Conversion from [`Px`] to `webrender` units.
///
/// All conversions are 1 to 1.
pub trait PxToWr {
    /// `Self` equivalent in [`webrender_api::units::DevicePixel`] units.
    type AsDevice;
    /// `Self` equivalent in [`webrender_api::units::LayoutPixel`] units.
    type AsLayout;
    /// `Self` equivalent in [`webrender_api::units::WorldPixel`] units.
    type AsWorld;

    /// Returns `self` in [`webrender_api::units::DevicePixel`] units.
    fn to_wr_device(self) -> Self::AsDevice;

    /// Returns `self` in [`webrender_api::units::WorldPixel`] units.
    fn to_wr_world(self) -> Self::AsWorld;

    /// Returns `self` in [`webrender_api::units::LayoutPixel`] units.
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
    type AsDevice = wr::DeviceIntLength;

    type AsWorld = euclid::Length<f32, wr::WorldPixel>;
    type AsLayout = euclid::Length<f32, wr::LayoutPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntLength::new(self.0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        euclid::Length::new(self.0 as f32)
    }

    fn to_wr(self) -> Self::AsLayout {
        euclid::Length::new(self.0 as f32)
    }
}

impl PxToWr for PxPoint {
    type AsDevice = wr::DeviceIntPoint;
    type AsWorld = wr::WorldPoint;
    type AsLayout = wr::LayoutPoint;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntPoint::new(self.x.to_wr_device().0, self.y.to_wr_device().0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::WorldPoint::new(self.x.to_wr_world().0, self.y.to_wr_world().0)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutPoint::new(self.x.to_wr().0, self.y.to_wr().0)
    }
}
impl WrToPx for wr::LayoutPoint {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x.round() as i32), Px(self.y.round() as i32))
    }
}

impl PxToWr for PxSize {
    type AsDevice = wr::DeviceIntSize;
    type AsWorld = wr::WorldSize;
    type AsLayout = wr::LayoutSize;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntSize::new(self.width.to_wr_device().0, self.height.to_wr_device().0)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::WorldSize::new(self.width.to_wr_world().0, self.height.to_wr_world().0)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutSize::new(self.width.to_wr().0, self.height.to_wr().0)
    }
}
impl WrToPx for wr::LayoutSize {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width.round() as i32), Px(self.height.round() as i32))
    }
}

impl WrToPx for wr::DeviceIntSize {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width), Px(self.height))
    }
}
impl PxToWr for PxVector {
    type AsDevice = wr::DeviceVector2D;

    type AsLayout = wr::LayoutVector2D;

    type AsWorld = wr::WorldVector2D;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::WorldVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutVector2D::new(self.x.0 as f32, self.y.0 as f32)
    }
}
impl WrToPx for wr::LayoutVector2D {
    type AsPx = PxVector;

    fn to_px(self) -> Self::AsPx {
        PxVector::new(Px(self.x.round() as i32), Px(self.y.round() as i32))
    }
}

impl PxToWr for PxRect {
    type AsDevice = wr::DeviceIntRect;

    type AsWorld = wr::WorldRect;

    type AsLayout = wr::LayoutRect;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntRect::from_origin_and_size(self.origin.to_wr_device(), self.size.to_wr_device())
    }

    fn to_wr_world(self) -> Self::AsWorld {
        wr::WorldRect::from_origin_and_size(self.origin.to_wr_world(), self.size.to_wr_world())
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutRect::from_origin_and_size(self.origin.to_wr(), self.size.to_wr())
    }
}
impl WrToPx for wr::LayoutRect {
    type AsPx = PxRect;

    fn to_px(self) -> Self::AsPx {
        self.to_rect().to_px()
    }
}
impl WrToPx for euclid::Rect<f32, wr::LayoutPixel> {
    type AsPx = PxRect;

    fn to_px(self) -> Self::AsPx {
        PxRect::new(self.origin.to_px(), self.size.to_px())
    }
}

impl PxToWr for PxBox {
    type AsDevice = wr::DeviceBox2D;

    type AsLayout = wr::LayoutRect;

    type AsWorld = euclid::Box2D<f32, wr::WorldPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceBox2D::new(self.min.to_wr_device().cast(), self.max.to_wr_device().cast())
    }

    fn to_wr_world(self) -> Self::AsWorld {
        euclid::Box2D::new(self.min.to_wr_world(), self.max.to_wr_world())
    }

    fn to_wr(self) -> Self::AsLayout {
        wr::LayoutRect::new(self.min.to_wr(), self.max.to_wr())
    }
}

impl PxToWr for PxSideOffsets {
    type AsDevice = wr::DeviceIntSideOffsets;

    type AsLayout = wr::LayoutSideOffsets;

    type AsWorld = euclid::SideOffsets2D<f32, wr::WorldPixel>;

    fn to_wr_device(self) -> Self::AsDevice {
        wr::DeviceIntSideOffsets::new(
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
        wr::LayoutSideOffsets::from_lengths(self.top.to_wr(), self.right.to_wr(), self.bottom.to_wr(), self.left.to_wr())
    }
}

impl PxToWr for PxCornerRadius {
    type AsLayout = webrender_api::BorderRadius;
    type AsDevice = ();
    type AsWorld = ();

    /// Convert to `webrender` border radius.
    fn to_wr(self) -> webrender_api::BorderRadius {
        webrender_api::BorderRadius {
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
    type AsDevice = euclid::Transform3D<f32, wr::DevicePixel, wr::DevicePixel>;

    type AsLayout = wr::LayoutTransform;

    type AsWorld = euclid::Transform3D<f32, wr::WorldPixel, wr::WorldPixel>;

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
