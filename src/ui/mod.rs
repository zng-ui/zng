mod layout;

pub use layout::*;

use webrender::api::*;
pub use webrender::api::{LayoutPoint, LayoutRect, LayoutSize};

pub struct RenderContext<'b> {
    pub builder: &'b mut DisplayListBuilder,
    spatial_id: SpatialId,
    final_size: LayoutSize,
}
impl<'b> RenderContext<'b> {
    pub fn new(builder: &'b mut DisplayListBuilder, spatial_id: SpatialId, final_size: LayoutSize) -> Self {
        RenderContext {
            builder,
            spatial_id,
            final_size,
        }
    }

    pub fn push_child(&mut self, child: &impl Ui, final_rect: &LayoutRect) {
        let spatial_id = self.builder.push_reference_frame(
            final_rect,
            self.spatial_id,
            TransformStyle::Flat,
            PropertyBinding::Value(LayoutTransform::default()),
            ReferenceFrameKind::Transform,
        );
        child.render(RenderContext::new(self.builder, spatial_id, final_rect.size));
        self.builder.pop_reference_frame();
    }
}

impl<'b> RenderContext<'b> {
    pub fn spatial_id(&self) -> SpatialId {
        self.spatial_id
    }
    pub fn final_size(&self) -> LayoutSize {
        self.final_size
    }
}

pub trait Ui {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;
    fn arrange(&mut self, _final_size: LayoutSize) {}
    fn render(&self, c: RenderContext);
}

impl Ui for Box<dyn Ui> {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.as_mut().measure(available_size)
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        self.as_mut().arrange(final_size)
    }
    fn render(&self, c: RenderContext) {
        self.as_ref().render(c)
    }
}

pub struct Rect {
    color: ColorF,
}

impl Rect {
    pub fn new(color: ColorF) -> Self {
        Rect { color }
    }
}

impl Ui for Rect {
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        if available_size.width.is_infinite() {
            available_size.width = 0.;
        }

        if available_size.height.is_infinite() {
            available_size.height = 0.;
        }

        available_size
    }

    fn render(&self, c: RenderContext) {
        let lpi = LayoutPrimitiveInfo::new(LayoutRect::from_size(c.final_size()));
        let sci = SpaceAndClipInfo {
            spatial_id: c.spatial_id(),
            clip_id: ClipId::root(c.spatial_id().pipeline_id()),
        };
        c.builder.push_rect(&lpi, &sci, self.color);
    }
}
