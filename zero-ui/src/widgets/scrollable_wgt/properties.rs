//! Properties that configure [`scrollable!`] widgets from parent widgets.
//!
//! Note that this properties are already available in the [`scrollable!`] widget directly.
//!
//! [`scrollable!`]: mod@crate::widgets::scrollable

use crate::widgets::fill_color;

use super::{parts::*, types::*, *};

context_var! {
    /// View generator for creating the vertical scrollbar of an scrollable widget.
    pub struct VerticalScrollBarViewVar: ViewGenerator<ScrollBarArgs> = default_scrollbar();

    /// View generator for creating the vertical scrollbar of an scrollable widget.
    pub struct HorizontalScrollBarViewVar: ViewGenerator<ScrollBarArgs> = default_scrollbar();

    /// View generator for the little square that joins the two scrollbars when both are visible.
    pub struct ScrollBarJoinerViewVar: ViewGenerator<()> = view_generator!(|_, _| fill_color(scrollbar::theme::BackgroundVar));

    /// Vertical offset added when the [`ScrollDownCommand`] runs and removed when the [`ScrollUpCommand`] runs.
    ///
    /// Relative lengths are relative to the viewport height, default value is `1.3.em()`.
    ///
    /// [`ScrollDownCommand`]: crate::widgets::scrollable::commands::ScrollDownCommand
    /// [`ScrollUpCommand`]: crate::widgets::scrollable::commands::ScrollUpCommand
    pub struct VerticalLineUnitVar: Length = 1.3.em();

    /// Horizontal offset added when the [`ScrollRightCommand`] runs and removed when the [`ScrollLeftCommand`] runs.
    ///
    /// Relative lengths are relative to the viewport width, default value is `1.3.em()`.
    ///
    /// [`ScrollLeftCommand`]: crate::widgets::scrollable::commands::ScrollLeftCommand
    /// [`ScrollRightCommand`]: crate::widgets::scrollable::commands::ScrollRightCommand
    pub struct HorizontalLineUnitVar: Length = 1.3.em();

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

    /// Scroll unit multiplier used when alternate scrolling.
    pub struct AltFactorVar: Factor = 3.fct();
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
#[property(context, default(VerticalLineUnitVar::default_value()))]
pub fn v_line_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, VerticalLineUnitVar, unit)
}

/// Horizontal offset added when the [`ScrollRightCommand`] runs and removed when the [`ScrollLeftCommand`] runs.
///
/// Relative lengths are relative to the viewport width.
///
/// [`ScrollLeftCommand`]: crate::widgets::scrollable::commands::ScrollLeftCommand
/// [`ScrollRightCommand`]: crate::widgets::scrollable::commands::ScrollRightCommand
#[property(context, default(HorizontalLineUnitVar::default_value()))]
pub fn h_line_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, HorizontalLineUnitVar, unit)
}

/// Horizontal and vertical offsets used when scrolling.
///
/// This property sets the [`h_line_unit`] and [`v_line_unit`].
///
/// [`h_line_unit`]: fn@h_line_unit
/// [`v_line_unit`]: fn@v_line_unit
#[property(context, default(HorizontalLineUnitVar::default_value(), VerticalLineUnitVar::default_value()))]
pub fn line_units(child: impl UiNode, horizontal: impl IntoVar<Length>, vertical: impl IntoVar<Length>) -> impl UiNode {
    let child = h_line_unit(child, horizontal);
    v_line_unit(child, vertical)
}

/// Scroll unit multiplier used when alternate scrolling.
#[property(context, default(AltFactorVar::default_value()))]
pub fn alt_factor(child: impl UiNode, factor: impl IntoVar<Factor>) -> impl UiNode {
    with_context_var(child, AltFactorVar, factor)
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
}
impl ScrollBarArgs {
    /// Arguments from scroll context.
    pub fn new(orientation: scrollbar::Orientation) -> Self {
        Self { orientation }
    }

    /// Gets the context variable that gets and sets the offset for the orientation.
    ///
    /// See [`ScrollVerticalOffsetVar`] and [`ScrollHorizontalOffsetVar`] for more details.
    pub fn offset(&self) -> BoxedVar<Factor> {
        use scrollbar::Orientation::*;

        match self.orientation {
            Vertical => ScrollVerticalOffsetVar::new().boxed(),
            Horizontal => ScrollHorizontalOffsetVar::new().boxed(),
        }
    }

    /// Gets the context variable that gets the viewport/content ratio for the orientation.
    ///
    /// See [`ScrollContext`] for more details.
    pub fn viewport_ratio(&self) -> BoxedVar<Factor> {
        use scrollbar::Orientation::*;

        match self.orientation {
            Vertical => ScrollContext::vertical_ratio().boxed(),
            Horizontal => ScrollContext::horizontal_ratio().boxed(),
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
