use crate::prelude::new_widget::*;

#[widget($crate::widgets::image)]
pub mod image {
    use super::*;

    properties! {
        child {
            /// The image source.
            ///
            /// Can be a file path, an URI, binary included in the app and more.
            source(impl IntoVar<Text>) = "";
        }

        /// Sets the image scaling algorithm used to rescale the image in the renderer.
        ///
        /// If the image layout size is not the same as the `source` pixel size the image must be re-scaled
        /// during rendering, this property selects what algorithm is used to do this re-scaling.
        ///
        /// Note that the algorithms used in the renderer value performance over quality and do a good
        /// enough job for small or temporary changes in scale only. If the image stays at a very different scale
        /// after a short time a CPU re-scale task is automatically started to generate a better quality re-scaling.
        ///
        /// If the image is an app resource known during build time you should consider pre-scaling it to match the screen
        /// size at different DPIs using mipmaps.
        ///
        /// This is [`ImageRendering::Auto`] by default.
        properties::image_rendering as rendering;
    }

    fn new_child(source: impl IntoVar<Text>) -> impl UiNode {
        struct ImageNode<T> {
            path: T,
            image: Option<Image>,
            final_size: LayoutSize,
        }
        #[impl_ui_node(none)]
        impl<T: Var<Text>> UiNode for ImageNode<T> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                let path = self.path.get_clone(ctx);
                self.image = Some(ctx.services.images().get_file(path));
            }
            fn deinit(&mut self, _: &mut WidgetContext) {
                self.image = None;
            }
            fn arrange(&mut self, _: &mut LayoutContext, final_size: LayoutSize) {
                self.final_size = final_size;
            }
            fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
                frame.push_image(
                    LayoutRect::from(self.final_size),
                    self.image.as_ref().unwrap(),
                    ImageRendering::Pixelated,
                );
            }
        }
        ImageNode {
            path: source.into_var(),
            image: None,
            final_size: LayoutSize::zero(),
        }
    }

    /// Properties that configure [`image!`] widgets from parent widgets.
    ///
    /// Note that this properties are already available in the [`image!`] widget directly without the `image_` prefix.
    ///
    /// [`image!`]: mod@crate::widgets::image
    pub mod properties {
        use super::*;

        context_var! {
            /// The Image scaling algorithm in the renderer.
            ///
            /// Is [`ImageRendering::Auto`] by default.
            pub struct ImageRenderingVar: ImageRendering = const ImageRendering::Auto;
        }

        /// Sets the [`ImageRendering`] of all inner images.
        ///
        /// This property binds `rendering` to the [`ImageRenderingVar`] in the widget context.
        #[property(context)]
        pub fn image_rendering(child: impl UiNode, rendering: impl IntoVar<ImageRendering>) -> impl UiNode {
            with_context_var(child, ImageRenderingVar, rendering)
        }
    }
}
