use zero_ui_core::mouse::MOUSE_HOVERED_EVENT;

use crate::prelude::{new_property::*, *};

/// Arguments for the [`tooltip`] generator.
///
/// Is empty as of the current release.
///
/// [`tooltip`]: fn@tooltip
#[derive(Debug, Clone)]
pub struct TooltipArgs {}

/// Set a widget generator that is used to create a tooltip widget that is generated in inserted as a top-most layer
/// when the widget is hovered or the tooltip is activated in any other way.
#[property(CONTEXT, default(WidgetGenerator::nil()))]
pub fn tooltip(child: impl UiNode, tooltip: impl IntoVar<WidgetGenerator<TooltipArgs>>) -> impl UiNode {
    #[ui_node(struct TooltipNode {
        child: impl UiNode,
        tooltip: impl Var<WidgetGenerator<TooltipArgs>>,
        open: Option<WidgetId>,
    })]
    impl UiNode for TooltipNode {
        fn init(&mut self) {
            WIDGET.sub_var(&self.tooltip).sub_event(&MOUSE_HOVERED_EVENT);
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
                    self.open = Some(open_tooltip(self.tooltip.get()));
                }
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);
            if let Some(tooltip_id) = self.open {
                if let Some(gen) = self.tooltip.get_new() {
                    LAYERS.remove(tooltip_id);
                    self.open = Some(open_tooltip(gen));
                }
            }
        }
    }
    TooltipNode {
        child,
        tooltip: tooltip.into_var(),
        open: None,
    }
}

fn open_tooltip(gen: WidgetGenerator<TooltipArgs>) -> WidgetId {
    let tooltip = TooltipLayerNode {
        child: gen.generate(TooltipArgs {}).into_widget(),
        anchor_id: WIDGET.id(),
    };

    let id = tooltip.with_context(|| WIDGET.id()).unwrap();

    let mode = AnchorMode {
        transform: window::AnchorTransform::InnerOffset(Point::bottom_left()),
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
