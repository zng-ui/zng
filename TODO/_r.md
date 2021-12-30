* Adjust when respawn stops happening, it can enter an infinite loop in panics like the large image.
* Prevent windows from losing their maximized state when changing the screen dpi.
  - Refresh max and min sizes after screen dpi change.
* Image example panics when changing the screen dpi when web is `loading...`
    - Also happened once without it being in the `loading...` state when changing from 175% to 100%.
    - The panic message is: `thread 'main' panicked at 'assertion failed: !self.view_is_rendering()', C:\Users\Well\Desktop\New_folder\zero-ui\zero-ui-core\src\app.rs:1458:9`
* When changing the dpi upwards (tested by going from 100% to 175%) the frame is heavily scrambled.
    - Tested with the image example.
* Dragging to restore "image" example ends-up in an incorrect smaller size.
* Test events when taskbar changes position.