* Text shaping needs "Language" and "Script".
* Expand WidgetInfo to provide a RenderTransform.

* In the `layer` example the `LayerIndex::ADORNER, LayerMode::OFFSET` and `LayerMode::ALL` buttons are not producing the expected(?) result, this is more noticeable by upping the rotation of the buttons to -45.

* Try implementing "layers" in the window widget instead, in the renderer we are getting the parent widget context for all methods
except the render, this causes problems with context variables, like enabled.

* Linux does not open maximized example image some times.
* Windows does not respawn fullscreen, ends the size of the normal window, borderless.
* Return restore position selected by system when starting maximized with default position.

# String format style
* Simplify string formattings like `"{:?}", ident` to `"{ident:?}"`, now that the second style is stabilized for idents with Rust v1.58.0.
* To that end use this regex: `"[^"]*\{[^"]*\}[^"]*"`
* Since that regex still applies to the new style, update the files in the order the search functions lists them and take note of the next file that still needs to be updated here:
  * stopped before `crate_util.rs in zero-ui-core\src`, 70 files to go.
