//! Scroll properties.
//!
//! Properties that make an widget content scrollable.

use std::fmt;

use crate::prelude::new_property::*;
use bitflags::*;

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

/// Enable or disable scrolling in the widget.
#[property(outer, default(ScrollMode::NONE))]
pub fn scrollable(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {
    struct ScrollableNode<C, M> {
        child: C,
        mode: M,

        h_ratio: RcVar<FactorNormal>,
        v_ratio: RcVar<FactorNormal>,
        scroll_ctx: ScrollContext,

        child_size: PxSize,
        viewport: PxRect,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, M: Var<ScrollMode>> UiNode for ScrollableNode<C, M> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.mode.is_new(ctx) {
                ctx.updates.layout();
            } else {
                let mode = self.mode.copy(ctx);

                if (mode.contains(ScrollMode::VERTICAL) && self.scroll_ctx.v_offset.is_new(ctx))
                    || (mode.contains(ScrollMode::HORIZONTAL) && self.scroll_ctx.h_offset.is_new(ctx))
                {
                    ctx.updates.render_update();
                }
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
            let mode = self.mode.copy(ctx);
            if mode.contains(ScrollMode::VERTICAL) {
                available_size.height = AvailablePx::Infinite;
            }
            if mode.contains(ScrollMode::HORIZONTAL) {
                available_size.width = AvailablePx::Infinite;
            }

            self.child_size = self.child.measure(ctx, available_size);

            available_size.clip(self.child_size)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            self.viewport.size = final_size;

            let mode = self.mode.copy(ctx);

            if !mode.contains(ScrollMode::VERTICAL) && self.child_size.height > final_size.height {
                self.child_size.height = final_size.height;
            }
            if !mode.contains(ScrollMode::HORIZONTAL) && self.child_size.width > final_size.width {
                self.child_size.width = final_size.width;
            }

            let h_ratio = final_size.width.0 as f32 / self.child_size.width.0 as f32;
            let v_ratio = final_size.height.0 as f32 / self.child_size.height.0 as f32;

            //self.h_ratio.set_ne(ctx, h_ratio.normal());
            //self.v_ratio.set_ne(ctx, v_ratio.normal());

            self.child.arrange(ctx, self.child_size);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            todo!()
        }

        fn render_update(&self, ctx: &mut RenderContext, updates: &mut FrameUpdate) {
            todo!()
        }
    }

    let h_ratio = var(0.0.normal());
    let v_ratio = var(0.0.normal());

    ScrollableNode {
        child,
        mode: mode.into_var(),

        scroll_ctx: ScrollContext {
            v_offset: var(0.0.normal()),
            h_offset: var(0.0.normal()),
            v_ratio: v_ratio.clone().into_read_only(),
            h_ratio: h_ratio.clone().into_read_only(),
        },
        h_ratio,
        v_ratio,

        child_size: PxSize::zero(),
        viewport: PxRect::zero(),
    }
}

context_var! {
    /// View generator for creating the vertical scrollbar of an scrollable widget.
    pub struct VerticalScrollBarViewVar: ViewGenerator<ScrollBarArgs> = ViewGenerator::nil();

    /// View generator for creating the vertical scrollbar of an scrollable widget.
    pub struct HorizontalScrollBarViewVar: ViewGenerator<ScrollBarArgs> = ViewGenerator::nil();
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

/// Create a node that generates and presents the [vertical scrollbar].
///
/// [vertical scrollbar]: VerticalScrollBarViewVar
pub fn v_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(VerticalScrollBarViewVar, ScrollBarOrientation::Horizontal)
}

/// Create a node that generates and presents the [horizontal scrollbar].
///
/// [horizontal scrollbar]: HorizontalScrollBarViewVar
pub fn h_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(HorizontalScrollBarViewVar, ScrollBarOrientation::Vertical)
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

/// Info about the parent scrollable widget.
#[derive(Clone)]
pub struct ScrollContext {
    /// Amount of vertical scroll.
    pub v_offset: RcVar<FactorNormal>,
    /// Amount of horizontal scroll.
    pub h_offset: RcVar<FactorNormal>,

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
