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

* Document that hot-libs never unload.

* Implement cancel rebuild.
    - VsCode touches the file multiple times when saving.
    - Add `HOT_RELOAD.cancel_rebuild_after` minimal time.

# Issues after merge

* Tracing context did not bridge.
* Implement panel to show status.

*/
