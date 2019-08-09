mod app;
mod button;
mod window;

use webrender::api::*;

fn main() {
    app::App::new()
        .window("window1", ColorF::new(0.1, 0.2, 0.3, 1.0))
        .window("window2", ColorF::new(0.3, 0.2, 0.1, 1.0))
        .run();
}

type UiSize = euclid::TypedSize2D<f32, LayoutPixel>;

trait Ui {
    fn render(&self, rend_ctxt: RenderContext) {}
    fn measure(&mut self) -> UiSize {
        UiSize::default()
    }
    fn arrange(&mut self, final_size: UiSize) {}
}

struct Rect {}

impl Ui for Rect {}

pub struct RenderContext<'b> {
    dl_builder: &'b mut DisplayListBuilder,
    final_size: UiSize,
    spatial_id: SpatialId,
}

impl<'b> RenderContext<'b> {
    pub fn new(pipeline_id: PipelineId, dl_builder: &'b mut DisplayListBuilder, window_size: UiSize) -> Self {
        RenderContext {
            final_size: window_size,
            dl_builder,
            spatial_id: SpatialId::root_reference_frame(pipeline_id),
        }
    }

    fn child_context(&'b mut self, final_rect: &LayoutRect) -> Self {
        let spatial_id = self.dl_builder.push_reference_frame(
            final_rect,
            self.spatial_id,
            TransformStyle::Flat,
            PropertyBinding::Value(LayoutTransform::default()),
            ReferenceFrameKind::Transform,
        );
        RenderContext {
            final_size: final_rect.size,
            dl_builder: self.dl_builder,
            spatial_id: spatial_id,
        }
    }
}
