use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{fmt, mem};

use atomic::Atomic;
use atomic::Ordering::Relaxed;

use crate::core::{mouse::MOUSE_HOVERED_EVENT, timer::DeadlineVar};

use crate::core::widget_instance::extend_widget;
use crate::prelude::{
    layers::{AnchorOffset, AnchorSize, AnchorTransform},
    new_property::*,
    *,
};

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

app_local! {
    /// Tracks the instant the last tooltip was closed on the widget.
    ///
    /// This value is used to implement the [`TOOLTIP_INTERVAL_VAR`], custom tooltip implementers must set it
    /// to integrate with the [`tooltip`] implementation.
    ///
    /// [`tooltip`]: fn@tooltip
    pub static TOOLTIP_LAST_CLOSED: Option<Instant> = None;
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

#[derive(Default)]
enum TooltipState {
    #[default]
    Closed,
    Delay(DeadlineVar),
    /// Tip layer ID and duration deadline.
    Open(Arc<Atomic<Option<WidgetId>>>, Option<DeadlineVar>),
}
impl fmt::Debug for TooltipState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => write!(f, "Closed"),
            Self::Delay(_) => write!(f, "Delay(_)"),
            Self::Open(id, _) => write!(f, "Open({id:?}, _)"),
        }
    }
}

fn tooltip_node(child: impl UiNode, tip: impl IntoVar<WidgetFn<TooltipArgs>>, disabled_only: bool) -> impl UiNode {
    let tip = tip.into_var();
    let mut state = TooltipState::Closed;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&tip).sub_event(&MOUSE_HOVERED_EVENT);
        }
        UiNodeOp::Deinit => {
            child.deinit();
            if let TooltipState::Open(tooltip_id, _) = mem::take(&mut state) {
                LAYERS.remove(tooltip_id.load(Relaxed).unwrap());
                TOOLTIP_LAST_CLOSED.set(Some(Instant::now()));
            }
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                if let TooltipState::Open(tooltip_id, _) = &state {
                    if tooltip_id
                        .load(Relaxed)
                        .map(|id| !WINDOW.widget_tree().contains(id))
                        .unwrap_or(true)
                    {
                        // already closed (from the layer probably)
                        state = TooltipState::Closed;
                    }
                }
                match &state {
                    TooltipState::Open(tooltip_id, _) => {
                        let tooltip_id = tooltip_id.load(Relaxed).unwrap();
                        if !args
                            .target
                            .as_ref()
                            .map(|t| t.contains(tooltip_id) || t.contains(WIDGET.id()))
                            .unwrap_or(true)
                        {
                            LAYERS.remove(tooltip_id);
                            TOOLTIP_LAST_CLOSED.set(Some(Instant::now()));
                            state = TooltipState::Closed;
                        }
                    }
                    TooltipState::Delay(_) => {
                        if args.target.as_ref().map(|t| !t.contains(WIDGET.id())).unwrap_or(true) {
                            // cancel
                            state = TooltipState::Closed;
                        }
                    }
                    TooltipState::Closed => {
                        if args.is_mouse_enter() && args.is_enabled(WIDGET.id()) != disabled_only {
                            let mut delay = if TOOLTIP_LAST_CLOSED
                                .get()
                                .map(|t| t.elapsed() > TOOLTIP_INTERVAL_VAR.get())
                                .unwrap_or(true)
                            {
                                TOOLTIP_DELAY_VAR.get()
                            } else {
                                Duration::ZERO
                            };

                            if let Some(open) = OPEN_TOOLTIP.write().take() {
                                // close already open
                                open.cancellable.set(false);
                                LAYERS.remove(open.id);

                                // yield an update for the close deinit
                                // the `tooltip` property is a singleton
                                // that takes the widget on init, this op
                                // only takes the widget immediately if it
                                // is already deinited
                                delay = 1.ms();
                            }

                            state = if delay == Duration::ZERO {
                                TooltipState::Open(open_tooltip(tip.get(), disabled_only), duration_timer())
                            } else {
                                let delay = TIMERS.deadline(delay);
                                delay.subscribe(UpdateOp::Update, WIDGET.id()).perm();
                                TooltipState::Delay(delay)
                            };
                        }
                    }
                }
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            match &mut state {
                TooltipState::Open(tooltip_id, timer) => {
                    let id = tooltip_id.load(Relaxed);
                    #[allow(clippy::unnecessary_unwrap)]
                    if id.is_none() || OPEN_TOOLTIP.read().as_ref().map(|o| o.id) != id {
                        // closed by other tooltip
                        state = TooltipState::Closed;
                        TOOLTIP_LAST_CLOSED.set(Some(Instant::now()));
                    } else if let Some(t) = &timer {
                        if let Some(t) = t.get_new() {
                            if t.has_elapsed() {
                                LAYERS.remove(id.unwrap());
                                TOOLTIP_LAST_CLOSED.set(Some(Instant::now()));
                                state = TooltipState::Closed;
                            }
                        }
                    } else if let Some(func) = tip.get_new() {
                        LAYERS.remove(id.unwrap());
                        *tooltip_id = open_tooltip(func, disabled_only);
                    }
                }
                TooltipState::Delay(delay) => {
                    if let Some(t) = delay.get_new() {
                        if t.has_elapsed() {
                            state = TooltipState::Open(open_tooltip(tip.get(), disabled_only), duration_timer());
                        }
                    }
                }
                TooltipState::Closed => {}
            }
        }
        _ => {}
    })
}

fn open_tooltip(func: WidgetFn<TooltipArgs>, disabled: bool) -> Arc<Atomic<Option<WidgetId>>> {
    let child_id = Arc::new(Atomic::new(None));
    let anchor_id = WIDGET.id();

    let tooltip = tooltip_layer_wgt(func(TooltipArgs { disabled }).boxed(), child_id.clone(), anchor_id);

    let mode = AnchorMode {
        transform: AnchorTransform::CursorOnce(AnchorOffset::out_bottom_in_left()),
        size: AnchorSize::Window,
        viewport_bound: true,
        visibility: true,
        interactivity: false,
        corner_radius: false,
    };
    LAYERS.insert_anchored(LayerIndex::TOP_MOST, anchor_id, mode, tooltip);

    child_id
}

fn duration_timer() -> Option<DeadlineVar> {
    let duration = TOOLTIP_DURATION_VAR.get();
    if duration > Duration::ZERO {
        let dur = TIMERS.deadline(duration);
        dur.subscribe(UpdateOp::Update, WIDGET.id()).perm();
        Some(dur)
    } else {
        None
    }
}

fn tooltip_layer_wgt(child: BoxedUiNode, child_id: Arc<Atomic<Option<WidgetId>>>, anchor_id: WidgetId) -> impl UiNode {
    match_widget(child, move |c, op| match op {
        UiNodeOp::Init => {
            let mut inited = false;
            if !c.is_widget() {
                // try init, some nodes become a full widget only after init.
                c.init();
                inited = true;
            }

            if !c.is_widget() {
                // we can't create an anonymous widget here
                tracing::error!("tooltip must be a full widget after init");
                c.deinit();
                *c.child() = NilUiNode.boxed();
                return;
            }

            // inject the `layer_remove_cancellable` used to quick close
            // we can't just wrap the widget node because it needs to be
            // an widget in LAYERS (and the same widget so we can find it).
            let cancellable = var(true);
            if inited {
                c.deinit();
            }
            let widget = mem::replace(c.child(), NilUiNode.boxed());
            *c.child() = extend_widget(widget, |w| layers::layer_remove_cancellable(w, cancellable.clone()).boxed()).boxed();
            c.init();

            c.with_context(WidgetUpdateMode::Bubble, || {
                // if the tooltip is hit-testable and the mouse hovers it, the anchor widget
                // will not receive mouse-leave, because it is not the logical parent of the tooltip,
                // so we need to duplicate cleanup logic here.
                WIDGET.sub_event(&MOUSE_HOVERED_EVENT);

                let id = WIDGET.id();
                child_id.store(Some(id), Relaxed);

                // force close the other tooltip already open.
                if let Some(prev) = OPEN_TOOLTIP.write().replace(OpenTooltip {
                    id,
                    anchor_id,
                    cancellable: cancellable.clone(),
                }) {
                    if prev.id != id {
                        prev.cancellable.set(false);
                        LAYERS.remove(prev.id);
                        UPDATES.update(prev.anchor_id);
                    }
                }
            });
        }
        UiNodeOp::Deinit => {
            let mut open = OPEN_TOOLTIP.write();
            if let Some(o) = &*open {
                if Some(o.id) == c.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()) {
                    *open = None;
                }
            }
        }
        UiNodeOp::Event { update } => {
            c.event(update);

            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                let tooltip_id = match c.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()) {
                    Some(id) => id,
                    None => {
                        // was widget on init, now is not,
                        // this can happen if child is an `ArcNode` that was moved
                        return;
                    }
                };
                let keep_open = if let Some(t) = &args.target {
                    t.contains(anchor_id) || t.contains(tooltip_id)
                } else {
                    false
                };
                if !keep_open {
                    LAYERS.remove(tooltip_id);
                    TOOLTIP_LAST_CLOSED.set(Some(Instant::now()));
                }
            }
        }
        _ => {}
    })
}

app_local! {
    static OPEN_TOOLTIP: Option<OpenTooltip> = None;
}
struct OpenTooltip {
    id: WidgetId,
    anchor_id: WidgetId,
    cancellable: ArcVar<bool>,
}
