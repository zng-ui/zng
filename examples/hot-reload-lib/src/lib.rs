use zng::{prelude::*, prelude_wgt::*};

// Declare hot reload dynamic entry.
zng::hot_reload::zng_hot_entry!();

#[hot_node]
pub fn hot_node() -> impl UiNode {
    tracing::info!("`hot_node()` called");
    Text! {
        widget::on_init = hn!(|_|{
            tracing::info!("hot node on_init");
        });
        widget::on_deinit = hn!(|_|{
            tracing::info!("hot node on_deinit");
        });
        widget::background_color = colors::RED;
        txt = "Hello, this node is hot!";
    }
}

/*
!!: ISSUES:

* `Static#UniqueId` is not possible.
    - We use `StaticStateId` extensively.
    - **Breaking**, replace with a macro, `static_id!(static MY_ID: StateId<T>|WidgetId;)`.
    - After hot reload is mostly working.

* Tracing context did not bridge.

* Hot Libraries can never unload because hot nodes can "leak" static references, in a `Txt(&static str)` for example.

*/
