* Accept single reference in ContextVar binding.
* Don't use ContextVar of Var in image.
* Review IsEnabled.
* RcNode not updating in example focus -> detach.
* In the window example the `Debug Inspector` button is only showing as enabled after the UI is interacted with.
     Cause: `on_command` "enabled" closure is only run when something else updates, need to review all these kind of closures.
* In the window example the `Maximized` button is considerably slower to render the next frame than when maximizing using Windows' button.