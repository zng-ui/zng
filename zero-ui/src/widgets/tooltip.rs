//! Tooltip widget, properties and nodes.

use std::time::{Duration, Instant};

use zero_ui_core::mouse::MOUSE_HOVERED_EVENT;

use crate::prelude::{layers::AnchorTransform, new_widget::*, AnchorOffset};

use super::popup::{Popup, PopupState, POPUP};

/// Widget tooltip.
///
/// Any other widget can be used as tooltip, the recommended widget is the [`Tip!`] container, it provides the tooltip style. Note
/// that if the `tip` node is not a widget even after initializing it will not be shown.
///
/// # Context
///
/// This property can be configured by [`tooltip_transform`], [`tooltip_delay`], [`tooltip_interval`] and [`tooltip_duration`].
///
/// # Disabled
///
/// This tooltip only opens if the widget is enabled, see [`disabled_tooltip`] for a tooltip that only shows when the widget is disabled.
///
/// [`Tip!`]: struct@crate::widgets::Tip
/// [`tooltip_transform`]: fn@tooltip_transform
/// [`tooltip_delay`]: fn@tooltip_delay
/// [`tooltip_interval`]: fn@tooltip_interval
/// [`tooltip_duration`]: fn@tooltip_duration
/// [`disabled_tooltip`]: fn@disabled_tooltip
#[property(EVENT)]
pub fn tooltip(child: impl UiNode, tip: impl UiNode) -> impl UiNode {
    tooltip_fn(child, WidgetFn::singleton(tip))
}

/// Widget tooltip set as an widget function that is called every time the tooltip must be shown.
///
/// The `tip` widget function is used to instantiate a new tip widget when one needs to be shown, any widget
/// can be used as tooltip, the recommended widget is the [`Tip!`] container, it provides the tooltip style.
///
/// # Context
///
/// This property can be configured by [`tooltip_transform`], [`tooltip_delay`], [`tooltip_interval`] and [`tooltip_duration`].
///
/// # Disabled
///
/// This tooltip only opens if the widget is enabled, see [`disabled_tooltip_fn`] for a tooltip that only shows when the widget is disabled.
///
/// [`Tip!`]: struct@crate::widgets::Tip
/// [`tooltip_transform`]: fn@tooltip_transform
/// [`tooltip_delay`]: fn@tooltip_delay
/// [`tooltip_interval`]: fn@tooltip_interval
/// [`tooltip_duration`]: fn@tooltip_duration
/// [`disabled_tooltip_fn`]: fn@disabled_tooltip_fn
#[property(EVENT, default(WidgetFn::nil()))]
pub fn tooltip_fn(child: impl UiNode, tip: impl IntoVar<WidgetFn<TooltipArgs>>) -> impl UiNode {
    tooltip_node(child, tip, false)
}

/// Disabled widget tooltip.
///
/// This property behaves like [`tooltip`], but the tooltip only opens if the widget is disabled.
///
/// [`tooltip`]: fn@tooltip
#[property(EVENT)]
pub fn disabled_tooltip(child: impl UiNode, tip: impl UiNode) -> impl UiNode {
    disabled_tooltip_fn(child, WidgetFn::singleton(tip))
}

/// Disabled widget tooltip.
///
/// This property behaves like [`tooltip_fn`], but the tooltip only opens if the widget is disabled.
///
/// [`tooltip_fn`]: fn@tooltip
#[property(EVENT, default(WidgetFn::nil()))]
pub fn disabled_tooltip_fn(child: impl UiNode, tip: impl IntoVar<WidgetFn<TooltipArgs>>) -> impl UiNode {
    tooltip_node(child, tip, true)
}

fn tooltip_node(child: impl UiNode, tip: impl IntoVar<WidgetFn<TooltipArgs>>, disabled_only: bool) -> impl UiNode {
    let tip = tip.into_var();
    let mut pop_state = var(PopupState::Closed).read_only();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&tip).sub_event(&MOUSE_HOVERED_EVENT);
        }
        UiNodeOp::Deinit => {
            child.deinit();

            if let PopupState::Open(not_closed) = pop_state.get() {
                POPUP.force_close(not_closed);
                TOOLTIP_LAST_CLOSED.set(Some(Instant::now()));
            }
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                match pop_state.get() {
                    PopupState::Opening => {
                        if args.is_mouse_leave() {
                            pop_state
                                .on_pre_new(app_hn_once!(|a: &OnVarArgs<PopupState>| {
                                    match a.value {
                                        PopupState::Open(id) => {
                                            POPUP.force_close(id);
                                        }
                                        PopupState::Closed => {}
                                        PopupState::Opening => unreachable!(),
                                    }
                                }))
                                .perm();
                        }
                    }
                    PopupState::Open(id) => {
                        if args.is_mouse_leave() {
                            POPUP.close(id);
                        }
                    }
                    PopupState::Closed => if args.is_mouse_enter() {
                        // open
                        
                    },
                }
            }
        }
        _ => {}
    })
}

/// Set the position of the tip widgets opened for the widget or its descendants.
///
/// Tips are inserted as [`LayerIndex::TOP_MOST`] when shown, this property defines how the tip layer
/// is aligned with the *anchor* widget, or the cursor.
///
/// By default tips are aligned below the cursor position at the time they are opened.
///
/// This property sets the [`TOOLTIP_TRANSFORM_VAR`].
#[property(CONTEXT, default(TOOLTIP_TRANSFORM_VAR))]
pub fn tooltip_transform(child: impl UiNode, transform: impl IntoVar<AnchorTransform>) -> impl UiNode {
    with_context_var(child, TOOLTIP_TRANSFORM_VAR, transform)
}

/// Set the duration the cursor must be over the widget or its descendants before the tip widget is opened.
///
/// This delay applies when no other tooltip was opened within the [`tooltip_interval`] duration, otherwise the
/// tooltip opens instantly.
///
/// This property sets the [`TOOLTIP_DELAY_VAR`].
///
/// [`tooltip_interval`]: fn@tooltip_interval
#[property(CONTEXT, default(TOOLTIP_DELAY_VAR))]
pub fn tooltip_delay(child: impl UiNode, delay: impl IntoVar<Duration>) -> impl UiNode {
    with_context_var(child, TOOLTIP_DELAY_VAR, delay)
}

/// Sets the maximum interval a second tooltip is opened instantly if a previous tip was just closed. The config
/// applies for tooltips opening on the widget or descendants, but considers previous tooltips opened on any widget.
///
/// This property sets the [`TOOLTIP_INTERVAL_VAR`].
#[property(CONTEXT, default(TOOLTIP_INTERVAL_VAR))]
pub fn tooltip_interval(child: impl UiNode, interval: impl IntoVar<Duration>) -> impl UiNode {
    with_context_var(child, TOOLTIP_INTERVAL_VAR, interval)
}

/// Sets the maximum duration a tooltip stays open on the widget or descendants.
///
/// Note that the tooltip closes at the moment the cursor leaves the widget, this duration defines the
/// time the tooltip is closed even if the cursor is still hovering the widget.
///
/// This property sets the [`TOOLTIP_DURATION_VAR`].
#[property(CONTEXT, default(TOOLTIP_DURATION_VAR))]
pub fn tooltip_duration(child: impl UiNode, duration: impl IntoVar<Duration>) -> impl UiNode {
    with_context_var(child, TOOLTIP_DURATION_VAR, duration)
}

/// Arguments for tooltip widget functions.
///
/// [`tooltip_fn`]: fn@tooltip_fn
/// [`disabled_tooltip_fn`]: fn@disabled_tooltip_fn
pub struct TooltipArgs {
    /// If the tooltip is for [`disabled_tooltip_fn`], if `false` is for [`tooltip_fn`].
    ///
    /// [`tooltip_fn`]: fn@tooltip_fn
    /// [`disabled_tooltip_fn`]: fn@disabled_tooltip_fn
    pub disabled: bool,
}

app_local! {
    /// Tracks the instant the last tooltip was closed on the widget.
    ///
    /// This value is used to implement the [`TOOLTIP_INTERVAL_VAR`], custom tooltip implementers must set it
    /// to integrate with the [`tooltip`] implementation.
    ///
    /// [`tooltip`]: fn@tooltip
    pub static TOOLTIP_LAST_CLOSED: Option<Instant> = None;
}

context_var! {
    /// Position of the tip widget in relation to the anchor widget.
    ///
    /// By default the tip widget is shown below the cursor.
    pub static TOOLTIP_TRANSFORM_VAR: AnchorTransform = AnchorTransform::CursorOnce(AnchorOffset::out_bottom_in_left());

    /// Duration the cursor must be over the anchor widget before the tip widget is opened.
    pub static TOOLTIP_DELAY_VAR: Duration = 500.ms();

    /// Maximum duration from the last time a tooltip was shown that a new tooltip opens instantly.
    pub static TOOLTIP_INTERVAL_VAR: Duration = 100.ms();

    /// Maximum time a tooltip stays open.
    pub static TOOLTIP_DURATION_VAR: Duration = 0.ms();
}

/// A tooltip popup.
///
/// Can be set on the [`tooltip`] property.
///
/// [`tooltip`]: fn@tooltip
#[widget($crate::widgets::Tip {
    ($child:expr) => {
        child = $child;
    };
})]
pub struct Tip(StyleMix<FocusableMix<Popup>>);
impl Tip {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            hit_test_mode = false;
            style_fn = STYLE_VAR;
        }
    }

    widget_impl! {
        /// If the tooltip can be interacted with the mouse.
        ///
        /// This is disabled by default.
        pub crate::properties::hit_test_mode(mode: impl IntoVar<HitTestMode>);
    }
}

context_var! {
    /// Tip style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// Idle background dark and light color.
    pub static BASE_COLORS_VAR: ColorPair = (rgb(20, 20, 20), rgb(235, 235, 235));
}

/// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the tip style.
#[property(CONTEXT, default(BASE_COLORS_VAR), widget_impl(DefaultStyle))]
pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
    with_context_var(child, BASE_COLORS_VAR, color)
}

/// Sets the tip style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the tip style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Tip default style.
#[widget($crate::widgets::tooltip::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            crate::properties::padding = (2, 4);
            crate::properties::corner_radius = 3;
            crate::properties::background_color = color_scheme_pair(BASE_COLORS_VAR);
            crate::widgets::text::font_size = 10.pt();
            crate::properties::border = {
                widths: 1.px(),
                sides: color_scheme_highlight(BASE_COLORS_VAR, 0.5).map_into()
            };
        }
    }
}
