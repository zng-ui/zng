* Implement `Font::outline`.
* implement underline_skip GLYPHS, see `core::text::shaping::outline`.

* Review child priority integration with `WidgetLayout`, what happens when we add a border in child { }?
   - It does not work:
    - What do we lose if we remove `child_border` and `child_fill`?
    - Or do we make it work like an *anonymous* container widget, `WidgetLayout::with_widget_child` in `new_child_context`? 

# Bug

* Example does not close if:
 1 - Run Adobe Illustrator.
 2 - Run Example.
 3 - Close Illustrator.
 4 - Example does not close.

# Cause

* Illustrator causes a `RawFontChangedEvent`.
* The event causes the font references to be removed from the cache, the `text!` reference is now holding the font
  reference.
* The window close causes a `UiNode::deinit`, the `text!` drops the font reference.
* The font reference drops the render instance reference, causing a `delete_font_instance` and `delete_font` requests to be send.
* ? deadlock.