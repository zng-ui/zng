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
!!: ISSUES:

* `PatchKey` is not strong enough.
    - Breaks to easy, if the user adds a line break for hot reload before the static declaration it will break.
    - If the user changes the static type it will not break, attempt to patch to a different type and explode.
    - Try using `TypeId` keys, they should be the same because we are rebuilding in the same env?

* `Static#UniqueId` is not possible.
    - We use `StaticStateId` extensively.
    - **Breaking**, replace with a macro, `static_id!(static MY_ID: StateId<T>|WidgetId;)`.
    - After hot reload is mostly working.

* Hot Libraries can never unload because hot nodes can "leak" static references, in a `Txt(&static str)` for example.

*/
