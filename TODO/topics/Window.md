# Windows TODO

* `WindowCloseRequestedArgs` windows list does not differentiate between headed and headless.
    - Can be confusing due to render tasks being able to set the headless window parent.
* Finish window vars.
    - Implement read-only properties?
* Drag regions.
    - Winit has `drag_window`.
* Custom resize borders.
    - Winit has `drag_resize_window`.
* Show window menu.
    - Winit has `show_window_menu`.
* Parent/child.
    - Z-order, always on-top of parent, but no focus stealing.

* Implement window close cancel when OS is shutting down.
    - Apparently this is implemented with the winapi function `ShutdownBlockReasonCreate`

* Force close windows after view-process killed by signal/Task Manager, to let app handle shutdown of app-process.

* Opening a window maximized shows two icons in taskbar (Windows 10).
