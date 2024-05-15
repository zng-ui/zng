use zng::{prelude::*, prelude_wgt::*};

// Declare hot reload dynamic entry.
zng::hot_reload::zng_hot_entry!();

#[hot_node]
pub fn hot_node() -> impl UiNode {
    Text! {
        txt = "Hello, this node is hot!";
    }
}

/*
ISSUES:

* Unique ID statics are not the same in dynamic lib, need to propagate that too.
    - Use `linkme` in `zng-unique-id` to identifies all IDs, make the static a pointer?
    - Add an API to the future `HOT_LOADER` service that can register "propagation handlers" for the user
      to propagate their custom statics on init?
    -

*/
