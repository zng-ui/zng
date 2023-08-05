# TextInput

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
        - VsCode copies the current line if there is no selection, how common is this?
* Research text editors.

* Review `WhiteSpace::Merge`.
    - HTML `textarea` with `white-space: normal` does not show extra spaces, but it does add the spaces if typed.
    - Implement `WhiteSpace` in `ShapedText`.
    - Can collapsed white spaces be handled like ligatures?
* Review `TextTransformFn::Uppercase`.
    - Same behavior as whitespace, only a visual change.
    - See https://drafts.csswg.org/css-text/#text-transform
    - How does `TextTransformFn::Custom` event works?
        - CSS is always the same char length?
        - Maybe when editable transform the text by grapheme?
            - User may have a find/replace transform.
        - Custom needs to be a trait that maps caret points back to the source text.
    - Maybe don't allow `WhiteSpace` and `TextTransformFn` when the text is editable.
        - Apply it when the text is changed to not editable.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

* Ctrl+Shift+I when focusing TextInput inserts a tab and still opens the Inspector.
    - We are receiving a TAB for some reason, but we are stopping propagation.
    - Char event is not linked with key press event stop propagation does nothing.
        - Is a different event from Winit.
        - The next version of Winit will fix this: https://github.com/rust-windowing/winit/issues/753´
* Pressing dead-key twice does not receive the key twice in Windows.
    - Pressing `^` + `x` in pt-BR generates two chars `^x`, but pressing `^^` does not generate the chars.

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement OpenGL example.
    - Overlay and texture image.
* Implement automation/screen reader APIs.


## WR Items

* Finish items implemented by webrender.
    - iFrame.
        - Host headless window?
            - Not exactly, similar but needs to be spawned by the window.
        - More generic possible take a `PipelineId` and give layout snapshot?
        - Primary use case is fully parallel and async rendering of a part of the screen.
            - If fully async can't really trust the full context of placement.
            - Need to transfer size and scale factor at least.
        - API change:
            - Create/destroy pipelines for window&surface.
            - iFrame display item.
        - Integration:
            - Focus nav goes through iFrames in browsers.
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
* Review `RasterSpace::Screen` usage.
    - Firefox controls this (only Screen if stacking-context is not animating).
    - https://searchfox.org/mozilla-central/source/layout/painting/nsDisplayList.cpp#6675

# Issue

* Markdown example does not scroll to title or footnote.