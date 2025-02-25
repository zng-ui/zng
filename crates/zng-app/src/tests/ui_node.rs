//! Tests for `#[ui_node(..)]` macro.
//!
//! Note: Compile error tests are in the integration tests folder: `tests/macro-tests/ui_node`

use util::{TestTraceNode, assert_did_not_trace, assert_only_traced};
use zng_app_proc_macros::ui_node;
use zng_layout::unit::{Px, PxConstraints2d};

use crate::{
    APP, ui_vec,
    update::WidgetUpdates,
    widget::{
        WIDGET, WidgetUpdateMode,
        node::{UiNode, UiNodeList},
    },
    window::WINDOW,
};

#[test]
pub fn default_child() {
    #[ui_node(struct Node { child: impl UiNode })]
    impl UiNode for Node {}

    test_trace(Node {
        child: TestTraceNode::default(),
    });
}
#[test]
pub fn default_delegate() {
    struct Node<C> {
        inner: C,
    }
    #[ui_node(delegate = &mut self.inner)]
    impl<C: UiNode> UiNode for Node<C> {}

    test_trace(Node {
        inner: TestTraceNode::default(),
    });
}
#[test]
pub fn default_children() {
    #[ui_node(struct Node { children: impl UiNodeList })]
    impl UiNode for Node {}

    test_trace(Node {
        children: ui_vec![TestTraceNode::default(), TestTraceNode::default()],
    });
}
#[test]
pub fn default_delegate_list() {
    struct Node<C> {
        inner: C,
    }
    #[ui_node(delegate_list = &mut self.inner)]
    impl<C: UiNodeList> UiNode for Node<C> {}

    test_trace(Node {
        inner: ui_vec![TestTraceNode::default(), TestTraceNode::default()],
    });
}
fn test_trace(node: impl UiNode) {
    let _app = APP.minimal().run_headless(false);
    let mut wgt = util::test_wgt(node);

    WINDOW.with_test_context(WidgetUpdateMode::Bubble, || {
        WINDOW.test_init(&mut wgt);
        assert_only_traced!(wgt, "init");

        WINDOW.test_info(&mut wgt);
        assert_only_traced!(wgt, "info");

        WINDOW.test_update(&mut wgt, None);
        assert_only_traced!(wgt, "update");

        WINDOW.test_layout(&mut wgt, None);
        assert_only_traced!(wgt, "layout");

        WINDOW.test_render(&mut wgt);
        assert_only_traced!(wgt, "render");

        TestTraceNode::notify_render_update(&mut wgt);
        assert_only_traced!(wgt, "event");

        WINDOW.test_render_update(&mut wgt);
        assert_only_traced!(wgt, "render_update");

        WINDOW.test_deinit(&mut wgt);
        assert_only_traced!(wgt, "deinit");
    });
}

#[test]
pub fn allow_missing_delegate() {
    #[ui_node(struct Node1 { child: impl UiNode })]
    impl UiNode for Node1 {
        #[allow_(zng::missing_delegate)]
        fn update(&mut self, _: &WidgetUpdates) {
            // self.child.update(updates);
        }
    }
    #[ui_node(struct Node2 { child: impl UiNode })]
    #[allow_(zng::missing_delegate)]
    impl UiNode for Node2 {
        fn update(&mut self, _: &WidgetUpdates) {
            // self.child.update(updates);
        }
    }

    fn test(node: impl UiNode) {
        let _app = APP.minimal().run_headless(false);
        let mut wgt = util::test_wgt(node);

        WINDOW.with_test_context(WidgetUpdateMode::Bubble, || {
            WINDOW.test_init(&mut wgt);
            assert_only_traced!(wgt, "init");
            WINDOW.test_info(&mut wgt);
            assert_only_traced!(wgt, "info");

            WINDOW.test_update(&mut wgt, None);
            assert_did_not_trace!(wgt);
        });
    }

    test(Node1 {
        child: TestTraceNode::default(),
    });
    test(Node2 {
        child: TestTraceNode::default(),
    });
}

#[test]
pub fn default_no_child() {
    let _app = APP.minimal().run_headless(false);
    crate::test_log();

    #[ui_node(struct Node { })]
    impl UiNode for Node {}

    let mut wgt = util::test_wgt(Node {});

    WINDOW.with_test_context(WidgetUpdateMode::Bubble, || {
        let wu = WINDOW.test_init(&mut wgt);
        assert!(wu.info);
        assert!(wu.layout);
        assert!(wu.render);

        let wu = WINDOW.test_info(&mut wgt);
        assert!(!wu.info);
        assert!(wu.layout);
        assert!(wu.render);

        let (_, wu) = WINDOW.test_layout(&mut wgt, None);
        assert!(!wu.info);
        assert!(!wu.layout);
        assert!(wu.render);

        let (_, wu) = WINDOW.test_render(&mut wgt);
        assert!(!wu.info);
        assert!(!wu.layout);
        assert!(!wu.render);

        let wu = WINDOW.test_update(&mut wgt, None);
        assert!(!wu.has_updates());

        let wu = WINDOW.test_deinit(&mut wgt);
        assert!(wu.layout);
        assert!(wu.render);

        WINDOW.test_init(&mut wgt);
        WINDOW.test_info(&mut wgt);

        wgt.with_context(WidgetUpdateMode::Ignore, || {
            let tree = WINDOW.info();
            let wgt_info = tree.get(WIDGET.id()).unwrap();
            assert!(wgt_info.descendants().next().is_none());
        })
        .unwrap();

        let constraints = PxConstraints2d::new_unbounded()
            .with_min(Px(1), Px(8))
            .with_max(Px(100), Px(800))
            .with_fill(true, true);
        let (desired_size, _) = WINDOW.test_layout(&mut wgt, Some(constraints));
        assert_eq!(desired_size, constraints.max_size().unwrap());

        let constraints = constraints.with_fill(false, false);
        let (desired_size, _) = WINDOW.test_layout(&mut wgt, Some(constraints));
        assert_eq!(desired_size, constraints.min_size());

        WINDOW.test_render(&mut wgt);
        let (update, _) = WINDOW.test_render_update(&mut wgt);
        assert!(!update.transforms.is_empty());
        assert!(update.floats.is_empty());
        assert!(update.colors.is_empty());
        assert!(update.clear_color.is_none());
    });
}

mod util {
    use parking_lot::Mutex;
    use std::sync::Arc;
    use zng_app_proc_macros::ui_node;
    use zng_layout::unit::{Px, PxSize};
    use zng_state_map::{StateId, static_id};

    static_id! {
        pub(super) static ref TRACE_ID: StateId<Vec<TraceRef>>;
    }

    type TraceRef = Arc<Mutex<Vec<&'static str>>>;

    /// Asserts that only `method` was traced and clears the trace.
    #[macro_export]
    macro_rules! __ui_node_util_assert_only_traced {
        ($wgt:ident, $method:expr) => {{
            let method = $method;
            $wgt.with_context(WidgetUpdateMode::Bubble, || {
                WIDGET.with_state(|s| {
                    if let Some(db) = s.get(*util::TRACE_ID) {
                        for (i, trace_ref) in db.iter().enumerate() {
                            let mut any = false;
                            for trace_entry in trace_ref.lock().drain(..) {
                                assert_eq!(trace_entry, method, "tracer_0 traced `{trace_entry}`, expected only `{method}`");
                                any = true;
                            }
                            assert!(any, "tracer_{i} did not trace anything, expected `{method}`");
                        }
                    } else {
                        panic!("no trace initialized, expected `{method}`");
                    }
                })
            })
            .expect("expected widget");
        }};
    }
    pub use __ui_node_util_assert_only_traced as assert_only_traced;

    /// Asserts that no trace entry was pushed.
    #[macro_export]
    macro_rules! __ui_node_util_assert_did_not_trace {
        ($wgt:ident) => {{
            $wgt.with_context(WidgetUpdateMode::Bubble, || {
                WIDGET.with_state(|s| {
                    if let Some(db) = s.get(*util::TRACE_ID) {
                        for (i, trace_ref) in db.iter().enumerate() {
                            let mut any = false;
                            for trace_entry in trace_ref.lock().iter() {
                                assert!(any, "tracer_{i} traced `{trace_entry}`, expected nothing");
                                any = true;
                            }
                        }
                    } else {
                        panic!("no trace initialized");
                    }
                })
            })
            .expect("expected widget");
        }};
    }
    pub use __ui_node_util_assert_did_not_trace as assert_did_not_trace;

    use crate::{
        event::{event, event_args},
        render::{FrameBuilder, FrameUpdate},
        update::{EventUpdate, UpdateDeliveryList, WidgetUpdates},
        widget::{
            WIDGET, WidgetId, WidgetUpdateMode,
            info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
            node::UiNode,
        },
        window::WINDOW,
    };

    #[derive(Default)]
    pub struct TestTraceNode {
        trace: TraceRef,
    }
    impl TestTraceNode {
        fn test_trace(&self, method: &'static str) {
            self.trace.lock().push(method);
        }

        pub fn notify_render_update(wgt: &mut impl UiNode) {
            let id = wgt.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()).expect("expected widget");
            let mut update = RENDER_UPDATE_EVENT.new_update_custom(RenderUpdateArgs::now(id), UpdateDeliveryList::new_any());
            WINDOW.test_event(wgt, &mut update);
        }
    }
    impl UiNode for TestTraceNode {
        fn init(&mut self) {
            WIDGET.with_state_mut(|mut s| {
                let db = s.entry(*TRACE_ID).or_default();
                assert!(db.iter().all(|t| !Arc::ptr_eq(t, &self.trace)), "TraceNode::init called twice");
                db.push(Arc::clone(&self.trace));
            });

            self.test_trace("init");
        }

        fn info(&mut self, _: &mut WidgetInfoBuilder) {
            self.test_trace("info");
        }

        fn deinit(&mut self) {
            self.test_trace("deinit");
        }

        fn update(&mut self, _: &WidgetUpdates) {
            self.test_trace("update");
        }

        fn event(&mut self, update: &EventUpdate) {
            self.test_trace("event");

            if RENDER_UPDATE_EVENT.has(update) {
                WIDGET.render_update();
            }
        }

        fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
            self.test_trace("measure");
            PxSize::zero()
        }

        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            self.test_trace("layout");
            PxSize::zero()
        }

        fn render(&mut self, _: &mut FrameBuilder) {
            self.test_trace("render");
        }

        fn render_update(&mut self, _: &mut FrameUpdate) {
            self.test_trace("render_update");
        }
    }

    event_args! {
        struct RenderUpdateArgs {
            target: WidgetId,

            ..

            fn delivery_list(&self, list: &mut UpdateDeliveryList) {
                list.search_widget(self.target);
            }
        }
    }

    event! {
        static RENDER_UPDATE_EVENT: RenderUpdateArgs;
    }

    pub fn test_wgt(node: impl UiNode) -> impl UiNode {
        MinSizeNode {
            child: node,
            min_size: PxSize::new(Px(1), Px(1)),
        }
        .into_widget()
    }

    struct MinSizeNode<C> {
        child: C,
        min_size: PxSize,
    }
    #[ui_node(child)]
    impl<C: UiNode> UiNode for MinSizeNode<C> {
        fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm).max(self.min_size)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            self.child.layout(wl).max(self.min_size)
        }
    }
}
