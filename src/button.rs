//use crate::window::RenderContext;
//use euclid::rect;
//use webrender::api::*;
//
//#[derive(Default)]
//pub struct Button {
//    tag: (u64, u16),
//    is_hovered: bool,
//}
//
//impl Button {
//    pub fn event(&mut self, event: &glutin::WindowEvent, context: &RenderContext) -> bool {
//        match event {
//            glutin::WindowEvent::CursorMoved { position, .. } => {
//                let new_is_hovered =
//                    context.hit_test(WorldPoint::new(position.x as f32, position.y as f32), self.tag);
//
//                if self.is_hovered != new_is_hovered {
//                    self.is_hovered = new_is_hovered;
//                    return true;
//                }
//            }
//            glutin::WindowEvent::CursorLeft { .. } => {
//                if self.is_hovered {
//                    self.is_hovered = false;
//                    return true;
//                }
//            }
//            _ => {}
//        }
//        false
//    }
//
//    pub fn render(&self, pipeline_id: PipelineId, builder: &mut DisplayListBuilder) {
//        let mut layour_primitive_info = LayoutPrimitiveInfo::new(rect(80.0, 2.0, 554., 50.));
//        layour_primitive_info.tag = Some(self.tag);
//        builder.push_rect(
//            &layour_primitive_info,
//            &SpaceAndClipInfo::root_scroll(pipeline_id),
//            if self.is_hovered {
//                ColorF::new(0.2, 0.4, 0.1, 1.)
//            } else {
//                ColorF::new(0.5, 0., 0.7, 1.)
//            },
//        );
//    }
//}
//
