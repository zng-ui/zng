//! Properties that configure [`scroll!`] widgets from parent widgets.
//!
//! Note that this properties are already available in the [`scroll!`] widget directly.
//!
//! [`scroll!`]: mod@crate::widgets::scroll

use crate::widgets::flood;

use super::{commands::ScrollToMode, parts::*, types::*, *};

context_var! {
    /// View generator for creating the vertical scrollbar of an scroll widget.
    pub static VERTICAL_SCROLLBAR_VIEW_VAR: ViewGenerator<ScrollBarArgs> = default_scrollbar();

    /// View generator for creating the vertical scrollbar of an scroll widget.
    pub static HORIZONTAL_SCROLLBAR_VIEW_VAR: ViewGenerator<ScrollBarArgs> = default_scrollbar();

    /// View generator for the little square that joins the two scrollbars when both are visible.
    pub static SCROLLBAR_JOINER_VIEW_VAR: ViewGenerator<()> = view_generator!(|_, _| flood(scrollbar::vis::BACKGROUND_VAR));

    /// Vertical offset added when the [`ScrollDownCommand`] runs and removed when the [`ScrollUpCommand`] runs.
    ///
    /// Relative lengths are relative to the viewport height, default value is `1.3.em()`.
    ///
    /// [`ScrollDownCommand`]: crate::widgets::scroll::commands::ScrollDownCommand
    /// [`ScrollUpCommand`]: crate::widgets::scroll::commands::ScrollUpCommand
    pub static VERTICAL_LINE_UNIT_VAR: Length = 1.3.em();

    /// Horizontal offset added when the [`ScrollRightCommand`] runs and removed when the [`ScrollLeftCommand`] runs.
    ///
    /// Relative lengths are relative to the viewport width, default value is `1.3.em()`.
    ///
    /// [`ScrollLeftCommand`]: crate::widgets::scroll::commands::ScrollLeftCommand
    /// [`ScrollRightCommand`]: crate::widgets::scroll::commands::ScrollRightCommand
    pub static HORIZONTAL_LINE_UNIT_VAR: Length = 1.3.em();

    /// Vertical offset added when the [`PageDownCommand`] runs and removed when the [`PageUpCommand`] runs.
    ///
    /// Relative lengths are relative to the viewport height, default value is `100.pct()`.
    ///
    /// [`ScrollDownCommand`]: crate::widgets::scroll::commands::ScrollDownCommand
    /// [`ScrollUpCommand`]: crate::widgets::scroll::commands::ScrollUpCommand
    pub static VERTICAL_PAGE_UNIT_VAR: Length = 100.pct();

    /// Horizontal offset added when the [`PageRightCommand`] runs and removed when the [`PageLeftCommand`] runs.
    ///
    /// Relative lengths are relative to the viewport width, default value is `100.pct()`.
    ///
    /// [`PageLeftCommand`]: crate::widgets::scroll::commands::PageLeftCommand
    /// [`PageRightCommand`]: crate::widgets::scroll::commands::PageRightCommand
    pub static HORIZONTAL_PAGE_UNIT_VAR: Length = 100.pct();

    /// Scroll unit multiplier used when alternate scrolling.
    pub static ALT_FACTOR_VAR: Factor = 3.fct();

    /// Smooth scrolling config for an scroll widget.
    pub static SMOOTH_SCROLLING_VAR: SmoothScrolling = SmoothScrolling::default();

    /// If a scroll widget defines its viewport size as the [`LayoutMetrics::viewport`] for the scroll content.
    ///
    /// This is `true` by default.
    pub static DEFINE_VIEWPORT_UNIT_VAR: bool = true;

    /// Scroll to mode used by scroll widgets when scrolling to make the focused child visible.
    ///
    /// Default is minimal 0dip on all sides.
    pub static SCROLL_TO_FOCUSED_MODE_VAR: ScrollToMode = ScrollToMode::Minimal {
        margin: SideOffsets::new_all(0.dip())
    };
}

fn default_scrollbar() -> ViewGenerator<ScrollBarArgs> {
    view_generator!(|_, args: ScrollBarArgs| {
        scrollbar! {
            thumb = scrollbar::thumb! {
                orientation = args.orientation;
                viewport_ratio = args.viewport_ratio();
                offset = args.offset();
            };
            orientation = args.orientation;
            visibility = args.content_overflows().map_into()
        }
    })
}

/// Vertical scrollbar generator for all scroll widget descendants.
#[property(context, default(VERTICAL_SCROLLBAR_VIEW_VAR))]
pub fn v_scrollbar_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ScrollBarArgs>>) -> impl UiNode {
    with_context_var(child, VERTICAL_SCROLLBAR_VIEW_VAR, generator)
}

/// Horizontal scrollbar generator for all scroll widget descendants.
#[property(context, default(HORIZONTAL_SCROLLBAR_VIEW_VAR))]
pub fn h_scrollbar_view(child: impl UiNode, generator: impl IntoVar<ViewGenerator<ScrollBarArgs>>) -> impl UiNode {
    with_context_var(child, HORIZONTAL_SCROLLBAR_VIEW_VAR, generator)
}

/// Scrollbar generator for both orientations applicable to all scroll widget descendants.
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
/// [`ScrollUpCommand`]: crate::widgets::scroll::commands::ScrollUpCommand
/// [`ScrollDownCommand`]: crate::widgets::scroll::commands::ScrollDownCommand
#[property(context, default(VERTICAL_LINE_UNIT_VAR))]
pub fn v_line_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, VERTICAL_LINE_UNIT_VAR, unit)
}

/// Horizontal offset added when the [`ScrollRightCommand`] runs and removed when the [`ScrollLeftCommand`] runs.
///
/// Relative lengths are relative to the viewport width.
///
/// [`ScrollLeftCommand`]: crate::widgets::scroll::commands::ScrollLeftCommand
/// [`ScrollRightCommand`]: crate::widgets::scroll::commands::ScrollRightCommand
#[property(context, default(HORIZONTAL_LINE_UNIT_VAR))]
pub fn h_line_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, HORIZONTAL_LINE_UNIT_VAR, unit)
}

/// Horizontal and vertical offsets used when scrolling.
///
/// This property sets the [`h_line_unit`] and [`v_line_unit`].
///
/// [`h_line_unit`]: fn@h_line_unit
/// [`v_line_unit`]: fn@v_line_unit
#[property(context, default(HORIZONTAL_LINE_UNIT_VAR, VERTICAL_LINE_UNIT_VAR))]
pub fn line_units(child: impl UiNode, horizontal: impl IntoVar<Length>, vertical: impl IntoVar<Length>) -> impl UiNode {
    let child = h_line_unit(child, horizontal);
    v_line_unit(child, vertical)
}

/// Scroll unit multiplier used when alternate scrolling.
#[property(context, default(ALT_FACTOR_VAR))]
pub fn alt_factor(child: impl UiNode, factor: impl IntoVar<Factor>) -> impl UiNode {
    with_context_var(child, ALT_FACTOR_VAR, factor)
}

/// Vertical offset added when the [`PageDownCommand`] runs and removed when the [`PageUpCommand`] runs.
///
/// Relative lengths are relative to the viewport height.
///
/// [`PageUpCommand`]: crate::widgets::scroll::commands::PageUpCommand
/// [`PageDownCommand`]: crate::widgets::scroll::commands::PageDownCommand
#[property(context, default(VERTICAL_PAGE_UNIT_VAR))]
pub fn v_page_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, VERTICAL_PAGE_UNIT_VAR, unit)
}

/// Horizontal offset added when the [`PageRightCommand`] runs and removed when the [`PageLeftCommand`] runs.
///
/// Relative lengths are relative to the viewport width.
///
/// [`PageLeftCommand`]: crate::widgets::scroll::commands::PageLeftCommand
/// [`PageRightCommand`]: crate::widgets::scroll::commands::PageRightCommand
#[property(context, default(HORIZONTAL_PAGE_UNIT_VAR))]
pub fn h_page_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, HORIZONTAL_PAGE_UNIT_VAR, unit)
}

/// Horizontal and vertical offsets used when page-scrolling.
///
/// This property sets the [`h_page_unit`] and [`v_page_unit`].
///
/// [`h_page_unit`]: fn@h_page_unit
/// [`v_page_unit`]: fn@v_page_unit
#[property(context, default(HORIZONTAL_PAGE_UNIT_VAR, VERTICAL_PAGE_UNIT_VAR))]
pub fn page_units(child: impl UiNode, horizontal: impl IntoVar<Length>, vertical: impl IntoVar<Length>) -> impl UiNode {
    let child = h_page_unit(child, horizontal);
    v_page_unit(child, vertical)
}

/// Smooth scrolling config.
#[property(context, default(SMOOTH_SCROLLING_VAR))]
pub fn smooth_scrolling(child: impl UiNode, config: impl IntoVar<SmoothScrolling>) -> impl UiNode {
    with_context_var(child, SMOOTH_SCROLLING_VAR, config)
}

/// If the scroll defines its viewport size as the [`LayoutMetrics::viewport`] for the scroll content.
#[property(context, default(DEFINE_VIEWPORT_UNIT_VAR))]
pub fn define_viewport_unit(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, DEFINE_VIEWPORT_UNIT_VAR, enabled)
}

/// Scroll to mode used by scroll widgets when scrolling to make the focused child visible.
#[property(context, default(SCROLL_TO_FOCUSED_MODE_VAR))]
pub fn scroll_to_focused_mode(child: impl UiNode, mode: impl IntoVar<ScrollToMode>) -> impl UiNode {
    with_context_var(child, SCROLL_TO_FOCUSED_MODE_VAR, mode)
}

/// Arguments for scrollbar view generators.
#[derive(Clone)]
pub struct ScrollBarArgs {
    /// Scrollbar orientation.
    pub orientation: scrollbar::Orientation,
}
impl ScrollBarArgs {
    /// Arguments from scroll context.
    pub fn new(orientation: scrollbar::Orientation) -> Self {
        Self { orientation }
    }

    /// Gets the context variable that gets and sets the offset for the orientation.
    ///
    /// See [`SCROLL_VERTICAL_OFFSET_VAR`] and [`SCROLL_HORIZONTAL_OFFSET_VAR`] for more details.
    pub fn offset(&self) -> ContextVar<Factor> {
        use scrollbar::Orientation::*;

        match self.orientation {
            Vertical => SCROLL_VERTICAL_OFFSET_VAR,
            Horizontal => SCROLL_HORIZONTAL_OFFSET_VAR,
        }
    }

    /// Gets the context variable that gets the viewport/content ratio for the orientation.
    ///
    /// See [`ScrollContext`] for more details.
    pub fn viewport_ratio(&self) -> ReadOnlyContextVar<Factor> {
        use scrollbar::Orientation::*;

        match self.orientation {
            Vertical => ScrollContext::vertical_ratio(),
            Horizontal => ScrollContext::horizontal_ratio(),
        }
    }

    /// Gets the context variable that gets if the scrollbar should be visible.
    ///
    /// See [`ScrollContext`] for more details.
    pub fn content_overflows(&self) -> BoxedVar<bool> {
        use scrollbar::Orientation::*;

        match self.orientation {
            Vertical => ScrollContext::vertical_content_overflows().boxed(),
            Horizontal => ScrollContext::horizontal_content_overflows().boxed(),
        }
    }
}
