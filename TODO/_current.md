# TextInput

* Test clicks in, does not snap to the tips, clicking at width 70% goes to the start:
    - Problem is caused by these chars being rendered using multiple glyphs.
        - We just check the middle of a single glyph.
    - They are sort of inverse of ligatures.
```
1. ﷽

2. 𒐫

3. 𒈙

4. ⸻

5. ꧅

```

* What happens with undo if the text var is modified externally?
    - Say a text var in a "properties grid".
    - It is editable from the text and as a part of whatever is selected in the "canvas".
    - How does the undo even work in this case?
        - Review Microsoft Blend 
        - Unreal Engine.
            - Uses a global undo history.
            - Property text inputs has an undo context menu, it does nothing.


* Support replace (Insert mode in command line).
* Support buttons:
    - page up and page down
* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
* Research text editors.

* Implement custom node access to text.
    - Clone text var in `ResolvedText`?
    - Getter property `get_transformed_text`, to get the text after whitespace transforms?

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

* Ctrl+Shift+I when focusing TextInput inserts a tab and still opens the Inspector.
    - We are receiving a TAB for some reason, but we are stopping propagation.
    - Char event is not linked with key press event stop propagation does nothing.
        - Is a different event from Winit.
        - The next version of Winit will fix this: https://github.com/rust-windowing/winit/issues/753

# Gradient

* Add cool examples of radial and conic gradients.

# Undo Service

* Refactored interval to always work from the previous action.
    - Set it to the cover the key repeat interval.
    - Basically we want to undo a long press run in one go.
* Configure max and interval per-scope?
    - Can be `CowVar<u32>`, with the parent context config?

* Test undo actions trying to register undo.

- UNDO_CMD.
    - Add undo list info to the command meta?
        - For use in "UndoStack" widgets.
    - Can undo/redo.

- `UndoStack!` widget.
    - Pop-up hover down selecting (See Word widget).

# View-Process

* Implement OpenGL example.
    - Overlay and texture image.

## WR Items

* Finish items implemented by webrender.
    - Nine-patch border.
    - Backdrop filter.
    - iFrame.
    - Review why there is a `push_shadow`?
        - Can already do shadow with filters.