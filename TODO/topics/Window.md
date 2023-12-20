# Windows TODO

* `WindowCloseRequestedArgs` windows list does not differentiate between headed and headless.
    - Can be confusing due to render tasks being able to set the headless window parent.
* Restore to Maximized from Fullscreen.
* Finish window vars.
    - Implement read-only properties?
* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?
* Drag regions.
    - Winit has `drag_window`.
* Custom resize borders.
    - Winit has `drag_resize_window`.
* Show window menu.
    - Winit has `show_window_menu`.
* Parent/child.
    - Z-order, always on-top of parent, but no focus stealing.
* Video rendering.
* Finish Monitors, needs the window target + update all layouts on change.
* Implement window close cancel when OS is shutting down.
    - Apparently this is implemented with the winapi function `ShutdownBlockReasonCreate`
* Implement direct-composition to support effects like semi-transparent blur the pixels "behind" the window.
        See: https://github.com/servo/webrender/blob/master/example-compositor/compositor/src/main.rs
* Force close windows after view-process killed by signal/Task Manager, to let app handle shutdown of app-process.
* Opening a window maximized shows two icons in taskbar (Windows 10).

* Mac window icon.
    - Only has app icon: https://developer.apple.com/documentation/appkit/nsapplication/1428744-applicationiconimage.
    - Maybe we can always use the last set icon, except if set to `None` it falls-back to some icon from an older open window.