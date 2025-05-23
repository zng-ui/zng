use std::collections::HashSet;

use zng_app_proc_macros::{property, widget};
use zng_var::ContextInitHandle;

use crate::{
    APP, ui_vec,
    widget::{
        WidgetUpdateMode,
        base::PARALLEL_VAR,
        node::{PanelList, UiNode, UiNodeList, UiVec},
    },
    window::WINDOW,
};

use super::widget::EmptyWgt;

#[test]
pub fn init_many() {
    let _app = APP.minimal().run_headless(false);

    let list: Vec<_> = (0..1000)
        .map(|_| {
            EmptyWgt! {
                util::trace = "inited";
                util::log_init_thread = true;
            }
            .boxed()
        })
        .collect();
    let mut list = PanelList::new(list);

    WINDOW.with_test_context(WidgetUpdateMode::Bubble, || {
        PARALLEL_VAR.with_context_var(ContextInitHandle::new(), true, || {
            list.init_all();
        })
    });

    let mut count = 0;
    let mut threads = HashSet::new();
    list.for_each(|i, wgt, _| {
        assert!(util::traced(wgt, "inited"));
        assert_eq!(count, i);
        count += 1;
        threads.insert(util::get_init_thread(wgt));
    });
    assert_eq!(count, 1000);
    // assert!(threads.len() > 1); // this can fail in CI due to low core count.
}

#[test]
pub fn nested_par_each_ctx() {
    let _app = APP.minimal().run_headless(false);
    let mut test = ListWgt! {
        children = (0..1000)
            .map(|_| {
                ListWgt! {
                    children = ui_vec![
                        EmptyWgt! {
                            util::ctx_val = true;
                            util::assert_ctx_val = true;
                        },
                        EmptyWgt! {
                            util::assert_ctx_val = false;
                        }
                    ];
                }
            })
            .collect::<UiVec>();
    };

    WINDOW.with_test_context(WidgetUpdateMode::Bubble, || {
        WINDOW.test_init(&mut test);
    });
}

#[test]
pub fn par_each_ctx() {
    let _app = APP.minimal().run_headless(false);
    let mut test = ListWgt! {
        children = (0..1000)
            .flat_map(|_| {
                ui_vec![
                    EmptyWgt! {
                        util::ctx_val = true;
                        util::assert_ctx_val = true;
                    },
                    EmptyWgt! {
                        util::assert_ctx_val = false;
                    }
                ]
            })
            .collect::<UiVec>();
    };

    WINDOW.with_test_context(WidgetUpdateMode::Bubble, || {
        WINDOW.test_init(&mut test);
    });
}

#[widget($crate::tests::ui_node_list::ListWgt)]
pub struct ListWgt(crate::widget::base::WidgetBase);
impl ListWgt {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let child = util::list_node(wgt.capture_ui_node_list_or_empty(crate::property_id!(Self::children)));
            wgt.set_child(child);
        });
    }
}
#[property(CHILD, capture, widget_impl(ListWgt))]
pub fn children(children: impl UiNodeList) {}

mod util {
    use std::{
        any::Any,
        sync::Arc,
        thread::{self, ThreadId},
    };

    use zng_app_context::{ContextLocal, context_local};
    use zng_app_proc_macros::property;
    use zng_state_map::StateId;
    use zng_unique_id::static_id;
    use zng_var::IntoValue;

    use crate::widget::{
        WIDGET, WidgetUpdateMode,
        node::{UiNode, UiNodeList, UiNodeOp, match_node, match_node_list},
    };

    pub use super::super::widget::util::*;

    #[property(CONTEXT)]
    pub fn log_init_thread(child: impl UiNode, enabled: impl IntoValue<bool>) -> impl UiNode {
        let enabled = enabled.into();
        match_node(child, move |child, op| {
            if let UiNodeOp::Init = op {
                child.init();
                if enabled {
                    WIDGET.set_state(*INIT_THREAD_ID, thread::current().id());
                }
            }
        })
    }

    pub fn get_init_thread(wgt: &mut impl UiNode) -> ThreadId {
        wgt.with_context(WidgetUpdateMode::Ignore, || {
            WIDGET.get_state(*INIT_THREAD_ID).expect("did not log init thread")
        })
        .expect("node is not a widget")
    }

    static_id! {
        static ref INIT_THREAD_ID: StateId<ThreadId>;
    }

    context_local! {
        static CTX_VAL: bool = false;
    }

    #[property(CONTEXT, default(*CTX_VAL.get()))]
    pub fn ctx_val(child: impl UiNode, value: impl IntoValue<bool>) -> impl UiNode {
        with_context_local(child, &CTX_VAL, value)
    }

    #[property(CHILD)]
    pub fn assert_ctx_val(child: impl UiNode, expected: impl IntoValue<bool>) -> impl UiNode {
        let expected = expected.into();
        match_node(child, move |child, op| {
            if let UiNodeOp::Init = op {
                child.init();

                // thread::sleep(1.ms());

                assert_eq!(expected, *CTX_VAL.get());
            }
        })
    }

    pub fn list_node(children: impl UiNodeList) -> impl UiNode {
        match_node_list(children, |_, _| {})
    }

    fn with_context_local<T: Any + Send + Sync + 'static>(
        child: impl UiNode,
        context: &'static ContextLocal<T>,
        value: impl Into<T>,
    ) -> impl UiNode {
        let mut value = Some(Arc::new(value.into()));

        match_node(child, move |child, op| {
            context.with_context(&mut value, || child.op(op));
        })
    }
}
