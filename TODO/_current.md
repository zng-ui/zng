# TextInput

* Implement WhiteSpace in Markdown.
* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.
        - VsCode copies the current line if there is no selection, how common is this?

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

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
