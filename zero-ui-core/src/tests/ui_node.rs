//! Tests for `#[ui_node(..)]` macro.
//!
//! Note: Compile error tests are in the integration tests folder: `tests/build/ui_node`

use util::{assert_did_not_trace, assert_only_traced, TestTraceNode};

use crate::{
    color::RenderColor,
    context::{TestWidgetContext, WidgetContext, WidgetUpdates},
    render::{FrameBuilder, FrameId, FrameUpdate},
    ui_node,
    units::*,
    widget_info::{WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder},
    widget_instance::{ui_list, UiNode, UiNodeList},
    window::WindowId,
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
    #[ui_node(delegate = &self.inner, delegate_mut = &mut self.inner)]
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
        children: ui_list![TestTraceNode::default(), TestTraceNode::default()],
    });
}
#[test]
pub fn default_delegate_list() {
    struct Node<C> {
        inner: C,
    }
    #[ui_node(delegate_list = &self.inner, delegate_list_mut = &mut self.inner)]
    impl<C: UiNodeList> UiNode for Node<C> {}

    test_trace(Node {
        inner: ui_list![TestTraceNode::default(), TestTraceNode::default()],
    });
}
fn test_trace(node: impl UiNode) {
    let mut wgt = util::test_wgt(node);
    let mut ctx = TestWidgetContext::new();

    ctx.init(&mut wgt);
    assert_only_traced!(wgt, "init");

    let l_size = PxSize::new(1000.into(), 800.into());
    let window_id = WindowId::new_unique();
    let mut info = WidgetInfoBuilder::new(
        window_id,
        ctx.root_id,
        WidgetBoundsInfo::new_size(l_size, l_size),
        WidgetBorderInfo::new(),
        1.fct(),
        None,
    );

    ctx.info(&wgt, &mut info);
    ctx.info_tree = info.finalize().0;
    assert_only_traced!(wgt, "info");

    ctx.update(&mut wgt, None);
    assert_only_traced!(wgt, "update");

    ctx.layout(&mut wgt, PxConstrains2d::new_bounded_size(l_size).into());
    assert_only_traced!(wgt, "layout");

    let mut frame = FrameBuilder::new_renderless(
        FrameId::INVALID,
        ctx.root_id,
        &ctx.widget_info.bounds,
        &ctx.info_tree,
        1.0.fct(),
        Default::default(),
        None,
    );
    ctx.render(&wgt, &mut frame);
    assert_only_traced!(wgt, "render");

    TestTraceNode::notify_render_update(&mut wgt, &mut ctx);
    assert_only_traced!(wgt, "event");

    let mut update = FrameUpdate::new(
        FrameId::INVALID,
        ctx.root_id,
        wgt.with_context(|w| w.widget_info.bounds.clone()).expect("expected widget"),
        None,
        RenderColor::BLACK,
        None,
    );
    ctx.render_update(&wgt, &mut update);
    assert_only_traced!(wgt, "render_update");

    ctx.deinit(&mut wgt);
    assert_only_traced!(wgt, "deinit");
}

#[test]
pub fn allow_missing_delegate() {
    #[ui_node(struct Node1 { child: impl UiNode })]
    impl UiNode for Node1 {
        #[allow_(zero_ui::missing_delegate)]
        fn update(&mut self, _: &mut WidgetContext, _: &mut WidgetUpdates) {
            // self.child.update(ctx, updates);
        }
    }
    #[ui_node(struct Node2 { child: impl UiNode })]
    #[allow_(zero_ui::missing_delegate)]
    impl UiNode for Node2 {
        fn update(&mut self, _: &mut WidgetContext, _: &mut WidgetUpdates) {
            // self.child.update(ctx, updates);
        }
    }

    fn test(node: impl UiNode) {
        let mut wgt = util::test_wgt(node);
        let mut ctx = TestWidgetContext::new();

        ctx.init(&mut wgt);
        assert_only_traced!(wgt, "init");

        ctx.update(&mut wgt, None);
        assert_did_not_trace!(wgt);
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
    crate::test_log();

    #[ui_node(struct Node { })]
    impl UiNode for Node {}

    let mut wgt = util::test_wgt(Node {});
    let mut ctx = TestWidgetContext::new();

    ctx.init(&mut wgt);
    ctx.update(&mut wgt, None);
    ctx.deinit(&mut wgt);
    let (wu, u) = ctx.apply_updates();

    // we expect `test_init` to just be an init call, no extra flagging.
    assert!(!wu.info);

    // we expect defaults to make no requests.
    assert!(!wu.layout);
    assert!(wu.render.is_none());
    assert!(u.events.is_empty());
    assert!(!u.update);
    assert!(!u.layout);
    assert!(!u.render);

    ctx.init(&mut wgt);

    // we expect default to fill or collapsed depending on the
    let constrains = PxConstrains2d::new_unbounded()
        .with_min(Px(1), Px(8))
        .with_max(Px(100), Px(800))
        .with_fill(true, true);

    let desired_size = ctx.layout(&mut wgt, constrains.into());
    assert_eq!(desired_size, constrains.max_size().unwrap());

    let constrains = constrains.with_fill(false, false);
    let desired_size = ctx.layout(&mut wgt, constrains.into());
    assert_eq!(desired_size, constrains.min_size());

    // we expect default to not render anything (except a hit-rect for the window).
    let window_id = WindowId::new_unique();

    let mut info = WidgetInfoBuilder::new(
        window_id,
        ctx.root_id,
        WidgetBoundsInfo::new_size(desired_size, desired_size),
        WidgetBorderInfo::new(),
        1.fct(),
        None,
    );
    ctx.info(&wgt, &mut info);
    let (build_info, _) = info.finalize();
    let wgt_info = build_info.get(wgt.with_context(|w| w.id).unwrap()).unwrap();
    assert!(wgt_info.descendants().next().is_none());
    assert!(wgt_info.meta().is_empty());
    ctx.info_tree = build_info;

    let mut frame = FrameBuilder::new_renderless(
        FrameId::INVALID,
        ctx.root_id,
        &ctx.widget_info.bounds,
        &ctx.info_tree,
        1.0.fct(),
        Default::default(),
        None,
    );

    ctx.render(&wgt, &mut frame);
    let (_, _) = frame.finalize(&ctx.info_tree);

    // and not update render.
    let mut update = FrameUpdate::new(
        FrameId::INVALID,
        ctx.root_id,
        wgt.with_context(|w| w.widget_info.bounds.clone()).expect("expected widget"),
        None,
        RenderColor::BLACK,
        None,
    );
    ctx.render_update(&wgt, &mut update);
    let (update, _) = update.finalize(&ctx.info_tree);
    assert!(!update.transforms.is_empty());
    assert!(update.floats.is_empty());
    assert!(update.colors.is_empty());
    assert!(update.clear_color.is_none());
}

mod util {
    use parking_lot::Mutex;
    use std::sync::Arc;

    use crate::{
        context::{
            InfoContext, LayoutContext, MeasureContext, RenderContext, StaticStateId, TestWidgetContext, UpdateDeliveryList, WidgetContext,
            WidgetUpdates,
        },
        event::{event, event_args, EventUpdate},
        render::{FrameBuilder, FrameUpdate},
        units::*,
        widget_base,
        widget_info::{WidgetInfoBuilder, WidgetLayout},
        widget_instance::{UiNode, WidgetId},
    };

    pub(super) static TRACE_ID: StaticStateId<Vec<TraceRef>> = StaticStateId::new_unique();

    type TraceRef = Arc<Mutex<Vec<&'static str>>>;

    /// Asserts that only `method` was traced and clears the trace.
    #[macro_export]
    macro_rules! __ui_node_util_assert_only_traced {
        ($wgt:ident, $method:expr) => {{
            let method = $method;
            $wgt.with_context(|ctx| {
                if let Some(db) = ctx.widget_state.get(&util::TRACE_ID) {
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
            .expect("expected widget");
        }};
    }
    pub use __ui_node_util_assert_only_traced as assert_only_traced;

    /// Asserts that no trace entry was pushed.
    #[macro_export]
    macro_rules! __ui_node_util_assert_did_not_trace {
        ($wgt:ident) => {{
            $wgt.with_context(|ctx| {
                if let Some(db) = ctx.widget_state.get(&util::TRACE_ID) {
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
            .expect("expected widget");
        }};
    }
    pub use __ui_node_util_assert_did_not_trace as assert_did_not_trace;

    #[derive(Default)]
    pub struct TestTraceNode {
        trace: TraceRef,
    }
    impl TestTraceNode {
        fn test_trace(&self, method: &'static str) {
            self.trace.lock().push(method);
        }

        pub fn notify_render_update(wgt: &mut impl UiNode, ctx: &mut TestWidgetContext) {
            let id = wgt.with_context(|ctx| ctx.id).expect("expected widget");
            let mut update = RENDER_UPDATE_EVENT.new_update_custom(RenderUpdateArgs::now(id), UpdateDeliveryList::new_any());
            ctx.event(wgt, &mut update);
        }
    }
    impl UiNode for TestTraceNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let db = ctx.widget_state.entry(&TRACE_ID).or_default();
            assert!(db.iter().all(|t| !Arc::ptr_eq(t, &self.trace)), "TraceNode::init called twice");
            db.push(Arc::clone(&self.trace));

            self.test_trace("init");
        }

        fn info(&self, _: &mut InfoContext, _: &mut WidgetInfoBuilder) {
            self.test_trace("info");
        }

        fn deinit(&mut self, _: &mut WidgetContext) {
            self.test_trace("deinit");
        }

        fn update(&mut self, _: &mut WidgetContext, _: &mut WidgetUpdates) {
            self.test_trace("update");
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.test_trace("event");

            if RENDER_UPDATE_EVENT.has(update) {
                ctx.updates.render_update();
            }
        }

        fn measure(&self, _: &mut MeasureContext) -> PxSize {
            self.test_trace("measure");
            PxSize::zero()
        }

        fn layout(&mut self, _: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            self.test_trace("layout");
            PxSize::zero()
        }

        fn render(&self, _: &mut RenderContext, _: &mut FrameBuilder) {
            self.test_trace("render");
        }

        fn render_update(&self, _: &mut RenderContext, _: &mut FrameUpdate) {
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
        let node = MinSizeNode {
            child: node,
            min_size: PxSize::new(Px(1), Px(1)),
        };
        let node = widget_base::nodes::inner(node);
        widget_base::nodes::widget(node, crate::widget_instance::WidgetId::new_unique())
    }

    struct MinSizeNode<C> {
        child: C,
        min_size: PxSize,
    }
    #[crate::ui_node(child)]
    impl<C: UiNode> UiNode for MinSizeNode<C> {
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx).max(self.min_size)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            self.child.layout(ctx, wl).max(self.min_size)
        }
    }
}
