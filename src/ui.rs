use webrender::api::*;
pub use webrender::api::{LayoutRect, LayoutSize};

pub fn push_child_context(
    builder: &mut DisplayListBuilder,
    spatial_id: SpatialId,
    final_rect: &LayoutRect,
) -> SpatialId {
    builder.push_reference_frame(
        final_rect,
        spatial_id,
        TransformStyle::Flat,
        PropertyBinding::Value(LayoutTransform::default()),
        ReferenceFrameKind::Transform,
    )
}

pub fn pop_child_context(builder: &mut DisplayListBuilder) {
    builder.pop_reference_frame();
}

pub trait Ui {
    fn render(&self, builder: &mut DisplayListBuilder, spatial_id: SpatialId, final_size: LayoutSize);
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;
    fn arrange(&mut self, _final_size: LayoutSize) {}
}
