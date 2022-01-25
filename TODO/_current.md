* Text shaping needs "Language" and "Script".
* Expand WidgetInfo to provide a RenderTransform.

* Try implementing "layers" in the window widget instead, in the renderer we are getting the parent widget context for all methods
except the render, this causes problems with context variables, like enabled.
 - Use new `SortedWidgetVec`.

* Colors don't match other apps, clear_color and background_color also does not match.