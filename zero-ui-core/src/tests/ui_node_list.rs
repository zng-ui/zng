use std::collections::HashSet;

use super::widget::EmptyWgt;
use crate::{
    app::App,
    context::{WidgetUpdateMode, WINDOW},
    ui_vec,
    var::ContextInitHandle,
    widget,
    widget_base::{parallel, PARALLEL_VAR},
    widget_instance::{PanelList, UiNode, UiNodeList, UiNodeVec},
};

#[test]
pub fn init_many() {
    let _app = App::minimal().run_headless(false);

    let list: Vec<_> = (0..1000)
        .map(|_| {
            EmptyWgt! {
                parallel = true;
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
    assert!(threads.len() > 1);
}

#[test]
pub fn nested_par_each_ctx() {
    let _app = App::minimal().run_headless(false);
    let mut test = ListWgt! {
        parallel = true;
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
            .collect::<UiNodeVec>();
    };

    WINDOW.with_test_context(WidgetUpdateMode::Bubble, || {
        WINDOW.test_init(&mut test);
    });
}

#[test]
pub fn par_each_ctx() {
    let _app = App::minimal().run_headless(false);
    let mut test = ListWgt! {
        parallel = true;
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
            .collect::<UiNodeVec>();
    };

    WINDOW.with_test_context(WidgetUpdateMode::Bubble, || {
        WINDOW.test_init(&mut test);
    });
}

#[widget($crate::tests::ui_node_list::ListWgt)]
pub struct ListWgt(crate::widget_base::WidgetBase);
impl ListWgt {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let child = util::list_node(wgt.capture_ui_node_list_or_empty(crate::property_id!(Self::children)));
            wgt.set_child(child);
        });
    }
}
#[crate::property(CHILD, capture, widget_impl(ListWgt))]
pub fn children(children: impl UiNodeList) {}

mod util {
    use std::thread::{self, ThreadId};

    use crate::{
        context::{with_context_local, StaticStateId, WIDGET},
        context_local, property,
        var::IntoValue,
        widget_instance::{match_node, match_node_list, UiNode, UiNodeList, UiNodeOp},
    };

    pub use super::super::widget::util::*;

    #[property(CONTEXT)]
    pub fn log_init_thread(child: impl UiNode, enabled: impl IntoValue<bool>) -> impl UiNode {
        let enabled = enabled.into();
        match_node(child, move |child, op| {
            if let UiNodeOp::Init = op {
                child.init();
                if enabled {
                    WIDGET.set_state(&INIT_THREAD_ID, thread::current().id());
                }
            }
        })
    }

    pub fn get_init_thread(wgt: &mut impl UiNode) -> ThreadId {
        wgt.with_context(|| WIDGET.get_state(&INIT_THREAD_ID).expect("did not log init thread"))
            .expect("node is not an widget")
    }

    static INIT_THREAD_ID: StaticStateId<ThreadId> = StaticStateId::new_unique();

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
}
