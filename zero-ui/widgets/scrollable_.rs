use crate::prelude::new_widget::*;
use std::fmt;

/// A single content container that can be larger on the inside.
#[widget($crate::widgets::scrollable)]
pub mod scrollable {
    use super::*;
    use bitflags::*;
    use properties::*;

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
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }

    fn new_child_context(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {
        struct ScrollableNode<N> {
            children: N,
            viewport: PxSize,
            joiner: PxSize,
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
                let _joiner = self.children.widget_measure(3, ctx, AvailableSize::from_size(self.joiner));

                PxSize::new(viewport.width + v_scroll.width, viewport.height + h_scroll.height)
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
                self.viewport = final_size - self.joiner;

                if self.viewport.width < self.joiner.width * 3.0.normal() {
                    self.joiner.width = Px(0);
                    self.viewport.width = final_size.width;
                } else {
                    self.children
                        .widget_arrange(1, ctx, PxSize::new(self.joiner.width, final_size.height - self.joiner.height))
                }

                if self.viewport.height < self.joiner.height * 3.0.normal() {
                    self.joiner.width = Px(0);
                    self.viewport.height = final_size.height;
                } else {
                    self.children
                        .widget_arrange(2, ctx, PxSize::new(final_size.width - self.joiner.width, final_size.height))
                }

                self.children.widget_arrange(0, ctx, self.viewport);
                self.children.widget_arrange(3, ctx, self.joiner);
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                self.children.widget_render(0, ctx, frame);

                if self.joiner.width > Px(0) {
                    frame.push_reference_frame(PxPoint::new(self.viewport.width, Px(0)), |frame| {
                        self.children.widget_render(1, ctx, frame);
                    });
                }

                if self.joiner.height > Px(0) {
                    frame.push_reference_frame(PxPoint::new(Px(0), self.viewport.height), |frame| {
                        self.children.widget_render(2, ctx, frame);
                    });
                }

                if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                    frame.push_reference_frame(self.viewport.to_vector().to_point(), |frame| {
                        self.children.widget_render(3, ctx, frame);
                    });
                }
            }
        }
        ScrollableNode {
            children: nodes![
                nodes::viewport(child, mode.into_var()),
                nodes::v_scrollbar_presenter(),
                nodes::h_scrollbar_presenter(),
                nodes::scrollbar_joiner_presenter(),
            ],
            viewport: PxSize::zero(),
            joiner: PxSize::zero(),
        }
    }

    fn new(child: impl UiNode, id: impl Into<WidgetId>) -> impl Widget {
        let child = with_context_var(child, ScrollContextVar, Some(ScrollContext::new()));
        crate::core::widget_base::implicit_base::new(child, id)
    }

    /// Properties that configure [`scrollable!`] widgets from parent widgets.
    ///
    /// Note that this properties are already available in the [`scrollable!`] widget directly.
    ///
    /// [`scrollable!`]: mod@crate::widgets::scrollable
    pub mod properties {
        use super::*;
        use crate::widgets::{blank, fill_color};

        context_var! {
            /// View generator for creating the vertical scrollbar of an scrollable widget.
            pub struct VerticalScrollBarViewVar: ViewGenerator<ScrollBarArgs> = view_generator!(|_, args: ScrollBarArgs| {
                blank! {
                    background_color = colors::RED;
                    width = 10;
                    visibility = args.viewport_ratio.map(|&r| if r < 1.0.normal() { Visibility::Visible } else { Visibility::Collapsed })
                }
            });

            /// View generator for creating the vertical scrollbar of an scrollable widget.
            pub struct HorizontalScrollBarViewVar: ViewGenerator<ScrollBarArgs> = view_generator!(|_, args: ScrollBarArgs| {
                blank! {
                    background_color = colors::RED;
                    height = 10;
                }
            });

            /// View generator for the little square that joins the two scrollbars when both are visible.
            pub struct ScrollBarJoinerViewVar: ViewGenerator<()> = view_generator!(|_, _| fill_color(colors::BLUE));
        }

        /// Vertical scrollbar generator for all scrollable widget descendants.
        #[property(context, default(ViewGenerator::nil()))]
        pub fn v_scrollbar_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ScrollBarArgs>>) -> impl UiNode {
            with_context_var(child, VerticalScrollBarViewVar, generator)
        }

        /// Horizontal scrollbar generator for all scrollable widget descendants.
        #[property(context, default(ViewGenerator::nil()))]
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
            pub orientation: ScrollBarOrientation,

            /// Amount scrolled.
            ///
            /// If the content the content top or left is fully visible it is `0.0`, the the content bottom or right is
            /// fully visible it is `1.0`.
            pub offset: RcVar<FactorNormal>,

            /// Viewport size / content size.
            ///
            /// If the content is smaller or equal to the available area this var is `1.0`, if the content is ten times
            /// larger then the available size this var is `0.1`.
            pub viewport_ratio: ReadOnlyRcVar<FactorNormal>,
        }
        impl ScrollBarArgs {
            /// Arguments from scroll context.
            pub fn new(ctx: &ScrollContext, orientation: ScrollBarOrientation) -> Self {
                match orientation {
                    ScrollBarOrientation::Horizontal => Self {
                        orientation,
                        offset: ctx.h_offset.clone(),
                        viewport_ratio: ctx.h_ratio.clone(),
                    },
                    ScrollBarOrientation::Vertical => Self {
                        orientation,
                        offset: ctx.v_offset.clone(),
                        viewport_ratio: ctx.v_ratio.clone(),
                    },
                }
            }
        }

        /// Orientation of a scrollbar.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ScrollBarOrientation {
            /// Bar fills the in the ***x*** dimension and scrolls left-right.
            Horizontal,
            /// Bar fills the in the ***y*** dimension and scrolls top-bottom.
            Vertical,
        }
    }

    /// UI nodes used for building the scrollable widget.
    pub mod nodes {
        use super::*;

        /// The actual content presenter, measures the content allowing any size and renders it clipped and scrolled
        /// by the TODO
        pub fn viewport(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {
            struct ViewportNode<C, M> {
                child: C,
                mode: M,
                viewport_size: PxSize,
                content_size: PxSize,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, M: Var<ScrollMode>> UiNode for ViewportNode<C, M> {
                fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
                    let mode = self.mode.copy(ctx);
                    if mode.contains(ScrollMode::VERTICAL) {
                        available_size.height = AvailablePx::Infinite;
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        available_size.width = AvailablePx::Infinite;
                    }
                    self.content_size = self.child.measure(ctx, available_size);
                    self.content_size
                }

                fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
                    self.viewport_size = final_size;

                    let mode = self.mode.copy(ctx);
                    if !mode.contains(ScrollMode::VERTICAL) {
                        self.content_size.height = final_size.height;
                    }
                    if !mode.contains(ScrollMode::HORIZONTAL) {
                        self.content_size.width = final_size.width;
                    }

                    self.child.arrange(ctx, self.content_size)
                }
            }
            ViewportNode {
                child,
                mode: mode.into_var(),
                viewport_size: PxSize::zero(),
                content_size: PxSize::zero(),
            }
        }

        /// Create a node that generates and presents the [vertical scrollbar].
        ///
        /// [vertical scrollbar]: VerticalScrollBarViewVar
        pub fn v_scrollbar_presenter() -> impl UiNode {
            scrollbar_presenter(VerticalScrollBarViewVar, ScrollBarOrientation::Vertical)
        }

        /// Create a node that generates and presents the [horizontal scrollbar].
        ///
        /// [horizontal scrollbar]: HorizontalScrollBarViewVar
        pub fn h_scrollbar_presenter() -> impl UiNode {
            scrollbar_presenter(HorizontalScrollBarViewVar, ScrollBarOrientation::Horizontal)
        }

        fn scrollbar_presenter(var: impl IntoVar<ViewGenerator<ScrollBarArgs>>, orientation: ScrollBarOrientation) -> impl UiNode {
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
        pub v_offset: RcVar<FactorNormal>,
        /// Amount of horizontal scroll.
        pub h_offset: RcVar<FactorNormal>,

        v_ratio_var: RcVar<FactorNormal>,
        h_ratio_var: RcVar<FactorNormal>,

        /// Viewport width / content width.
        pub v_ratio: ReadOnlyRcVar<FactorNormal>,
        /// Viewport height / content height.
        pub h_ratio: ReadOnlyRcVar<FactorNormal>,
    }
    impl fmt::Debug for ScrollContext {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "ScrollContext {{ .. }}")
        }
    }
    impl ScrollContext {
        fn new() -> Self {
            let v_ratio_var = var(1.0.normal());
            let h_ratio_var = var(1.0.normal());

            ScrollContext {
                v_offset: var(0.0.normal()),
                h_offset: var(0.0.normal()),
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
