use crate::prelude::new_widget::*;
use std::fmt;

/// A single content container that can be larger on the inside.
#[widget($crate::widgets::scrollable)]
pub mod scrollable {
    use super::*;
    use bitflags::*;
    use properties::*;

    #[doc(inline)]
    pub use super::scrollbar;

    properties! {
        child {
            /// Content UI.
            ///
            /// Can be any type that implements [`UiNode`](zero_ui::core::UiNode), any widget.
            #[allowed_in_when = false]
            #[required]
            content(impl UiNode);

            /// Content margin.
            margin as padding;

            /// Content alignment when it is smaller then the viewport.
            align as content_align = Align::CENTER;

            /// Scroll mode.
            ///
            /// By default scrolls in both dimensions.
            mode(impl IntoVar<ScrollMode>) = ScrollMode::ALL;
        }

        /// Scrollbar generator for both orientations.
        ///
        /// This property sets both [`v_scrollbar_view`] and [`h_scrollbar_view`] to the same `generator`.
        ///
        /// [`v_scrollbar_view`]: #wp-v_scrollbar_view
        /// [`h_scrollbar_view`]: #wp-h_scrollbar_view
        scrollbar_view;

        /// Horizontal scrollbar generator.
        h_scrollbar_view;
        /// Vertical scrollbar generator.
        v_scrollbar_view;

        /// Horizontal and vertical offsets used when scrolling.
        ///
        /// This property sets the [`h_scroll_unit`] and [`v_scroll_unit`].
        ///
        /// [`h_scroll_unit`]: #wp-h_scroll_unit
        /// [`v_scroll_unit`]: #wp-v_scroll_unit
        scroll_units;
        h_scroll_unit;
        v_scroll_unit;

        /// Horizontal and vertical offsets used when page-scrolling.
        ///
        /// This property sets the [`h_page_unit`] and [`v_page_unit`].
        ///
        /// [`h_page_unit`]: fn@h_page_unit
        /// [`v_page_unit`]: fn@v_page_unit
        page_units;
        h_page_unit;
        v_page_unit;

        /// Clip content to only be visible within the scrollable bounds, including under scrollbars.
        ///
        /// Enabled by default.
        clip_to_bounds = true;

        /// Clip content to only be visible within the viewport.
        ///
        /// Disabled by default.
        clip_to_viewport(impl IntoVar<bool>) = false;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }

    fn new_child_context(child: impl UiNode, mode: impl IntoVar<ScrollMode>, clip_to_viewport: impl IntoVar<bool>) -> impl UiNode {
        struct ScrollableNode<N> {
            children: N,
            viewport: PxSize,
            joiner: PxSize,
            spatial_id: SpatialFrameId,
        }
        #[impl_ui_node(children)]
        impl<N: UiNodeList> UiNode for ScrollableNode<N> {
            // # Layout
            //
            // +-----------------+-+
            // |                 | |
            // | 0 - viewport    |1| - v_scrollbar
            // |                 | |
            // +-----------------+-+
            // | 2 - h_scrollbar |3| - scrollbar_joiner
            ///+-----------------+-+
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                let viewport = self.children.widget_measure(0, ctx, available_size);
                let v_scroll = self.children.widget_measure(1, ctx, available_size);
                let h_scroll = self.children.widget_measure(2, ctx, available_size);

                self.joiner = PxSize::new(v_scroll.width, h_scroll.height);
                let _ = self.children.widget_measure(3, ctx, AvailableSize::from_size(self.joiner));

                available_size.clip(viewport + self.joiner)
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                let mut viewport = final_size - self.joiner;

                if viewport.width < self.joiner.width * 3.0.fct() {
                    self.joiner.width = Px(0);
                    viewport.width = final_size.width;
                }
                if viewport.height < self.joiner.height * 3.0.fct() {
                    self.joiner.height = Px(0);
                    viewport.height = final_size.height;
                }

                if viewport != self.viewport {
                    self.viewport = viewport;
                    ctx.updates.render();
                }

                self.children.widget_arrange(0, ctx, widget_layout, self.viewport);

                let joiner_offset = self.viewport.to_vector();
                widget_layout.with_custom_transform(&RenderTransform::translation_px(PxVector::new(joiner_offset.x, Px(0))), |wo| {
                    self.children
                        .widget_arrange(1, ctx, wo, PxSize::new(self.joiner.width, self.viewport.height))
                });
                widget_layout.with_custom_transform(&RenderTransform::translation_px(PxVector::new(Px(0), joiner_offset.y)), |wo| {
                    self.children
                        .widget_arrange(2, ctx, wo, PxSize::new(self.viewport.width, self.joiner.height))
                });

                widget_layout.with_custom_transform(&RenderTransform::translation_px(joiner_offset), |wo| {
                    self.children.widget_arrange(3, ctx, wo, self.joiner)
                });
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                self.children.widget_render(0, ctx, frame);

                if self.joiner.width > Px(0) {
                    let transform = RenderTransform::translation_px(PxVector::new(self.viewport.width, Px(0)));
                    frame.push_reference_frame_item(self.spatial_id, 1, FrameBinding::Value(transform), true, |frame| {
                        self.children.widget_render(1, ctx, frame);
                    });
                }

                if self.joiner.height > Px(0) {
                    let transform = RenderTransform::translation_px(PxVector::new(Px(0), self.viewport.height));
                    frame.push_reference_frame_item(self.spatial_id, 2, FrameBinding::Value(transform), true, |frame| {
                        self.children.widget_render(2, ctx, frame);
                    });
                }

                if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                    let transform = RenderTransform::translation_px(self.viewport.to_vector());
                    frame.push_reference_frame_item(self.spatial_id, 3, FrameBinding::Value(transform), true, |frame| {
                        self.children.widget_render(3, ctx, frame);
                    });
                }
            }
        }
        ScrollableNode {
            children: nodes![
                clip_to_bounds(nodes::viewport(child, mode.into_var()), clip_to_viewport.into_var()),
                nodes::v_scrollbar_presenter(),
                nodes::h_scrollbar_presenter(),
                nodes::scrollbar_joiner_presenter(),
            ],
            viewport: PxSize::zero(),
            joiner: PxSize::zero(),
            spatial_id: SpatialFrameId::new_unique(),
        }
    }

    fn new_context(child: impl UiNode) -> impl UiNode {
        let child = with_context_var(child, ScrollContextVar, Some(ScrollContext::new()));
        let child = nodes::scroll_commands_node(child);
        let child = nodes::page_commands_node(child);
        nodes::scroll_to_command_node(child)
    }

    /// Commands that control the scoped scrollable widget.
    ///
    /// The scrollable widget implements all of this commands scoped to its widget ID.
    pub mod commands {
        use super::*;
        use zero_ui::core::gesture::*;

        command! {
            /// Represents the scrollable **scroll up** by one [`v_scroll_unit`] action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                  |
            /// |--------------|--------------------------------------------------------|
            /// | [`name`]     | "Scroll Up"                                            |
            /// | [`info`]     | "Scroll the focused scrollable UP by one scroll unit." |
            /// | [`shortcut`] | `Up`                                                   |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            /// [`v_scroll_unit`]: fn@super::properties::v_scroll_unit
            pub ScrollUpCommand
                .init_name("Scroll Up")
                .init_info("Scroll the focused scrollable UP by one scroll unit.")
                .init_shortcut([shortcut!(Up)]);

            /// Represents the scrollable **scroll down** by one [`v_scroll_unit`] action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                    |
            /// |--------------|----------------------------------------------------------|
            /// | [`name`]     | "Scroll Down"                                            |
            /// | [`info`]     | "Scroll the focused scrollable DOWN by one scroll unit." |
            /// | [`shortcut`] | `Down`                                                   |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            /// [`v_scroll_unit`]: fn@super::properties::v_scroll_unit
            pub ScrollDownCommand
                .init_name("Scroll Down")
                .init_info("Scroll the focused scrollable DOWN by one scroll unit.")
                .init_shortcut([shortcut!(Down)]);

            /// Represents the scrollable **scroll left** by one [`h_scroll_unit`] action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                    |
            /// |--------------|----------------------------------------------------------|
            /// | [`name`]     | "Scroll Left"                                            |
            /// | [`info`]     | "Scroll the focused scrollable LEFT by one scroll unit." |
            /// | [`shortcut`] | `Left`                                                   |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            /// [`h_scroll_unit`]: fn@super::properties::h_scroll_unit
            pub ScrollLeftCommand
                .init_name("Scroll Left")
                .init_info("Scroll the focused scrollable LEFT by one scroll unit.")
                .init_shortcut([shortcut!(Left)]);

            /// Represents the scrollable **scroll right** by one [`h_scroll_unit`] action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                     |
            /// |--------------|-----------------------------------------------------------|
            /// | [`name`]     | "Scroll Right"                                            |
            /// | [`info`]     | "Scroll the focused scrollable RIGHT by one scroll unit." |
            /// | [`shortcut`] | `Down`                                                    |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            /// [`h_scroll_unit`]: fn@super::properties::h_scroll_unit
            pub ScrollRightCommand
                .init_name("Scroll Right")
                .init_info("Scroll the focused scrollable RIGHT by one scroll unit.")
                .init_shortcut([shortcut!(Right)]);


            /// Represents the scrollable **page up** by one [`v_page_unit`] action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                 |
            /// |--------------|-------------------------------------------------------|
            /// | [`name`]     | "Page Up"                                             |
            /// | [`info`]     | "Scroll the focused scrollable UP by one page unit."  |
            /// | [`shortcut`] | `PageUp`                                              |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            /// [`v_page_unit`]: fn@super::properties::v_page_unit
            pub PageUpCommand
                .init_name("Page Up")
                .init_info("Scroll the focused scrollable UP by one page unit.")
                .init_shortcut([shortcut!(PageUp)]);

            /// Represents the scrollable **page down** by one [`v_page_unit`] action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                   |
            /// |--------------|---------------------------------------------------------|
            /// | [`name`]     | "Page Down"                                             |
            /// | [`info`]     | "Scroll the focused scrollable DOWN by one page unit."  |
            /// | [`shortcut`] | `PageDown`                                              |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            /// [`v_page_unit`]: fn@super::properties::v_page_unit
            pub PageDownCommand
                .init_name("Page Down")
                .init_info("Scroll the focused scrollable DOWN by one page unit.")
                .init_shortcut([shortcut!(PageDown)]);

            /// Represents the scrollable **page left** by one [`h_page_unit`] action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                  |
            /// |--------------|--------------------------------------------------------|
            /// | [`name`]     | "Page Left"                                            |
            /// | [`info`]     | "Scroll the focused scrollable LEFT by one page unit." |
            /// | [`shortcut`] | `ALT+PageLeft`                                         |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            /// [`h_scroll_unit`]: fn@super::properties::h_scroll_unit
            pub PageLeftCommand
                .init_name("Page Left")
                .init_info("Scroll the focused scrollable LEFT by one page unit.")
                .init_shortcut([shortcut!(ALT+PageUp)]);

            /// Represents the scrollable **page right** by one [`h_page_unit`] action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                   |
            /// |--------------|---------------------------------------------------------|
            /// | [`name`]     | "Page Right"                                            |
            /// | [`info`]     | "Scroll the focused scrollable RIGHT by one page unit." |
            /// | [`shortcut`] | `ALT+PageDown`                                          |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            /// [`h_page_unit`]: fn@super::properties::h_page_unit
            pub PageRightCommand
                .init_name("Page Right")
                .init_info("Scroll the focused scrollable RIGHT by one page unit.")
                .init_shortcut([shortcut!(ALT+PageDown)]);

            /// Represents the action of scrolling until a child widget is fully visible.
            ///
            /// # Metadata
            ///
            /// This command initializes with no extra metadata.
            ///
            /// # Parameter
            ///
            /// This command requires a parameter to work, it can be the [`WidgetId`] of a child widget or
            /// a [`ScrollToRequest`] instance.
            ///
            /// You can use the [`scroll_to`] function to invoke this command.
            pub ScrollToCommand;
        }

        /// Parameters for the [`ScrollToCommand`].
        #[derive(Debug, Clone)]
        pub struct ScrollToRequest {
            /// Widget that will be scrolled into view.
            pub widget_id: WidgetId,

            /// How much the scroll position will change to showcase the target widget.
            pub mode: ScrollToMode,
        }
        impl ScrollToRequest {
            /// Pack the request into a command parameter.
            pub fn to_parameter(self) -> CommandParam {
                CommandParam::new(self)
            }

            /// Extract a clone of the request from the command parameter if it is of a compatible type.
            pub fn from_parameter(p: &CommandParam) -> Option<Self> {
                if let Some(req) = p.downcast_ref::<Self>() {
                    Some(req.clone())
                } else {
                    p.downcast_ref::<WidgetId>().map(|id| ScrollToRequest {
                        widget_id: *id,
                        mode: ScrollToMode::default(),
                    })
                }
            }

            /// Extract a clone of the request from [`CommandArgs::parameter`] if it is set to a compatible type and
            /// stop-propagation was not requested for the event.
            pub fn from_args(args: &CommandArgs) -> Option<Self> {
                if let Some(p) = &args.parameter {
                    if args.stop_propagation_requested() {
                        None
                    } else {
                        Self::from_parameter(p)
                    }
                } else {
                    None
                }
            }
        }
        impl_from_and_into_var! {
            fn from(widget_id: WidgetId) -> ScrollToRequest {
                ScrollToRequest {
                    widget_id,
                    mode: ScrollToMode::default()
                }
            }
        }

        /// Defines how much the [`ScrollToCommand`] will scroll to showcase the target widget.
        #[derive(Debug, Clone)]
        pub enum ScrollToMode {
            /// Scroll will change only just enough so that the widget inner rect is fully visible with the optional
            /// extra margin offsets.
            Minimal {
                /// Extra margin added so that the widget is touching the scrollable edge.
                margin: SideOffsets,
            },
            /// Scroll so that the point relative to the widget inner rectangle is at the same screen point on
            /// the scrollable viewport.
            Center {
                /// A point relative to the target widget inner size.
                widget_point: Point,
                /// A point relative to the scrollable viewport.
                scrollable_point: Point,
            },
        }
        impl ScrollToMode {
            /// New [`Minimal`] mode.
            ///
            /// [`Minimal`]: Self::Minimal
            pub fn minimal(margin: impl Into<SideOffsets>) -> Self {
                ScrollToMode::Minimal { margin: margin.into() }
            }

            /// New [`Center`] mode.
            ///
            /// [`Center`]: Self::Center
            pub fn center_points(widget_point: impl Into<Point>, scrollable_point: impl Into<Point>) -> Self {
                ScrollToMode::Center {
                    widget_point: widget_point.into(),
                    scrollable_point: scrollable_point.into(),
                }
            }

            /// New [`Center`] mode using the center points of widget and scrollable.
            ///
            /// [`Center`]: Self::Center
            pub fn center() -> Self {
                Self::center_points(Point::center(), Point::center())
            }
        }
        impl Default for ScrollToMode {
            /// Minimal with margin 10.
            fn default() -> Self {
                Self::minimal(10)
            }
        }

        /// Scroll the scrollable widget so that the child widget is fully visible.
        ///
        /// This function is a helper for firing a [`ScrollToCommand`].
        pub fn scroll_to<Evs: WithEvents>(events: &mut Evs, scrollable_id: WidgetId, child_id: WidgetId, mode: impl Into<ScrollToMode>) {
            ScrollToCommand.scoped(scrollable_id).notify(
                events,
                Some(
                    ScrollToRequest {
                        widget_id: child_id,
                        mode: mode.into(),
                    }
                    .to_parameter(),
                ),
            );
        }
    }

    /// Properties that configure [`scrollable!`] widgets from parent widgets.
    ///
    /// Note that this properties are already available in the [`scrollable!`] widget directly.
    ///
    /// [`scrollable!`]: mod@crate::widgets::scrollable
    pub mod properties {
        use super::*;
        use crate::widgets::fill_color;

        context_var! {
            /// View generator for creating the vertical scrollbar of an scrollable widget.
            pub struct VerticalScrollBarViewVar: ViewGenerator<ScrollBarArgs> = default_scrollbar();

            /// View generator for creating the vertical scrollbar of an scrollable widget.
            pub struct HorizontalScrollBarViewVar: ViewGenerator<ScrollBarArgs> = default_scrollbar();

            /// View generator for the little square that joins the two scrollbars when both are visible.
            pub struct ScrollBarJoinerViewVar: ViewGenerator<()> = view_generator!(|_, _| fill_color(scrollbar::theme::BackgroundVar));

            /// Vertical offset added when the [`ScrollDownCommand`] runs and removed when the [`ScrollUpCommand`] runs.
            ///
            /// Relative lengths are relative to the viewport height, default value is `1.dip()`.
            ///
            /// [`ScrollDownCommand`]: crate::widgets::scrollable::commands::ScrollDownCommand
            /// [`ScrollUpCommand`]: crate::widgets::scrollable::commands::ScrollUpCommand
            pub struct VerticalScrollUnitVar: Length = 1.dip();

            /// Horizontal offset added when the [`ScrollRightCommand`] runs and removed when the [`ScrollLeftCommand`] runs.
            ///
            /// Relative lengths are relative to the viewport width, default value is `1.dip()`.
            ///
            /// [`ScrollLeftCommand`]: crate::widgets::scrollable::commands::ScrollLeftCommand
            /// [`ScrollRightCommand`]: crate::widgets::scrollable::commands::ScrollRightCommand
            pub struct HorizontalScrollUnitVar: Length = 1.dip();

            /// Vertical offset added when the [`PageDownCommand`] runs and removed when the [`PageUpCommand`] runs.
            ///
            /// Relative lengths are relative to the viewport height, default value is `100.pct()`.
            ///
            /// [`ScrollDownCommand`]: crate::widgets::scrollable::commands::ScrollDownCommand
            /// [`ScrollUpCommand`]: crate::widgets::scrollable::commands::ScrollUpCommand
            pub struct VerticalPageUnitVar: Length = 100.pct().into();

            /// Horizontal offset added when the [`PageRightCommand`] runs and removed when the [`PageLeftCommand`] runs.
            ///
            /// Relative lengths are relative to the viewport width, default value is `100.pct()`.
            ///
            /// [`PageLeftCommand`]: crate::widgets::scrollable::commands::PageLeftCommand
            /// [`PageRightCommand`]: crate::widgets::scrollable::commands::PageRightCommand
            pub struct HorizontalPageUnitVar: Length = 100.pct().into();
        }

        fn default_scrollbar() -> ViewGenerator<ScrollBarArgs> {
            view_generator!(|_, args: ScrollBarArgs| {
                scrollbar! {
                    thumb = scrollbar::thumb! {
                        orientation = args.orientation;
                        viewport_ratio = args.viewport_ratio.clone();
                    };
                    orientation = args.orientation;
                    visibility = args.viewport_ratio.map(|&r| if r < 1.0.fct() { Visibility::Visible } else { Visibility::Collapsed })
                }
            })
        }

        /// Vertical scrollbar generator for all scrollable widget descendants.
        #[property(context, default(default_scrollbar()))]
        pub fn v_scrollbar_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ScrollBarArgs>>) -> impl UiNode {
            with_context_var(child, VerticalScrollBarViewVar, generator)
        }

        /// Horizontal scrollbar generator for all scrollable widget descendants.
        #[property(context, default(default_scrollbar()))]
        pub fn h_scrollbar_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ScrollBarArgs>>) -> impl UiNode {
            with_context_var(child, HorizontalScrollBarViewVar, generator)
        }

        /// Scrollbar generator for both orientations applicable to all scrollable widget descendants.
        ///
        /// This property sets both [`v_scrollbar_view`] and [`h_scrollbar_view`] to the same `generator`.
        ///
        /// [`v_scrollbar_view`]: fn@v_scrollbar_view
        /// [`h_scrollbar_view`]: fn@h_scrollbar_view
        #[property(context, default(ViewGenerator::nil()))]
        pub fn scrollbar_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ScrollBarArgs>>) -> impl UiNode {
            let generator = generator.into_var();
            let child = v_scrollbar_view(child, generator.clone());
            h_scrollbar_view(child, generator)
        }

        /// Vertical offset added when the [`ScrollDownCommand`] runs and removed when the [`ScrollUpCommand`] runs.
        ///
        /// Relative lengths are relative to the viewport height.
        ///
        /// [`ScrollUpCommand`]: crate::widgets::scrollable::commands::ScrollUpCommand
        /// [`ScrollDownCommand`]: crate::widgets::scrollable::commands::ScrollDownCommand
        #[property(context, default(VerticalScrollUnitVar::default_value()))]
        pub fn v_scroll_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
            with_context_var(child, VerticalScrollUnitVar, unit)
        }

        /// Horizontal offset added when the [`ScrollRightCommand`] runs and removed when the [`ScrollLeftCommand`] runs.
        ///
        /// Relative lengths are relative to the viewport width.
        ///
        /// [`ScrollLeftCommand`]: crate::widgets::scrollable::commands::ScrollLeftCommand
        /// [`ScrollRightCommand`]: crate::widgets::scrollable::commands::ScrollRightCommand
        #[property(context, default(HorizontalScrollUnitVar::default_value()))]
        pub fn h_scroll_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
            with_context_var(child, HorizontalScrollUnitVar, unit)
        }

        /// Horizontal and vertical offsets used when scrolling.
        ///
        /// This property sets the [`h_scroll_unit`] and [`v_scroll_unit`].
        ///
        /// [`h_scroll_unit`]: fn@h_scroll_unit
        /// [`v_scroll_unit`]: fn@v_scroll_unit
        #[property(context, default(HorizontalScrollUnitVar::default_value(), VerticalScrollUnitVar::default_value()))]
        pub fn scroll_units(child: impl UiNode, horizontal: impl IntoVar<Length>, vertical: impl IntoVar<Length>) -> impl UiNode {
            let child = h_scroll_unit(child, horizontal);
            v_scroll_unit(child, vertical)
        }

        /// Vertical offset added when the [`PageDownCommand`] runs and removed when the [`PageUpCommand`] runs.
        ///
        /// Relative lengths are relative to the viewport height.
        ///
        /// [`PageUpCommand`]: crate::widgets::scrollable::commands::PageUpCommand
        /// [`PageDownCommand`]: crate::widgets::scrollable::commands::PageDownCommand
        #[property(context, default(VerticalPageUnitVar::default_value()))]
        pub fn v_page_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
            with_context_var(child, VerticalPageUnitVar, unit)
        }

        /// Horizontal offset added when the [`PageRightCommand`] runs and removed when the [`PageLeftCommand`] runs.
        ///
        /// Relative lengths are relative to the viewport width.
        ///
        /// [`PageLeftCommand`]: crate::widgets::scrollable::commands::PageLeftCommand
        /// [`PageRightCommand`]: crate::widgets::scrollable::commands::PageRightCommand
        #[property(context, default(HorizontalPageUnitVar::default_value()))]
        pub fn h_page_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
            with_context_var(child, HorizontalPageUnitVar, unit)
        }

        /// Horizontal and vertical offsets used when page-scrolling.
        ///
        /// This property sets the [`h_page_unit`] and [`v_page_unit`].
        ///
        /// [`h_page_unit`]: fn@h_page_unit
        /// [`v_page_unit`]: fn@v_page_unit
        #[property(context, default(HorizontalPageUnitVar::default_value(), VerticalPageUnitVar::default_value()))]
        pub fn page_units(child: impl UiNode, horizontal: impl IntoVar<Length>, vertical: impl IntoVar<Length>) -> impl UiNode {
            let child = h_page_unit(child, horizontal);
            v_page_unit(child, vertical)
        }

        /// Arguments for scrollbar view generators.
        #[derive(Clone)]
        pub struct ScrollBarArgs {
            /// Scrollbar orientation.
            pub orientation: scrollbar::Orientation,

            /// Amount scrolled.
            ///
            /// If the content the content top or left is fully visible it is `0.0`, the the content bottom or right is
            /// fully visible it is `1.0`.
            pub offset: RcVar<Factor>,

            /// Viewport size / content size.
            ///
            /// If the content is smaller or equal to the available area this var is `1.0`, if the content is ten times
            /// larger then the available size this var is `0.1`.
            pub viewport_ratio: ReadOnlyRcVar<Factor>,
        }
        impl ScrollBarArgs {
            /// Arguments from scroll context.
            pub fn new(ctx: &ScrollContext, orientation: scrollbar::Orientation) -> Self {
                match orientation {
                    scrollbar::Orientation::Horizontal => Self {
                        orientation,
                        offset: ctx.h_offset.clone(),
                        viewport_ratio: ctx.h_ratio.clone(),
                    },
                    scrollbar::Orientation::Vertical => Self {
                        orientation,
                        offset: ctx.v_offset.clone(),
                        viewport_ratio: ctx.v_ratio.clone(),
                    },
                }
            }
        }
    }

    /// UI nodes used for building the scrollable widget.
    pub mod nodes {
        use super::commands::*;
        use super::*;

        /// The actual content presenter.
        pub fn viewport(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {
            use crate::core::render::ScrollId;

            struct ViewportNode<C, M> {
                scroll_id: ScrollId,
                child: C,
                mode: M,

                viewport_size: PxSize,
                content_size: PxSize,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, M: Var<ScrollMode>> UiNode for ViewportNode<C, M> {
                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    subscriptions.var(ctx, &self.mode);
                    self.child.subscriptions(ctx, subscriptions);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    self.child.update(ctx);

                    if self.mode.is_new(ctx) {
                        ctx.updates.layout();
                    }
                }

                fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
                    let mode = self.mode.copy(ctx);
                    if mode.contains(ScrollMode::VERTICAL) {
                        available_size.height = AvailablePx::Infinite;
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        available_size.width = AvailablePx::Infinite;
                    }

                    let ct_size = self.child.measure(ctx, available_size);

                    if mode.contains(ScrollMode::VERTICAL) && ct_size.height != self.content_size.height {
                        self.content_size.height = ct_size.height;
                        ctx.updates.render();
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) && ct_size.width != self.content_size.width {
                        self.content_size.width = ct_size.width;
                        ctx.updates.render();
                    }

                    ct_size
                }

                fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                    if self.viewport_size != final_size {
                        self.viewport_size = final_size;
                        ctx.updates.render();
                    }

                    let mode = self.mode.copy(ctx);
                    if !mode.contains(ScrollMode::VERTICAL) {
                        self.content_size.height = final_size.height;
                    }
                    if !mode.contains(ScrollMode::HORIZONTAL) {
                        self.content_size.width = final_size.width;
                    }

                    self.child.arrange(ctx, widget_layout, self.content_size);

                    let cell_ctx = ScrollContextVar::get(ctx.vars).as_ref().unwrap();
                    let v_ratio = self.viewport_size.height.0 as f32 / self.content_size.height.0 as f32;
                    let h_ratio = self.viewport_size.width.0 as f32 / self.content_size.width.0 as f32;

                    cell_ctx.v_ratio_var.set_ne(ctx, v_ratio.fct());
                    cell_ctx.h_ratio_var.set_ne(ctx, h_ratio.fct());
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    frame.push_scroll_frame(self.scroll_id, self.viewport_size, PxRect::from_size(self.content_size), |frame| {
                        self.child.render(ctx, frame);
                    })
                }
            }
            ViewportNode {
                child,
                scroll_id: ScrollId::new_unique(),
                mode: mode.into_var(),
                viewport_size: PxSize::zero(),
                content_size: PxSize::zero(),
            }
        }

        /// Create a node that generates and presents the [vertical scrollbar].
        ///
        /// [vertical scrollbar]: VerticalScrollBarViewVar
        pub fn v_scrollbar_presenter() -> impl UiNode {
            scrollbar_presenter(VerticalScrollBarViewVar, scrollbar::Orientation::Vertical)
        }

        /// Create a node that generates and presents the [horizontal scrollbar].
        ///
        /// [horizontal scrollbar]: HorizontalScrollBarViewVar
        pub fn h_scrollbar_presenter() -> impl UiNode {
            scrollbar_presenter(HorizontalScrollBarViewVar, scrollbar::Orientation::Horizontal)
        }

        fn scrollbar_presenter(var: impl IntoVar<ViewGenerator<ScrollBarArgs>>, orientation: scrollbar::Orientation) -> impl UiNode {
            ViewGenerator::presenter(
                var,
                |_vars, _widget| {
                    // TODO
                },
                move |ctx, is_new| {
                    if is_new {
                        if let Some(ctx) = ScrollContext::get(ctx) {
                            DataUpdate::Update(ScrollBarArgs::new(ctx, orientation))
                        } else {
                            DataUpdate::None
                        }
                    } else if let Some(new_ctx) = ScrollContext::get_new(ctx) {
                        if let Some(ctx) = new_ctx {
                            DataUpdate::Update(ScrollBarArgs::new(ctx, orientation))
                        } else {
                            DataUpdate::None
                        }
                    } else {
                        DataUpdate::Same
                    }
                },
            )
        }

        /// Create a node that generates and presents the [scrollbar joiner].
        ///
        /// [scrollbar joiner]: ScrollBarJoinerViewVar
        pub fn scrollbar_joiner_presenter() -> impl UiNode {
            ViewGenerator::presenter_default(ScrollBarJoinerViewVar)
        }

        /// Create a node that implements [`ScrollUpCommand`], [`ScrollDownCommand`],
        /// [`ScrollLeftCommand`] and [`ScrollRightCommand`] scoped on the widget.
        pub fn scroll_commands_node(child: impl UiNode) -> impl UiNode {
            struct ScrollCommandsNode<C> {
                child: C,

                up: CommandHandle,
                down: CommandHandle,
                left: CommandHandle,
                right: CommandHandle,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for ScrollCommandsNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let scope = ctx.path.widget_id();

                    self.up = ScrollUpCommand.scoped(scope).new_handle(ctx, false);
                    self.down = ScrollDownCommand.scoped(scope).new_handle(ctx, false);
                    self.left = ScrollDownCommand.scoped(scope).new_handle(ctx, false);
                    self.right = ScrollDownCommand.scoped(scope).new_handle(ctx, false);

                    self.child.init(ctx);
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.child.deinit(ctx);

                    self.up = CommandHandle::dummy();
                    self.down = CommandHandle::dummy();
                    self.left = CommandHandle::dummy();
                    self.right = CommandHandle::dummy();
                }

                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    let scope = ctx.path.widget_id();

                    subscriptions
                        .event(ScrollUpCommand.scoped(scope))
                        .event(ScrollDownCommand.scoped(scope))
                        .event(ScrollLeftCommand.scoped(scope))
                        .event(ScrollRightCommand.scoped(scope));

                    self.child.subscriptions(ctx, subscriptions);
                }

                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    let scope = ctx.path.widget_id();

                    if let Some(args) = ScrollUpCommand.scoped(scope).update(args) {
                        self.child.event(ctx, args);

                        if !args.stop_propagation_requested() {
                            let _unit = VerticalScrollUnitVar::get(ctx);
                            // TODO get viewport height
                        }
                    } else if let Some(args) = ScrollDownCommand.scoped(scope).update(args) {
                        self.child.event(ctx, args);

                        if !args.stop_propagation_requested() {}
                    } else if let Some(args) = ScrollLeftCommand.scoped(scope).update(args) {
                        self.child.event(ctx, args);

                        if !args.stop_propagation_requested() {}
                    } else if let Some(args) = ScrollRightCommand.scoped(scope).update(args) {
                        self.child.event(ctx, args);

                        if !args.stop_propagation_requested() {}
                    } else {
                        self.child.event(ctx, args);
                    }
                }
            }

            ScrollCommandsNode {
                child,

                up: CommandHandle::dummy(),
                down: CommandHandle::dummy(),
                left: CommandHandle::dummy(),
                right: CommandHandle::dummy(),
            }
        }

        /// Create a node that implements [`PageUpCommand`], [`PageDownCommand`],
        /// [`PageLeftCommand`] and [`PageRightCommand`] scoped on the widget.
        pub fn page_commands_node(child: impl UiNode) -> impl UiNode {
            struct PageCommandsNode<C> {
                child: C,

                up: CommandHandle,
                down: CommandHandle,
                left: CommandHandle,
                right: CommandHandle,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for PageCommandsNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let scope = ctx.path.widget_id();

                    self.up = ScrollUpCommand.scoped(scope).new_handle(ctx, false);
                    self.down = ScrollDownCommand.scoped(scope).new_handle(ctx, false);
                    self.left = ScrollDownCommand.scoped(scope).new_handle(ctx, false);
                    self.right = ScrollDownCommand.scoped(scope).new_handle(ctx, false);

                    self.child.init(ctx);
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.child.deinit(ctx);

                    self.up = CommandHandle::dummy();
                    self.down = CommandHandle::dummy();
                    self.left = CommandHandle::dummy();
                    self.right = CommandHandle::dummy();
                }

                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    let scope = ctx.path.widget_id();

                    subscriptions
                        .event(PageUpCommand.scoped(scope))
                        .event(PageDownCommand.scoped(scope))
                        .event(PageLeftCommand.scoped(scope))
                        .event(PageRightCommand.scoped(scope));

                    self.child.subscriptions(ctx, subscriptions);
                }

                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    let scope = ctx.path.widget_id();

                    if let Some(args) = PageUpCommand.scoped(scope).update(args) {
                        self.child.event(ctx, args);

                        if !args.stop_propagation_requested() {
                            let _unit = VerticalPageUnitVar::get(ctx);
                            // TODO get viewport height
                        }
                    } else if let Some(args) = PageDownCommand.scoped(scope).update(args) {
                        self.child.event(ctx, args);

                        if !args.stop_propagation_requested() {}
                    } else if let Some(args) = PageLeftCommand.scoped(scope).update(args) {
                        self.child.event(ctx, args);

                        if !args.stop_propagation_requested() {}
                    } else if let Some(args) = PageRightCommand.scoped(scope).update(args) {
                        self.child.event(ctx, args);

                        if !args.stop_propagation_requested() {}
                    } else {
                        self.child.event(ctx, args);
                    }
                }
            }

            PageCommandsNode {
                child,

                up: CommandHandle::dummy(),
                down: CommandHandle::dummy(),
                left: CommandHandle::dummy(),
                right: CommandHandle::dummy(),
            }
        }

        /// Create a node that implements [`ScrollToCommand`] scoped on the widget.
        pub fn scroll_to_command_node(child: impl UiNode) -> impl UiNode {
            struct ScrollToCommandNode<C> {
                child: C,

                handle: CommandHandle,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for ScrollToCommandNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    self.handle = ScrollToCommand.scoped(ctx.path.widget_id()).new_handle(ctx, true);
                    self.child.init(ctx);
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.handle = CommandHandle::dummy();
                    self.child.deinit(ctx);
                }

                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    subscriptions.event(ScrollToCommand.scoped(ctx.path.widget_id()));
                    self.child.subscriptions(ctx, subscriptions);
                }

                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    if let Some(args) = ScrollToCommand.scoped(ctx.path.widget_id()).update(args) {
                        if let Some(_request) = ScrollToRequest::from_args(args) {
                            // TODO
                        }
                        self.child.event(ctx, args);
                    } else {
                        self.child.event(ctx, args);
                    }
                }
            }

            ScrollToCommandNode {
                child,

                handle: CommandHandle::dummy(),
            }
        }
    }

    bitflags! {
        /// What dimensions are scrollable in an widget.
        ///
        /// If a dimension is scrollable the content can be any size in that dimension, if the size
        /// is more then available scrolling is enabled for that dimension.
        pub struct ScrollMode: u8 {
            /// Content is not scrollable.
            const NONE = 0;
            /// Content can be any height.
            const VERTICAL = 0b01;
            /// Content can be any width.
            const HORIZONTAL = 0b10;
            /// Content can be any size.
            const ALL = 0b11;
        }
    }
    impl_from_and_into_var! {
        /// Returns [`ALL`] for `true` and [`NONE`] for `false`.
        ///
        /// [`ALL`]: ScrollMode::ALL
        /// [`NONE`]: ScrollMode::NONE
        fn from(all: bool) -> ScrollMode {
            if all {
                ScrollMode::ALL
            } else {
                ScrollMode::NONE
            }
        }
    }

    /// Info about the parent scrollable widget.
    #[derive(Clone)]
    pub struct ScrollContext {
        /// Amount of vertical scroll.
        pub v_offset: RcVar<Factor>,
        /// Amount of horizontal scroll.
        pub h_offset: RcVar<Factor>,

        v_ratio_var: RcVar<Factor>,
        h_ratio_var: RcVar<Factor>,

        /// Viewport width / content width.
        pub v_ratio: ReadOnlyRcVar<Factor>,
        /// Viewport height / content height.
        pub h_ratio: ReadOnlyRcVar<Factor>,
    }
    impl fmt::Debug for ScrollContext {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "ScrollContext {{ .. }}")
        }
    }
    impl ScrollContext {
        fn new() -> Self {
            let v_ratio_var = var(1.0.fct());
            let h_ratio_var = var(1.0.fct());

            ScrollContext {
                v_offset: var(0.0.fct()),
                h_offset: var(0.0.fct()),
                v_ratio: v_ratio_var.clone().into_read_only(),
                h_ratio: h_ratio_var.clone().into_read_only(),
                v_ratio_var,
                h_ratio_var,
            }
        }

        /// Returns a context if is within an scrollable widget.
        pub fn get(vars: &impl AsRef<VarsRead>) -> Option<&Self> {
            ScrollContextVar::get(vars).as_ref()
        }

        /// Returns the context if one or more variables in it where replaced.
        pub fn get_new(vars: &impl AsRef<Vars>) -> Option<Option<&Self>> {
            ScrollContextVar::get_new(vars).map(|o| o.as_ref())
        }

        /// Call closure `f` within a context.
        pub fn with_context(vars: &impl WithVarsRead, context: &Option<ScrollContext>, f: impl FnOnce()) {
            vars.with_vars_read(|vars| vars.with_context_var(ScrollContextVar, ContextVarData::fixed(context), f))
        }
    }

    context_var! {
        struct ScrollContextVar: Option<ScrollContext> = None;
    }
}

/// Shorthand [`scrollable!`] with default properties.
///
/// [`scrollable!`]: mod@scrollable
pub fn scrollable(content: impl UiNode) -> impl UiNode {
    scrollable!(content)
}

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

        /// Width if orientation is vertical, otherwise height if orientation is horizontal.
        cross_length(impl IntoVar<Length>) = 16;

        /// Fills the thumb with [`theme::BackgroundVar`].
        background_color = theme::BackgroundVar;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the thumb is pressed.
        capture_mouse = true;

        /// When the pointer device is over this thumb.
        when self.is_cap_hovered {
            background_color = theme::hovered::BackgroundVar;
        }

        /// When the thumb is pressed.
        when self.is_pressed  {
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
            start: Option<DipPoint>,
            offset: DipVector,
            final_offset: PxVector,
            spatial_id: SpatialFrameId,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for DragNode<C> {
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                subscriptions.event(MouseMoveEvent).event(MouseInputEvent);
                self.child.subscriptions(ctx, subscriptions);
            }

            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                if let Some(start) = self.start {
                    if let Some(args) = MouseMoveEvent.update(args) {
                        match *ThumbOrientationVar::get(ctx) {
                            scrollbar::Orientation::Vertical => {
                                self.offset.y = args.position.y - start.y;
                            }
                            scrollbar::Orientation::Horizontal => {
                                self.offset.x = args.position.x - start.x;
                            }
                        }
                        ctx.updates.layout();
                        self.child.event(ctx, args);
                    } else if let Some(args) = MouseInputEvent.update(args) {
                        if args.is_primary() && args.is_mouse_up() {
                            self.start = None;
                            self.offset = DipVector::zero();
                            ctx.updates.layout();
                        }
                        self.child.event(ctx, args);
                    } else {
                        self.child.event(ctx, args);
                    }
                } else if let Some(args) = MouseInputEvent.update(args) {
                    if args.is_primary() && args.is_mouse_down() && args.concerns_widget(ctx) {
                        self.start = Some(args.position);
                    }
                    self.child.event(ctx, args);
                } else {
                    self.child.event(ctx, args);
                }
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                self.final_offset = self.offset.to_px(ctx.metrics.scale_factor.0);

                let ratio = *ThumbViewportRatioVar::get(ctx);
                match *ThumbOrientationVar::get(ctx) {
                    scrollbar::Orientation::Vertical => {
                        let thumb_height = final_size.height * ratio;
                        self.final_offset.y = self.final_offset.y.max(Px(0)).min(final_size.height - thumb_height);
                    }
                    scrollbar::Orientation::Horizontal => {
                        let thumb_width = final_size.width * ratio;
                        self.final_offset.x = self.final_offset.x.max(Px(0)).min(final_size.width - thumb_width);
                    }
                }

                widget_layout.with_custom_transform(&RenderTransform::translation_px(self.final_offset), |wo| {
                    self.child.arrange(ctx, wo, final_size)
                });
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                if self.final_offset != PxVector::zero() {
                    let transform = RenderTransform::translation(self.final_offset.x.0 as f32, self.final_offset.y.0 as f32, 0.0);
                    frame.push_reference_frame(self.spatial_id, FrameBinding::Value(transform), true, |f| self.child.render(ctx, f));
                } else {
                    self.child.render(ctx, frame);
                }
            }
        }

        DragNode {
            child,
            start: None,
            offset: DipVector::zero(),
            final_offset: PxVector::zero(),
            spatial_id: SpatialFrameId::new_unique(),
        }
    }

    fn new_context(
        child: impl UiNode,
        orientation: impl IntoVar<scrollbar::Orientation>,
        viewport_ratio: impl IntoVar<Factor>,
    ) -> impl UiNode {
        let child = with_context_var(child, ThumbOrientationVar, orientation);
        let child = with_context_var(child, ThumbViewportRatioVar, viewport_ratio);
        primitive_flags(child, PrimitiveFlags::IS_SCROLLBAR_THUMB)
    }

    context_var! {
        struct ThumbOrientationVar: scrollbar::Orientation = scrollbar::Orientation::Vertical;
        struct ThumbViewportRatioVar: Factor = 1.fct();
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
