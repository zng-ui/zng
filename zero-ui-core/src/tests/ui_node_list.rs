use std::collections::HashSet;

use super::widget::empty_wgt;
use crate::{
    app::App,
    context::WINDOW,
    widget_base::parallel,
    widget_instance::{PanelList, UiNode, UiNodeList},
};

#[test]
pub fn init_many() {
    let _app = App::minimal().run_headless(false);

    let list: Vec<_> = (0..1000)
        .map(|_| {
            empty_wgt! {
                parallel = true;
                util::trace = "inited";
                util::log_init_thread = true;
            }
            .boxed()
        })
        .collect();
    let mut list = PanelList::new(list);

    WINDOW.with_test_context(|| {
        list.init_all();
    });

    let mut count = 0;
    let mut threads = HashSet::new();
    list.for_each(|i, wgt, _| {
        assert!(util::traced(wgt, "inited"));
        assert_eq!(count, i);
        count += 1;
        threads.insert(util::get_init_thread(wgt));
        true
    });
    assert_eq!(count, 1000);
    assert!(threads.len() > 1);
}

mod util {
    use std::thread::ThreadId;

    use crate::{
        context::{StaticStateId, WIDGET},
        property, ui_node,
        var::IntoValue,
        widget_instance::UiNode,
    };

    pub use super::super::widget::util::*;

    #[property(CONTEXT)]
    pub fn log_init_thread(child: impl UiNode, enabled: impl IntoValue<bool>) -> impl UiNode {
        #[ui_node(struct ThreadNode {
            child: impl UiNode,
            enabled: bool,
        })]
        impl UiNode for ThreadNode {
            fn init(&mut self) {
                self.child.init();
                if self.enabled {
                    WIDGET.set_state(&INIT_THREAD_ID, std::thread::current().id());
                }
            }
        }
        ThreadNode {
            child,
            enabled: enabled.into(),
        }
    }

    pub fn get_init_thread(wgt: &impl UiNode) -> ThreadId {
        wgt.with_context(|| WIDGET.get_state(&INIT_THREAD_ID).expect("did not log init thread"))
            .expect("node is not an widget")
    }

    static INIT_THREAD_ID: StaticStateId<ThreadId> = StaticStateId::new_unique();
}
