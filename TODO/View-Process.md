* Implement monitor changed event.
  - when monitor changes: See WindowVars::monitor()
  - actual_monitor: Computed by intersection between window and monitors? (the monitor area that contains more than half of the window?)

* Implement and test window respawn (need to test NVIDIA actual driver version change (two blinks)).
* Implement software rendering using https://github.com/servo/webrender/tree/master/swgl
  - Test integrated, Intel Graphics not working with glutin?
  - Winit only, no OpenGL? Should speedup startup for software only.
  - Headless software does not need a native backend.
  - Test OpenGl 1.1, virtual machines.

* Review screenshot, are we using webrender "async" screenshot correctly?