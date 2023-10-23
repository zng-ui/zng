# Drag&Drop TODO

* Drag/move inside window.
    - Integrate with `touch_transform`.
* Drag and drop across apps with visual feedback.
    - Visual can be a screen capture of the widget by default.
    - Browsers do this, with some fade-out mask effect and text selection clipping.
    - Winit only implements for files.
        - Even for files it is broken https://github.com/rust-windowing/winit/issues/1550