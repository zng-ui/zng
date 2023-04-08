use zero_ui_core::mouse::MOUSE_HOVERED_EVENT;

use crate::prelude::{new_property::*, *};

/// Arguments for the [`tooltip`] widget function.
///
/// Is empty as of the current release.
///
/// [`tooltip`]: fn@tooltip
#[derive(Debug, Clone)]
pub struct TooltipArgs {}

/// Widget tooltip.
///
/// Any other widget can be used as tooltip, the recommended widget is the [`tip!`] container, it provides the tooltip style.
///
/// [`tip!`]: mod@crate::widgets::tip
#[property(CONTEXT)]
pub fn tooltip(child: impl UiNode, tip: impl UiNode) -> impl UiNode {
    tooltip_fn(child, WidgetFn::singleton(tip))
}

/// Widget tooltip set as an widget function that is called every time the tooltip must be shown.
///
/// The `tip` widget function is used to instantiate a new tip widget when one needs to be shown, any widget
/// can be used as tooltip, the recommended widget is the [`tip!`] container, it provides the tooltip style.
///
/// [`tip!`]: mod@crate::widgets::tip
#[property(CONTEXT, default(WidgetFn::nil()))]
pub fn tooltip_fn(child: impl UiNode, tip: impl IntoVar<WidgetFn<TooltipArgs>>) -> impl UiNode {
    #[ui_node(struct TooltipNode {
        child: impl UiNode,
        tip: impl Var<WidgetFn<TooltipArgs>>,
        open: Option<WidgetId>,
    })]
    impl UiNode for TooltipNode {
        fn init(&mut self) {
            WIDGET.sub_var(&self.tip).sub_event(&MOUSE_HOVERED_EVENT);
            self.child.init()
        }

        fn deinit(&mut self) {
            self.child.deinit();
            if let Some(id) = self.open.take() {
                LAYERS.remove(id);
            }
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.event(update);

            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                if let Some(t) = self.open {
                    if !WINDOW.widget_tree().contains(t) {
                        self.open = None;
                    }
                }
                if let Some(tooltip_id) = self.open {
                    let keep_open = if let Some(t) = &args.target {
                        t.contains(tooltip_id) || t.contains(WIDGET.id())
                    } else {
                        false
                    };
                    if !keep_open {
                        LAYERS.remove(tooltip_id);
                        self.open = None
                    }
                } else if args.is_mouse_enter() {
                    self.open = Some(open_tooltip(self.tip.get()));
                }
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);
            if let Some(tooltip_id) = self.open {
                if let Some(func) = self.tip.get_new() {
                    LAYERS.remove(tooltip_id);
                    self.open = Some(open_tooltip(func));
                }
            }
        }
    }
    TooltipNode {
        child,
        tip: tip.into_var(),
        open: None,
    }
}

fn open_tooltip(func: WidgetFn<TooltipArgs>) -> WidgetId {
    let mut child = func(TooltipArgs {}).boxed();

    if !child.is_widget() {
        let node = widget_base::nodes::widget_inner(child);

        // set hit test mode so that it's only hit-testable if the child is hit-testable
        let node = hit_test_mode(node, HitTestMode::Visual);

        child = widget_base::nodes::widget(node, WidgetId::new_unique()).boxed();
    }

    let tooltip = TooltipLayerNode {
        child,
        anchor_id: WIDGET.id(),
    };

    let id = tooltip.with_context(|| WIDGET.id()).unwrap();

    let mode = AnchorMode {
        transform: window::AnchorTransform::CursorOnce(window::AnchorOffset::out_bottom_in_left()),
        size: window::AnchorSize::Unbounded,
        visibility: true,
        interactivity: false,
        corner_radius: false,
    };

    LAYERS.insert_anchored(LayerIndex::TOP_MOST, tooltip.anchor_id, mode, tooltip);

    id
}

#[ui_node(struct TooltipLayerNode {
    child: impl UiNode,
    anchor_id: WidgetId,
})]
impl UiNode for TooltipLayerNode {
    fn with_context<R, F: FnOnce() -> R>(&self, f: F) -> Option<R> {
        self.child.with_context(f)
    }

    fn init(&mut self) {
        // if the tooltip is hit-testable and the mouse hovers it, the anchor widget
        // will not receive mouse-leave, because it is not the logical parent of the tooltip,
        // so we need to duplicate cleanup logic here.
        self.with_context(|| {
            WIDGET.sub_event(&MOUSE_HOVERED_EVENT);
        });
        self.child.init()
    }

    fn event(&mut self, update: &EventUpdate) {
        self.child.event(update);

        if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
            let tooltip_id = self.with_context(|| WIDGET.id()).unwrap();
            let keep_open = if let Some(t) = &args.target {
                t.contains(self.anchor_id) || t.contains(tooltip_id)
            } else {
                false
            };
            if !keep_open {
                LAYERS.remove(tooltip_id);
            }
        }
    }
}
