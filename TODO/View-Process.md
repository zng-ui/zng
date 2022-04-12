* Implement monitor changed event.
  - when monitor changes: See WindowVars::monitor()
  - actual_monitor: Computed by intersection between window and monitors? (the monitor area that contains more than half of the window?)

* Test window respawn for NVIDIA actual driver version change (two blinks).

* Review screenshot, are we using webrender "async" screenshot correctly?

* Review/detect view <==> app-process communication deadlock that happens in some rare cases (don't know how to trigger it). This is expressed as the layout not centering/adjusting when the window is resized, as well as the window not closing when the close button is clicked.
  - This time it happened when in RenderMode::Software, once, and then didn't happen again.

* Reuse same renderer for multiple windows?

* Hosting Ui in a custom OpenGL window, like a game engine window.

* Adding custom event source that needs window raw handle.