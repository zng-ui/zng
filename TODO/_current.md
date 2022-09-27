# Var/Update Rewrite

* Get `merge_var!` and `when_var!` contextualized.
* Implement "specialization" of mapping vars by returning boxed.
* Review every old var API, do we really don't need `switch_var!` and `map_ref`, the many helper mapping stuff? 
* Implement delivery-list for update requests.
    - Use it in new var API.
    - Add the current updates list to `UiNode::update` ?
* Remove `UiNode::subscriptions`.
    - Remove old var, rename `var2`, rewrite everything.
    - Tests.
* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.

* Review `unsafe`, only use when there is no alternative.

* Implement event handlers using a callback in the event that queues the handlers to run once. 
    - This avoids the linear event update search.
* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
    - Have an AppId?
* Implement all `todo!` code.

# Better Node Macro

* We really need a better way to declare nodes, some property nodes have have 20 lines of generics metadata.
    - And now they all have init/deinit to event and var handles.