use crate::prelude::new_widget::*;

/// Base single content container.
#[widget($crate::widgets::container)]
pub mod container {
    use super::*;

    properties! {
        child {
            /// Content UI.
            ///
            /// Can be any type that implements [`UiNode`], any widget.
            ///
            /// [`UiNode`]: zero_ui::core::UiNode
            #[allowed_in_when = false]
            #[required]
            content(impl UiNode);

            /// Content margin.
            margin as padding;

            /// Content alignment.
            align as content_align = Align::CENTER;

            /// Content overflow clipping.
            clip_to_bounds;
        }
    }

    #[inline]
    fn new_child(content: impl UiNode) -> impl UiNode {
        nodes::leaf_transform(content)
    }

    /// Nodes used for implementing the container.
    pub mod nodes {
        use super::*;

        /// Applies pending widget transforms if `content` is not an widget.
        ///
        /// This node makes `padding` and `content_align` work for content that does not implement [`Widget`].
        pub fn leaf_transform(content: impl UiNode) -> impl UiNode {
            struct LeafTransformNode<C> {
                child: C,
                leaf_transform: Option<Box<(SpatialFrameId, RenderTransform)>>,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for LeafTransformNode<C> {
                fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                    if let Some(t) = widget_layout.leaf_transform(ctx.metrics, final_size, |wl| self.child.arrange(ctx, wl, final_size)) {
                        if let Some(lt) = &mut self.leaf_transform {
                            if t != lt.1 {
                                lt.1 = t;
                                ctx.updates.render();
                            }
                        } else {
                            self.leaf_transform = Some(Box::new((SpatialFrameId::new_unique(), t)));
                            ctx.updates.render();
                        }
                    }
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    if let Some(lt) = &self.leaf_transform {
                        frame.push_reference_frame(lt.0, FrameBinding::Value(lt.1), false, |f| self.child.render(ctx, f));
                    } else {
                        self.child.render(ctx, frame);
                    }
                }
            }
            LeafTransformNode {
                child: content,
                leaf_transform: None,
            }
        }
    }
}
