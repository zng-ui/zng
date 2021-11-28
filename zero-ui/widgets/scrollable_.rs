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
            align as content_align = Alignment::CENTER;

            /// Scroll mode.
            ///
            /// By default scrolls in both dimensions.
            mode(impl IntoVar<ScrollMode>) = ScrollMode::ALL;
        }

        /// Scrollbar generator for both orientations applicable to all scrollable widget descendants.
        ///
        /// This property sets both [`v_scrollbar_view`] and [`h_scrollbar_view`] to the same `generator`.
        ///
        /// [`v_scrollbar_view`]: #wp-v_scrollbar_view
        /// [`h_scrollbar_view`]: #wp-h_scrollbar_view
        scrollbar_view;

        /// Vertical scrollbar generator.
        v_scrollbar_view;

        /// Horizontal scrollbar generator.
        h_scrollbar_view;

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

            fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
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

                self.children.widget_arrange(0, ctx, widget_offset, self.viewport);

                let joiner_offset = self.viewport.to_vector();
                widget_offset.with_offset(PxVector::new(joiner_offset.x, Px(0)), |wo| {
                    self.children
                        .widget_arrange(1, ctx, wo, PxSize::new(self.joiner.width, self.viewport.height))
                });
                widget_offset.with_offset(PxVector::new(Px(0), joiner_offset.y), |wo| {
                    self.children
                        .widget_arrange(2, ctx, wo, PxSize::new(self.viewport.width, self.joiner.height))
                });

                widget_offset.with_offset(joiner_offset, |wo| self.children.widget_arrange(3, ctx, wo, self.joiner));
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                self.children.widget_render(0, ctx, frame);

                if self.joiner.width > Px(0) {
                    frame.push_reference_frame_item(self.spatial_id, 1, PxPoint::new(self.viewport.width, Px(0)), |frame| {
                        self.children.widget_render(1, ctx, frame);
                    });
                }

                if self.joiner.height > Px(0) {
                    frame.push_reference_frame_item(self.spatial_id, 2, PxPoint::new(Px(0), self.viewport.height), |frame| {
                        self.children.widget_render(2, ctx, frame);
                    });
                }

                if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                    frame.push_reference_frame_item(self.spatial_id, 3, self.viewport.to_vector().to_point(), |frame| {
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
        with_context_var(child, ScrollContextVar, Some(ScrollContext::new()))
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

                fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
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

                    self.child.arrange(ctx, widget_offset, self.content_size);

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
            ViewGenerator::presenter(var, move |ctx, is_new| {
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
            })
        }

        /// Create a node that generates and presents the [scrollbar joiner].
        ///
        /// [scrollbar joiner]: ScrollBarJoinerViewVar
        pub fn scrollbar_joiner_presenter() -> impl UiNode {
            ViewGenerator::presenter_default(ScrollBarJoinerViewVar)
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
        ///
        /// The context is never [new] when set using this function.
        ///
        /// [new]: Self::get_new
        pub fn with_context(vars: &impl WithVarsRead, context: &Option<ScrollContext>, f: impl FnOnce()) {
            vars.with_vars_read(|vars| vars.with_context_var(ScrollContextVar, context, 0, f))
        }

        /// Call closure `f` within a context.
        ///
        /// The `is_new` parameter indicates that one or more variables in the context has changed.
        pub fn with_context_upt(vars: &impl WithVars, context: &Option<ScrollContext>, is_new: bool, f: impl FnOnce()) {
            vars.with_vars(|vars| vars.with_context_var(ScrollContextVar, context, is_new, 1, f))
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

    fn new_outer(child: impl UiNode, orientation: impl IntoVar<Orientation>) -> impl UiNode {
        let orientation = orientation.into_var();
        align(
            child,
            orientation.map(|o| match o {
                Orientation::Vertical => Alignment::FILL_RIGHT,
                Orientation::Horizontal => Alignment::FILL_BOTTOM,
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

    fn new_size(
        child: impl UiNode,
        orientation: impl IntoVar<scrollbar::Orientation>,
        viewport_ratio: impl IntoVar<Factor>,
        cross_length: impl IntoVar<Length>,
    ) -> impl UiNode {
        size(
            child,
            merge_var!(
                orientation.into_var(),
                viewport_ratio.into_var(),
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

    fn new_outer(child: impl UiNode) -> impl UiNode {
        struct DragNode<C> {
            child: C,
            start: Option<DipPoint>,
            offset: DipVector,
            spatial_id: SpatialFrameId,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for DragNode<C> {
            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                if let Some(start) = self.start {
                    if let Some(args) = MouseMoveEvent.update(args) {
                        self.offset = args.position - start;
                        ctx.updates.render();
                        self.child.event(ctx, args);
                    } else if let Some(args) = MouseUpEvent.update(args) {
                        self.start = None;
                        self.offset = DipVector::zero();
                        ctx.updates.render();
                        self.child.event(ctx, args);
                    } else {
                        self.child.event(ctx, args);
                    }
                } else if let Some(args) = MouseDownEvent.update(args) {
                    if args.concerns_widget(ctx) {
                        self.start = Some(args.position);
                    }
                    self.child.event(ctx, args);
                } else {
                    self.child.event(ctx, args);
                }
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                let offset = self.offset.to_px(frame.scale_factor().0);
                if offset != PxVector::zero() {
                    frame.push_reference_frame(self.spatial_id, offset.to_point(), |f| self.child.render(ctx, f));
                } else {
                    self.child.render(ctx, frame);
                }
            }
        }

        DragNode {
            child,
            start: None,
            offset: DipVector::zero(),
            spatial_id: SpatialFrameId::new_unique(),
        }
    }

    fn new_context(child: impl UiNode) -> impl UiNode {
        primitive_flags(child, PrimitiveFlags::IS_SCROLLBAR_THUMB)
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
