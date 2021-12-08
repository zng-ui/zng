* Context-var needs to handle recursion, it can happen accidentally and we use explicitly to define property default values.

* Turn command handle enabled into a variable.
* Don't use ContextVar of Var in image.
* Review IsEnabled.
* Review EventArgs, should we target a specific path?.
* Sub-divide UiNodeList masks.
* In the window example the `Maximized` button is considerably slower to render the next frame than when maximizing using Windows' button.