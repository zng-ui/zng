# Windows TODO

* Restore to Maximized from Fullscreen.
* Finish window vars.
* Drag regions.
* Custom resize borders.
* Parent/child.
* Modal.
* Video rendering.
* Finish Monitors, needs the window target + update all layouts on change.
* Implement window close cancel when OS is shutting down.
    - Apparently this is implemented with the winapi function `ShutdownBlockReasonCreate`
* Implement direct-composition to support effects like semi-transparent blur the pixels "behind" the window.
        See: https://github.com/servo/webrender/blob/master/example-compositor/compositor/src/main.rs
* Force close windows after view-process killed by signal/Task Manager, to let app handle shutdown of app-process.