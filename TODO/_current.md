* Implement delivery-list/subscribers for variables.
* Implement delivery-list for raw update requests.
* Remove UiNode::subscriptions.
* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.

* Review `unsafe`, only use when there is no alternative.

* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
    - Have an AppId?
* Implement all `todo!` code.
