use crate::prelude::new_widget::*;

/// Scrollbar widget.
#[widget($crate::widgets::scroll::scrollbar)]
pub mod scrollbar {
    use super::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use super::thumb;

    properties! {
        /// Thumb widget.
        ///
        /// Recommended widget is [`thumb!`], but can be any widget that implements
        /// thumb behavior and tags it-self in the frame.
        ///
        /// [`thumb!`]: mod@thumb
        pub thumb;

        /// Fills the track with [`vis::BACKGROUND_VAR`]
        pub crate::properties::background_color = vis::BACKGROUND_VAR;

        /// Scrollbar orientation.
        ///
        /// This sets the scrollbar alignment to fill its axis and take the cross-length from the thumb.
        pub orientation(impl IntoVar<Orientation>) = Orientation::Vertical;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let thumb = wgt.capture_ui_node_or_else(property_id!(self.thumb), || NilUiNode);
            wgt.set_child(thumb);

            let orientation = wgt.capture_var_or_else(property_id!(self.orientation), || Orientation::Vertical);
            wgt.push_intrinsic(Priority::Layout, "orientation-align", move |child| {
                align(
                    child,
                    orientation.map(|o| match o {
                        Orientation::Vertical => Align::FILL_RIGHT,
                        Orientation::Horizontal => Align::FILL_BOTTOM,
                    }),
                )
            });
        });
    }

    /// Style variables and properties.
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

    inherit!(widget_base::base);

    properties! {
        /// Scrollbar orientation.
        pub orientation(impl IntoVar<scrollbar::Orientation>) = scrollbar::Orientation::Vertical;

        /// Viewport/content ratio.
        ///
        /// This becomes the height for vertical and width for horizontal.
        pub viewport_ratio(impl IntoVar<Factor>);

        /// Content offset.
        pub offset(impl IntoVar<Factor>);

        /// Width if orientation is vertical, otherwise height if orientation is horizontal.
        pub cross_length(impl IntoVar<Length>) = 16;

        /// Fills the thumb with [`vis::BACKGROUND_VAR`].
        pub crate::properties::background_color = vis::BACKGROUND_VAR;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the thumb is pressed.
        capture_mouse = true;

        /// When the pointer device is over this thumb.
        when *#is_hovered {
            background_color = vis::hovered::BACKGROUND_VAR;
        }

        /// When the thumb is pressed.
        when *#is_cap_pressed  {
            background_color = vis::pressed::BACKGROUND_VAR;
        }
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(on_build);
    }
    fn on_build(wgt: &mut WidgetBuilding) {
        let cross_length = wgt.capture_var_or_default::<Length>(property_id!(self.cross_length));
        wgt.push_intrinsic(Priority::Size, "orientation-size", move |child| {
            size(
                child,
                merge_var!(THUMB_ORIENTATION_VAR, THUMB_VIEWPORT_RATIO_VAR, cross_length, |o, r, l| {
                    match o {
                        scrollbar::Orientation::Vertical => Size::new(l.clone(), *r),
                        scrollbar::Orientation::Horizontal => Size::new(*r, l.clone()),
                    }
                }),
            )
        });

        wgt.push_intrinsic(Priority::Layout, "thumb_layout", thumb_layout);

        let orientation = wgt.capture_var_or_else(property_id!(self.orientation), || scrollbar::Orientation::Vertical);
        let viewport_ratio = wgt.capture_var_or_else(property_id!(self.viewport_ratio), || 1.fct());
        let offset = wgt.capture_var_or_else(property_id!(self.offset), || 0.fct());

        wgt.push_intrinsic(Priority::Context, "thumb-context", move |child| {
            let child = with_context_var(child, THUMB_ORIENTATION_VAR, orientation);
            let child = with_context_var(child, THUMB_VIEWPORT_RATIO_VAR, viewport_ratio);
            with_context_var(child, THUMB_OFFSET_VAR, offset)
        });
    }

    fn thumb_layout(child: impl UiNode) -> impl UiNode {
        #[ui_node(struct DragNode {
            child: impl UiNode,
            content_length: Px,
            viewport_length: Px,
            thumb_length: Px,
            scale_factor: Factor,

            mouse_down: Option<(Px, Factor)>,
        })]
        impl UiNode for DragNode {
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.sub_event(&MOUSE_MOVE_EVENT)
                    .sub_event(&MOUSE_INPUT_EVENT)
                    .sub_var(&THUMB_OFFSET_VAR);
                self.child.init(ctx);
            }

            fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
                if let Some((mouse_down, start_offset)) = self.mouse_down {
                    if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                        let offset = match THUMB_ORIENTATION_VAR.get() {
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
                    } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                        if args.is_primary() && args.is_mouse_up() {
                            self.mouse_down = None;
                        }
                    }
                } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    if args.is_primary() && args.is_mouse_down() {
                        let a = match THUMB_ORIENTATION_VAR.get() {
                            scrollbar::Orientation::Vertical => args.position.y.to_px(self.scale_factor.0),
                            scrollbar::Orientation::Horizontal => args.position.x.to_px(self.scale_factor.0),
                        };
                        self.mouse_down = Some((a, THUMB_OFFSET_VAR.get()));
                    }
                }
                self.child.event(ctx, update);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if THUMB_OFFSET_VAR.is_new(ctx) {
                    ctx.updates.layout();
                }

                self.child.update(ctx, updates);
            }

            fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                self.child.measure(ctx)
            }
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                let final_size = ctx.constrains().fill_size();

                let mut final_offset = PxVector::zero();
                let (px_vp_length, final_offset_d) = match THUMB_ORIENTATION_VAR.get() {
                    scrollbar::Orientation::Vertical => (final_size.height, &mut final_offset.y),
                    scrollbar::Orientation::Horizontal => (final_size.width, &mut final_offset.x),
                };

                let ratio = THUMB_VIEWPORT_RATIO_VAR.get();
                let px_tb_length = px_vp_length * ratio;
                *final_offset_d = (px_vp_length - px_tb_length) * THUMB_OFFSET_VAR.get();

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

    context_var! {
        static THUMB_ORIENTATION_VAR: scrollbar::Orientation = scrollbar::Orientation::Vertical;
        static THUMB_VIEWPORT_RATIO_VAR: Factor = 1.fct();
        static THUMB_OFFSET_VAR: Factor = 0.fct();
    }

    /// Style variables.
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

    #[doc(hidden)]
    #[property(context, capture, default(thumb!()))]
    pub fn thumb_property(child: impl UiNode, thumb: impl UiNode) -> impl UiNode {
        child
    }
    #[doc(hidden)]
    pub use thumb_property::*;
}
