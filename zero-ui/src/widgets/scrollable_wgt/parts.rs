use crate::prelude::new_widget::*;

/// Scrollbar widget.
#[widget($crate::widgets::scrollable::scrollbar)]
pub mod scrollbar {
    use super::*;
    use crate::core::render::webrender_api::PrimitiveFlags;

    #[doc(inline)]
    pub use super::thumb;

    properties! {
        /// Thumb widget.
        ///
        /// Recommended widget is [`thumb!`], but can be any widget that implements
        /// thumb behavior and tags it-self in the frame.
        ///
        /// [`thumb!`]: mod@thumb
        #[required]
        #[allowed_in_when = false]
        thumb(impl UiNode);

        /// Fills the track with [`theme::BackgroundVar`]
        background_color = theme::BackgroundVar;

        /// Scrollbar orientation.
        ///
        /// This sets the scrollbar alignment to fill its axis and take the cross-length from the thumb.
        orientation(impl IntoVar<Orientation>) = Orientation::Vertical;
    }

    fn new_child(thumb: impl UiNode) -> impl UiNode {
        implicit_base::nodes::leaf_transform(thumb)
    }

    fn new_layout(child: impl UiNode, orientation: impl IntoVar<Orientation>) -> impl UiNode {
        let orientation = orientation.into_var();
        align(
            child,
            orientation.map(|o| match o {
                Orientation::Vertical => Align::FILL_RIGHT,
                Orientation::Horizontal => Align::FILL_BOTTOM,
            }),
        )
    }

    fn new_context(child: impl UiNode) -> impl UiNode {
        primitive_flags(child, PrimitiveFlags::IS_SCROLLBAR_CONTAINER)
    }

    /// Theme variables and properties.
    pub mod theme {
        use crate::prelude::new_property::*;

        context_var! {
            /// Scrollbar track background color
            pub struct BackgroundVar: Rgba = rgba(80, 80, 80, 50.pct());
        }
    }

    /// Orientation of a scrollbar.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Orientation {
        /// Bar fills the in the ***x*** dimension and scrolls left-right.
        Horizontal,
        /// Bar fills the in the ***y*** dimension and scrolls top-bottom.
        Vertical,
    }
}

/// Scrollbar thumb widget.
#[widget($crate::widgets::scrollable::scrollbar::thumb)]
pub mod thumb {
    use super::*;
    use crate::core::{mouse::*, render::webrender_api::PrimitiveFlags};

    properties! {
        /// Scrollbar orientation.
        orientation(impl IntoVar<scrollbar::Orientation>) = scrollbar::Orientation::Vertical;

        /// Viewport/content ratio.
        ///
        /// This becomes the height for vertical and width for horizontal.
        #[required]
        viewport_ratio(impl IntoVar<Factor>);

        /// Content offset.
        #[required]
        offset(impl IntoVar<Factor>);

        /// Width if orientation is vertical, otherwise height if orientation is horizontal.
        cross_length(impl IntoVar<Length>) = 16;

        /// Fills the thumb with [`theme::BackgroundVar`].
        background_color = theme::BackgroundVar;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the thumb is pressed.
        capture_mouse = true;

        /// When the pointer device is over this thumb.
        when self.is_hovered {
            background_color = theme::hovered::BackgroundVar;
        }

        /// When the thumb is pressed.
        when self.is_cap_pressed  {
            background_color = theme::pressed::BackgroundVar;
        }
    }

    fn new_size(child: impl UiNode, cross_length: impl IntoVar<Length>) -> impl UiNode {
        size(
            child,
            merge_var!(
                ThumbOrientationVar::new(),
                ThumbViewportRatioVar::new(),
                cross_length.into_var(),
                |o, r, l| {
                    match o {
                        scrollbar::Orientation::Vertical => Size::new(l.clone(), *r),
                        scrollbar::Orientation::Horizontal => Size::new(*r, l.clone()),
                    }
                }
            ),
        )
    }

    fn new_layout(child: impl UiNode) -> impl UiNode {
        struct DragNode<C> {
            child: C,
            viewport_length: Dip,
            thumb_length: Dip,

            mouse_down: Option<(Dip, Factor)>,

            final_offset: PxVector,
            spatial_id: SpatialFrameId,
            offset_key: FrameBindingKey<RenderTransform>,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for DragNode<C> {
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                subscriptions.event(MouseMoveEvent).event(MouseInputEvent);
                self.child.subscriptions(ctx, subscriptions);
            }

            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                if let Some((mouse_down, start_offset)) = self.mouse_down {
                    if let Some(args) = MouseMoveEvent.update(args) {
                        let offset = match *ThumbOrientationVar::get(ctx) {
                            scrollbar::Orientation::Vertical => args.position.y,
                            scrollbar::Orientation::Horizontal => args.position.x,
                        } - mouse_down;

                        let max_length = self.viewport_length - self.thumb_length;
                        let start_offset = max_length * start_offset.0;

                        let offset = offset + start_offset;
                        let offset = (offset.to_f32() / max_length.to_f32()).max(0.0).min(1.0);

                        ThumbOffsetVar::new()
                            .set_ne(ctx.vars, Factor(offset))
                            .expect("ThumbOffsetVar is read-only");

                        ctx.updates.layout();
                        self.child.event(ctx, args);
                    } else if let Some(args) = MouseInputEvent.update(args) {
                        if args.is_primary() && args.is_mouse_up() {
                            self.mouse_down = None;
                        }
                        self.child.event(ctx, args);
                    } else {
                        self.child.event(ctx, args);
                    }
                } else if let Some(args) = MouseInputEvent.update(args) {
                    if args.is_primary() && args.is_mouse_down() && args.concerns_widget(ctx) {
                        let a = match *ThumbOrientationVar::get(ctx) {
                            scrollbar::Orientation::Vertical => args.position.y,
                            scrollbar::Orientation::Horizontal => args.position.x,
                        };
                        self.mouse_down = Some((a, *ThumbOffsetVar::get(ctx.vars)));
                    }
                    self.child.event(ctx, args);
                } else {
                    self.child.event(ctx, args);
                }
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                let mut final_offset = self.final_offset;
                let (px_vp_length, final_offset_d) = match *ThumbOrientationVar::get(ctx) {
                    scrollbar::Orientation::Vertical => (final_size.height, &mut final_offset.y),
                    scrollbar::Orientation::Horizontal => (final_size.width, &mut final_offset.x),
                };

                let ratio = *ThumbViewportRatioVar::get(ctx);
                let px_tb_length = px_vp_length * ratio;
                *final_offset_d = (px_vp_length - px_tb_length) * ThumbOffsetVar::get_clone(ctx.vars);

                let fct = ctx.metrics.scale_factor.0;
                self.viewport_length = px_vp_length.to_dip(fct);
                self.thumb_length = px_tb_length.to_dip(fct);

                if self.final_offset != final_offset {
                    self.final_offset = final_offset;
                    ctx.updates.render_update();
                }

                widget_layout.with_custom_transform(&RenderTransform::translation_px(self.final_offset), |wo| {
                    self.child.arrange(ctx, wo, final_size)
                });
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                let transform = RenderTransform::translation_px(self.final_offset);
                frame.push_reference_frame(self.spatial_id, self.offset_key.bind(transform), true, |f| {
                    self.child.render(ctx, f)
                });
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                let transform = RenderTransform::translation_px(self.final_offset);
                update.update_transform(self.offset_key.update(transform));

                self.child.render_update(ctx, update);
            }
        }
        DragNode {
            child,
            viewport_length: Dip::new(0),
            thumb_length: Dip::new(0),

            mouse_down: None,

            final_offset: PxVector::zero(),
            spatial_id: SpatialFrameId::new_unique(),
            offset_key: FrameBindingKey::new_unique(),
        }
    }

    fn new_context(
        child: impl UiNode,
        orientation: impl IntoVar<scrollbar::Orientation>,
        viewport_ratio: impl IntoVar<Factor>,
        offset: impl IntoVar<Factor>,
    ) -> impl UiNode {
        let child = with_context_var(child, ThumbOrientationVar, orientation);
        let child = with_context_var(child, ThumbViewportRatioVar, viewport_ratio);
        let child = with_context_var(child, ThumbOffsetVar, offset);
        primitive_flags(child, PrimitiveFlags::IS_SCROLLBAR_THUMB)
    }

    context_var! {
        struct ThumbOrientationVar: scrollbar::Orientation = scrollbar::Orientation::Vertical;
        struct ThumbViewportRatioVar: Factor = 1.fct();
        struct ThumbOffsetVar: Factor = 0.fct();
    }

    /// Theme variables.
    pub mod theme {
        use crate::prelude::new_property::*;

        context_var! {
            /// Fill color.
            pub struct BackgroundVar: Rgba = rgba(200, 200, 200, 50.pct());
        }

        /// Variables when the pointer device is over the thumb.
        pub mod hovered {
            use super::*;

            context_var! {
                /// Fill color.
                pub struct BackgroundVar: Rgba = rgba(200, 200, 200, 70.pct());
            }
        }

        /// Variables when the pointer device is pressing the thumb.
        pub mod pressed {
            use super::*;

            context_var! {
                /// Fill color.
                pub struct BackgroundVar: Rgba = rgba(200, 200, 200, 90.pct());
            }
        }
    }
}
