use crate::prelude::new_widget::*;

#[widget($crate::widgets::image)]
pub mod image {
    use super::*;

    properties!{
        path(impl IntoVar<Text>) = "";
    }

    fn new_child(path: impl IntoVar<Text>) -> impl UiNode {
        struct ImageNode<T> {
            path: T,
            image: Option<Image>,
        }
        #[impl_ui_node(none)]
        impl<T: Var<Text>> UiNode for ImageNode<T> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.image = Some(Image::from_file(self.path.get_clone(ctx)))
            }
            fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
                frame.push_image(self.image.as_ref().unwrap())
            }
        }
        ImageNode {
            path: path.into_var(),
            image: None
        }
    }
}