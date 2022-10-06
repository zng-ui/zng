# Windows TODO

* `WindowCloseRequestedArgs` windows list does not differentiate between headed and headless.
    - Can be confusing due to render tasks being able to set the headless window parent.
* Restore to Maximized from Fullscreen.
* Finish window vars.
    - Implement read-only properties?
* Drag regions.
* Custom resize borders.
* Parent/child.
    - Z-order, always on-top of parent, but no focus stealing.
* Modal.
    - Steal focus back to modal.
    - Window level "interactivity", parent window must not receive any event (other than forced close).
* Video rendering.
* Finish Monitors, needs the window target + update all layouts on change.
* Implement window close cancel when OS is shutting down.
    - Apparently this is implemented with the winapi function `ShutdownBlockReasonCreate`
* Implement direct-composition to support effects like semi-transparent blur the pixels "behind" the window.
        See: https://github.com/servo/webrender/blob/master/example-compositor/compositor/src/main.rs
* Force close windows after view-process killed by signal/Task Manager, to let app handle shutdown of app-process.
* Opening a window maximized shows two icons in taskbar (Windows 10).