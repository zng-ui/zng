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
        widget::background_color = rgb(255, 0, 0);
        txt = "Hello, this node is hot!";
    }
}

/*
!!: ISSUES:

* Tracing context did not bridge.
* Hot Libraries can never unload because hot nodes can "leak" static references, in a `Txt(&static str)` for example.
    - Right now we unload when the last node drops.
    - Check what happens, access violation:
* Document that statics can't be modified in hot code too.

*/
