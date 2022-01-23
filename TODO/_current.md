* Text shaping needs "Language" and "Script".
* Expand WidgetInfo to provide a RenderTransform.

* In the `layer` example the `LayerIndex::ADORNER, LayerMode::OFFSET` and `LayerMode::ALL` buttons are not producing the expected(?) result, this is more noticeable by upping the rotation of the buttons to -45.

* Try implementing "layers" in the window widget instead, in the renderer we are getting the parent widget context for all methods
except the render, this causes problems with context variables, like enabled.

* Linux does not open maximized example image some times.
* Window does not restore to fullscreen from minimized.
* Colors don't match other apps, clear_color and background_color also does not match.