use webrender::api::*;
pub use webrender::api::{LayoutRect, LayoutSize};

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
    fn render(&self, c: RenderContext);
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;
    fn arrange(&mut self, _final_size: LayoutSize) {}
}
