* Text shaping needs "Language" and "Script".
* Expand WidgetInfo to provide a RenderTransform.

* In the `layer` example the `LayerIndex::ADORNER, LayerMode::OFFSET` and `LayerMode::ALL` buttons are not producing the expected(?) result, this is more noticeable by upping the rotation of the buttons to -45.

* Try implementing "layers" in the window widget instead, in the renderer we are getting the parent widget context for all methods
except the render, this causes problems with context variables, like enabled.

* Linux does not open maximized example image some times.
* Windows does not respawn fullscreen, ends the size of the normal window, borderless.
* Return restore position selected by system when starting maximized with default position.

# Pre-Merge Review

* respawn.rs Does not respawn on the previous state.

* calculator.rs number keys not working.
* focus.rs shortcuts not working.
* focus.rs ALT+F4 "New Window" focus he menu of the main window.
* gradient.rs Cannot resize.
* stress.rs not run yet.