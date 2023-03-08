# Preload Idea

Basic code for the `preload` property idea that probably does not work.

```rust
use std::sync::Arc;

use zero_ui::core::task::parking_lot::Mutex;

use crate::{core::widget_instance::ArcNode, prelude::new_property::*};

/// Replaces the widget with `loading` until the widget completes one init, layout and render cycle.
///
/// Heavy widgets tend to be fast to update after the first cycle, but the first cycle is slow, this property
/// keeps the rest of the screen responsive while the widget "pre-loads" in background worker threads.
/// 
/// Note that while the widget is pre-loading it will not receive event and update notifications, after it is loaded
/// it will only receive new events and updates, this can cause 
#[property(CONTEXT)]
pub fn preload(child: impl UiNode, loading: impl UiNode) -> impl UiNode {
    let actual = ArcNode::new(child.boxed());
    let loading = ArcNode::new(loading.boxed());

    LoadingNode {
        child: loading.take_on_init().boxed(),
        child_is_loading: true,

        loading,
        actual,
        actual_state: Arc::new(Mutex::new(None)),
        actual_context: Arc::new(Mutex::new(None)),
    }
}

/// Latest layout context used by the loading child.
struct ActualContext {
    metrics: LayoutMetrics,
}
struct ActualState {
    info: Option<WidgetInfoBuilder>,
    frame: FrameBuilder,
}
#[ui_node(struct LoadingNode {
    child: BoxedUiNode,
    child_is_loading: bool,

    loading: ArcNode<BoxedUiNode>,

    actual: ArcNode<BoxedUiNode>,
    actual_context: Arc<Mutex<Option<ActualContext>>>,
    actual_state: Arc<Mutex<Option<ActualState>>>,
})]
impl UiNode for LoadingNode {
    // # TODO
    //
    // * Delegate all to `loading`.
    // * Spawn init, info, layout and render.
    //      - Update offloaded context after each, so get properties again after child init, and layout constrains again after info.
    // * Request info, layout, render once done.
    //      - Plug computed info, layout and frame as the swap to `child` happen.
    // * Delegate all to `child`, even if the updates turn-out to be slow.

    fn init(&mut self) {
        let mut actual = self.actual.take_on_init();
        let actual_ctx = self.actual_context.clone();
        let actual_state = self.actual_state.clone();
        task::spawn(async move {
            actual.init();
            // TODO info
            // TODO layout
            let metrics = actual_ctx.lock().take().expect("TODO, use a channel?").metrics;
            // TODO render
        });

        self.child.init();
    }

    fn deinit(&mut self) {
        self.child.deinit();

        if !self.child_is_loading {
            self.child = self.loading.take_on_init().boxed();
            self.child_is_loading = true;
            // TODO, cancel loading
        }
    }

    fn update(&mut self, updates: &mut WidgetUpdates) {
        if self.child_is_loading {
            if self.actual_state.lock().is_some() {
                // finished loading child

                self.child.deinit(); // deinit loading view.
                self.child = self.actual.take_on_init().boxed();
                self.child_is_loading = false;
                // already init, requests:
                //  info to plug the build info,
                //  layout to update the cached layout,
                //  render to plug the build frame,
                WIDGET.info().layout().render();
            } else {
                // TODO, cache updates?
                //      - vars will not be new
                //      - users should expect no updates like this?
            }
        }

        self.child.update(updates)
    }

    fn info(&self, info: &mut WidgetInfoBuilder) {
        if !self.child_is_loading {
            if let Some(done) = self.actual_state.lock().as_mut().and_then(|s| s.info.take()) {
                // TODO, plug the child info in.
                return;
            }
        }
        self.child.info(info);
    }

    fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
        self.child.measure(wm)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        if self.child_is_loading {
            *self.actual_context.lock() = Some(ActualContext { metrics: LAYOUT.metrics() });
        }
        // if is newly inited child it will reuse, or update changed metrics.
        self.child.layout(wl)
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if !self.child_is_loading {
            if let Some(done) = self.actual_state.lock().take() {
                let child_frame = done.frame;
                // TODO, plug the child frame in.
                return;
            }
        }
        self.child.render(frame);
    }
}

```