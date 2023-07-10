# Panel Index

* Grid implements `column::get_index` and `row::get_index` other panels should implement something similar?
* `stack::get_index` and `wrap::get_index` should be exactly the same.
    - We need to use widget state for grid, but do we need for these two?
    - Test implementation done for `stack`.
        - Don't like how it is always set and we don't even use it in any of the stack and wraps so far.
    - Can we just use the info tree?
        - If a widget is set as background for the `Stack!` it counts as an info child.
        - If there are non-widget nodes in the children list they don't increment the count.
            - Right now we can't probe from a non-widget node anyway, but they are skipped in the count.
            - Ok with changing this.
        - We could mark the first and last children in the info tree meta.
            - Could do this in the `PanelList`?
                - We only use one panel list per panel right now, this may change?
                - Maybe an option in the panel list, that defines the state ID.
                - `PanelList::children_range(&self) -> StaticStateId<PanelListRange { first: WidgetId, last: WidgetId, version: u32 }>`.
                - This reduces the cost to just one extra info, only adds cost if getter properties are set.
                - The getter properties don't need to be set on the child directly too?
                    - Only if we can refactor `grid::column::get_index` to work the same.
                - Still need to declare the properties for each panel.
                    - Declare some helpers nodes to do it.
                    - Maybe also a macro?
                        - They only need to be declared in the panel module.
                        - They maybe associated with a child widget.
                            - Like `grid::Cell`?
                        - Stack `get_index` also added an example, lets not do a macro.

# TextInput

* Caret not always positioned at end of wrapped line.
    - Include a line_index in the caret position.

* Large single word does not wrap (it wraps for a bit then it becomes a single line again).
* Support replace (Insert mode in command line).
* Implement scroll integration:
    - scroll to caret
* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
* Research text editors.

* Review `WhiteSpace::Merge`.
    - HTML `textarea` with `white-space: normal` does not show extra spaces, but it does add the spaces if typed.
* Review `TextTransformFn::Uppercase`.
    - Same behavior as whitespace, only a visual change.
    - See https://drafts.csswg.org/css-text/#text-transform
    - How does `TextTransformFn::Custom` event works?
        - CSS is always the same char length?
        - Maybe when editable transform the text by grapheme?
            - User may have a find/replace transform.
        - Custom needs to be a trait that maps caret points back to the source text.
* Getter property `get_transformed_txt`, to get the text after whitespace & transforms?
    - Transformed should be in the SegmentedText already.
    - Whitespace needs processing.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

* Ctrl+Shift+I when focusing TextInput inserts a tab and still opens the Inspector.
    - We are receiving a TAB for some reason, but we are stopping propagation.
    - Char event is not linked with key press event stop propagation does nothing.
        - Is a different event from Winit.
        - The next version of Winit will fix this: https://github.com/rust-windowing/winit/issues/753

* Spellchecker.
    - Use https://docs.rs/hunspell-rs
    - Used by Firefox and Chrome.

# Gradient

* Add cool examples of radial and conic gradients.

# Undo Service

* `UndoHistory!` widget.
    - Property to select the stack?
    - Need to support pop-up hover down selecting (See Word widget).
        - Need a `HOVERED_TIMESTAMP_VAR`?
        - To highlight the covered entries.

# View-Process

* Implement OpenGL example.
    - Overlay and texture image.

## WR Items

* Finish items implemented by webrender.
    - Nine-patch border.
        - Can accept gradients as "image".
        - CSS does not support corner radius for this.
            - We could clip the border for the user.
    - Backdrop filter.
    - iFrame.
    - 3D transforms.
        - "transform-style".
            - Is flat by default?
            - If yes, we may not want to implement the other.
            - The user should use sibling widgets to preserve-3d.
        - rotate_x.
        - Perspective.
            - These are just matrix API and testing.
        - Backface vis.
    - Touch events.
        - Use `Spacedesk` to generate touch events.