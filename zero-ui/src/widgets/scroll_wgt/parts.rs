use crate::prelude::new_widget::*;

/// Scrollbar widget.
#[widget($crate::widgets::scroll::scrollbar)]
pub mod scrollbar {
    use super::*;

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

        /// Fills the track with [`vis::BACKGROUND_VAR`]
        background_color = vis::BACKGROUND_VAR;

        /// Scrollbar orientation.
        ///
        /// This sets the scrollbar alignment to fill its axis and take the cross-length from the thumb.
        orientation(impl IntoVar<Orientation>) = Orientation::Vertical;
    }

    fn new_child(thumb: impl UiNode) -> impl UiNode {
        thumb
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

    /// Theme variables and properties.
    pub mod vis {
        use crate::prelude::new_property::*;

        context_var! {
            /// Scrollbar track background color
            pub static BACKGROUND_VAR: Rgba = rgba(80, 80, 80, 50.pct());
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
#[widget($crate::widgets::scroll::scrollbar::thumb)]
pub mod thumb {
    use super::*;
    use crate::core::mouse::*;

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

        /// Fills the thumb with [`theme::BACKGROUND_VAR`].
        background_color = vis::BACKGROUND_VAR;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the thumb is pressed.
        capture_mouse = true;

        /// When the pointer device is over this thumb.
        when self.is_hovered {
            background_color = vis::hovered::BACKGROUND_VAR;
        }

        /// When the thumb is pressed.
        when self.is_cap_pressed  {
            background_color = vis::pressed::BACKGROUND_VAR;
        }
    }

    fn new_size(child: impl UiNode, cross_length: impl IntoVar<Length>) -> impl UiNode {
        size(
            child,
            merge_var!(
                THUMB_ORIENTATION_VAR,
                THUMB_VIEWPORT_RATIO_VAR,
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
            content_length: Px,
            viewport_length: Px,
            thumb_length: Px,
            scale_factor: Factor,

            mouse_down: Option<(Px, Factor)>,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for DragNode<C> {
            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                subs.event(MouseMoveEvent).event(MouseInputEvent).var(ctx, &THUMB_OFFSET_VAR);
                self.child.subscriptions(ctx, subs);
            }

            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                if let Some((mouse_down, start_offset)) = self.mouse_down {
                    if let Some(args) = MouseMoveEvent.update(args) {
                        let offset = match THUMB_ORIENTATION_VAR.copy(ctx) {
                            scrollbar::Orientation::Vertical => args.position.y.to_px(self.scale_factor.0),
                            scrollbar::Orientation::Horizontal => args.position.x.to_px(self.scale_factor.0),
                        } - mouse_down;

                        let max_length = self.viewport_length - self.thumb_length;
                        let start_offset = max_length * start_offset.0;

                        let offset = offset + start_offset;
                        let offset = (offset.0 as f32 / max_length.0 as f32).max(0.0).min(1.0);

                        // snap to pixel
                        let max_length = self.viewport_length - self.content_length;
                        let offset = max_length * offset;
                        let offset = offset.0 as f32 / max_length.0 as f32;

                        THUMB_OFFSET_VAR
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
                    if args.is_primary() && args.is_mouse_down() {
                        let a = match THUMB_ORIENTATION_VAR.copy(ctx) {
                            scrollbar::Orientation::Vertical => args.position.y.to_px(self.scale_factor.0),
                            scrollbar::Orientation::Horizontal => args.position.x.to_px(self.scale_factor.0),
                        };
                        self.mouse_down = Some((a, THUMB_OFFSET_VAR.copy(ctx.vars)));
                    }
                    self.child.event(ctx, args);
                } else {
                    self.child.event(ctx, args);
                }
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                if THUMB_OFFSET_VAR.is_new(ctx) {
                    ctx.updates.layout();
                }

                self.child.update(ctx);
            }

            fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                self.child.measure(ctx)
            }
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                let final_size = ctx.constrains().fill_size();

                let mut final_offset = PxVector::zero();
                let (px_vp_length, final_offset_d) = match THUMB_ORIENTATION_VAR.copy(ctx) {
                    scrollbar::Orientation::Vertical => (final_size.height, &mut final_offset.y),
                    scrollbar::Orientation::Horizontal => (final_size.width, &mut final_offset.x),
                };

                let ratio = THUMB_VIEWPORT_RATIO_VAR.copy(ctx);
                let px_tb_length = px_vp_length * ratio;
                *final_offset_d = (px_vp_length - px_tb_length) * THUMB_OFFSET_VAR.get_clone(ctx.vars);

                self.scale_factor = ctx.metrics.scale_factor();
                self.content_length = px_vp_length / ratio;
                self.viewport_length = px_vp_length;
                self.thumb_length = px_tb_length;

                wl.translate(final_offset);

                self.child.layout(ctx, wl)
            }
        }
        DragNode {
            child,
            content_length: Px(0),
            viewport_length: Px(0),
            thumb_length: Px(0),
            scale_factor: 1.fct(),

            mouse_down: None,
        }
    }

    fn new_context(
        child: impl UiNode,
        orientation: impl IntoVar<scrollbar::Orientation>,
        viewport_ratio: impl IntoVar<Factor>,
        offset: impl IntoVar<Factor>,
    ) -> impl UiNode {
        let child = with_context_var(child, THUMB_ORIENTATION_VAR, orientation);
        let child = with_context_var(child, THUMB_VIEWPORT_RATIO_VAR, viewport_ratio);
        with_context_var(child, THUMB_OFFSET_VAR, offset)
    }

    context_var! {
        static THUMB_ORIENTATION_VAR: scrollbar::Orientation = scrollbar::Orientation::Vertical;
        static THUMB_VIEWPORT_RATIO_VAR: Factor = 1.fct();
        static THUMB_OFFSET_VAR: Factor = 0.fct();
    }

    /// Theme variables.
    pub mod vis {
        use crate::prelude::new_property::*;

        context_var! {
            /// Fill color.
            pub static BACKGROUND_VAR: Rgba = rgba(200, 200, 200, 50.pct());
        }

        /// Variables when the pointer device is over the thumb.
        pub mod hovered {
            use super::*;

            context_var! {
                /// Fill color.
                pub static BACKGROUND_VAR: Rgba = rgba(200, 200, 200, 70.pct());
            }
        }

        /// Variables when the pointer device is pressing the thumb.
        pub mod pressed {
            use super::*;

            context_var! {
                /// Fill color.
                pub static BACKGROUND_VAR: Rgba = rgba(200, 200, 200, 90.pct());
            }
        }
    }
}
