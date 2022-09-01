//! Tests for `#[impl_ui_node(..)]` macro.
//!
//! Note: Compile error tests are in the integration tests folder: `tests/build/impl_ui_node`

use util::{assert_did_not_trace, assert_only_traced, TestTraceNode};

use crate::{
    color::RenderColor,
    context::{TestWidgetContext, WidgetContext},
    impl_ui_node, node_vec, nodes,
    render::{FrameBuilder, FrameId, FrameUpdate},
    ui_list::UiNodeVec,
    units::*,
    widget_info::{UpdateMask, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder, WidgetSubscriptions},
    window::WindowId,
    UiNode, UiNodeList, Widget,
};

#[test]
pub fn default_child() {
    struct Node<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for Node<C> {}

    test_trace(Node {
        child: TestTraceNode::default(),
    });
}
#[test]
pub fn default_delegate() {
    struct Node<C> {
        inner: C,
    }
    #[impl_ui_node(delegate = &self.inner, delegate_mut = &mut self.inner)]
    impl<C: UiNode> UiNode for Node<C> {}

    test_trace(Node {
        inner: TestTraceNode::default(),
    });
}
#[test]
pub fn default_children() {
    struct Node<C> {
        children: C,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList> UiNode for Node<C> {}

    test_trace(Node {
        children: nodes![TestTraceNode::default(), TestTraceNode::default()],
    });
}
#[test]
pub fn default_delegate_list() {
    struct Node<C> {
        inner: C,
    }
    #[impl_ui_node(delegate_list = &self.inner, delegate_list_mut = &mut self.inner)]
    impl<C: UiNodeList> UiNode for Node<C> {}

    test_trace(Node {
        inner: nodes![TestTraceNode::default(), TestTraceNode::default()],
    });
}
#[test]
pub fn default_children_iter() {
    struct Node {
        children: UiNodeVec,
    }
    #[impl_ui_node(children_iter)]
    impl UiNode for Node {}

    test_trace(Node {
        children: node_vec![TestTraceNode::default(), TestTraceNode::default()],
    })
}
#[test]
pub fn default_delegate_iter() {
    struct Node {
        inner: UiNodeVec,
    }
    #[impl_ui_node(delegate_iter = self.inner.iter(), delegate_iter_mut = self.inner.iter_mut())]
    impl UiNode for Node {}

    test_trace(Node {
        inner: node_vec![TestTraceNode::default(), TestTraceNode::default()],
    })
}
fn test_trace(node: impl UiNode) {
    let mut wgt = util::test_wgt(node);
    let mut ctx = TestWidgetContext::new();

    wgt.test_init(&mut ctx);
    assert_only_traced!(wgt.state(), "init");

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

    wgt.test_info(&mut ctx, &mut info);
    ctx.info_tree = info.finalize().0;
    assert_only_traced!(wgt.state(), "info");

    wgt.test_subscriptions(&mut ctx, &mut WidgetSubscriptions::new());
    assert_only_traced!(wgt.state(), "subscriptions");

    ctx.set_current_update(UpdateMask::all());
    wgt.test_update(&mut ctx);
    assert_only_traced!(wgt.state(), "update");

    wgt.test_layout(&mut ctx, PxConstrains2d::new_bounded_size(l_size).into());
    assert_only_traced!(wgt.state(), "layout");

    let mut frame = FrameBuilder::new_renderless(
        FrameId::INVALID,
        ctx.root_id,
        &ctx.widget_info.bounds,
        &ctx.info_tree,
        1.0.fct(),
        Default::default(),
        None,
    );
    wgt.test_render(&mut ctx, &mut frame);
    assert_only_traced!(wgt.state(), "render");

    TestTraceNode::notify_render_update(&mut wgt, &mut ctx);
    assert_only_traced!(wgt.state(), "event");

    let mut update = FrameUpdate::new(
        FrameId::INVALID,
        ctx.root_id,
        wgt.bounds_info().clone(),
        None,
        RenderColor::BLACK,
        None,
    );
    wgt.test_render_update(&mut ctx, &mut update);
    assert_only_traced!(wgt.state(), "render_update");

    wgt.test_deinit(&mut ctx);
    assert_only_traced!(wgt.state(), "deinit");
}

#[test]
pub fn allow_missing_delegate() {
    struct Node1<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for Node1<C> {
        #[allow_(zero_ui::missing_delegate)]
        fn update(&mut self, _: &mut WidgetContext) {
            // self.child.update(ctx);
        }
    }
    struct Node2<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    #[allow_(zero_ui::missing_delegate)]
    impl<C: UiNode> UiNode for Node2<C> {
        fn update(&mut self, _: &mut WidgetContext) {
            // self.child.update(ctx);
        }
    }

    fn test(node: impl UiNode) {
        let mut wgt = util::test_wgt(node);
        let mut ctx = TestWidgetContext::new();

        wgt.test_init(&mut ctx);
        assert_only_traced!(wgt.state(), "init");

        wgt.test_update(&mut ctx);
        assert_did_not_trace!(wgt.state());
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

    struct Node;
    #[impl_ui_node(none)]
    impl UiNode for Node {}

    let mut wgt = util::test_wgt(Node);
    let mut ctx = TestWidgetContext::new();

    wgt.test_init(&mut ctx);
    wgt.test_update(&mut ctx);
    wgt.test_deinit(&mut ctx);
    let (wu, u) = ctx.apply_updates();

    // we expect `test_init` to just be an init call, no extra flagging.
    assert!(!wu.info);
    assert!(!wu.subscriptions);

    // we expect defaults to make no requests.
    assert!(!wu.layout);
    assert!(wu.render.is_none());
    assert!(u.events.is_empty());
    assert!(!u.update);
    assert!(!u.layout);
    assert!(!u.render);

    wgt.test_init(&mut ctx);

    // we expect default to fill or collapsed depending on the
    let constrains = PxConstrains2d::new_unbounded()
        .with_min(Px(1), Px(8))
        .with_max(Px(100), Px(800))
        .with_fill(true, true);

    let desired_size = wgt.test_layout(&mut ctx, constrains.into());
    assert_eq!(desired_size, constrains.max_size().unwrap());

    let constrains = constrains.with_fill(false, false);
    let desired_size = wgt.test_layout(&mut ctx, constrains.into());
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
    wgt.test_info(&mut ctx, &mut info);
    let (build_info, _) = info.finalize();
    let wgt_info = build_info.get(wgt.id()).unwrap();
    assert!(wgt_info.descendants().next().is_none());
    assert!(wgt_info.meta().is_empty());

    let mut subs = WidgetSubscriptions::new();
    wgt.test_subscriptions(&mut ctx, &mut subs);
    assert!(subs.update_mask().is_none());
    assert!(subs.event_mask().is_none());

    let mut frame = FrameBuilder::new_renderless(
        FrameId::INVALID,
        ctx.root_id,
        &ctx.widget_info.bounds,
        &ctx.info_tree,
        1.0.fct(),
        Default::default(),
        None,
    );

    wgt.test_render(&mut ctx, &mut frame);
    let (_, _) = frame.finalize(&ctx.info_tree);

    // and not update render.
    let mut update = FrameUpdate::new(
        FrameId::INVALID,
        ctx.root_id,
        wgt.bounds_info().clone(),
        None,
        RenderColor::BLACK,
        None,
    );
    wgt.test_render_update(&mut ctx, &mut update);
    let (update, _) = update.finalize(&ctx.info_tree);
    assert!(!update.transforms.is_empty());
    assert!(update.floats.is_empty());
    assert!(update.colors.is_empty());
    assert!(update.clear_color.is_none());
}

mod util {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        context::{InfoContext, LayoutContext, MeasureContext, RenderContext, StaticStateId, TestWidgetContext, WidgetContext},
        event::{event, event_args, EventUpdate, EventUpdateArgs},
        render::{FrameBuilder, FrameUpdate},
        units::*,
        widget_base::implicit_base,
        widget_info::{EventMask, UpdateMask, WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions},
        UiNode, Widget,
    };

    pub(super) static TRACE_ID: StaticStateId<Vec<TraceRef>> = StaticStateId::new_unique();

    type TraceRef = Rc<RefCell<Vec<&'static str>>>;

    /// Asserts that only `method` was traced and clears the trace.
    #[macro_export]
    macro_rules! __impl_ui_node_util_assert_only_traced {
        ($state:expr, $method:expr) => {{
            let state = $state;
            let method = $method;
            if let Some(db) = state.get(&util::TRACE_ID) {
                for (i, trace_ref) in db.iter().enumerate() {
                    let mut any = false;
                    for trace_entry in trace_ref.borrow_mut().drain(..) {
                        assert_eq!(trace_entry, method, "tracer_0 traced `{trace_entry}`, expected only `{method}`");
                        any = true;
                    }
                    assert!(any, "tracer_{i} did not trace anything, expected `{method}`");
                }
            } else {
                panic!("no trace initialized, expected `{method}`");
            }
        }};
    }
    pub use __impl_ui_node_util_assert_only_traced as assert_only_traced;

    /// Asserts that no trace entry was pushed.
    #[macro_export]
    macro_rules! __impl_ui_node_util_assert_did_not_trace {
        ($state:expr) => {{
            let state = $state;
            if let Some(db) = state.get(&util::TRACE_ID) {
                for (i, trace_ref) in db.iter().enumerate() {
                    let mut any = false;
                    for trace_entry in trace_ref.borrow().iter() {
                        assert!(any, "tracer_{i} traced `{trace_entry}`, expected nothing");
                        any = true;
                    }
                }
            } else {
                panic!("no trace initialized");
            }
        }};
    }
    pub use __impl_ui_node_util_assert_did_not_trace as assert_did_not_trace;

    #[derive(Default)]
    pub struct TestTraceNode {
        trace: TraceRef,
    }
    impl TestTraceNode {
        fn test_trace(&self, method: &'static str) {
            self.trace.borrow_mut().push(method);
        }

        pub fn notify_render_update(wgt: &mut impl Widget, ctx: &mut TestWidgetContext) {
            wgt.test_event(ctx, &EventUpdate::new(RenderUpdateEvent, RenderUpdateArgs::now()));
        }
    }
    impl UiNode for TestTraceNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let db = ctx.widget_state.entry(&TRACE_ID).or_default();
            assert!(db.iter().all(|t| !Rc::ptr_eq(t, &self.trace)), "TraceNode::init called twice");
            db.push(Rc::clone(&self.trace));

            self.test_trace("init");
        }

        fn info(&self, _: &mut InfoContext, _: &mut WidgetInfoBuilder) {
            self.test_trace("info");
        }

        fn subscriptions(&self, _: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.updates(&UpdateMask::all()).events(&EventMask::all());
            self.test_trace("subscriptions");
        }

        fn deinit(&mut self, _: &mut WidgetContext) {
            self.test_trace("deinit");
        }

        fn update(&mut self, _: &mut WidgetContext) {
            self.test_trace("update");
        }

        fn event<U: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &U) {
            self.test_trace("event");

            if RenderUpdateEvent.update(args).is_some() {
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
            ..

            fn delivery_list(&self) -> EventDeliveryList {
                EventDeliveryList::all()
            }
        }
    }

    event! {
        RenderUpdateEvent: RenderUpdateArgs;
    }

    pub fn test_wgt(node: impl UiNode) -> impl Widget {
        let node = MinSizeNode {
            child: node,
            min_size: PxSize::new(Px(1), Px(1)),
        };
        let node = implicit_base::nodes::inner(node);
        implicit_base::nodes::widget(node, crate::WidgetId::new_unique())
    }

    struct MinSizeNode<C> {
        child: C,
        min_size: PxSize,
    }
    #[crate::impl_ui_node(child)]
    impl<C: UiNode> UiNode for MinSizeNode<C> {
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx).max(self.min_size)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            self.child.layout(ctx, wl).max(self.min_size)
        }
    }
}
