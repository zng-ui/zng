# Pre-Merge

* Panorama fill does not change width when height change.
    - Implement special image Fill for unconstrained again, or is this another kind of fit?

* Fix all warnings.
* Pass all tests.
* Docs without warnings.

# After-Merge

* Image example panorama loading is not centered, given the size of the image.
    - Before it was centered, but not visible all the same because of the size of the image.
    - This is due to the `stack_nodes` not doing a second pass when a larger child is found.
* Button example can click disabled after adding a button.
* Review layout double-pass of stacks.
* Rename ` AnchorSize::Infinite` to Unbounded.
* Cursor demo, cursor does not clear on mouse-leave.
* Master branch TODOs.
* Fix text final size, either clip or return accurate size.
* Get Widget from UiNode (at least to avoid having to do custom reference frames in fill_node).