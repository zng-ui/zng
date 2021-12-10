* In the window example the `Maximized` button is considerably slower to render the next frame than when maximizing using Windows' button.
* Wait until window redraw finish before sending the `FrameRendered` event, it takes more time then we expected and is blocking a hit-test.
* The `FrameRendered` is splitting the cursor move coalesce in two.