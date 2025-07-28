use super::{cmd::ScrollToMode, types::*, *};
use zng_ext_input::{
    mouse::{MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT},
    pointer_capture::POINTER_CAPTURE,
};
use zng_wgt::visibility;
use zng_wgt_fill::node::flood;

context_var! {
    /// Widget function for creating the vertical scrollbar of an scroll widget.
    pub static VERTICAL_SCROLLBAR_FN_VAR: WidgetFn<ScrollBarArgs> = default_scrollbar();

    /// Widget function for creating the vertical scrollbar of an scroll widget.
    pub static HORIZONTAL_SCROLLBAR_FN_VAR: WidgetFn<ScrollBarArgs> = default_scrollbar();

    /// Widget function for the little square that joins the two scrollbars when both are visible.
    pub static SCROLLBAR_JOINER_FN_VAR: WidgetFn<()> = wgt_fn!(|_| flood(scrollbar::vis::BACKGROUND_VAR));

    /// Vertical offset added when the [`SCROLL_DOWN_CMD`] runs and removed when the [`SCROLL_UP_CMD`] runs.
    ///
    /// Relative lengths are relative to the viewport height, default value is `1.3.em()`.
    ///
    /// [`SCROLL_DOWN_CMD`]: crate::cmd::SCROLL_DOWN_CMD
    /// [`SCROLL_UP_CMD`]: crate::cmd::SCROLL_UP_CMD
    pub static VERTICAL_LINE_UNIT_VAR: Length = 1.3.em();

    /// Horizontal offset added when the [`SCROLL_RIGHT_CMD`] runs and removed when the [`SCROLL_LEFT_CMD`] runs.
    ///
    /// Relative lengths are relative to the viewport width, default value is `1.3.em()`.
    ///
    /// [`SCROLL_LEFT_CMD`]: crate::cmd::SCROLL_LEFT_CMD
    /// [`SCROLL_RIGHT_CMD`]: crate::cmd::SCROLL_RIGHT_CMD
    pub static HORIZONTAL_LINE_UNIT_VAR: Length = 1.3.em();

    /// Vertical offset added when the [`PAGE_DOWN_CMD`] runs and removed when the [`PAGE_UP_CMD`] runs.
    ///
    /// Relative lengths are relative to the viewport height, default value is `100.pct()`.
    ///
    /// [`PAGE_DOWN_CMD`]: crate::cmd::PAGE_DOWN_CMD
    /// [`PAGE_UP_CMD`]: crate::cmd::PAGE_UP_CMD
    pub static VERTICAL_PAGE_UNIT_VAR: Length = 100.pct();

    /// Horizontal offset multiplied by the [`MouseScrollDelta::LineDelta`] ***x***.
    ///
    /// [`MouseScrollDelta::LineDelta`]: zng_ext_input::mouse::MouseScrollDelta::LineDelta
    pub static HORIZONTAL_WHEEL_UNIT_VAR: Length = 60;

    /// Vertical offset multiplied by the [`MouseScrollDelta::LineDelta`] ***y***.
    ///
    /// [`MouseScrollDelta::LineDelta`]: zng_ext_input::mouse::MouseScrollDelta::LineDelta
    pub static VERTICAL_WHEEL_UNIT_VAR: Length = 60;

    /// Scale delta added or removed from the zoom scale by [`MouseScrollDelta::LineDelta`] used in zoom operations.
    ///
    /// [`MouseScrollDelta::LineDelta`]: zng_ext_input::mouse::MouseScrollDelta::LineDelta
    pub static ZOOM_WHEEL_UNIT_VAR: Factor = 10.pct();

    /// Horizontal offset added when the [`PAGE_RIGHT_CMD`] runs and removed when the [`PAGE_LEFT_CMD`] runs.
    ///
    /// Relative lengths are relative to the viewport width, default value is `100.pct()`.
    ///
    /// [`PAGE_LEFT_CMD`]: crate::cmd::PAGE_LEFT_CMD
    /// [`PAGE_RIGHT_CMD`]: crate::cmd::PAGE_RIGHT_CMD
    pub static HORIZONTAL_PAGE_UNIT_VAR: Length = 100.pct();

    /// Scroll unit multiplier used when alternate scrolling.
    pub static ALT_FACTOR_VAR: Factor = 3.fct();

    /// Smooth scrolling config for an scroll widget.
    pub static SMOOTH_SCROLLING_VAR: SmoothScrolling = SmoothScrolling::default();

    /// If a scroll widget defines its viewport size as the [`LayoutMetrics::viewport`] for the scroll content.
    ///
    /// This is `true` by default.
    ///
    /// [`LayoutMetrics::viewport`]: zng_wgt::prelude::LayoutMetrics::viewport
    pub static DEFINE_VIEWPORT_UNIT_VAR: bool = true;

    /// Scroll to mode used when scrolling to make the focused child visible.
    ///
    /// Default is minimal 0dip on all sides.
    pub static SCROLL_TO_FOCUSED_MODE_VAR: Option<ScrollToMode> = ScrollToMode::Minimal {
        margin: SideOffsets::new_all(0.dip()),
    };

    /// Extra space added to the viewport auto-hide rectangle.
    ///
    /// The scroll sets the viewport plus these offsets as the [`FrameBuilder::auto_hide_rect`], this value is used
    /// for optimizations from the render culling to lazy widgets.
    ///
    /// By default is `500.dip().min(100.pct())`, one full viewport extra capped at 500.
    ///
    /// [`FrameBuilder::auto_hide_rect`]: zng_wgt::prelude::FrameBuilder::auto_hide_rect
    pub static AUTO_HIDE_EXTRA_VAR: SideOffsets = 500.dip().min(100.pct());

    /// Color of the overscroll indicator.
    pub static OVERSCROLL_COLOR_VAR: Rgba = colors::GRAY.with_alpha(50.pct());

    /// Minimum scale allowed when [`ScrollMode::ZOOM`] is enabled.
    pub static MIN_ZOOM_VAR: Factor = 10.pct();

    /// Maximum scale allowed when [`ScrollMode::ZOOM`] is enabled.
    pub static MAX_ZOOM_VAR: Factor = 500.pct();

    /// Center point of zoom scaling done using the mouse scroll wheel.
    ///
    /// Relative values are resolved in the viewport space. The scroll offsets so that the point in the
    /// viewport and content stays as close as possible after the scale change.
    ///
    /// The default value ([`Length::Default`]) is the cursor position.
    ///
    /// [`Length::Default`]: zng_wgt::prelude::Length::Default
    pub static ZOOM_WHEEL_ORIGIN_VAR: Point = Point::default();

    /// Center point of zoom scaling done using the touch *pinch* gesture.
    ///
    /// Relative values are resolved in the viewport space. The scroll offsets so that the point in the
    /// viewport and content stays as close as possible after the scale change.
    ///
    /// The default value ([`Length::Default`]) is center point between the two touch contact points.
    ///
    /// [`Length::Default`]: zng_wgt::prelude::Length::Default
    pub static ZOOM_TOUCH_ORIGIN_VAR: Point = Point::default();

    /// If auto-scrolling on middle click is enabled.
    ///
    /// Is `true` by default.
    pub static AUTO_SCROLL_VAR: bool = true;

    /// Auto scroll icon/indicator node. The node is child of the auto scroll indicator widget, the full
    /// [`SCROLL`] context can be used in the indicator.
    ///
    /// Is [`node::default_auto_scroll_indicator`] by default.
    ///
    /// [`node::default_auto_scroll_indicator`]: crate::node::default_auto_scroll_indicator
    pub static AUTO_SCROLL_INDICATOR_VAR: WidgetFn<AutoScrollArgs> = wgt_fn!(|_| { crate::node::default_auto_scroll_indicator() });
}

fn default_scrollbar() -> WidgetFn<ScrollBarArgs> {
    wgt_fn!(|args: ScrollBarArgs| {
        Scrollbar! {
            thumb = Thumb! {
                viewport_ratio = args.viewport_ratio();
                offset = args.offset();
            };
            orientation = args.orientation;
            visibility = args.content_overflows().map_into()
        }
    })
}

/// Vertical scrollbar function for all scroll widget descendants.
///
/// This property sets the [`VERTICAL_SCROLLBAR_FN_VAR`].
#[property(CONTEXT, default(VERTICAL_SCROLLBAR_FN_VAR), widget_impl(super::ScrollbarFnMix<P>))]
pub fn v_scrollbar_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<ScrollBarArgs>>) -> impl UiNode {
    with_context_var(child, VERTICAL_SCROLLBAR_FN_VAR, wgt_fn)
}

/// Horizontal scrollbar function for all scroll widget descendants.
///
/// This property sets the [`HORIZONTAL_SCROLLBAR_FN_VAR`].
#[property(CONTEXT, default(HORIZONTAL_SCROLLBAR_FN_VAR), widget_impl(super::ScrollbarFnMix<P>))]
pub fn h_scrollbar_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<ScrollBarArgs>>) -> impl UiNode {
    with_context_var(child, HORIZONTAL_SCROLLBAR_FN_VAR, wgt_fn)
}

/// Scrollbar function for both orientations applicable to all scroll widget descendants.
///
/// This property sets both [`v_scrollbar_fn`] and [`h_scrollbar_fn`] to the same `wgt_fn`.
///
/// [`v_scrollbar_fn`]: fn@v_scrollbar_fn
/// [`h_scrollbar_fn`]: fn@h_scrollbar_fn
#[property(CONTEXT, default(WidgetFn::nil()), widget_impl(super::ScrollbarFnMix<P>))]
pub fn scrollbar_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<ScrollBarArgs>>) -> impl UiNode {
    let wgt_fn = wgt_fn.into_var();
    let child = v_scrollbar_fn(child, wgt_fn.clone());
    h_scrollbar_fn(child, wgt_fn)
}

/// Widget function for the little square in the corner that joins the two scrollbars when both are visible.
///
/// This property sets the [`SCROLLBAR_JOINER_FN_VAR`].
#[property(CONTEXT, default(SCROLLBAR_JOINER_FN_VAR), widget_impl(super::ScrollbarFnMix<P>))]
pub fn scrollbar_joiner_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    with_context_var(child, SCROLLBAR_JOINER_FN_VAR, wgt_fn)
}

/// Vertical offset added when the [`SCROLL_DOWN_CMD`] runs and removed when the [`SCROLL_UP_CMD`] runs.
///
/// Relative lengths are relative to the viewport height.
///
/// [`SCROLL_UP_CMD`]: crate::cmd::SCROLL_UP_CMD
/// [`SCROLL_DOWN_CMD`]: crate::cmd::SCROLL_DOWN_CMD
///  
/// This property sets the [`VERTICAL_LINE_UNIT_VAR`].
#[property(CONTEXT, default(VERTICAL_LINE_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn v_line_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, VERTICAL_LINE_UNIT_VAR, unit)
}

/// Horizontal offset added when the [`SCROLL_RIGHT_CMD`] runs and removed when the [`SCROLL_LEFT_CMD`] runs.
///
/// Relative lengths are relative to the viewport width.
///
/// [`SCROLL_LEFT_CMD`]: crate::cmd::SCROLL_LEFT_CMD
/// [`SCROLL_RIGHT_CMD`]: crate::cmd::SCROLL_RIGHT_CMD
///
/// This property sets the [`HORIZONTAL_LINE_UNIT_VAR`].
#[property(CONTEXT, default(HORIZONTAL_LINE_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn h_line_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, HORIZONTAL_LINE_UNIT_VAR, unit)
}

/// Horizontal and vertical offsets used when scrolling.
///
/// This property sets the [`h_line_unit`] and [`v_line_unit`].
///
/// [`h_line_unit`]: fn@h_line_unit
/// [`v_line_unit`]: fn@v_line_unit
#[property(CONTEXT, default(HORIZONTAL_LINE_UNIT_VAR, VERTICAL_LINE_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn line_units(child: impl UiNode, horizontal: impl IntoVar<Length>, vertical: impl IntoVar<Length>) -> impl UiNode {
    let child = h_line_unit(child, horizontal);
    v_line_unit(child, vertical)
}

/// Scroll unit multiplier used when alternate scrolling.
///
/// This property sets the [`ALT_FACTOR_VAR`].
#[property(CONTEXT, default(ALT_FACTOR_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn alt_factor(child: impl UiNode, factor: impl IntoVar<Factor>) -> impl UiNode {
    with_context_var(child, ALT_FACTOR_VAR, factor)
}

/// Vertical offset added when the [`PAGE_DOWN_CMD`] runs and removed when the [`PAGE_UP_CMD`] runs.
///
/// Relative lengths are relative to the viewport height.
///
/// [`PAGE_UP_CMD`]: crate::cmd::PAGE_UP_CMD
/// [`PAGE_DOWN_CMD`]: crate::cmd::PAGE_DOWN_CMD
///
/// This property sets the [`VERTICAL_PAGE_UNIT_VAR`].
#[property(CONTEXT, default(VERTICAL_PAGE_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn v_page_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, VERTICAL_PAGE_UNIT_VAR, unit)
}

/// Horizontal offset added when the [`PAGE_RIGHT_CMD`] runs and removed when the [`PAGE_LEFT_CMD`] runs.
///
/// Relative lengths are relative to the viewport width.
///
/// [`PAGE_LEFT_CMD`]: crate::cmd::PAGE_LEFT_CMD
/// [`PAGE_RIGHT_CMD`]: crate::cmd::PAGE_RIGHT_CMD
///
/// This property sets the [`HORIZONTAL_PAGE_UNIT_VAR`].
#[property(CONTEXT, default(HORIZONTAL_PAGE_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn h_page_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, HORIZONTAL_PAGE_UNIT_VAR, unit)
}

/// Horizontal and vertical offsets used when page-scrolling.
///
/// This property sets the [`h_page_unit`] and [`v_page_unit`].
///
/// [`h_page_unit`]: fn@h_page_unit
/// [`v_page_unit`]: fn@v_page_unit
#[property(CONTEXT, default(HORIZONTAL_PAGE_UNIT_VAR, VERTICAL_PAGE_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn page_units(child: impl UiNode, horizontal: impl IntoVar<Length>, vertical: impl IntoVar<Length>) -> impl UiNode {
    let child = h_page_unit(child, horizontal);
    v_page_unit(child, vertical)
}

/// Horizontal offset added when the mouse wheel is scrolling by lines.
///
/// The `unit` value is multiplied by the [`MouseScrollDelta::LineDelta`] ***x*** value to determinate the scroll delta.
///
/// [`MouseScrollDelta::LineDelta`]: zng_ext_input::mouse::MouseScrollDelta::LineDelta
///
/// This property sets the [`HORIZONTAL_WHEEL_UNIT_VAR`].
#[property(CONTEXT, default(HORIZONTAL_WHEEL_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn h_wheel_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, HORIZONTAL_WHEEL_UNIT_VAR, unit)
}

/// Vertical offset added when the mouse wheel is scrolling by lines.
///
/// The `unit` value is multiplied by the [`MouseScrollDelta::LineDelta`] ***y*** value to determinate the scroll delta.
///
/// [`MouseScrollDelta::LineDelta`]: zng_ext_input::mouse::MouseScrollDelta::LineDelta
///
/// This property sets the [`VERTICAL_WHEEL_UNIT_VAR`]`.
#[property(CONTEXT, default(VERTICAL_WHEEL_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn v_wheel_unit(child: impl UiNode, unit: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, VERTICAL_WHEEL_UNIT_VAR, unit)
}

/// Horizontal and vertical offsets used when mouse wheel scrolling.
///
/// This property sets the [`h_wheel_unit`] and [`v_wheel_unit`].
///
/// [`h_wheel_unit`]: fn@h_wheel_unit
/// [`v_wheel_unit`]: fn@v_wheel_unit
#[property(CONTEXT, default(HORIZONTAL_WHEEL_UNIT_VAR, VERTICAL_WHEEL_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn wheel_units(child: impl UiNode, horizontal: impl IntoVar<Length>, vertical: impl IntoVar<Length>) -> impl UiNode {
    let child = h_wheel_unit(child, horizontal);
    v_wheel_unit(child, vertical)
}

/// Scale delta added when the mouse wheel is zooming by lines.
///
/// The `unit` value is multiplied by the [`MouseScrollDelta::LineDelta`] value to determinate the scale delta.
///
/// [`MouseScrollDelta::LineDelta`]: zng_ext_input::mouse::MouseScrollDelta::LineDelta
///
/// This property sets the [`ZOOM_WHEEL_UNIT_VAR`].
#[property(CONTEXT, default(ZOOM_WHEEL_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn zoom_wheel_unit(child: impl UiNode, unit: impl IntoVar<Factor>) -> impl UiNode {
    with_context_var(child, ZOOM_WHEEL_UNIT_VAR, unit)
}

/// If the scroll defines its viewport size as the [`LayoutMetrics::viewport`] for the scroll content.
///
/// This property sets the [`DEFINE_VIEWPORT_UNIT_VAR`].
///
/// [`LayoutMetrics::viewport`]: zng_wgt::prelude::LayoutMetrics::viewport
#[property(CONTEXT, default(DEFINE_VIEWPORT_UNIT_VAR), widget_impl(super::ScrollUnitsMix<P>))]
pub fn define_viewport_unit(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, DEFINE_VIEWPORT_UNIT_VAR, enabled)
}

/// Smooth scrolling config.
///
/// Defines the easing animation applied to scroll offset and zoom value changes.
///
/// This property sets the [`SMOOTH_SCROLLING_VAR`].
#[property(CONTEXT, default(SMOOTH_SCROLLING_VAR), widget_impl(Scroll))]
pub fn smooth_scrolling(child: impl UiNode, config: impl IntoVar<SmoothScrolling>) -> impl UiNode {
    with_context_var(child, SMOOTH_SCROLLING_VAR, config)
}

/// Scroll-to mode used by scroll widgets when scrolling to make the focused child visible.
///
/// Default is minimal 0dip on all sides, set to `None` to disable.
///
/// Note that [`SCROLL_TO_CMD`] requests have priority over scroll-to focused if both requests
/// happen in the same event cycle.
///
/// [`SCROLL_TO_CMD`]: crate::cmd::SCROLL_TO_CMD
///
/// This property sets the [`SCROLL_TO_FOCUSED_MODE_VAR`].
#[property(CONTEXT, default(SCROLL_TO_FOCUSED_MODE_VAR), widget_impl(Scroll))]
pub fn scroll_to_focused_mode(child: impl UiNode, mode: impl IntoVar<Option<ScrollToMode>>) -> impl UiNode {
    with_context_var(child, SCROLL_TO_FOCUSED_MODE_VAR, mode)
}

/// Extra space added to the viewport auto-hide rectangle.
///
/// The scroll sets the viewport plus these offsets as the [`FrameBuilder::auto_hide_rect`], this value is used
/// for optimizations from the render culling to lazy widgets.
///
/// Scrolling can use only lightweight render updates to scroll within the extra margin, so there is an exchange of
/// performance, a larger extra space means that more widgets are rendering, but also can mean less full frame
/// requests, if there is no other widget requesting render.
///
/// By default is `500.dip().min(100.pct())`, one full viewport extra capped at 500.
///
/// This property sets the [`AUTO_HIDE_EXTRA_VAR`].
///
/// [`FrameBuilder::auto_hide_rect`]: zng_wgt::prelude::FrameBuilder::auto_hide_rect
#[property(CONTEXT, default(AUTO_HIDE_EXTRA_VAR), widget_impl(Scroll))]
pub fn auto_hide_extra(child: impl UiNode, extra: impl IntoVar<SideOffsets>) -> impl UiNode {
    with_context_var(child, AUTO_HIDE_EXTRA_VAR, extra)
}

/// Color of the overscroll indicator.
///
/// The overscroll indicator appears when touch scroll tries to scroll past an edge in a dimension
/// that can scroll.
///
/// This property sets the [`OVERSCROLL_COLOR_VAR`].
#[property(CONTEXT, default(OVERSCROLL_COLOR_VAR), widget_impl(Scroll))]
pub fn overscroll_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, OVERSCROLL_COLOR_VAR, color)
}

/// Minimum scale allowed when [`ScrollMode::ZOOM`] is enabled.
///
/// This property sets the [`MIN_ZOOM_VAR`].
#[property(CONTEXT, default(MIN_ZOOM_VAR), widget_impl(Scroll))]
pub fn min_zoom(child: impl UiNode, min: impl IntoVar<Factor>) -> impl UiNode {
    with_context_var(child, MIN_ZOOM_VAR, min)
}

/// Maximum scale allowed when [`ScrollMode::ZOOM`] is enabled.
///
/// This property sets the [`MAX_ZOOM_VAR`].
#[property(CONTEXT, default(MAX_ZOOM_VAR), widget_impl(Scroll))]
pub fn max_zoom(child: impl UiNode, max: impl IntoVar<Factor>) -> impl UiNode {
    with_context_var(child, MAX_ZOOM_VAR, max)
}

/// Center point of zoom scaling done using the mouse scroll wheel.
///
/// Relative values are resolved in the viewport space. The scroll offsets so that the point in the
/// viewport and content stays as close as possible after the scale change.
///
/// The default value ([`Length::Default`]) is the cursor position.
///
/// This property sets the [`ZOOM_WHEEL_ORIGIN_VAR`]
///
/// [`Length::Default`]: zng_wgt::prelude::Length::Default
#[property(CONTEXT, default(ZOOM_WHEEL_ORIGIN_VAR), widget_impl(Scroll))]
pub fn zoom_wheel_origin(child: impl UiNode, origin: impl IntoVar<Point>) -> impl UiNode {
    with_context_var(child, ZOOM_WHEEL_ORIGIN_VAR, origin)
}

/// Center point of zoom scaling done using the touch *pinch* gesture.
///
/// Relative values are resolved in the viewport space. The scroll offsets so that the point in the
/// viewport and content stays as close as possible after the scale change.
///
/// The default value ([`Length::Default`]) is center point between the two touch contact points.
///
/// This property sets the [`ZOOM_TOUCH_ORIGIN_VAR`].
///
/// [`Length::Default`]: zng_wgt::prelude::Length::Default
#[property(CONTEXT, default(ZOOM_TOUCH_ORIGIN_VAR), widget_impl(Scroll))]
pub fn zoom_touch_origin(child: impl UiNode, origin: impl IntoVar<Point>) -> impl UiNode {
    with_context_var(child, ZOOM_TOUCH_ORIGIN_VAR, origin)
}

/// Center point of zoom scaling done using mouse scroll wheel and touch gesture.
///
/// This property sets both [`zoom_wheel_origin`] and [`zoom_touch_origin`] to the same point.
///
/// [`zoom_wheel_origin`]: fn@zoom_wheel_origin
/// [`zoom_touch_origin`]: fn@zoom_touch_origin
#[property(CONTEXT, default(Point::default()), widget_impl(Scroll))]
pub fn zoom_origin(child: impl UiNode, origin: impl IntoVar<Point>) -> impl UiNode {
    let origin = origin.into_var();
    let child = zoom_wheel_origin(child, origin.clone());
    zoom_touch_origin(child, origin)
}

/// Enables or disables auto scroll on mouse middle click.
///
/// This is enabled by default, when enabled on middle click the [`auto_scroll_indicator`] is generated and
/// the content auto scrolls depending on the direction the mouse pointer moves away from the indicator.
///
/// This property sets the [`AUTO_SCROLL_VAR`].
///
/// [`auto_scroll_indicator`]: fn@auto_scroll_indicator
#[property(CONTEXT, default(AUTO_SCROLL_VAR), widget_impl(Scroll))]
pub fn auto_scroll(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, AUTO_SCROLL_VAR, enabled)
}

/// Auto scroll icon/indicator node.
///
/// The `indicator` is instantiated on middle click if [`auto_scroll`] is enabled, the node is layered as an adorner of the
/// scroll. All context vars and the full [`SCROLL`] context are captured and can be used in the indicator.
///
/// Is [`node::default_auto_scroll_indicator`] by default.
///
/// [`node::default_auto_scroll_indicator`]: crate::node::default_auto_scroll_indicator
/// [`auto_scroll`]: fn@auto_scroll
#[property(CONTEXT, default(AUTO_SCROLL_INDICATOR_VAR), widget_impl(Scroll))]
pub fn auto_scroll_indicator(child: impl UiNode, indicator: impl IntoVar<WidgetFn<AutoScrollArgs>>) -> impl UiNode {
    with_context_var(child, AUTO_SCROLL_INDICATOR_VAR, indicator)
}

/// Binds the [`horizontal_offset`] scroll var to the property value.
///
/// The binding is bidirectional and the scroll variable is assigned on init.
///
/// Note that settings the offset directly overrides effects like smooth scrolling, prefer using
/// the scroll commands to scroll over this property.
///
/// [`horizontal_offset`]: super::SCROLL::horizontal_offset
#[property(EVENT, widget_impl(Scroll))]
pub fn horizontal_offset(child: impl UiNode, offset: impl IntoVar<Factor>) -> impl UiNode {
    let offset = offset.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            let scroll_offset = super::SCROLL.horizontal_offset();

            if !offset.capabilities().is_always_static() {
                let binding = offset.bind_bidi(&scroll_offset);
                WIDGET.push_var_handles(binding);
            }
            scroll_offset.set_from(&offset);
        }
    })
}

/// Binds the [`vertical_offset`] scroll var to the property value.
///
/// The binding is bidirectional and the scroll variable is assigned on init.
///
/// Note that settings the offset directly overrides effects like smooth scrolling, prefer using
/// the scroll commands to scroll over this property.
///
/// [`vertical_offset`]: super::SCROLL::vertical_offset
#[property(EVENT, widget_impl(Scroll))]
pub fn vertical_offset(child: impl UiNode, offset: impl IntoVar<Factor>) -> impl UiNode {
    let offset = offset.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            let scroll_offset = super::SCROLL.vertical_offset();

            if !offset.capabilities().is_always_static() {
                let binding = offset.bind_bidi(&scroll_offset);
                WIDGET.push_var_handles(binding);
            }
            scroll_offset.set_from(&offset);
        }
    })
}

/// Binds the [`zoom_scale`] scroll var to the property value.
///
/// The binding is bidirectional and the scroll variable is assigned on init.
///
/// Note that settings the offset directly overrides effects like smooth scrolling, prefer using
/// the scroll commands to scroll over this property.
///
/// [`zoom_scale`]: super::SCROLL::zoom_scale
#[property(EVENT, widget_impl(Scroll))]
pub fn zoom_scale(child: impl UiNode, scale: impl IntoVar<Factor>) -> impl UiNode {
    let scale = scale.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            let scroll_scale = super::SCROLL.zoom_scale();

            if !scale.capabilities().is_always_static() {
                let binding = scale.bind_bidi(&scroll_scale);
                WIDGET.push_var_handles(binding);
            }
            scroll_scale.set_from(&scale);
        }
    })
}

/// Arguments for scrollbar widget functions.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
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
    /// See [`SCROLL.vertical_offset`] and [`SCROLL.horizontal_offset`] for more details.
    ///
    /// [`SCROLL.vertical_offset`]: SCROLL::vertical_offset
    /// [`SCROLL.horizontal_offset`]: SCROLL::horizontal_offset
    pub fn offset(&self) -> ContextVar<Factor> {
        use scrollbar::Orientation::*;

        match self.orientation {
            Vertical => SCROLL_VERTICAL_OFFSET_VAR,
            Horizontal => SCROLL_HORIZONTAL_OFFSET_VAR,
        }
    }

    /// Gets the context variable that gets the viewport/content ratio for the orientation.
    ///
    /// See [`SCROLL`] for more details.
    pub fn viewport_ratio(&self) -> Var<Factor> {
        use scrollbar::Orientation::*;

        match self.orientation {
            Vertical => SCROLL.vertical_ratio(),
            Horizontal => SCROLL.horizontal_ratio(),
        }
    }

    /// Gets the context variable that gets if the scrollbar should be visible.
    ///
    /// See [`SCROLL`] for more details.
    pub fn content_overflows(&self) -> Var<bool> {
        use scrollbar::Orientation::*;

        match self.orientation {
            Vertical => SCROLL.vertical_content_overflows(),
            Horizontal => SCROLL.horizontal_content_overflows(),
        }
    }
}

/// Scroll by grabbing and dragging the content with the mouse primary button.
///
/// This is not enabled by default. Note that couch pan is always enabled, this property implements
/// a similar behavior for the mouse pointer.
#[property(LAYOUT, default(false), widget_impl(Scroll))]
pub fn mouse_pan(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();
    let mut mouse_input = EventHandle::dummy();

    struct Dragging {
        _mouse_move: EventHandle,
        start: PxPoint,
        applied_offset: PxVector,
        factor: Factor,
    }
    let mut dragging = None;

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&enabled);
            if enabled.get() {
                mouse_input = MOUSE_INPUT_EVENT.subscribe(WIDGET.id());
            }
        }
        UiNodeOp::Deinit => {
            mouse_input = EventHandle::dummy();
            dragging = None;
        }
        UiNodeOp::Update { .. } => {
            if let Some(enabled) = enabled.get_new() {
                if enabled && mouse_input.is_dummy() {
                    mouse_input = MOUSE_INPUT_EVENT.subscribe(WIDGET.id());
                } else {
                    mouse_input = EventHandle::dummy();
                    dragging = None;
                }
            }
        }
        UiNodeOp::Event { update } => {
            if enabled.get() {
                c.event(update);

                if let Some(args) = MOUSE_INPUT_EVENT.on_unhandled(update) {
                    if args.is_primary() {
                        if args.is_mouse_down() {
                            let id = WIDGET.id();
                            POINTER_CAPTURE.capture_widget(id);
                            let factor = WINDOW.info().scale_factor();
                            dragging = Some(Dragging {
                                _mouse_move: MOUSE_MOVE_EVENT.subscribe(id),
                                start: args.position.to_px(factor),
                                applied_offset: PxVector::zero(),
                                factor,
                            });
                        } else {
                            dragging = None;
                            SCROLL.clear_vertical_overscroll();
                            SCROLL.clear_horizontal_overscroll();
                        }
                    }
                } else if let Some(d) = &mut dragging {
                    if let Some(args) = MOUSE_MOVE_EVENT.on_unhandled(update) {
                        let offset = d.start - args.position.to_px(d.factor);
                        let delta = d.applied_offset - offset;
                        d.applied_offset = offset;

                        if delta.y != Px(0) {
                            SCROLL.scroll_vertical_touch(-delta.y);
                        }
                        if delta.x != Px(0) {
                            SCROLL.scroll_horizontal_touch(-delta.x);
                        }
                    }
                }
            }
        }
        _ => {}
    })
}
