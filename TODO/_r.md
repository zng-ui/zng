* Accept single reference in ContextVar binding.
* Turn command handle enabled into a variable.
* Don't use ContextVar of Var in image.
* Review IsEnabled.
* RcNode not updating in example focus -> detach.
* In the window example the `Debug Inspector` button is only showing as enabled after the UI is interacted with.
     Cause: `on_command` "enabled" closure is only run when something else updates, need to review all these kind of closures.
     FIXED: Changed `Focus` service to only expose state as variables, like the other services, did it by duplicating state as a prof of concept, need
     to analyze properly if we can replace the state to variables only.
* In the window example the `Maximized` button is considerably slower to render the next frame than when maximizing using Windows' button.