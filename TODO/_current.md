# Pre-Merge

* Try to support global transforms like before in the new system, usage depending on the InfoTree is annoying and inefficient, it should be possible to
have a local in context only transform for the inner and a building transform structure that exists only in layout, the WidgetBoundsInfo just being global.
    - The layout transforms of a widget only stop receiving direct changes when the parent widget finishes layout, its global transform only after the entire
      window layout pass, so we can partially finalize widgets when its "known" ref is removed, but only globalize transforms after the window layout.
    - The inner node must get a shared reference to the inner transform.
    - So do the "child" fallback and "children" nodes.
    - We can reintroduce custom transforms too, they must be held separately and only added to the global transform.
    - Each `RenderTransform` is 64 bytes, but layout is always just translation?
        - Size is a simple PxSize, why not limit parent layout positioning to a PxVector?
        - We already assume that parent is only translate, we don't consider scale or rotate at all for sizing.
        - Can we even go a stop further and only update the global transform during render?
            - For layer decorators this may work, but services like FocusManager require a render?
    - How do we avoid visiting unchanged layout branches and have up-to-date global transform when only the parent moves.

* If we reduce the widget layout to be only offsets, and global transforms updated on render and render_update:
    - Decorators will never desync with target transform, unless a property adds a reference-frame to the display list directly.
    - Transforms live up to their name, `RenderTransform`.
    - Transform properties create their own reference-frame, no mixing accumulation with layout position.
    - Services that depend on the widget transform may en-dup one frame behind, review `FocusManager`.
        - No `FocusManager` waits render because of visibility already.
    - Render optimization may skip branches, webrender allows updating a reference-frame without trouble, but how do we update the info with global transforms?
        - We can invert the old parent transform, and apply the new one.
        - If the parent was no invertible all content was hidden, we could skip render at each transform point that is not invertible (CSS does this https://www.w3.org/TR/css-transforms-1/#transform-function-lists).
        - Problem solved, use visibility update mechanism in case of singular transform, otherwise patch all descendants.
    - Render update will need to track reference-frames also, and same consideration about optimization.
    - How to request render-update only when the bounds offset has changed?


* Layers & Custom transforms info.

* Merge master -> layout (has some scrollable updates).
* Scrollable.
    - Panorama image set to fill.

* Fix all warnings.
* Pass all tests.
* Docs without warnings.

# After-Merge

* Rename ` AnchorSize::Infinite` to Unbounded.
* Cursor demo, cursor does not clear on mouse-leave.
* Master branch TODOs.
* Fix text final size, either clip or return accurate size.
* Get Widget from UiNode (at least to avoid having to do custom reference frames in fill_node).